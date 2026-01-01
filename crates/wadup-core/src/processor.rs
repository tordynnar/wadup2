use anyhow::Result;
use std::thread;
use crossbeam_deque::{Worker, Stealer, Steal};
use crate::content::{Content, ContentData, ContentStore};
use crate::wasm::{WasmRuntime, ModuleInstance};
use crate::metadata::MetadataStore;
use crate::bindings_context::SubContentData;

pub struct ContentProcessor {
    runtime: WasmRuntime,
    metadata_store: MetadataStore,
    max_recursion_depth: usize,
}

impl ContentProcessor {
    pub fn new(
        runtime: WasmRuntime,
        metadata_store: MetadataStore,
        max_recursion_depth: usize,
    ) -> Self {
        Self {
            runtime,
            metadata_store,
            max_recursion_depth,
        }
    }

    pub fn process(&self, initial_contents: Vec<Content>, num_threads: usize) -> Result<()> {
        tracing::info!("Starting processing with {} threads", num_threads);
        tracing::info!("Initial content count: {}", initial_contents.len());
        tracing::info!("Max recursion depth: {}", self.max_recursion_depth);

        let content_store = ContentStore::new();

        // Store initial content data
        for content in &initial_contents {
            if let ContentData::Owned(data) = &content.data {
                content_store.insert(content.uuid, data.clone());
            }
        }

        // Create work queues
        let mut workers = Vec::new();
        let mut stealers = Vec::new();

        for _ in 0..num_threads {
            let worker = Worker::new_fifo();
            stealers.push(worker.stealer());
            workers.push(worker);
        }

        // Add initial contents to first worker
        if !workers.is_empty() {
            for content in initial_contents {
                workers[0].push(content);
            }
        }

        // Spawn worker threads
        let mut handles = Vec::new();

        for (thread_id, worker) in workers.into_iter().enumerate() {
            let thread_stealers: Vec<Stealer<Content>> = stealers.iter()
                .enumerate()
                .filter(|(i, _)| *i != thread_id)
                .map(|(_, s)| s.clone())
                .collect();

            let content_store = content_store.clone();
            let metadata_store = self.metadata_store.clone();
            let max_recursion_depth = self.max_recursion_depth;

            // Create module instances for this thread
            let instances = self.runtime.create_instances(metadata_store.clone())?;

            let handle = thread::spawn(move || -> Result<()> {
                let mut worker_thread = WorkerThread {
                    id: thread_id,
                    worker,
                    stealers: thread_stealers,
                    content_store,
                    metadata_store,
                    max_recursion_depth,
                    instances,
                };

                worker_thread.run()
            });

            handles.push(handle);
        }

        // Wait for all threads to complete
        for (i, handle) in handles.into_iter().enumerate() {
            match handle.join() {
                Ok(Ok(())) => {
                    tracing::debug!("Worker thread {} completed successfully", i);
                }
                Ok(Err(e)) => {
                    tracing::error!("Worker thread {} failed: {}", i, e);
                    return Err(e);
                }
                Err(_) => {
                    anyhow::bail!("Worker thread {} panicked", i);
                }
            }
        }

        tracing::info!("Processing complete");
        Ok(())
    }
}

struct WorkerThread {
    id: usize,
    worker: Worker<Content>,
    stealers: Vec<Stealer<Content>>,
    content_store: ContentStore,
    metadata_store: MetadataStore,
    max_recursion_depth: usize,
    instances: Vec<ModuleInstance>,
}

impl WorkerThread {
    fn run(&mut self) -> Result<()> {
        let mut processed_count = 0;

        loop {
            let content = match self.get_work() {
                Some(c) => c,
                None => break,
            };

            match self.process_content(content) {
                Ok(()) => {
                    processed_count += 1;
                }
                Err(e) => {
                    tracing::error!("Failed to process content: {}", e);
                }
            }
        }

        tracing::debug!("Worker {} processed {} items", self.id, processed_count);
        Ok(())
    }

    fn get_work(&self) -> Option<Content> {
        // Try local queue first (LIFO for depth-first)
        if let Some(content) = self.worker.pop() {
            return Some(content);
        }

        // Try stealing from others (FIFO from their bottom)
        loop {
            let mut retry = false;
            let mut found = None;

            for stealer in &self.stealers {
                match stealer.steal() {
                    Steal::Success(content) => {
                        found = Some(content);
                        break;
                    }
                    Steal::Empty => continue,
                    Steal::Retry => {
                        retry = true;
                    }
                }
            }

            if let Some(content) = found {
                return Some(content);
            }

            if !retry {
                break;
            }
        }

        None
    }

    fn process_content(&mut self, content: Content) -> Result<()> {
        tracing::debug!(
            "Worker {} processing content: {} (depth: {})",
            self.id,
            content.filename,
            content.depth
        );

        // Resolve content data
        let data = self.content_store.resolve(&content)
            .ok_or_else(|| anyhow::anyhow!("Content data not found for UUID: {}", content.uuid))?;

        // Store in content store if owned
        if let ContentData::Owned(ref owned_data) = content.data {
            self.content_store.insert(content.uuid, owned_data.clone());
        }

        // Record content in database FIRST (before processing) so foreign keys work
        let parent_uuid_str = content.parent_uuid.map(|u| u.to_string());
        let parent_uuid_ref = parent_uuid_str.as_deref();

        // Insert a "processing" placeholder that we'll update later
        self.metadata_store.record_content_success(
            &content.uuid.to_string(),
            &content.filename,
            parent_uuid_ref,
        )?;

        let mut all_subcontent = Vec::new();
        let mut processing_errors = Vec::new();

        // Process through each module
        for instance in &mut self.instances {
            match instance.process_content(content.uuid, data.clone()) {
                Ok(ctx) => {
                    // First, define any tables requested by the module
                    for table_schema in &ctx.table_schemas {
                        if let Err(e) = instance.metadata_store().define_table(table_schema.clone()) {
                            tracing::warn!(
                                "Failed to define table '{}' for module '{}': {}",
                                table_schema.name,
                                instance.name(),
                                e
                            );
                        }
                    }

                    // Handle metadata
                    for metadata_row in &ctx.metadata {
                        if let Err(e) = instance.metadata_store().insert_row(
                            &metadata_row.table_name,
                            &content.uuid.to_string(),
                            &metadata_row.values,
                        ) {
                            tracing::warn!(
                                "Failed to insert row for module '{}': {}",
                                instance.name(),
                                e
                            );
                        }
                    }

                    // Record module stdout/stderr output
                    if let Err(e) = self.metadata_store.record_module_output(
                        &content.uuid.to_string(),
                        instance.name(),
                        ctx.stdout.as_deref(),
                        ctx.stderr.as_deref(),
                        ctx.stdout_truncated,
                        ctx.stderr_truncated,
                    ) {
                        tracing::warn!(
                            "Failed to record module output for '{}': {}",
                            instance.name(),
                            e
                        );
                    }

                    // Collect sub-content
                    all_subcontent.extend(ctx.subcontent);
                }
                Err(e) => {
                    let error_msg = format!("Module '{}' failed: {}", instance.name(), e);
                    tracing::warn!("{}", error_msg);
                    processing_errors.push(error_msg);
                }
            }
        }

        // Record content processing result
        let parent_uuid_str = content.parent_uuid.map(|u| u.to_string());
        let parent_uuid_ref = parent_uuid_str.as_deref();

        if processing_errors.is_empty() {
            self.metadata_store.record_content_success(
                &content.uuid.to_string(),
                &content.filename,
                parent_uuid_ref,
            )?;
        } else {
            let error_summary = processing_errors.join("; ");
            self.metadata_store.record_content_failure(
                &content.uuid.to_string(),
                &content.filename,
                parent_uuid_ref,
                &error_summary,
            )?;
        }

        // Process sub-content (depth-first)
        for subcontent_emission in all_subcontent {
            let subcontent_data = match subcontent_emission.data {
                SubContentData::Bytes(bytes) => {
                    // Zero-copy: SharedBuffer wraps the Bytes directly
                    let buffer = crate::shared_buffer::SharedBuffer::from_bytes(bytes);
                    let uuid = uuid::Uuid::new_v4();
                    self.content_store.insert(uuid, buffer.clone());
                    ContentData::Owned(buffer)
                }
                SubContentData::Slice { offset, length } => {
                    ContentData::Borrowed {
                        parent_uuid: content.uuid,
                        offset,
                        length,
                    }
                }
            };

            match Content::new_subcontent(
                &content,
                subcontent_data,
                subcontent_emission.filename,
                self.max_recursion_depth,
            ) {
                Ok(subcontent) => {
                    tracing::debug!(
                        "Worker {} enqueuing sub-content: {} (depth: {})",
                        self.id,
                        subcontent.filename,
                        subcontent.depth
                    );
                    self.worker.push(subcontent);
                }
                Err(e) => {
                    tracing::warn!("Failed to create sub-content: {}", e);
                }
            }
        }

        Ok(())
    }
}
