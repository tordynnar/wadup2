// Stub implementations for POSIX functions that Python expects
// but are not available in WASI
// These are no-op or minimal stubs

// Signal handler function type
typedef void (*sighandler_t)(int);

// Process ID type
typedef int pid_t;

// Stub signal handler functions
__attribute__((visibility("default")))
void __SIG_DFL(int sig) {
    // No-op: Default signal handler stub
    (void)sig;
}

__attribute__((visibility("default")))
void __SIG_IGN(int sig) {
    // No-op: Ignore signal handler stub
    (void)sig;
}

__attribute__((visibility("default")))
void __SIG_ERR(int sig) {
    // No-op: Error signal handler stub
    (void)sig;
}

// Stub signal() function - just returns the handler without doing anything
// since WASI doesn't support signals
__attribute__((visibility("default")))
sighandler_t signal(int signum, sighandler_t handler) {
    // No-op: Just return the handler to indicate "success"
    (void)signum;
    return handler;
}

// Stub getpid() function - returns a fixed PID since WASI doesn't have processes
__attribute__((visibility("default")))
pid_t getpid(void) {
    // Return a fixed PID value (1) since WASI doesn't have meaningful PIDs
    return 1;
}

// Stub clock() function - returns a dummy value
// Note: Python expects this to return long long (i64) not long (i32)
__attribute__((visibility("default")))
long long clock(void) {
    // Return 0 to indicate no time has passed
    return 0;
}

// Stub raise() function - signals not supported in WASI
__attribute__((visibility("default")))
int raise(int sig) {
    // No-op: Just return 0 to indicate success
    (void)sig;
    return 0;
}

// Times structure for times() stub
struct tms {
    long tms_utime;
    long tms_stime;
    long tms_cutime;
    long tms_cstime;
};

// Stub times() function - process times not available in WASI
// Note: Python expects this to return long long (i64) not long (i32)
__attribute__((visibility("default")))
long long times(struct tms *buf) {
    // Fill with zeros
    if (buf) {
        buf->tms_utime = 0;
        buf->tms_stime = 0;
        buf->tms_cutime = 0;
        buf->tms_cstime = 0;
    }
    return 0;
}

// Stub strsignal() function - returns signal description
__attribute__((visibility("default")))
char* strsignal(int sig) {
    // Return a static error message
    (void)sig;
    return "Signal not supported in WASI";
}

// Stub dynamic linking functions - WASI doesn't support dynamic loading
__attribute__((visibility("default")))
void* dlopen(const char* filename, int flags) {
    // Return NULL to indicate failure
    (void)filename;
    (void)flags;
    return (void*)0;
}

__attribute__((visibility("default")))
void* dlsym(void* handle, const char* symbol) {
    // Return NULL to indicate symbol not found
    (void)handle;
    (void)symbol;
    return (void*)0;
}

__attribute__((visibility("default")))
int dlclose(void* handle) {
    // Return 0 to indicate success (no-op)
    (void)handle;
    return 0;
}

__attribute__((visibility("default")))
char* dlerror(void) {
    // Return a static error message
    return "Dynamic loading not supported in WASI";
}
