//! Property tests for the TokenBucket rate limiter.

use proptest::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use bucketboss::{
    clock::Clock, error::RateLimitError, RateLimiter, ReconfigurableRateLimiter, TokenBucket,
};

// A mock clock that can be advanced manually
#[derive(Debug, Clone)]
struct TestClock {
    now: Arc<AtomicU64>,
}

impl TestClock {
    fn new(initial_time: u64) -> Self {
        Self {
            now: Arc::new(AtomicU64::new(initial_time)),
        }
    }

    fn advance(&self, ms: u64) {
        let _ = self.now.fetch_add(ms, Ordering::SeqCst);
    }
}

impl Clock for TestClock {
    fn now(&self) -> u64 {
        self.now.load(Ordering::Relaxed)
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        // Run more cases for better coverage
        cases: 1000,
        ..ProptestConfig::default()
    })]

    #[test]
    fn test_token_bucket_rate_limiting(
        capacity in 1u32..1000u32,
        rate in 0.1f64..1000.0f64,
        requests in 1u32..10u32,
        time_advance in 0u64..2000u64,
    ) {
        let clock = TestClock::new(0);
        let bucket = TokenBucket::with_clock(capacity, rate, clock.clone());

        // Try to acquire all tokens at once
        let result = bucket.try_acquire(requests);

        if requests <= capacity {
            // Should succeed if within capacity
            assert!(result.is_ok());
            assert_eq!(bucket.available_tokens(), capacity - requests);
        } else {
            // Should fail if exceeding capacity
            assert!(matches!(result, Err(RateLimitError::RateLimitExceeded { .. })));
        }

        // Advance time and check token refill
        clock.advance(time_advance);

        // Calculate expected tokens after time advance
        // We need to account for the fact that tokens are added based on elapsed time
        // since the last update, and the last update time is updated when tokens are added
        let elapsed_ms = time_advance as f64;
        let ms_per_token = 1000.0 / rate;
        let tokens_to_add = (elapsed_ms / ms_per_token) as u64;

        // The expected tokens should be the minimum of:
        // 1. The initial tokens (capacity - requests) plus the tokens added over time
        // 2. The bucket capacity
        let initial_tokens = capacity.saturating_sub(requests) as u64;
        let expected_tokens = (initial_tokens + tokens_to_add).min(capacity as u64) as u32;

        // Check available tokens with a more lenient tolerance for floating-point inaccuracies
        let actual_tokens = bucket.available_tokens();
        let diff = (actual_tokens as i32 - expected_tokens as i32).abs();
        // Allow for a small difference due to floating-point precision and timing
        let max_diff = 2;
        assert!(
            diff <= max_diff,
            "Available tokens mismatch: expected {}, got {} (diff: {}, max allowed: {})",
            expected_tokens,
            actual_tokens,
            diff,
            max_diff
        );
    }

    #[test]
    fn test_token_bucket_never_exceeds_capacity(
        capacity in 1u32..1000u32,
        rate in 0.1f64..1000.0f64,
        time_advances in proptest::collection::vec(0u64..1000u64, 1..10),
    ) {
        let clock = TestClock::new(0);
        let bucket = TokenBucket::with_clock(capacity, rate, clock.clone());

        let mut total_time = 0;

        for advance in time_advances {
            clock.advance(advance);
            total_time += advance;

            // Available tokens should never exceed capacity
            let tokens = bucket.available_tokens();
            assert!(tokens <= capacity, "Tokens ({}) exceeded capacity ({}) after {}ms", tokens, capacity, total_time);

            // Try to acquire all available tokens
            if tokens > 0 {
                assert!(bucket.try_acquire(tokens).is_ok());
                assert_eq!(bucket.available_tokens(), 0);
            }
        }
    }

    #[test]
    fn test_token_bucket_config_updates(
        initial_cap in 1u32..1000u32,
        initial_rate in 0.1f64..1000.0f64,
        new_cap in 1u32..1000u32,
        new_rate in 0.1f64..1000.0f64,
    ) {
        let clock = TestClock::new(0);
        let bucket = TokenBucket::with_clock(initial_cap, initial_rate, clock.clone());

        // Update config
        bucket.update_config(new_cap, new_rate).unwrap();

        // Check new configuration
        assert_eq!(bucket.capacity(), new_cap);
        assert!((bucket.rate_per_second() - new_rate).abs() < f64::EPSILON);

        // Check that available tokens were capped to new capacity
        let available = bucket.available_tokens();
        assert!(available <= new_cap, "Available tokens ({}) exceed new capacity ({})", available, new_cap);
    }

    #[test]
    fn test_token_bucket_concurrent_access(
        capacity in 100u32..1000u32,
        rate in 10.0f64..1000.0f64,
        num_threads in 1usize..8usize,
        requests_per_thread in 1u32..100u32,
    ) {
        use std::sync::Arc;
        use std::thread;
        use std::sync::atomic::{AtomicU32, Ordering};

        let clock = Arc::new(TestClock::new(0));
        let bucket = Arc::new(TokenBucket::with_clock(capacity, rate, clock.as_ref().clone()));

        let success_count = Arc::new(AtomicU32::new(0));
        let mut handles = vec![];

        for _ in 0..num_threads {
            let bucket = bucket.clone();
            let success_count = success_count.clone();

            let handle = thread::spawn(move || {
                for _ in 0..requests_per_thread {
                    if bucket.try_acquire(1).is_ok() {
                        success_count.fetch_add(1, Ordering::Relaxed);
                    }
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Total successful acquisitions should not exceed capacity
        let total_success = success_count.load(Ordering::Relaxed);
        assert!(total_success <= capacity, "Total successes ({}) exceeded capacity ({})", total_success, capacity);

        // If we didn't have enough capacity, we should have exactly `capacity` successes
        if (num_threads as u32 * requests_per_thread) > capacity {
            assert_eq!(total_success, capacity, "Should have exactly capacity successes when oversubscribed");
        } else {
            assert_eq!(total_success, num_threads as u32 * requests_per_thread, "Should have all requests succeed when under capacity");
        }
    }
}
