#![allow(dead_code)]
use std::{
    collections::HashMap, 
    hash::Hash, 
    sync::{Arc, mpsc}, 
    time::Instant
};

/// A high level data struct that used to generate builder objects.
pub trait ResourceTemplate: Hash + Send + Sync + Eq + PartialEq + 'static {
    /// The specific builder struct that this template generates
    type Builder: ResourceBuilder;

    /// Uses the configured settings of the template and converts it into a builder instance
    fn to_builder(&self) -> Self::Builder;
}

/// Represents the builder pattern for resources
pub trait ResourceBuilder: Send + Clone + 'static {
    type Output: Send + 'static;
    type Context: Send + Sync + 'static;

    /// Contruct the Output instance with the settings provided
    fn build(&self, context: Arc<Self::Context>) -> Result<Self::Output, String>;
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

/// Manages and stores any memory resource with concurrent creation through builder objects.
/// 
/// K: The key type to store resouces with
/// 
/// R: the resource type that will be stored
/// 
/// TODO: force_request() and update() with prior thread cancelation (use thread JoinHandle)
pub struct ResourceHandler<K, R> {
    resource_map: HashMap<K, ResourceStatus<R>>,

    tx: mpsc::Sender<(K, Result<R, String>)>,
    rx: mpsc::Receiver<(K, Result<R, String>)>,

    rsc_timeout: u64, // time before a worker thread is considered 'dead' by the main thread
}

impl<K, R> ResourceHandler<K, R> 
where
    K: Hash + Eq + PartialEq + Clone + Send + 'static,
    R: Send + 'static,
{
    /// Create a new resource handler.
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
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
    /// context: C, an instance of the context type specfied by the builder B
    pub fn get_or_request<B, C>(&mut self, key: &K, builder: &B, context: Arc<C>) -> Option<&R> 
    where 
        B: ResourceBuilder<Output = R, Context = C>,
        C: Send + Sync + 'static
    {
        // resource does not exist in map
        if !self.resource_map.contains_key(key) {
            self.request_new(key, builder, context);
            return None;
        }

        // resource is in map, but may not be ready
        self.get(&key)
    }

    /// Request a new worker thread to track resource creation via a builder object.
    /// Does nothing if a resource with the matching key was already requested.
    /// 
    /// key: K, a handle to retrieve the resource when available
    /// 
    /// builder: B, Any object that implements the ResourceBuilder trait with the matching output 
    /// 
    /// context: C, an instance of the context type specfied by the builder B
    pub fn request_new<B, C>(&mut self, key: &K, builder: &B, context: Arc<C>) 
    where 
        B: ResourceBuilder<Output = R, Context = C>,
        C: Send + Sync + 'static
    {
        let key_cpy = key.clone();
        if self.resource_map.contains_key(&key_cpy) {
            return;
        }

        let context_cpy = context.clone();
        let builder_cpy = builder.clone();

        let status = ResourceStatus::Pending(Instant::now());
        self.resource_map.insert(key_cpy.clone(), status);

        let tx = self.tx.clone();

        tokio::task::spawn(async move {
            let result = builder_cpy.build(context_cpy); 
            let _ = tx.send((key_cpy, result));
        });
    }
    
    /// Request a new resource and wait for it's completion, blocking the calling thread until complete.
    /// 
    /// Returns a result object describing whether the the resource was successfuly created.
    pub fn request_wait<B, C>(&mut self, key: &K, builder: &B, context: Arc<C>) -> Result<(), String>
    where 
        B: ResourceBuilder<Output = R, Context = C>,
        C: Send + Sync + 'static
    {
        if self.resource_map.contains_key(key) {
            return Ok(());
        }
        
        // build is a blocking call
        match builder.build(context.clone()) {
            Ok(resource) => {
                self.store(key, resource);
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

    /// Check if the internal map contains a resource with the specified key (in any state)
    pub fn contains(&self, key: &K) -> bool {
        self.resource_map.contains_key(key)
    }

    /// Get a vector of known resource keys mapped to their resource status' in the form of a tuple. Useful for debugging purposes.
    pub fn status_of_all(&self) -> Vec<(&K, String)> {
        self.resource_map.iter().map(|(key, resource)| {
            let status = match resource {
                ResourceStatus::Failed(_) => "FAILED",
                ResourceStatus::Pending(_) => "PENDING",
                ResourceStatus::Ready(_) => "READY",
            }.to_string();

            (key, status)
        }).collect()
    }
}
