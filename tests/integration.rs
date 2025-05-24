//! Integration tests for the bucketboss crate.
//!
//! These tests verify that different components work together correctly
//! and that the public API behaves as expected.

use bucketboss::{
    clock::{Clock, MockClock},
    LeakyBucket, RateLimitError, RateLimiter, ReconfigurableRateLimiter, TokenBucket,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Test that the token bucket correctly enforces rate limits
#[test]
fn test_token_bucket_integration() {
    // Create a token bucket with capacity 5 and rate 10 tokens per second
    let clock = MockClock::new(0);
    let bucket = TokenBucket::with_clock(5, 10.0, clock.clone());

    // Should be able to acquire all 5 tokens immediately
    for _ in 0..5 {
        assert!(bucket.try_acquire(1).is_ok());
    }

    // Next acquire should fail (bucket is empty)
    assert!(matches!(
        bucket.try_acquire(1),
        Err(RateLimitError::RateLimitExceeded { .. })
    ));

    // Advance time by 100ms (1 token should be replenished)
    clock.advance(100);

    // Should be able to acquire 1 token
    assert!(bucket.try_acquire(1).is_ok());

    // Next acquire should fail again
    assert!(matches!(
        bucket.try_acquire(1),
        Err(RateLimitError::RateLimitExceeded { .. })
    ));
}

/// Test that the leaky bucket correctly enforces rate limits
#[test]
fn test_leaky_bucket_integration() {
    // Create a leaky bucket with rate 10 requests per second and burst of 5
    let clock = MockClock::new(0);
    let bucket = LeakyBucket::with_clock(10.0, Some(5), clock.clone());

    // Should be able to make 5 requests immediately (burst)
    for _ in 0..5 {
        assert!(bucket.try_acquire(1).is_ok());
    }

    // Next request should fail (burst limit reached)
    assert!(matches!(
        bucket.try_acquire(1),
        Err(RateLimitError::RateLimitExceeded { .. })
    ));

    // Advance time by 100ms (should process 1 request)
    clock.advance(100);

    // Should be able to make 1 more request
    assert!(bucket.try_acquire(1).is_ok());

    // Next request should fail again
    assert!(matches!(
        bucket.try_acquire(1),
        Err(RateLimitError::RateLimitExceeded { .. })
    ));
}

/// Test concurrent access to the token bucket
#[test]
fn test_token_bucket_concurrent() {
    // Create a token bucket with capacity 100 and high rate
    let clock = MockClock::new(0);
    let bucket = Arc::new(TokenBucket::with_clock(100, 1000.0, clock));

    let num_threads = 4;
    let requests_per_thread = 50;

    let success_count = Arc::new(AtomicU64::new(0));
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

    // Should have exactly 100 successful acquisitions (the bucket's capacity)
    assert_eq!(success_count.load(Ordering::Relaxed), 100);
}

/// Test that the rate limiters work with a custom clock
#[test]
fn test_custom_clock() {
    // Test with a custom clock
    let clock = TestClock::new(0);

    // Create rate limiters with the custom clock
    let tb = TokenBucket::with_clock(10, 1.0, clock.clone());

    // Test that the bucket starts with full capacity
    assert_eq!(tb.available_tokens(), 10);

    // Consume all tokens
    assert!(tb.try_acquire(10).is_ok());
    assert_eq!(tb.available_tokens(), 0);

    // Advance time by 1 second (should add 1 token)
    clock.advance(1000);
    assert_eq!(tb.available_tokens(), 1);

    // Test the leaky bucket with the same clock
    let lb = LeakyBucket::with_clock(1.0, Some(10), clock.clone());
    assert!(lb.try_acquire(1).is_ok());

    // Advance time and test again
    clock.advance(1000);
    assert!(lb.try_acquire(1).is_ok());
}

/// A test clock that can be advanced manually
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
