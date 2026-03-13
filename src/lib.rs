#![cfg_attr(not(feature = "std"), no_std)]
use bevy_platform::collections::HashMap;
use bevy_platform::sync::{Arc, RwLock};
use concurrent_queue::ConcurrentQueue;
use core::hash::Hash;

/// `HashMap<K, Arc<ConcurrentQueue<V>>>` behind a single `RwLock`.
/// - Writers only contend when creating a new key.
/// - `push` is almost always non-blocking (unbounded queue).
pub struct KeyedQueues<K, V> {
    inner: RwLock<HashMap<K, Arc<ConcurrentQueue<V>>>>,
}

impl<K, V> KeyedQueues<K, V>
where
    K: Eq + Hash + Clone,
    V: Send + 'static,
{
    pub const fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }

    #[inline]
    pub fn get_or_create(&self, key: &K) -> Arc<ConcurrentQueue<V>> {
        // Fast path: try read lock first
        if let Some(q) = self.inner.read().unwrap().get(key).cloned() {
            return q;
        }
        // Slow path: create under write lock if still absent
        let mut write = self.inner.write().unwrap();
        // We intentionally check a second time because of synchronization
        if let Some(q) = write.get(key).cloned() {
            return q;
        }
        let q = Arc::new(ConcurrentQueue::unbounded());
        write.insert(key.clone(), q.clone());
        q
    }

    /// Potentially-blocking send but almost never blocking (unbounded queue => `push` never fails).
    /// ( Only blocks when the `K` has never been used before )
    #[inline]
    pub fn try_send(&self, key: &K, val: V) -> Result<(), concurrent_queue::PushError<V>> {
        let q = self.get_or_create(key);
        q.push(val)
    }
}