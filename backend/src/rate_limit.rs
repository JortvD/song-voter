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
    fn resets_after_the_window() {
        let limiter = RateLimiter::new();
        let now = Instant::now();
        let window = Duration::from_secs(60);

        assert!(limiter.allow_at("key", 1, window, now));
        assert!(!limiter.allow_at("key", 1, window, now));
        assert!(limiter.allow_at("key", 1, window, now + window));
    }
}
