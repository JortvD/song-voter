use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// A small fixed-window in-memory rate limiter.
///
/// Good enough for a single-container app: it protects login and registration
/// from brute-force attempts without adding an external dependency.
pub struct RateLimiter {
    buckets: Mutex<HashMap<String, Bucket>>,
}

struct Bucket {
    window_start: Instant,
    count: u32,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            buckets: Mutex::new(HashMap::new()),
        }
    }

    pub fn allow(&self, key: &str, limit: u32, window: Duration) -> bool {
        self.allow_at(key, limit, window, Instant::now())
    }

    fn allow_at(&self, key: &str, limit: u32, window: Duration, now: Instant) -> bool {
        let mut buckets = self.buckets.lock().expect("rate limiter poisoned");

        if buckets.len() > 4096 {
            buckets.retain(|_, bucket| now.duration_since(bucket.window_start) < window);
        }

        let bucket = buckets.entry(key.to_owned()).or_insert(Bucket {
            window_start: now,
            count: 0,
        });

        if now.duration_since(bucket.window_start) >= window {
            bucket.window_start = now;
            bucket.count = 0;
        }

        if bucket.count >= limit {
            return false;
        }

        bucket.count += 1;
        true
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_up_to_the_limit_and_then_blocks() {
        let limiter = RateLimiter::new();
        let now = Instant::now();

        for _ in 0..3 {
            assert!(limiter.allow_at("key", 3, Duration::from_secs(60), now));
        }
        assert!(!limiter.allow_at("key", 3, Duration::from_secs(60), now));
    }

    #[test]
    fn independent_keys_do_not_share_buckets() {
        let limiter = RateLimiter::new();
        let now = Instant::now();
        let window = Duration::from_secs(60);

        assert!(limiter.allow_at("one", 1, window, now));
        assert!(!limiter.allow_at("one", 1, window, now));
        assert!(limiter.allow_at("two", 1, window, now));
    }

    #[test]
    fn zero_limit_blocks_every_request() {
        let limiter = RateLimiter::new();
        let now = Instant::now();
        assert!(!limiter.allow_at("key", 0, Duration::from_secs(60), now));
        assert!(!limiter.allow_at("key", 0, Duration::from_secs(60), now));
    }

    #[test]
    fn a_fresh_window_starts_from_the_first_request() {
        let limiter = RateLimiter::new();
        let now = Instant::now();
        let window = Duration::from_secs(60);

        assert!(limiter.allow_at("key", 1, window, now));
        assert!(!limiter.allow_at("key", 1, window, now + Duration::from_secs(59)));
        assert!(limiter.allow_at("key", 1, window, now + Duration::from_secs(60)));
    }

    #[test]
    fn keys_are_case_and_whitespace_sensitive() {
        let limiter = RateLimiter::new();
        let now = Instant::now();
        let window = Duration::from_secs(60);
        for key in ["ip", "IP", " ip "] {
            assert!(limiter.allow_at(key, 1, window, now));
            assert!(!limiter.allow_at(key, 1, window, now));
        }
    }

    #[test]
    fn each_key_uses_its_own_limit_argument() {
        let limiter = RateLimiter::new();
        let now = Instant::now();
        let window = Duration::from_secs(60);
        assert!(limiter.allow_at("strict", 1, window, now));
        assert!(limiter.allow_at("loose", 5, window, now));
        assert!(limiter.allow_at("loose", 5, window, now));
    }

    #[test]
    fn expired_buckets_are_pruned_even_for_new_keys() {
        let limiter = RateLimiter::new();
        let now = Instant::now();
        let window = Duration::from_secs(1);
        for index in 0..4097 {
            assert!(limiter.allow_at(&format!("ip-{index}"), 10, window, now));
        }
        // This request prunes the expired entries before adding its own.
        assert!(limiter.allow_at("later", 1, window, now + Duration::from_secs(2)));
        assert!(!limiter.allow_at("later", 1, window, now + Duration::from_secs(2)));
    }

    #[test]
    fn resets_after_the_window() {
        let limiter = RateLimiter::new();
        let now = Instant::now();
        let window = Duration::from_secs(60);

        assert!(limiter.allow_at("key", 1, window, now));
        assert!(!limiter.allow_at("key", 1, window, now));
        assert!(limiter.allow_at("key", 1, window, now + window));
    }
}
