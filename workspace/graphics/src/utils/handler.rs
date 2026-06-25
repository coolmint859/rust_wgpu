#![allow(dead_code)]
use std::{
    collections::HashMap, 
    fmt::Debug, 
    hash::Hash, 
    sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}, mpsc}, 
    time::{Duration, Instant}
};

use tokio::task::JoinHandle;

/// Specifies the type of work that a ResourceBuilder does.
pub enum BuilderType {
    /// For builders that do work that is mostly non-blocking, or io-bound (i.e. reading a file)
    NonBlocking,
    /// For builders that do work that is mostly blocking, or cpu-bound (i.e. procedural texture generation)
    Blocking
}

/// Represents the builder pattern for resources
pub trait ResourceBuilder: Send + Sync + Clone + 'static {
    type Output: Send + 'static;
    type Context: Send + Sync + 'static;

    /// Get the type of work (io-bound vs cpu-bound) that this builder does. Default is non-blocking (io-bound)
    fn builder_type(&self) -> BuilderType { BuilderType::NonBlocking }

    /// Get the amount of time in seconds this resource should be held after it was last accessed before being deallocated.
    /// 
    /// None (default) signifies that this resource lives indefinitely. 
    fn hold_time(&self) -> Option<u64> { None }

    /// Contruct the Output instance with the settings provided
    /// 
    /// * 'context' - dependency struct for creating the resource
    /// * 'cancel_flag' - a read only token indicating whether the main thread has requested this builder to cease execution. This is best used for Blocking (cpu bound) builders.
    fn build(&self, context: Arc<Self::Context>, cancel_flag: Arc<AtomicBool>) -> Result<Self::Output, String>;
}

/// Stores metadata about resources that finished completion
pub struct Ready<R> {
    /// The stored resource
    pub rsc: R,
    /// The time in seconds before this resource should be deallocated.
    pub hold_time: Option<u64>,
    /// The time stamp for when this resource was last accessed.
    pub accessed: Mutex<Instant>,
}

/// Stores metadata about resources that failed completion
pub struct Failed {
    /// An error message indicating why the Resource failed
    pub err_msg: String,
    /// The time stamp for when the resource failed.
    pub failed_at: Instant
}

/// Stores metadata about resources that are pending completion
pub struct Pending {
    /// A handle to the tokio thread responsible for creating the resource
    pub thread_handle: JoinHandle<()>,
    /// Token for indicating whether the builder thread should cancel execution
    pub cancel_flag: Arc<AtomicBool>,
    /// The time stamp for when the resource was requested.
    pub requested_at: Instant,
}

/// Represents the state of a resource requested by the user of a handler instance.
pub enum ResourceStatus<R> {
    /// Resource has been requested but is not yet ready
    Pending(Pending),

    /// Resource is ready for retrieval
    Ready(Ready<R>),

    /// Resource failed to complete
    Failed(Failed),
}

impl<R> ResourceStatus<R> {
    /// Retrieve the time the resource was requested if available
    pub fn requested_at(&self) -> Option<&Instant>{
        match self {
            ResourceStatus::Pending(pending) => Some(&pending.requested_at),
            _ => None
        }
    }

    /// Retreive a reference to the stored resource if available.
    pub fn value(&self) -> Option<&Ready<R>> {
        match self {
            ResourceStatus::Ready(resource) => Some(resource),
            _ => None
        }
    }

    /// Retreive a mutable reference to the stored resource if available.
    pub fn value_mut(&mut self) -> Option<&mut Ready<R>> {
        match self {
            ResourceStatus::Ready(resource) => Some(resource),
            _ => None
        }
    }

    /// Retreive the error message if the stored resource failed to complete.
    pub fn error_msg(&self) -> Option<&str> {
        match self {
            ResourceStatus::Failed(failed) => Some(&failed.err_msg),
            _ => None
        }
    }

    /// Check if this resource is ready (has completed loading/creation)
    pub fn is_ready(&self) -> bool {
        return self.value().is_some()
    }

    /// Check if this resource is pending (not yet loaded/created)
    pub fn is_pending(&self) -> bool {
        return self.requested_at().is_some()
    }

    /// Check if this resource is failed (didn't load/wasn't created)
    pub fn is_failed(&self) -> bool {
        return self.error_msg().is_some()
    }
}

/// Manages and stores any memory resources with concurrent creation through builders.
/// Allows any builder struct as long as the output type matches the resource type.
/// 
/// K: The key type to store resouces with
/// 
/// R: the resource type that will be stored
pub struct ResourceHandler<K, R> {
    resource_map: HashMap<K, ResourceStatus<R>>,

    tx: mpsc::Sender<(K, Result<Ready<R>, String>)>,
    rx: mpsc::Receiver<(K, Result<Ready<R>, String>)>,

    thread_timeout: Duration, // time before a builder thread is considered 'dead' by the main thread
    failed_timeout: Duration, // time before a failed resource is removed from the map
}

impl<K: Debug, R> ResourceHandler<K, R> 
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
            thread_timeout: Duration::from_secs(5),
            failed_timeout: Duration::from_secs(3)
        }
    }

    /// Set the resource timeout for builder threads, in seconds. The default is 5 seconds.
    /// 
    /// This is the amount of time before a thread is considered 'dead' and is told to stop executing.
    pub fn set_thread_tmt(&mut self,  timeout: u64) {
        self.thread_timeout = Duration::from_secs(timeout);
    }

    /// Set the timeout for failed resources, in seconds. The default is 3 seconds.
    /// 
    /// This is the amount of time before a failed resource is removed from the internal map. 
    pub fn set_failed_tmt(&mut self, timeout: u64) {
        self.failed_timeout = Duration::from_secs(timeout);
    }

    /// Retrieve a resource if is is ready. If the resource has not yet been requested, 
    /// a worker thread tracks its creation via a builder object, and None is returned.
    /// 
    /// * 'key' - A handle K to query the handler for the resource
    /// * 'builder' - A ResourceBuilder implementation that outputs a resource R
    /// * 'context' - An instance of the context type specfied by the builder B
    pub fn get_or_request<B, C>(&mut self, key: &K, builder: &B, context: Arc<C>) -> Option<&R> 
    where 
        B: ResourceBuilder<Output = R, Context = C>,
        C: Send + Sync + 'static
    {
        let needs_request = match self.resource_map.get(key) {
            None => true,                               // resource doesn't exist in map
            Some(ResourceStatus::Failed(_)) => true,    // resource exists but previously failed
            Some(_) => false                            // resource exists but is either pending or ready
        };

        if needs_request {
            self.remove(key);
            self.request_new(key, builder, context);
            return None;
        }

        self.get(key)
    }

    /// Request a builder thread to create a resource via a ResourceBuilder if previously failed.
    /// 
    /// If the resource does not exist, this method still spawns a builder thread.
    /// If the resource exists and is pending or ready, no thread is spawned.
    /// 
    /// * 'key' - A handle K to query the handler for the resource
    /// * 'builder' - A ResourceBuilder implementation that outputs a resource R
    /// * 'context' - An instance of the context type specfied by the builder B
    pub fn request_retry<B, C>(&mut self, key: &K, builder: &B, context: Arc<C>)
    where 
        B: ResourceBuilder<Output = R, Context = C>,
        C: Send + Sync + 'static
    {
        let needs_retry = match self.resource_map.get(key) {
            None => true,
            Some(ResourceStatus::Failed(_)) => true,
            Some(_) => false
        };

        if needs_retry {
            self.remove(key);
            self.request_new(key, builder, context);
        }
    }

    /// Request a new builder thread to create a resource via a ResourceBuilder.
    /// Does nothing if a resource with the matching key was already requested.
    /// 
    /// * 'key' - A handle K to query the handler for the resource
    /// * 'builder' - A ResourceBuilder implementation that outputs a resource R
    /// * 'context' - An instance of the context type specfied by the builder B
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
        let tx = self.tx.clone();

        let cancel_flag = Arc::new(AtomicBool::new(false));
        let cancel_flag_cpy = cancel_flag.clone();

        let tokio_handle = match builder.builder_type() {
            BuilderType::NonBlocking => {
                tokio::task::spawn( async move {
                    let result = ResourceHandler::<K, R>::load_rsc(builder_cpy, context_cpy, cancel_flag_cpy);
                    let _ = tx.send((key_cpy, result));
                })
            },
            BuilderType::Blocking => {
                tokio::task::spawn_blocking(move || {
                    let result = ResourceHandler::<K, R>::load_rsc(builder_cpy, context_cpy, cancel_flag_cpy);
                    let _ = tx.send((key_cpy, result));
                })
            }
        };

        let status = ResourceStatus::Pending(Pending {
            thread_handle: tokio_handle,
            requested_at: Instant::now(),
            cancel_flag
        });
        self.resource_map.insert(key.clone(), status);
    }

    /// Load a new resource by calling the builder, and wrapping the Ok result in a ReadyResource instance
    fn load_rsc<B, C>(builder: B, context: Arc<C>, cancel_flag: Arc<AtomicBool>) -> Result<Ready<R>, String>
    where 
        B: ResourceBuilder<Output = R, Context = C>,
        C: Send + Sync + 'static
    {
        builder.build(context, cancel_flag)
            .map(|rsc| Ready {
                rsc,
                hold_time: builder.hold_time(),
                accessed: Mutex::new(Instant::now())
            })
    }
    
    /// Request a new resource and wait for it's completion.
    /// 
    /// Returns a result object containing the completed resource, or an error message if failed.
    /// 
    /// * 'key' - A handle K to query the handler for the resource
    /// * 'builder' - A ResourceBuilder implementation that outputs a resource R
    /// * 'context' - An instance of the context type specfied by the builder B
    pub fn request_wait<B, C>(&mut self, key: &K, builder: &B, context: Arc<C>) -> Result<Option<&R>, String>
    where 
        B: ResourceBuilder<Output = R, Context = C>,
        C: Send + Sync + 'static
    {
        if self.resource_map.contains_key(key) {
            return Ok(self.get(key));
        }
        
        let cancel_flag= Arc::new(AtomicBool::new(false));
        builder.build(context, cancel_flag).map(|rsc| {
            self.store(key, rsc, builder.hold_time());
            self.get(key)
        })
    }

    /// Store a preloaded resource into the internal map
    /// 
    /// * 'key' - A handle K to query the handler for the resource
    /// * 'resource' - An instance of the expected Resource this handler stores
    /// * 'hold_time' - The time in seconds before a resource is considered 'dead' and is removed from the handler
    pub fn store(&mut self, key: &K, resource: R, hold_time: Option<u64>) {
        let status = ResourceStatus::Ready(Ready { 
            rsc: resource, 
            hold_time, 
            accessed: Mutex::new(Instant::now())
        });

        self.resource_map.insert(key.clone(), status);
    }

    /// Remove a resource from the internal map.
    pub fn remove(&mut self, key: &K) {
        if self.resource_map.contains_key(key) {
            self.resource_map.remove(key);
        }
    }

    /// Syncronize the resource builder threads with the main thread, making available any completed resources. Should be called regularly
    pub fn sync(&mut self) {
        while let Ok((key, result)) = self.rx.try_recv() {
            let status = match result {
                Ok(rsc) => ResourceStatus::Ready(rsc),
                Err(e) => ResourceStatus::Failed(Failed { 
                    err_msg: e, failed_at: Instant::now() 
                }),
            };
            self.resource_map.insert(key, status);
        }

        self.evaluate_rsc_statuses();
    }

    /// Evaluate the statuses of known resources, determining whether to mark as failed or remove from the map
    fn evaluate_rsc_statuses(&mut self) {
        let now = Instant::now();
        self.resource_map.retain(|key, status| {
            match status {
                ResourceStatus::Ready(ready_rsc) => {
                    if let Some(hold_time) = ready_rsc.hold_time {
                        let hold_duration = Duration::from_secs(hold_time);

                        if let Ok(accessed) = ready_rsc.accessed.lock() {
                            if now.saturating_duration_since(*accessed) > hold_duration {
                                println!("[ResourceHandler] Removed resource with key {:?} from handler due to hold timeout.", key);
                                
                                return false; // resource is considered 'dead', remove it from the handler
                            }
                        }
                    }
                },
                ResourceStatus::Pending(pending) => {
                    if now.saturating_duration_since(pending.requested_at) > self.thread_timeout {
                        println!("[ResourceHandler] Aborted builder thread for resource with key {:?} due to thread timeout.", key);
                                
                        pending.thread_handle.abort(); // cancel the thread if non-blocking
                        pending.cancel_flag.store(true, Ordering::Relaxed); // signify cancel if blocking

                        *status = ResourceStatus::Failed(Failed { 
                            err_msg: "Worker thread lost or stalled execution.".to_string(), 
                            failed_at: now 
                        });
                    }
                },
                ResourceStatus::Failed(failed_state) => {
                    if now.saturating_duration_since(failed_state.failed_at) > self.failed_timeout {
                        println!("[ResourceHandler] Removed resource with key {:?} from handler due to fail status timeout.", key);
                        
                        return false;
                    }
                }
            }
            return true; // resource is still active
        });
    }

    /// Check if a requested resource has finished completion and is stored in the map.
    pub fn is_ready(&self, key: &K) -> bool {
        self.resource_map.get(key).is_some_and(|rsc| rsc.is_ready())
    }

    /// Check if requested resource is still pending completion
    pub fn is_pending(&self, key: &K) -> bool {
        self.resource_map.get(key).is_some_and(|rsc| rsc.is_pending())
    }

    /// Check if a requested resource failed completion.
    pub fn is_failed(&self, key: &K) -> bool {
        self.resource_map.get(key).is_some_and(|rsc| rsc.is_failed())
    }

    /// Get the error message of a failed resource, if applicable.
    pub fn get_err(&self, key: &K) -> Option<&str> {
        return self.resource_map.get(key)?.error_msg();
    }

    /// Get the status of a resource. None is returned if the resource does not exist.
    pub fn status_of(&self, key: &K) -> Option<&ResourceStatus<R>> {
        self.resource_map.get(key)
    }

    /// Get a reference to a completed resource. Returns None if the resource does not exist/is unavailable.
    pub fn get(&self, key: &K) -> Option<&R> {
        match self.resource_map.get(key) {
            Some(ResourceStatus::Ready(ready_rsc)) => {
                if let Ok(mut accessed) = ready_rsc.accessed.lock() {
                    *accessed = Instant::now()
                }

                Some(&ready_rsc.rsc)
            },
            _ => None
        }
    }

    /// Get a mutable reference to a completed resource. Returns None if the resource does not exist/is unavailable.
    /// 
    /// Note: This locks the handler from retreival of other resources.
    pub fn get_mut(&mut self, key: &K) -> Option<&mut R> {
        match self.resource_map.get_mut(key) {
            Some(ResourceStatus::Ready(ready_rsc)) => {
                if let Ok(mut accessed) = ready_rsc.accessed.lock() {
                    *accessed = Instant::now()
                }

                Some(&mut ready_rsc.rsc)
            },
            _ => None
        }
    }

    /// Mark a resource as accessed. 
    /// 
    /// This is useful in cases where a resource may have dependencies, but you don't need to access the dependencies directly.
    pub fn mark_accessed(&self, key: &K) {
        match self.resource_map.get(key) {
            Some(ResourceStatus::Ready(ready_rsc)) => {
                if let Ok(mut accessed) = ready_rsc.accessed.lock() {
                    *accessed = Instant::now()
                }
            },
            _ => {}
        }
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
