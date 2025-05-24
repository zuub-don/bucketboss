//! Leaky Bucket rate limiting algorithm implementation.
//!
//! The Leaky Bucket algorithm enforces a strict rate limit by simulating a leaky bucket
//! where requests are added to the bucket and processed at a constant rate. This provides
//! a smoother traffic pattern compared to Token Bucket.

use crate::{
    clock::{Clock, SystemClock},
    error::{RateLimitError, Result},
    traits::{RateLimiter, ReconfigurableRateLimiter, WithClock},
};
use std::sync::atomic::{AtomicU64, Ordering};

// Helper functions for atomic float operations
fn f64_to_u64(value: f64) -> u64 {
    value.to_bits()
}

fn u64_to_f64(value: u64) -> f64 {
    f64::from_bits(value)
}

/// A thread-safe leaky bucket rate limiter.
///
/// This implementation uses atomic operations to ensure thread safety without requiring
/// external synchronization. It's designed for high throughput and low latency.
#[derive(Debug)]
pub struct LeakyBucket<C = SystemClock> {
    /// The clock used to track time.
    clock: C,
    /// The capacity of the bucket (maximum burst size).
    capacity: AtomicU64,
    /// The time in milliseconds between processing each request (stored as bits of f64).
    ms_per_request: AtomicU64,
    /// The time in milliseconds when the next request is allowed.
    next_allowed_time: AtomicU64,
    /// The current number of requests in the bucket.
    current_level: AtomicU64,
}

impl LeakyBucket<SystemClock> {
    /// Creates a new `LeakyBucket` with the specified rate and optional burst size.
    ///
    /// # Arguments
    ///
    /// * `requests_per_second` - The maximum rate of requests allowed, in requests per second.
    /// * `burst_size` - The maximum number of requests that can be bursted. If `None`,
    ///   it defaults to 1, which means no burst is allowed.
    ///
    /// # Panics
    ///
    /// Panics if `requests_per_second` is zero or if `burst_size` is zero.
    pub fn new(requests_per_second: f64, burst_size: Option<u32>) -> Self {
        assert!(
            requests_per_second > 0.0,
            "requests_per_second must be positive"
        );
        let burst_size = burst_size.unwrap_or(1);
        assert!(burst_size > 0, "burst_size must be greater than 0");

        let now = SystemClock.now();
        let ms_per_request = 1000.0 / requests_per_second;

        Self {
            capacity: AtomicU64::new(burst_size as u64),
            ms_per_request: AtomicU64::new(f64_to_u64(ms_per_request)),
            next_allowed_time: AtomicU64::new(now),
            current_level: AtomicU64::new(0),
            clock: SystemClock,
        }
    }

    /// Creates a new `LeakyBucket` that allows one request per second.
    ///
    /// This is equivalent to calling `new(1.0, None)`.
    pub fn one_per_second() -> Self {
        Self::new(1.0, None)
    }
}

impl<C> LeakyBucket<C>
where
    C: Clock,
{
    /// Creates a new `LeakyBucket` with the specified clock.
    ///
    /// This is useful for testing or for environments where you need to control time.
    pub fn with_clock(requests_per_second: f64, burst_size: Option<u32>, clock: C) -> Self {
        assert!(
            requests_per_second > 0.0,
            "requests_per_second must be positive"
        );
        let burst_size = burst_size.unwrap_or(1);
        assert!(burst_size > 0, "burst_size must be greater than 0");

        let now = clock.now();
        let ms_per_request = 1000.0 / requests_per_second;

        Self {
            capacity: AtomicU64::new(burst_size as u64),
            ms_per_request: AtomicU64::new(f64_to_u64(ms_per_request)),
            next_allowed_time: AtomicU64::new(now),
            current_level: AtomicU64::new(0),
            clock,
        }
    }

    /// Updates the internal state of the leaky bucket based on the current time.
    fn update_state(&self, now: u64) -> (u64, u64) {
        let mut current_level = self.current_level.load(Ordering::Relaxed);
        let mut next_allowed = self.next_allowed_time.load(Ordering::Acquire);
        let ms_per_request = u64_to_f64(self.ms_per_request.load(Ordering::Acquire));
        // Explicitly ignore the capacity load to prevent rate limit violations
        let _ = self.capacity.load(Ordering::Acquire);

        loop {
            // If there are no requests in the bucket, reset the state
            if current_level == 0 {
                return (0, next_allowed);
            }

            // Calculate how much time has passed since the last update
            let elapsed = now.saturating_sub(next_allowed);
            if elapsed == 0 {
                // No time has passed, state is up to date
                return (current_level, next_allowed);
            }

            // Calculate how many requests could have been processed in the elapsed time
            let processed = if ms_per_request > 0.0 {
                (elapsed as f64 / ms_per_request) as u64
            } else {
                current_level // If ms_per_request is 0, process all requests
            };

            if processed >= current_level {
                // All requests have been processed
                if self
                    .current_level
                    .compare_exchange(current_level, 0, Ordering::Release, Ordering::Relaxed)
                    .is_ok()
                {
                    // Update the next_allowed_time to be now
                    let new_next = now + ms_per_request as u64;
                    self.next_allowed_time.store(new_next, Ordering::Release);
                    return (0, new_next);
                }
            } else {
                // Some requests remain in the bucket
                let new_level = current_level - processed;
                let new_next = next_allowed + (processed as f64 * ms_per_request) as u64;

                // Try to update the state atomically
                if self
                    .current_level
                    .compare_exchange_weak(
                        current_level,
                        new_level,
                        Ordering::Release,
                        Ordering::Relaxed,
                    )
                    .is_ok()
                {
                    self.next_allowed_time.store(new_next, Ordering::Release);
                    return (new_level, new_next);
                }
            }

            // If we get here, the state changed and we need to retry
            current_level = self.current_level.load(Ordering::Relaxed);
            next_allowed = self.next_allowed_time.load(Ordering::Acquire);
        }
    }

    /// Updates the rate and capacity of the leaky bucket.
    ///
    /// # Arguments
    ///
    /// * `capacity` - The new capacity of the bucket (maximum burst size).
    /// * `requests_per_second` - The new rate of requests allowed, in requests per second.
    fn set_rate(&self, capacity: u64, requests_per_second: f64) {
        // Calculate the new ms_per_request value
        let ms_per_request = if requests_per_second > 0.0 {
            1000.0 / requests_per_second
        } else {
            0.0
        };

        // Store the new values atomically
        self.capacity.store(capacity, Ordering::Release);
        self.ms_per_request
            .store(f64_to_u64(ms_per_request), Ordering::Release);

        // Update the next_allowed_time to prevent rate limit violations
        let now = self.clock.now();
        // We don't need the next_allowed value here, so we can ignore it
        let (current_level, _) = self.update_state(now);

        // If the bucket is empty, reset the next_allowed_time to now
        if current_level == 0 {
            self.next_allowed_time.store(now, Ordering::Release);
        } else {
            // Otherwise, ensure next_allowed_time is not in the past
            let current_next = self.next_allowed_time.load(Ordering::Acquire);
            if current_next < now {
                self.next_allowed_time.store(now, Ordering::Release);
            }
        }
    }
}

impl<C> RateLimiter for LeakyBucket<C>
where
    C: Clock,
{
    fn try_acquire(&self, tokens: u32) -> Result<()> {
        if tokens == 0 {
            return Ok(());
        }

        let capacity = self.capacity.load(Ordering::Acquire);

        // Check if the request exceeds the bucket capacity
        if tokens > capacity as u32 {
            return Err(RateLimitError::rate_limit_exceeded(
                tokens,
                capacity as u32,
                0, // No wait time since the request is immediately rejected
            ));
        }

        let now = self.clock.now();
        // We don't need the next_allowed value here, so we can ignore it
        let (current_level, _) = self.update_state(now);

        // Check if we have enough capacity
        if current_level + (tokens as u64) > capacity {
            // Calculate wait time based on the current rate
            let ms_per_request = u64_to_f64(self.ms_per_request.load(Ordering::Acquire));
            let wait_ms = if ms_per_request > 0.0 {
                ((current_level + tokens as u64 - capacity) as f64 * ms_per_request).ceil() as u64
            } else {
                0
            };

            return Err(RateLimitError::rate_limit_exceeded(
                tokens,
                capacity.saturating_sub(current_level) as u32,
                wait_ms,
            ));
        }

        // Try to acquire the tokens
        let new_level = current_level + tokens as u64;
        if self
            .current_level
            .compare_exchange(
                current_level,
                new_level,
                Ordering::AcqRel,
                Ordering::Relaxed,
            )
            .is_ok()
        {
            Ok(())
        } else {
            // If we couldn't update atomically, retry the whole operation
            self.try_acquire(tokens)
        }
    }

    fn available_tokens(&self) -> u32 {
        let now = self.clock.now();
        let (current_level, _) = self.update_state(now);
        self.capacity
            .load(Ordering::Acquire)
            .saturating_sub(current_level) as u32
    }

    fn capacity(&self) -> u32 {
        self.capacity.load(Ordering::Acquire) as u32
    }

    fn rate_per_second(&self) -> f64 {
        let ms_per_request = u64_to_f64(self.ms_per_request.load(Ordering::Acquire));
        if ms_per_request > 0.0 {
            let rate = 1000.0 / ms_per_request;
            // Round to 6 decimal places to handle floating-point precision issues
            (rate * 1_000_000.0).round() / 1_000_000.0
        } else {
            0.0
        }
    }

    fn time_until_next_token_ms(&self) -> Option<u64> {
        let now = self.clock.now();
        let next_allowed = self.next_allowed_time.load(Ordering::Acquire);

        if next_allowed > now {
            Some(next_allowed - now)
        } else {
            None
        }
    }
}

impl<C> ReconfigurableRateLimiter for LeakyBucket<C>
where
    C: Clock,
{
    fn update_config(&self, capacity: u32, requests_per_second: f64) -> Result<()> {
        if capacity == 0 {
            return Err(RateLimitError::invalid_config(
                "capacity must be greater than 0",
            ));
        }
        if requests_per_second <= 0.0 {
            return Err(RateLimitError::invalid_config(
                "requests_per_second must be positive",
            ));
        }

        let now = self.clock.now();

        // Update the state first to process any pending requests
        let _ = self.update_state(now);

        // Update the rate and capacity
        self.set_rate(capacity as u64, requests_per_second);

        // Cap the current level to the new capacity
        let current_level = self.current_level.load(Ordering::Relaxed);
        if current_level > capacity as u64 {
            self.current_level.store(capacity as u64, Ordering::Release);
        }

        Ok(())
    }
}

impl<C> WithClock<C> for LeakyBucket<C> {
    fn with_clock(self, clock: C) -> Self {
        LeakyBucket {
            clock,
            capacity: self.capacity,
            ms_per_request: self.ms_per_request,
            next_allowed_time: self.next_allowed_time,
            current_level: self.current_level,
        }
    }
}

impl<C> Default for LeakyBucket<C>
where
    C: Clock + Default,
{
    fn default() -> Self {
        Self::with_clock(1.0, Some(1), C::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_leaky_bucket_acquire() {
        let bucket = LeakyBucket::new(1.0, Some(10));

        // Should be able to acquire up to capacity
        assert!(bucket.try_acquire(10).is_ok());

        // Should not be able to acquire more than capacity
        assert!(bucket.try_acquire(1).is_err());

        // After 1 second, should be able to acquire 1 more token
        std::thread::sleep(Duration::from_millis(1100));
        assert!(bucket.try_acquire(1).is_ok());
    }

    #[test]
    fn test_leaky_bucket_update_config() {
        let bucket = LeakyBucket::new(1.0, Some(10));

        // Update to higher capacity and rate
        assert!(bucket.update_config(20, 2.0).is_ok());

        // Should be able to acquire up to new capacity
        assert!(bucket.try_acquire(20).is_ok());

        // After 1 second, should be able to acquire 2 more tokens
        std::thread::sleep(Duration::from_millis(1100));
        assert!(bucket.try_acquire(2).is_ok());
    }
}
