pub mod client;
pub mod codec;
pub mod server;

/// A [`HashMap`](std::collections::HashMap) using [`ahash`] to hash items.
///
/// Since most of our locks guard HashMaps it makes sense to use a faster hasher to improve
/// contention.
pub type HashMap<K, V> = ahash::AHashMap<K, V>;

/// A [`HashMap`] wrapped to be safely sharable across threads.
pub type ConcurrentMap<K, V> = std::sync::Arc<tokio::sync::Mutex<HashMap<K, V>>>;
