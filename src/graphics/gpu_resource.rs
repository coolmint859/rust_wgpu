#![allow(dead_code)]
use std::{
    collections::HashMap, 
    hash::Hash, 
    sync::{Arc, mpsc}, 
    time::Instant
};

/// Represents the builder pattern for resources, constructing them asyncronously
pub trait ResourceBuilder: Send + 'static {
    type Key: Hash + Send + Eq + PartialEq + 'static;
    type Output: Send + 'static;
    
    /// Get the key associated with the constructed resource
    fn get_key(&self) -> Self::Key;

    /// Contruct the Output instance with the settings provided
    fn build(&self, device: Arc<wgpu::Device>) -> Result<Self::Output, String>
    where Self: Sized;
}

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
    pub fn creation_time(&self) -> Option<&Instant>{
        match self {
            ResourceStatus::Pending(time) => Some(time),
            _ => None
        }
    }

    /// Retreive a reference to the stored resource if available.
    pub fn value(&self) -> Option<&R> {
        match self {
            ResourceStatus::Ready(resource) => Some(resource),
            _ => None
        }
    }

    /// retreive the error message if the stored resource failed to complete.
    pub fn error_msg(&self) -> Option<&str> {
        match self {
            ResourceStatus::Failed(err) => Some(err.as_str()),
            _ => None
        }
    }
}

/// Manages and stores gpu memory resources with concurrent creation through builder objects.
/// 
/// TODO: force_request() and update() with prior thread cancelation (use thread JoinHandle)
pub struct GpuResourceHandler<K, R> {
    device: Arc<wgpu::Device>,
    resource_map: HashMap<K, ResourceStatus<R>>,

    tx: mpsc::Sender<(K, Result<R, String>)>,
    rx: mpsc::Receiver<(K, Result<R, String>)>,

    rsc_timeout: u64, // time before a worker thread is considered 'dead' by the main thread
}

impl<K, R> GpuResourceHandler<K, R> 
where
    K: Hash + Eq + PartialEq + Clone + Send + 'static,
    R: Send + 'static,
{
    /// Create a new resource handler.
    pub fn new(device: Arc<wgpu::Device>) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            device,
            resource_map: HashMap::new(),
            tx, rx,
            rsc_timeout: 5,
        }
    }

    /// Set the resource timeout for worker threads, in seconds.
    /// 
    /// This is the amount of time before a thread is considered 'dead' and is told to stop executing.
    pub fn set_timeout(&mut self,  rsc_timeout: u64) {
        self.rsc_timeout = rsc_timeout;
    }

    /// Retrieve a resource if is is ready. If the resource has not yet been requested, 
    /// a worker thread tracks its creation  via a builder object. Otherwise None is returned.
    /// 
    /// key: K, a handle to retrieve the resource when available
    /// 
    /// builder: B, Any object that implements the ResourceBuilder trait
    /// 
    /// Both the key and worker must implement the Send trait.
    pub fn get_or_request<B>(&mut self, builder: B) -> Option<&R> 
    where B: ResourceBuilder<Key = K, Output = R> + Clone
    {
        let key = builder.get_key();
        // resource does not exist in map
        if !self.resource_map.contains_key(&key) {
            self.request_new( builder);
            return None;
        }
        
        // resource exists in map but isn't ready yet
        if !self.is_ready(&key) { return None; }

        // resource is in map and is ready
        self.get(&key)
    }

    /// Request a new worker thread to track resource creation via a builder object.
    /// Does nothing if a resource with the matching key was already requested.
    /// 
    /// key: K, a handle to retrieve the resource when available
    /// 
    /// builder: B, Any object that implements the ResourceBuilder trait
    /// 
    /// Both the key and worker must implement the Send trait.
    pub fn request_new<B: ResourceBuilder>(&mut self, builder: B) 
    where B: ResourceBuilder<Key = K, Output = R> + Clone
    {
        let key = builder.get_key();
        if self.resource_map.contains_key(&key) {
            return;
        }

        let device_cpy = Arc::clone(&self.device);

        let status = ResourceStatus::Pending(Instant::now());
        self.resource_map.insert(key.clone(), status);

        let tx = self.tx.clone();

        tokio::task::spawn(async move {
            let result = builder.build(device_cpy); 
            let _ = tx.send((key, result));
        });
    }
    
    /// Request a new resource and wait for it's completion, blocking the calling thread until complete.
    /// 
    /// Returns a result object describing whether the the resource was successfuly created.
    pub fn request_wait<B>(&mut self, builder: B) -> Result<(), String>
    where B: ResourceBuilder<Key = K, Output = R> + Clone
    {
        let key = builder.get_key();
        if self.resource_map.contains_key(&key) {
            return Ok(());
        }
        
        // build is a blocking call
        match builder.build(Arc::clone(&self.device)) {
            Ok(resource) => {
                self.store(&key, resource);
                Ok(())
            },
            Err(msg) => Err(msg)
        }
    }

    /// Store a preloaded resource into the internal map
    pub fn store(&mut self, key: &K, resource: R) {
        let status = ResourceStatus::Ready(resource);
        self.resource_map.insert(key.clone(), status);
    }

    /// Remove a resource from the internal map. Can be used for manual retries.
    pub fn remove(&mut self, key: &K) {
        if self.resource_map.contains_key(key) {
            self.resource_map.remove(key);
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
            self.resource_map.insert(key, status);
        }

        let now = Instant::now();
        let max_wait = std::time::Duration::from_secs(self.rsc_timeout);

        // check for stalled/lost worker threads
        for status in self.resource_map.values_mut() {
            if let ResourceStatus::Pending(start_time) = status {
                if now.duration_since(*start_time) > max_wait {
                    *status = ResourceStatus::Failed("Worker thread lost or stalled execution.".to_string());
                }
            }
        }
    }

    /// Check if a requested resource has finished completion and is stored in the map.
    pub fn is_ready(&self, key: &K) -> bool {
        return matches!(
            self.resource_map.get(key),
            Some(ResourceStatus::Ready(_))
        )
    }

    /// Check if requested resource is still pending completion
    pub fn is_pending(&self, key: &K) -> bool {
        return matches!(
            self.resource_map.get(key),
            Some(ResourceStatus::Pending(_))
        )
    }

    /// Check if a requested resource failed completion.
    pub fn is_failed(&self, key: &K) -> bool {
        return matches!(
            self.resource_map.get(key),
            Some(ResourceStatus::Failed(_))
        )
    }

    /// Get the error message of a failed resource, if applicable.
    pub fn get_err(&self, key: &K) -> Option<&str> {
        return self.resource_map.get(key)?.error_msg();
    }

    /// Get the status of a resource. None is returned if the resource does not exist.
    pub fn status_of(&self, key: &K) -> Option<&ResourceStatus<R>> {
        self.resource_map.get(key)
    }

    /// Get a completed resource. Returns None if the resource does not exist/is unavailable.
    pub fn get(&self, key: &K) -> Option<&R> {
        (self.resource_map.get(key)?).value()
    }
}
