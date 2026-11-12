use std::collections::HashMap;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::Value;
use tokio::sync::Mutex as AsyncMutex;
use uuid::Uuid;

use crate::error::ApiError;
use crate::models::User;

/// Small in-process cache for expensive, non-personal responses.
///
/// A per-key async mutex means only one request computes a missing entry while
/// the others wait for that result instead of stampeding PostgreSQL together.
pub struct ResponseCache {
    entries: Mutex<HashMap<String, CacheEntry>>,
    flights: Mutex<HashMap<String, Arc<AsyncMutex<()>>>>,
    generation: AtomicU64,
    max_entries: usize,
}

struct CacheEntry {
    value: Value,
    expires_at: Instant,
}

impl ResponseCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            flights: Mutex::new(HashMap::new()),
            generation: AtomicU64::new(0),
            max_entries,
        }
    }

    pub async fn get_or_compute<F, Fut>(
        &self,
        key: String,
        ttl: Duration,
        compute: F,
    ) -> Result<Value, ApiError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<Value, ApiError>>,
    {
        if let Some(value) = self.lookup(&key) {
            return Ok(value);
        }

        let flight = {
            let mut flights = self
                .flights
                .lock()
                .expect("response cache flight mutex poisoned");
            flights
                .entry(key.clone())
                .or_insert_with(|| Arc::new(AsyncMutex::new(())))
                .clone()
        };

        let _guard = flight.lock().await;

        // Another request may have populated the entry while we waited.
        if let Some(value) = self.lookup(&key) {
            self.finish_flight(&key, &flight);
            return Ok(value);
        }

        let generation = self.generation.load(Ordering::SeqCst);
        match compute().await {
            Ok(value) => {
                // A clear/invalidate while we were computing must win over our
                // now-stale result.
                if self.generation.load(Ordering::SeqCst) == generation {
                    self.store(key.clone(), value.clone(), ttl);
                }
                self.finish_flight(&key, &flight);
                Ok(value)
            }
            Err(error) => {
                self.finish_flight(&key, &flight);
                Err(error)
            }
        }
    }

    fn lookup(&self, key: &str) -> Option<Value> {
        let now = Instant::now();
        let mut entries = self.entries.lock().expect("response cache mutex poisoned");

        match entries.get(key) {
            Some(entry) if now < entry.expires_at => Some(entry.value.clone()),
            Some(_) => {
                entries.remove(key);
                None
            }
            None => None,
        }
    }

    fn store(&self, key: String, value: Value, ttl: Duration) {
        let now = Instant::now();
        let mut entries = self.entries.lock().expect("response cache mutex poisoned");

        entries.retain(|_, entry| now < entry.expires_at);

        if entries.len() >= self.max_entries {
            // Filter combinations can grow the key space. Dropping the whole
            // cache is cheap here and bounds memory by construction.
            entries.clear();
        }

        entries.insert(
            key,
            CacheEntry {
                value,
                expires_at: now + ttl,
            },
        );
    }

    fn finish_flight(&self, key: &str, flight: &Arc<AsyncMutex<()>>) {
        let mut flights = self
            .flights
            .lock()
            .expect("response cache flight mutex poisoned");

        if flights
            .get(key)
            .is_some_and(|current| Arc::ptr_eq(current, flight))
        {
            flights.remove(key);
        }
    }

    /// Drop every cached aggregate. Used after imports, reveals,
    /// resets, removals and other playlist-level state changes.
    pub fn clear(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        self.entries
            .lock()
            .expect("response cache mutex poisoned")
            .clear();
    }
}

impl Default for ResponseCache {
    fn default() -> Self {
        Self::new(512)
    }
}

/// Short-lived cache for authenticated users.
///
/// Session revocation and account disabling remain enforced because every
/// admin action that changes those flags calls `invalidate`. The TTL is only a
/// safety net for changes made outside this process.
pub struct AuthUserCache {
    entries: Mutex<HashMap<Uuid, CachedUser>>,
    max_entries: usize,
}

struct CachedUser {
    user: User,
    expires_at: Instant,
}

impl AuthUserCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            max_entries,
        }
    }

    pub fn get(&self, id: Uuid) -> Option<User> {
        let now = Instant::now();
        let mut entries = self.entries.lock().expect("auth cache mutex poisoned");

        match entries.get(&id) {
            Some(entry) if now < entry.expires_at => Some(entry.user.clone()),
            Some(_) => {
                entries.remove(&id);
                None
            }
            None => None,
        }
    }

    pub fn insert(&self, user: User, ttl: Duration) {
        let now = Instant::now();
        let mut entries = self.entries.lock().expect("auth cache mutex poisoned");

        if entries.len() >= self.max_entries {
            entries.retain(|_, entry| now < entry.expires_at);
        }

        if entries.len() >= self.max_entries {
            entries.clear();
        }

        entries.insert(
            user.id,
            CachedUser {
                user,
                expires_at: now + ttl,
            },
        );
    }

    pub fn invalidate(&self, id: Uuid) {
        self.entries
            .lock()
            .expect("auth cache mutex poisoned")
            .remove(&id);
    }

    #[allow(dead_code)]
    pub fn clear(&self) {
        self.entries
            .lock()
            .expect("auth cache mutex poisoned")
            .clear();
    }
}

impl Default for AuthUserCache {
    fn default() -> Self {
        Self::new(4096)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn concurrent_misses_share_one_computation() {
        let cache = Arc::new(ResponseCache::new(8));
        let calls = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::new();

        for _ in 0..10 {
            let cache = cache.clone();
            let calls = calls.clone();
            handles.push(tokio::spawn(async move {
                cache
                    .get_or_compute("same-key".to_string(), Duration::from_secs(60), || {
                        let calls = calls.clone();
                        async move {
                            calls.fetch_add(1, Ordering::SeqCst);
                            tokio::time::sleep(Duration::from_millis(20)).await;
                            Ok::<Value, ApiError>(serde_json::json!({ "ok": true }))
                        }
                    })
                    .await
                    .unwrap()
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }

        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn clear_forces_a_fresh_computation() {
        let cache = ResponseCache::new(8);
        let calls = Arc::new(AtomicUsize::new(0));

        for expected in 1..=2 {
            let calls_for_compute = calls.clone();
            cache
                .get_or_compute("key".to_string(), Duration::from_secs(60), || async move {
                    calls_for_compute.fetch_add(1, Ordering::SeqCst);
                    Ok::<Value, ApiError>(serde_json::json!({ "value": expected }))
                })
                .await
                .unwrap();
            cache.clear();
        }

        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn cached_values_expire_after_the_ttl() {
        let cache = ResponseCache::new(8);
        let calls = Arc::new(AtomicUsize::new(0));

        for _ in 0..2 {
            let calls = calls.clone();
            cache
                .get_or_compute(
                    "short".to_string(),
                    Duration::from_millis(20),
                    || async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Ok::<Value, ApiError>(serde_json::json!({"ok": true}))
                    },
                )
                .await
                .unwrap();
            tokio::time::sleep(Duration::from_millis(30)).await;
        }

        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn failed_computations_are_not_cached() {
        let cache = ResponseCache::new(8);
        let calls = Arc::new(AtomicUsize::new(0));

        for attempt in 0..2 {
            let calls = calls.clone();
            let result = cache
                .get_or_compute(
                    "fails".to_string(),
                    Duration::from_secs(60),
                    || async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Err::<Value, ApiError>(ApiError::service_unavailable("nope"))
                    },
                )
                .await;
            assert!(result.is_err(), "attempt {attempt}");
        }

        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn a_failed_flight_does_not_block_the_next_request() {
        let cache = ResponseCache::new(8);
        let calls = Arc::new(AtomicUsize::new(0));

        let calls_for_failure = calls.clone();
        let failure = cache
            .get_or_compute(
                "retry".to_string(),
                Duration::from_secs(60),
                || async move {
                    calls_for_failure.fetch_add(1, Ordering::SeqCst);
                    Err::<Value, ApiError>(ApiError::bad_request("first"))
                },
            )
            .await;
        assert!(failure.is_err());

        let calls_for_success = calls.clone();
        let value = cache
            .get_or_compute(
                "retry".to_string(),
                Duration::from_secs(60),
                || async move {
                    calls_for_success.fetch_add(1, Ordering::SeqCst);
                    Ok::<Value, ApiError>(serde_json::json!({"attempt": 2}))
                },
            )
            .await
            .unwrap();

        assert_eq!(value["attempt"], 2);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn a_clear_during_computation_discards_the_result() {
        let cache = Arc::new(ResponseCache::new(8));
        let cache_for_task = cache.clone();
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_task = calls.clone();

        let handle = tokio::spawn(async move {
            cache_for_task
                .get_or_compute("race".to_string(), Duration::from_secs(60), || async move {
                    calls_for_task.fetch_add(1, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(30)).await;
                    Ok::<Value, ApiError>(serde_json::json!({"stale": true}))
                })
                .await
                .unwrap()
        });

        tokio::time::sleep(Duration::from_millis(5)).await;
        cache.clear();
        handle.await.unwrap();

        let calls_for_second = calls.clone();
        let value = cache
            .get_or_compute("race".to_string(), Duration::from_secs(60), || async move {
                calls_for_second.fetch_add(1, Ordering::SeqCst);
                Ok::<Value, ApiError>(serde_json::json!({"stale": false}))
            })
            .await
            .unwrap();

        assert_eq!(value["stale"], false);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn a_full_cache_drops_old_entries_instead_of_growing() {
        let cache = ResponseCache::new(2);
        for index in 0..3 {
            cache
                .get_or_compute(
                    format!("key-{index}"),
                    Duration::from_secs(60),
                    || async move { Ok::<Value, ApiError>(serde_json::json!({"index": index})) },
                )
                .await
                .unwrap();
        }

        // The third insert cleared the full cache; the newest entry remains.
        let value = cache
            .get_or_compute("key-2".to_string(), Duration::from_secs(60), || async {
                Ok::<Value, ApiError>(serde_json::json!({"computed": true}))
            })
            .await
            .unwrap();
        assert_eq!(value["index"], 2);
    }

    #[test]
    fn auth_cache_entries_expire() {
        let cache = AuthUserCache::new(8);
        let id = Uuid::new_v4();
        let user = User {
            id,
            username: "voter".to_string(),
            password_hash: String::new(),
            is_admin: false,
            created_at: chrono::Utc::now(),
            disabled_at: None,
            session_version: 0,
        };
        cache.insert(user, Duration::from_millis(1));
        std::thread::sleep(Duration::from_millis(5));
        assert!(cache.get(id).is_none());
    }

    #[test]
    fn auth_cache_clear_removes_every_user() {
        let cache = AuthUserCache::new(8);
        let first = Uuid::new_v4();
        let second = Uuid::new_v4();
        for (id, name) in [(first, "one"), (second, "two")] {
            cache.insert(
                User {
                    id,
                    username: name.to_string(),
                    password_hash: String::new(),
                    is_admin: false,
                    created_at: chrono::Utc::now(),
                    disabled_at: None,
                    session_version: 0,
                },
                Duration::from_secs(60),
            );
        }
        cache.clear();
        assert!(cache.get(first).is_none());
        assert!(cache.get(second).is_none());
    }

    #[test]
    fn auth_cache_stays_bounded() {
        let cache = AuthUserCache::new(2);
        for index in 0..3 {
            let id = Uuid::from_u128(index + 1);
            cache.insert(
                User {
                    id,
                    username: format!("voter-{index}"),
                    password_hash: String::new(),
                    is_admin: false,
                    created_at: chrono::Utc::now(),
                    disabled_at: None,
                    session_version: 0,
                },
                Duration::from_secs(60),
            );
        }
        assert!(cache.get(Uuid::from_u128(3)).is_some());
    }

    #[test]
    fn auth_cache_invalidation_removes_the_user() {
        let cache = AuthUserCache::new(8);
        let id = Uuid::new_v4();
        let user = User {
            id,
            username: "voter".to_string(),
            password_hash: String::new(),
            is_admin: false,
            created_at: chrono::Utc::now(),
            disabled_at: None,
            session_version: 0,
        };

        cache.insert(user, Duration::from_secs(60));
        assert!(cache.get(id).is_some());
        cache.invalidate(id);
        assert!(cache.get(id).is_none());
    }
}
