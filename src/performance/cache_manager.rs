//! Cache management for rfgrep performance optimization
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Cache entry with timestamp
#[derive(Debug, Clone)]
pub struct CacheEntry<V> {
    pub value: V,
    pub created_at: Instant,
    pub last_accessed: Instant,
}

impl<V> CacheEntry<V> {
    pub fn new(value: V) -> Self {
        let now = Instant::now();
        Self {
            value,
            created_at: now,
            last_accessed: now,
        }
    }

    pub fn is_expired(&self, ttl: Duration) -> bool {
        self.created_at.elapsed() > ttl
    }

    pub fn update_access(&mut self) {
        self.last_accessed = Instant::now();
    }
}

/// LRU cache implementation
pub struct LruCache<K, V> {
    capacity: usize,
    entries: HashMap<K, CacheEntry<V>>,
    access_order: Vec<K>,
}

impl<K, V> LruCache<K, V>
where
    K: Hash + Eq + Clone,
{
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: HashMap::new(),
            access_order: Vec::new(),
        }
    }

    pub fn get(&mut self, key: &K) -> Option<&V> {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.update_access();
            if let Some(pos) = self.access_order.iter().position(|k| k == key) {
                self.access_order.remove(pos);
            }
            self.access_order.push(key.clone());
            Some(&entry.value)
        } else {
            None
        }
    }

    pub fn insert(&mut self, key: K, value: V) {
        if self.entries.contains_key(&key) {
            if let Some(entry) = self.entries.get_mut(&key) {
                entry.value = value;
                entry.update_access();
            }
        } else {
            if self.entries.len() >= self.capacity {
                if !self.access_order.is_empty() {
                    let lru_key = self.access_order.remove(0);
                    self.entries.remove(&lru_key);
                }
            }
            self.entries.insert(key.clone(), CacheEntry::new(value));
            self.access_order.push(key);
        }
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        if let Some(entry) = self.entries.remove(key) {
            self.access_order.retain(|k| k != key);
            Some(entry.value)
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.access_order.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Thread-safe cache manager
pub struct CacheManager<K, V> {
    cache: Arc<RwLock<LruCache<K, V>>>,
    ttl: Duration,
}

impl<K, V> CacheManager<K, V>
where
    K: Hash + Eq + Clone + Send + Sync,
    V: Clone + Send + Sync,
{
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        Self {
            cache: Arc::new(RwLock::new(LruCache::new(capacity))),
            ttl,
        }
    }

    pub fn get(&self, key: &K) -> Option<V> {
        let mut cache = self.cache.write().unwrap();
        cache.get(key).cloned()
    }

    pub fn insert(&self, key: K, value: V) {
        let mut cache = self.cache.write().unwrap();
        cache.insert(key, value);
    }

    pub fn remove(&self, key: &K) -> Option<V> {
        let mut cache = self.cache.write().unwrap();
        cache.remove(key)
    }

    pub fn clear(&self) {
        let mut cache = self.cache.write().unwrap();
        cache.clear();
    }

    pub fn cleanup_expired(&self) {
        let mut cache = self.cache.write().unwrap();
        let expired_keys: Vec<K> = cache
            .entries
            .iter()
            .filter(|(_, entry)| entry.is_expired(self.ttl))
            .map(|(key, _)| key.clone())
            .collect();

        for key in expired_keys {
            cache.remove(&key);
        }
    }

    pub fn stats(&self) -> CacheStats {
        let cache = self.cache.read().unwrap();
        CacheStats {
            size: cache.len(),
            capacity: cache.capacity,
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub size: usize,
    pub capacity: usize,
}

/// Search result cache
pub type SearchCache = CacheManager<String, Vec<u8>>;

impl<K, V> Default for CacheManager<K, V>
where
    K: Hash + Eq + Clone + Send + Sync,
    V: Clone + Send + Sync,
{
    fn default() -> Self {
        Self::new(100, Duration::from_secs(60))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_lru_cache() {
        let mut cache = LruCache::new(3);

        cache.insert("key1", "value1");
        cache.insert("key2", "value2");
        cache.insert("key3", "value3");

        assert_eq!(cache.get(&"key1"), Some(&"value1"));
        assert_eq!(cache.get(&"key2"), Some(&"value2"));
        assert_eq!(cache.get(&"key3"), Some(&"value3"));

        cache.insert("key4", "value4");

        assert_eq!(cache.get(&"key1"), None);
        assert_eq!(cache.get(&"key4"), Some(&"value4"));
    }

    #[test]
    fn test_cache_manager() {
        let cache = CacheManager::new(10, Duration::from_secs(1));

        cache.insert("key1", "value1");
        assert_eq!(cache.get(&"key1"), Some("value1"));

        cache.remove(&"key1");
        assert_eq!(cache.get(&"key1"), None);
    }

    #[test]
    fn test_thread_safety() {
        let cache = Arc::new(CacheManager::new(100, Duration::from_secs(60)));
        let mut handles = vec![];

        for i in 0..10 {
            let cache_clone = cache.clone();
            let handle = thread::spawn(move || {
                for j in 0..100 {
                    let key = format!("key_{}_{}", i, j);
                    let value = format!("value_{}_{}", i, j);
                    cache_clone.insert(key, value);
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert!(cache.stats().size > 0);
    }
}
