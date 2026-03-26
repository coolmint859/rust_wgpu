#![allow(dead_code)]
use std::{collections::HashMap, hash::Hash, sync::mpsc, time::Instant};

/// Represents the state of a resource requested by the user of a registry instance.
pub enum ResourceStatus<R> {
    /// Resource has been requested but is not yet ready
    Pending(Instant),

    /// Resource is ready for retrieval
    Ready(R),

    /// Resource failed to complete
    Failed(String),
}

impl<R> ResourceStatus<R> {
    /// Retrieve the time the resource was added if still pending creation
    fn creation_time(&self) -> Option<&Instant>{
        match self {
            ResourceStatus::Pending(time) => Some(time),
            _ => None
        }
    }

    /// Retreive a reference to the stored resource if available.
    fn value(&self) -> Option<&R> {
        match self {
            ResourceStatus::Ready(resource) => Some(resource),
            _ => None
        }
    }

    /// retreive the error message if the stored resource failed to complete.
    fn error_msg(&self) -> Option<&str> {
        match self {
            ResourceStatus::Failed(err) => Some(err.as_str()),
            _ => None
        }
    }
}

/// Manages and stores memory resources with concurrent creation and access.
/// 
/// TODO: force_request() and update() with prior thread cancelation (use thread JoinHandle)
pub struct ResourceRegistry<K, R> {
    storage_map: HashMap<K, ResourceStatus<R>>,

    tx: mpsc::Sender<(K, Result<R, String>)>,
    rx: mpsc::Receiver<(K, Result<R, String>)>,

    thrd_timeout: u64, // time before a worker thread aborts execution
    rsc_timeout: u64, // time before a worker thread is considered 'dead' by the main thread
}

impl<K, R> ResourceRegistry<K, R> 
where 
    K: Hash + Eq + PartialEq + Clone + Send + 'static,
    R: Send + 'static,
{
    /// Create a new resource registry.
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            storage_map: HashMap::new(),
            tx, rx,
            thrd_timeout: 5,
            rsc_timeout: 7,
        }
    }

    /// Set the timeouts for worker threads. Note: It's generally better if thrd < rsc
    /// 
    /// thrd: internal abort timeout for worker threads
    /// 
    /// rsc: external timeout for worker thread stalls/losses
    pub fn set_timeouts(&mut self, worker_timout: u64, rsc_timeout: u64) {
        assert!(rsc_timeout > worker_timout, "Worker abort timeout should be slower than stall/loss timeout!");
        self.thrd_timeout = worker_timout;
        self.rsc_timeout = rsc_timeout;
    }

    /// Retrieve a resource if is is ready. If the resource has not yet been requested, 
    /// a worker thread tracks its creation via a Future. Otherwise None is returned.
    /// 
    /// key: K, a handle to retrieve the resource when available
    /// 
    /// worker: W, A Future that resolves with the resource to be stored.
    /// 
    /// Both the key and worker must implement the Send trait.
    pub fn get_or_request<W>(&mut self, key: &K, worker: W) -> Option<&R> 
    where W: Future<Output = Result<R, String>> + Send + 'static
    {
        // resource does not exist in map
        let exists = self.storage_map.contains_key(key);
        if !exists {
            self.request_new(key, worker);
            return None;
        }
        
        // resource exists in map but isn't ready yet
        if !self.is_ready(key) { return None; }

        // resource is in map and is ready
        self.get(key)
    }

    /// Request a new worker thread to track resource creation via a Future.
    /// Does nothing if a resource with the matching key was already requested.
    /// 
    /// key: K, a handle to retrieve the resource when available
    /// 
    /// worker: W, A Future that resolves with the resource to be stored.
    /// 
    /// Both the key and worker must implement the Send trait.
    pub fn request_new<W>(&mut self, key: &K, worker: W)
    where W: Future<Output = Result<R, String>> + Send + 'static
    {
        if self.storage_map.contains_key(key) {
            return;
        }

        let k = key.clone();
        let status = ResourceStatus::Pending(Instant::now());
        self.storage_map.insert(key.clone(), status);

        let tmt = std::time::Duration::from_secs(self.thrd_timeout);
        let tx = self.tx.clone();

        tokio::task::spawn(async move {
            let result = tokio::time::timeout(tmt, worker)
                .await
                .map_err(|_| "Worker thread timed out.".to_string())
                .and_then(|work_result| work_result);

            let _ = tx.send((k, result));
        });
    }

    /// Request a new resource and wait for it's completion, blocking the calling thread until complete.
    /// 
    /// Returns a result object describing whether the the resource was successfuly created.
    pub fn request_wait<W>(&mut self, key: &K, worker: W) -> Result<(), String>
    where W: Future<Output = Result<R, String>> + Send + 'static
    {
        match pollster::block_on(worker) {
            Ok(res) => {
                self.store(key, res);
                return Ok(());
            },
            Err(e) => return Err(e),
        };
    }

    /// Store a preloaded resource into the registry
    pub fn store(&mut self, key: &K, resource: R) {
        let status = ResourceStatus::Ready(resource);
        self.storage_map.insert(key.clone(), status);
    }

    /// Remove a resource from the map. Can be used for manual retries.
    pub fn remove(&mut self, key: &K) {
        if self.storage_map.contains_key(key) {
            self.storage_map.remove(key);
        }
    }

    /// Syncronize the internal worker threads with the main thread, making available any completed resources. Should be called regularly
    pub fn sync(&mut self) {
        // check for completed worker threads
        while let Ok((key, result)) = self.rx.try_recv() {
            let status = match result {
                Ok(res) => ResourceStatus::Ready(res),
                Err(e) => ResourceStatus::Failed(e),
            };
            self.storage_map.insert(key, status);
        }

        let now = Instant::now();
        let max_wait = std::time::Duration::from_secs(self.rsc_timeout);

        // check for stalled/lost worker threads
        for status in self.storage_map.values_mut() {
            if let ResourceStatus::Pending(start_time) = status {
                if now.duration_since(*start_time) > max_wait {
                    *status = ResourceStatus::Failed("Worker thread lost or stalled execution.".to_string());
                }
            }
        }
    }

    /// Check if the registry contains a resource (in any state)
    pub fn contains(&self, key: &K) -> bool {
        return self.storage_map.contains_key(key);
    }

    /// Check if a requested resource has finished completion and is stored in the map.
    pub fn is_ready(&self, key: &K) -> bool {
        return matches!(
            self.storage_map.get(key),
            Some(ResourceStatus::Ready(_))
        )
    }

    /// Check if requested resource is still pending completion
    pub fn is_pending(&self, key: &K) -> bool {
        return matches!(
            self.storage_map.get(key),
            Some(ResourceStatus::Pending(_))
        )
    }

    /// Check if a requested resource failed completion.
    pub fn is_failed(&self, key: &K) -> bool {
        return matches!(
            self.storage_map.get(key),
            Some(ResourceStatus::Failed(_))
        )
    }

    /// Get the error message of a failed resource, if applicable.
    pub fn get_err(&self, key: &K) -> Option<&str> {
        return self.storage_map.get(key)?.error_msg();
    }

    /// Get the status of a resource. None is returned if the resource does not exist.
    pub fn status_of(&self, key: &K) -> Option<&ResourceStatus<R>> {
        return Some(self.storage_map.get(key)?);
    }

    /// Get a completed resource. Returns None if the resource does not exist/is unavailable.
    pub fn get(&self, key: &K) -> Option<&R> {
        return self.storage_map.get(key)?.value();
    }
}