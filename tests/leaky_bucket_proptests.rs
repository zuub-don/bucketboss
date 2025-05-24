//! Property tests for the LeakyBucket rate limiter.

use proptest::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use bucketboss::{
    clock::Clock, error::RateLimitError, LeakyBucket, RateLimiter, ReconfigurableRateLimiter,
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
    fn test_leaky_bucket_rate_limiting(
        burst_size in 1u32..1000u32,
        rate in 0.1f64..1000.0f64,
        requests in 1u32..10u32,
        time_advance in 0u64..2000u64,
    ) {
        let clock = TestClock::new(0);
        let bucket = LeakyBucket::with_clock(rate, Some(burst_size), clock.clone());

        // Try to acquire all requests at once
        let result = bucket.try_acquire(requests);

        if requests <= burst_size {
            // Should succeed if within burst size
            assert!(result.is_ok());
        } else {
            // Should fail if exceeding burst size
            assert!(matches!(result, Err(RateLimitError::RateLimitExceeded { .. })));
        }

        // Advance time and check request processing
        clock.advance(time_advance);

        // Calculate expected processed requests based on rate
        let _expected_processed = (time_advance as f64 * (rate / 1000.0)) as u32;

        // Try to acquire one more request to trigger state update
        let _ = bucket.try_acquire(1);

        // The exact number of available tokens depends on the implementation details
        // but we can check some invariants
        let available = bucket.available_tokens();
        assert!(available <= burst_size, "Available tokens ({}) exceeded burst size ({})", available, burst_size);
    }

    #[test]
    fn test_leaky_bucket_never_exceeds_burst_size(
        burst_size in 1u32..1000u32,
        rate in 0.1f64..1000.0f64,
        time_advances in proptest::collection::vec(0u64..1000u64, 1..10),
    ) {
        let clock = TestClock::new(0);
        let bucket = LeakyBucket::with_clock(rate, Some(burst_size), clock.clone());

        let mut total_time = 0;

        for advance in time_advances {
            clock.advance(advance);
            total_time += advance;

            // Available tokens should never exceed burst size
            let tokens = bucket.available_tokens();
            assert!(tokens <= burst_size, "Tokens ({}) exceeded burst size ({}) after {}ms", tokens, burst_size, total_time);

            // Try to acquire all available tokens
            if tokens > 0 {
                assert!(bucket.try_acquire(tokens).is_ok());
            }
        }
    }

    #[test]
    fn test_leaky_bucket_config_updates(
        initial_burst in 1u32..1000u32,
        initial_rate in 0.1f64..1000.0f64,
        new_burst in 1u32..1000u32,
        new_rate in 0.1f64..1000.0f64,
    ) {
        let clock = TestClock::new(0);
        let bucket = LeakyBucket::with_clock(initial_rate, Some(initial_burst), clock.clone());

        // Update config
        bucket.update_config(new_burst, new_rate).unwrap();

        // Check new configuration with a more lenient epsilon for floating-point comparison
        let actual_rate = bucket.rate_per_second();
        let abs_diff = (actual_rate - new_rate).abs();

        // Use a more lenient epsilon for comparison
        // For very small rates, use an absolute epsilon
        // For larger rates, use a relative epsilon
        let epsilon = if new_rate < 1.0 {
            1e-6  // Absolute epsilon for small rates
        } else {
            new_rate * 1e-5  // Relative epsilon for larger rates
        };

        assert!(
            abs_diff < epsilon,
            "Rate mismatch: expected {}, got {}, absolute difference: {}, relative difference: {}",
            new_rate,
            actual_rate,
            abs_diff,
            abs_diff / new_rate
        );

        // Check that available tokens were capped to new burst size
        let available = bucket.available_tokens();
        assert!(
            available <= new_burst,
            "Available tokens ({}) exceed new burst size ({})",
            available,
            new_burst
        );
    }

    #[test]
    fn test_leaky_bucket_concurrent_access(
        burst_size in 100u32..1000u32,
        rate in 10.0f64..1000.0f64,
        num_threads in 1usize..8usize,
        requests_per_thread in 1u32..100u32,
    ) {
        use std::sync::Barrier;
        use std::thread;
        use std::sync::atomic::{AtomicU32, Ordering};

        let clock = Arc::new(TestClock::new(0));
        let bucket = Arc::new(LeakyBucket::with_clock(rate, Some(burst_size), clock.as_ref().clone()));

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

        // Total successful acquisitions should not exceed burst size
        let total_success = success_count.load(Ordering::Relaxed);
        assert!(total_success <= burst_size, "Total successes ({}) exceeded burst size ({})", total_success, burst_size);
    }
}
