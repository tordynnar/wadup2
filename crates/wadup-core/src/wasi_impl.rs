use crate::memory_fs::{MemoryFilesystem, MemoryFile, MemoryDirectory};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use std::io::{Read, Write, Seek, SeekFrom};

/// File descriptor
type Fd = u32;

/// WASI file types
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum Filetype {
    Unknown = 0,
    BlockDevice = 1,
    CharacterDevice = 2,
    Directory = 3,
    RegularFile = 4,
    SocketDgram = 5,
    SocketStream = 6,
    SymbolicLink = 7,
}

/// WASI errno values
#[repr(u16)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Errno {
    Success = 0,
    TooBig = 1,
    Acces = 2,
    Again = 6,
    Badf = 8,
    Exist = 20,
    Inval = 28,
    Io = 29,
    Isdir = 31,
    Noent = 44,
    Notdir = 54,
    Nosys = 52,
}

/// Open file handle
enum FileHandle {
    File(MemoryFile),
    Directory(MemoryDirectory, usize), // directory + readdir position
    Stdin,
    Stdout,
    Stderr,
}

/// WASI context with in-memory filesystem
pub struct WasiCtx {
    pub filesystem: Arc<MemoryFilesystem>,
    file_table: Arc<RwLock<HashMap<Fd, FileHandle>>>,
    next_fd: Arc<RwLock<Fd>>,
}

impl WasiCtx {
    pub fn new(filesystem: Arc<MemoryFilesystem>) -> Self {
        let mut file_table = HashMap::new();

        // Reserve FDs for stdio
        file_table.insert(0, FileHandle::Stdin);
        file_table.insert(1, FileHandle::Stdout);
        file_table.insert(2, FileHandle::Stderr);
        // FD 3 is reserved for the preopened root directory
        file_table.insert(3, FileHandle::Directory(filesystem.root().clone(), 0));

        Self {
            filesystem,
            file_table: Arc::new(RwLock::new(file_table)),
            next_fd: Arc::new(RwLock::new(4)),
        }
    }

    fn allocate_fd(&self) -> Fd {
        let mut next = self.next_fd.write();
        let fd = *next;
        *next += 1;
        fd
    }

    /// path_open - Open a file or directory
    ///
    /// oflags bits:
    /// - bit 0: O_CREAT - create file if it doesn't exist
    /// - bit 1: O_DIRECTORY - expect a directory
    /// - bit 2: O_EXCL - error if file exists when O_CREAT is set
    /// - bit 3: O_TRUNC - truncate file to 0 on open
    pub fn path_open(
        &self,
        dirfd: Fd,
        _dirflags: u32,
        path: &str,
        oflags: u16,
        _fs_rights_base: u64,
        _fs_rights_inheriting: u64,
        _fdflags: u16,
        fd_out: &mut Fd,
    ) -> Errno {
        // For now, only support opening from root directory (FD 3)
        if dirfd != 3 {
            return Errno::Badf;
        }

        let o_creat = (oflags & 1) != 0;
        let o_directory = (oflags & 2) != 0;
        let o_excl = (oflags & 4) != 0;
        let _o_trunc = (oflags & 8) != 0; // TODO: implement truncation

        // If O_DIRECTORY is set, only open as directory
        if o_directory {
            let (parent_dir, filename) = match self.resolve_path(path) {
                Ok(v) => v,
                Err(e) => return e,
            };

            match parent_dir.get_dir(&filename) {
                Ok(dir) => {
                    let new_fd = self.allocate_fd();
                    self.file_table.write().insert(new_fd, FileHandle::Directory(dir, 0));
                    *fd_out = new_fd;
                    Errno::Success
                }
                Err(_) => Errno::Noent,
            }
        } else {
            // Try to open as file first
            match self.filesystem.open_file(path) {
                Ok(file) => {
                    if o_excl && o_creat {
                        // O_EXCL with O_CREAT means error if file exists
                        return Errno::Exist;
                    }
                    let new_fd = self.allocate_fd();
                    self.file_table.write().insert(new_fd, FileHandle::File(file));
                    *fd_out = new_fd;
                    Errno::Success
                }
                Err(_) => {
                    // File doesn't exist
                    if o_creat {
                        // Create new file
                        match self.filesystem.create_file(path, Vec::new()) {
                            Ok(_) => {
                                match self.filesystem.open_file(path) {
                                    Ok(file) => {
                                        let new_fd = self.allocate_fd();
                                        self.file_table.write().insert(new_fd, FileHandle::File(file));
                                        *fd_out = new_fd;
                                        Errno::Success
                                    }
                                    Err(_) => Errno::Io,
                                }
                            }
                            Err(_) => Errno::Io,
                        }
                    } else {
                        // Try as directory
                        let (parent_dir, filename) = match self.resolve_path(path) {
                            Ok(v) => v,
                            Err(e) => return e,
                        };

                        match parent_dir.get_dir(&filename) {
                            Ok(dir) => {
                                let new_fd = self.allocate_fd();
                                self.file_table.write().insert(new_fd, FileHandle::Directory(dir, 0));
                                *fd_out = new_fd;
                                Errno::Success
                            }
                            Err(_) => Errno::Noent,
                        }
                    }
                }
            }
        }
    }

    /// fd_read - Read from file descriptor
    pub fn fd_read(&self, fd: Fd, bufs: &mut [&mut [u8]], nread_out: &mut usize) -> Errno {
        let mut file_table = self.file_table.write();

        let handle = match file_table.get_mut(&fd) {
            Some(h) => h,
            None => return Errno::Badf,
        };

        match handle {
            FileHandle::File(ref mut file) => {
                let mut total = 0;
                for buf in bufs {
                    match file.read(buf) {
                        Ok(n) => total += n,
                        Err(_) => return Errno::Io,
                    }
                }
                *nread_out = total;
                Errno::Success
            }
            FileHandle::Stdin => {
                // For now, stdin returns empty
                *nread_out = 0;
                Errno::Success
            }
            _ => Errno::Badf,
        }
    }

    /// fd_write - Write to file descriptor
    pub fn fd_write(&self, fd: Fd, bufs: &[&[u8]], nwritten_out: &mut usize) -> Errno {
        let mut file_table = self.file_table.write();

        let handle = match file_table.get_mut(&fd) {
            Some(h) => h,
            None => return Errno::Badf,
        };

        match handle {
            FileHandle::File(ref mut file) => {
                let mut total = 0;
                for buf in bufs {
                    match file.write(buf) {
                        Ok(n) => total += n,
                        Err(_) => return Errno::Io,
                    }
                }
                *nwritten_out = total;
                Errno::Success
            }
            FileHandle::Stdout | FileHandle::Stderr => {
                // Write to actual stdout/stderr
                let mut total = 0;
                for buf in bufs {
                    if fd == 1 {
                        match std::io::stdout().write(buf) {
                            Ok(n) => total += n,
                            Err(_) => return Errno::Io,
                        }
                    } else {
                        match std::io::stderr().write(buf) {
                            Ok(n) => total += n,
                            Err(_) => return Errno::Io,
                        }
                    }
                }
                *nwritten_out = total;
                Errno::Success
            }
            _ => Errno::Badf,
        }
    }

    /// fd_seek - Seek in file
    pub fn fd_seek(&self, fd: Fd, offset: i64, whence: u8, newoffset_out: &mut u64) -> Errno {
        let mut file_table = self.file_table.write();

        let handle = match file_table.get_mut(&fd) {
            Some(h) => h,
            None => return Errno::Badf,
        };

        if let FileHandle::File(ref mut file) = handle {
            let seek_from = match whence {
                0 => SeekFrom::Start(offset as u64),
                1 => SeekFrom::Current(offset),
                2 => SeekFrom::End(offset),
                _ => return Errno::Inval,
            };

            match file.seek(seek_from) {
                Ok(pos) => {
                    *newoffset_out = pos;
                    Errno::Success
                }
                Err(_) => Errno::Io,
            }
        } else {
            Errno::Badf
        }
    }

    /// fd_close - Close file descriptor
    pub fn fd_close(&self, fd: Fd) -> Errno {
        if fd <= 2 {
            // Don't close stdio
            return Errno::Success;
        }

        let mut file_table = self.file_table.write();
        if file_table.remove(&fd).is_some() {
            Errno::Success
        } else {
            Errno::Badf
        }
    }

    /// fd_filestat_get - Get file metadata
    pub fn fd_filestat_get(&self, fd: Fd, filestat: &mut [u8; 64]) -> Errno {
        let file_table = self.file_table.read();

        let handle = match file_table.get(&fd) {
            Some(h) => h,
            None => return Errno::Badf,
        };

        // Clear the filestat buffer
        filestat.fill(0);

        match handle {
            FileHandle::File(file) => {
                // Set filetype to regular file (byte 16)
                filestat[16] = Filetype::RegularFile as u8;
                // Set file size (bytes 32-39, little endian)
                let size = file.len() as u64;
                filestat[32..40].copy_from_slice(&size.to_le_bytes());
                Errno::Success
            }
            FileHandle::Directory(_, _) => {
                filestat[16] = Filetype::Directory as u8;
                Errno::Success
            }
            _ => Errno::Success,
        }
    }

    /// fd_prestat_get - Get preopen info
    pub fn fd_prestat_get(&self, fd: Fd, prestat_out: &mut [u8; 8]) -> Errno {
        if fd != 3 {
            return Errno::Badf;
        }

        // Type 0 = directory
        prestat_out[0] = 0;
        // Name length = 1 (for "/")
        prestat_out[4] = 1;
        Errno::Success
    }

    /// fd_prestat_dir_name - Get preopen directory name
    pub fn fd_prestat_dir_name(&self, fd: Fd, path_buf: &mut [u8]) -> Errno {
        if fd != 3 {
            return Errno::Badf;
        }

        if path_buf.is_empty() {
            return Errno::Inval;
        }

        path_buf[0] = b'/';
        Errno::Success
    }

    /// path_filestat_get - Get file metadata by path
    pub fn path_filestat_get(
        &self,
        dirfd: Fd,
        _flags: u32,
        path: &str,
        filestat: &mut [u8; 64],
    ) -> Errno {
        if dirfd != 3 {
            return Errno::Badf;
        }

        filestat.fill(0);

        // Try to open as file
        match self.filesystem.open_file(path) {
            Ok(file) => {
                filestat[16] = Filetype::RegularFile as u8;
                let size = file.len() as u64;
                filestat[32..40].copy_from_slice(&size.to_le_bytes());
                Errno::Success
            }
            Err(_) => {
                // Try as directory
                match self.resolve_path(path) {
                    Ok((parent_dir, filename)) => {
                        if parent_dir.get_dir(&filename).is_ok() {
                            filestat[16] = Filetype::Directory as u8;
                            Errno::Success
                        } else {
                            Errno::Noent
                        }
                    }
                    Err(e) => e,
                }
            }
        }
    }

    /// fd_readdir - Read directory entries
    pub fn fd_readdir(
        &self,
        fd: Fd,
        buf: &mut [u8],
        _cookie: u64,
        bufused_out: &mut usize,
    ) -> Errno {
        let mut file_table = self.file_table.write();

        let handle = match file_table.get_mut(&fd) {
            Some(h) => h,
            None => return Errno::Badf,
        };

        if let FileHandle::Directory(dir, ref mut pos) = handle {
            let entries = dir.list();

            let mut offset = 0;
            let start_pos = *pos;

            for (idx, (name, is_dir)) in entries.iter().enumerate().skip(start_pos) {
                // dirent structure: next(8) + ino(8) + namelen(4) + type(1)
                let entry_size = 8 + 8 + 4 + 1 + name.len();

                if offset + entry_size > buf.len() {
                    break;
                }

                // next cookie
                let next = (idx + 1) as u64;
                buf[offset..offset+8].copy_from_slice(&next.to_le_bytes());
                offset += 8;

                // inode (fake)
                let ino = (idx + 1) as u64;
                buf[offset..offset+8].copy_from_slice(&ino.to_le_bytes());
                offset += 8;

                // name length
                let namelen = name.len() as u32;
                buf[offset..offset+4].copy_from_slice(&namelen.to_le_bytes());
                offset += 4;

                // file type
                let filetype = if *is_dir {
                    Filetype::Directory as u8
                } else {
                    Filetype::RegularFile as u8
                };
                buf[offset] = filetype;
                offset += 1;

                // name
                buf[offset..offset+name.len()].copy_from_slice(name.as_bytes());
                offset += name.len();

                *pos = idx + 1;
            }

            *bufused_out = offset;
            Errno::Success
        } else {
            Errno::Notdir
        }
    }

    fn resolve_path(&self, path: &str) -> Result<(MemoryDirectory, String), Errno> {
        let path = path.trim_start_matches('/');

        if path.is_empty() {
            return Err(Errno::Inval);
        }

        let parts: Vec<&str> = path.split('/').collect();
        let filename = parts.last().unwrap().to_string();

        if parts.len() == 1 {
            return Ok((self.filesystem.root().clone(), filename));
        }

        let mut current_dir = self.filesystem.root().clone();
        for &part in &parts[..parts.len() - 1] {
            current_dir = current_dir.get_dir(part).map_err(|_| Errno::Noent)?;
        }

        Ok((current_dir, filename))
    }
}
