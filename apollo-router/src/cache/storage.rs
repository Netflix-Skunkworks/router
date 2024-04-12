use std::fmt::Display;
use std::fmt::{self};
use std::hash::Hash;
use std::num::NonZeroUsize;
use std::sync::Arc;

use lru::LruCache;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::sync::Mutex;
use tokio::time::Instant;
use tower::BoxError;

use super::redis::*;
use crate::configuration::RedisCache;

pub(crate) trait KeyType:
    Clone + fmt::Debug + fmt::Display + Hash + Eq + Send + Sync
{
}
pub(crate) trait ValueType:
    Clone + fmt::Debug + Send + Sync + Serialize + DeserializeOwned
{
}

// Blanket implementation which satisfies the compiler
impl<K> KeyType for K
where
    K: Clone + fmt::Debug + fmt::Display + Hash + Eq + Send + Sync,
{
    // Nothing to implement, since K already supports the other traits.
    // It has the functions it needs already
}

// Blanket implementation which satisfies the compiler
impl<V> ValueType for V
where
    V: Clone + fmt::Debug + Send + Sync + Serialize + DeserializeOwned,
{
    // Nothing to implement, since V already supports the other traits.
    // It has the functions it needs already
}

pub(crate) type InMemoryCache<K, V> = Arc<Mutex<LruCache<K, V>>>;

#[async_trait::async_trait]
pub trait SecondaryCacheStorage<K, V>: Send + Sync {
    async fn get(&self, key: &K) -> Option<V>;
    async fn insert(&self, key: K, value: V);
}

#[async_trait::async_trait]
impl<K, V> SecondaryCacheStorage<K, V> for RedisCacheStorage
where
    K: KeyType + 'static,
    V: ValueType + 'static,
{
    async fn get(&self, key: &K) -> Option<V> {
        self.get(RedisKey(key.clone())).await.map(|k| k.0)
    }

    async fn insert(&self, key: K, value: V) {
        self.insert(RedisKey(key), RedisValue(value), None).await;
    }
}

// placeholder storage module
//
// this will be replaced by the multi level (in memory + redis/memcached) once we find
// a suitable implementation.
#[derive(Clone)]
pub(crate) struct CacheStorage<K: KeyType, V: ValueType> {
    caller: String,
    inner: Arc<Mutex<LruCache<K, V>>>,
    redis: Option<Arc<dyn SecondaryCacheStorage<K, V>>>,
}

impl<K, V> CacheStorage<K, V>
where
    K: KeyType + 'static,
    V: ValueType + 'static,
{
    pub(crate) async fn new(
        max_capacity: NonZeroUsize,
        config: Option<RedisCache>,
        caller: &str,
        secondary_cache: Option<Arc<dyn SecondaryCacheStorage<K, V>>>,
    ) -> Result<Self, BoxError> {
        let redis = if let Some(config) = config {
            let required_to_start = config.required_to_start;
            match RedisCacheStorage::new(config).await {
                Err(e) => {
                    tracing::error!(
                        cache = caller,
                        e,
                        "could not open connection to Redis for caching",
                    );
                    if required_to_start {
                        return Err(e);
                    }
                    None
                }
                Ok(storage) => Some(Arc::new(storage)),
            }
        } else {
            None
        };

        let redis: Option<Arc<dyn SecondaryCacheStorage<K, V>>> = redis.map(|r| r as _);

        Ok(Self {
            caller: caller.to_string(),
            inner: Arc::new(Mutex::new(LruCache::new(max_capacity))),
            redis: secondary_cache.or(redis),
        })
    }

    pub(crate) async fn get(&self, key: &K) -> Option<V> {
        let instant_memory = Instant::now();
        let res = self.inner.lock().await.get(key).cloned();

        match res {
            Some(v) => {
                tracing::info!(
                    monotonic_counter.apollo_router_cache_hit_count = 1u64,
                    kind = %self.caller,
                    storage = &tracing::field::display(CacheStorageName::Memory),
                );
                let duration = instant_memory.elapsed().as_secs_f64();
                tracing::info!(
                    histogram.apollo_router_cache_hit_time = duration,
                    kind = %self.caller,
                    storage = &tracing::field::display(CacheStorageName::Memory),
                );
                Some(v)
            }
            None => {
                let duration = instant_memory.elapsed().as_secs_f64();
                tracing::info!(
                    histogram.apollo_router_cache_miss_time = duration,
                    kind = %self.caller,
                    storage = &tracing::field::display(CacheStorageName::Memory),
                );
                tracing::info!(
                    monotonic_counter.apollo_router_cache_miss_count = 1u64,
                    kind = %self.caller,
                    storage = &tracing::field::display(CacheStorageName::Memory),
                );

                let instant_redis = Instant::now();
                if let Some(redis) = self.redis.as_ref() {
                    match redis.get(key).await {
                        Some(v) => {
                            self.inner.lock().await.put(key.clone(), v.clone());

                            tracing::info!(
                                monotonic_counter.apollo_router_cache_hit_count = 1u64,
                                kind = %self.caller,
                                storage = &tracing::field::display(CacheStorageName::Redis),
                            );
                            let duration = instant_redis.elapsed().as_secs_f64();
                            tracing::info!(
                                histogram.apollo_router_cache_hit_time = duration,
                                kind = %self.caller,
                                storage = &tracing::field::display(CacheStorageName::Redis),
                            );
                            Some(v)
                        }
                        None => {
                            tracing::info!(
                                monotonic_counter.apollo_router_cache_miss_count = 1u64,
                                kind = %self.caller,
                                storage = &tracing::field::display(CacheStorageName::Redis),
                            );
                            let duration = instant_redis.elapsed().as_secs_f64();
                            tracing::info!(
                                histogram.apollo_router_cache_miss_time = duration,
                                kind = %self.caller,
                                storage = &tracing::field::display(CacheStorageName::Redis),
                            );
                            None
                        }
                    }
                } else {
                    None
                }
            }
        }
    }

    pub(crate) async fn insert(&self, key: K, value: V) {
        if let Some(redis) = self.redis.as_ref() {
            redis.insert(key.clone(), value.clone()).await;
        }

        let mut in_memory = self.inner.lock().await;
        in_memory.put(key, value);
        let size = in_memory.len() as u64;
        tracing::info!(
            value.apollo_router_cache_size = size,
            kind = %self.caller,
            storage = &tracing::field::display(CacheStorageName::Memory),
        );
    }

    pub(crate) async fn insert_in_memory(&self, key: K, value: V) {
        let mut in_memory = self.inner.lock().await;
        in_memory.put(key, value);
        let size = in_memory.len() as u64;
        tracing::info!(
            value.apollo_router_cache_size = size,
            kind = %self.caller,
            storage = &tracing::field::display(CacheStorageName::Memory),
        );
    }

    pub(crate) fn in_memory_cache(&self) -> InMemoryCache<K, V> {
        self.inner.clone()
    }

    #[cfg(test)]
    pub(crate) async fn len(&self) -> usize {
        self.inner.lock().await.len()
    }
}

enum CacheStorageName {
    Redis,
    Memory,
}

impl Display for CacheStorageName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CacheStorageName::Redis => write!(f, "redis"),
            CacheStorageName::Memory => write!(f, "memory"),
        }
    }
}
