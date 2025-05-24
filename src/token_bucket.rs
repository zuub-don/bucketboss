//! Token Bucket rate limiting algorithm implementation.
//!
//! The Token Bucket algorithm allows for a certain number of tokens to be consumed over time,
//! with tokens being replenished at a fixed rate. This allows for bursts of traffic up to the
//! bucket's capacity, followed by a steady rate of traffic.

use core::{
    f64,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::{
    clock::{Clock, SystemClock},
    error::{RateLimitError, Result},
    traits::{RateLimiter, ReconfigurableRateLimiter, WithClock},
};

// Helper functions for atomic float operations
fn f64_to_u64(value: f64) -> u64 {
    value.to_bits()
}

fn u64_to_f64(value: u64) -> f64 {
    f64::from_bits(value)
}

/// A thread-safe token bucket rate limiter.
///
/// This implementation uses atomic operations to ensure thread safety without requiring
/// external synchronization. It's designed for high throughput and low latency.
#[derive(Debug)]
pub struct TokenBucket<C = SystemClock> {
    /// The clock used to track time.
    clock: C,
    /// The maximum number of tokens the bucket can hold.
    capacity: AtomicU64,
    /// The number of tokens added per second (stored as bits of f64).
    tokens_per_second: AtomicU64,
    /// The time in milliseconds between adding each token (stored as bits of f64).
    ms_per_token: AtomicU64,
    /// The current number of tokens in the bucket.
    tokens: AtomicU64,
    /// The last time the token count was updated.
    last_update: AtomicU64,
}

impl TokenBucket<SystemClock> {
    /// Creates a new `TokenBucket` with the specified capacity and rate.
    ///
    /// # Arguments
    ///
    /// * `capacity` - The maximum number of tokens the bucket can hold.
    /// * `tokens_per_second` - The rate at which tokens are replenished, in tokens per second.
    ///
    /// # Returns
    ///
    /// A new `TokenBucket` instance.
    ///
    /// # Panics
    ///
    /// Panics if `capacity` is 0 or if `tokens_per_second` is not positive.
    pub fn new(capacity: u32, tokens_per_second: f64) -> Self {
        assert!(capacity > 0, "capacity must be greater than 0");
        assert!(
            tokens_per_second > 0.0,
            "tokens_per_second must be positive"
        );

        let now = SystemClock.now();
        let ms_per_token = 1000.0 / tokens_per_second;

        Self {
            capacity: AtomicU64::new(capacity as u64),
            tokens_per_second: AtomicU64::new(f64_to_u64(tokens_per_second)),
            ms_per_token: AtomicU64::new(f64_to_u64(ms_per_token)),
            clock: SystemClock,
            tokens: AtomicU64::new(capacity as u64),
            last_update: AtomicU64::new(now),
        }
    }
}

impl<C> TokenBucket<C>
where
    C: Clock,
{
    /// Creates a new `TokenBucket` with the specified clock.
    ///
    /// This is useful for testing or for environments where you need to control time.
    pub fn with_clock(capacity: u32, tokens_per_second: f64, clock: C) -> Self {
        assert!(capacity > 0, "capacity must be greater than 0");
        assert!(
            tokens_per_second > 0.0,
            "tokens_per_second must be positive"
        );

        let now = clock.now();
        let ms_per_token = 1000.0 / tokens_per_second;

        Self {
            capacity: AtomicU64::new(capacity as u64),
            tokens_per_second: AtomicU64::new(f64_to_u64(tokens_per_second)),
            ms_per_token: AtomicU64::new(f64_to_u64(ms_per_token)),
            clock,
            tokens: AtomicU64::new(capacity as u64),
            last_update: AtomicU64::new(now),
        }
    }

    /// Updates the internal state of the token bucket based on the current time.
    ///
    /// This method is called internally by `try_acquire` and `available_tokens`
    /// to ensure the token count is up to date.
    fn update_state(&self, now: u64) -> u32 {
        let last = self.last_update.load(Ordering::Acquire);
        let elapsed = now.saturating_sub(last);

        if elapsed == 0 {
            return self.tokens.load(Ordering::Relaxed) as u32;
        }

        // Get the current ms_per_token as f64
        let ms_per_token = u64_to_f64(self.ms_per_token.load(Ordering::Acquire));

        // Calculate how many tokens to add based on elapsed time
        let tokens_to_add = if ms_per_token > 0.0 {
            (elapsed as f64 / ms_per_token) as u64
        } else {
            0
        };

        if tokens_to_add == 0 {
            return self.tokens.load(Ordering::Relaxed) as u32;
        }

        // Update the last update time
        self.last_update.store(now, Ordering::Release);

        // Add the tokens, but don't exceed capacity
        let current_tokens = self.tokens.load(Ordering::Relaxed);
        let capacity = self.capacity.load(Ordering::Acquire);
        let new_tokens = current_tokens.saturating_add(tokens_to_add);
        let capped_tokens = new_tokens.min(capacity);

        // Store the new token count
        self.tokens.store(capped_tokens, Ordering::Release);

        capped_tokens as u32
    }

    /// Updates the rate and capacity of the token bucket.
    ///
    /// # Arguments
    ///
    /// * `capacity` - The new capacity of the bucket (maximum tokens).
    /// * `tokens_per_second` - The new rate at which tokens are added to the bucket.
    fn set_rate(&self, capacity: u32, tokens_per_second: f64) {
        // Update the atomic values
        self.capacity.store(capacity as u64, Ordering::Release);
        self.tokens_per_second
            .store(f64_to_u64(tokens_per_second), Ordering::Release);

        // Calculate and store the new ms_per_token
        let ms_per_token = if tokens_per_second > 0.0 {
            1000.0 / tokens_per_second
        } else {
            0.0
        };
        self.ms_per_token
            .store(f64_to_u64(ms_per_token), Ordering::Release);
    }
}

impl<C> RateLimiter for TokenBucket<C>
where
    C: Clock,
{
    fn try_acquire(&self, tokens: u32) -> Result<()> {
        if tokens == 0 {
            return Ok(());
        }

        let now = self.clock.now();
        let current_tokens = self.update_state(now);

        if tokens > current_tokens {
            let tokens_needed = tokens - current_tokens;
            let ms_per_token = u64_to_f64(self.ms_per_token.load(Ordering::Acquire));
            let wait_ms = (tokens_needed as f64 * ms_per_token).ceil() as u64;

            return Err(RateLimitError::rate_limit_exceeded(
                tokens,
                current_tokens,
                wait_ms,
            ));
        }

        // Try to acquire the tokens
        let new_tokens = current_tokens - tokens;
        if self
            .tokens
            .compare_exchange(
                current_tokens as u64,
                new_tokens as u64,
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
        self.update_state(now)
    }

    fn capacity(&self) -> u32 {
        self.capacity.load(Ordering::Acquire) as u32
    }

    fn rate_per_second(&self) -> f64 {
        u64_to_f64(self.tokens_per_second.load(Ordering::Acquire))
    }

    fn time_until_next_token_ms(&self) -> Option<u64> {
        let now = self.clock.now();
        let last_update = self.last_update.load(Ordering::Acquire);
        let ms_per_token = u64_to_f64(self.ms_per_token.load(Ordering::Acquire));

        if ms_per_token == 0.0 {
            return None;
        }

        let next_token_time = last_update + ms_per_token.ceil() as u64;
        if next_token_time > now {
            Some(next_token_time - now)
        } else {
            None
        }
    }
}

impl<C> ReconfigurableRateLimiter for TokenBucket<C>
where
    C: Clock,
{
    fn update_config(&self, capacity: u32, tokens_per_second: f64) -> Result<()> {
        if capacity == 0 {
            return Err(RateLimitError::invalid_config(
                "capacity must be greater than 0",
            ));
        }
        if tokens_per_second <= 0.0 {
            return Err(RateLimitError::invalid_config(
                "tokens_per_second must be positive",
            ));
        }

        let now = self.clock.now();
        let _ = self.update_state(now);

        // Update the rate and capacity first
        self.set_rate(capacity, tokens_per_second);

        // Then update the available tokens to the new capacity
        self.tokens.store(capacity as u64, Ordering::Release);

        Ok(())
    }
}

impl<C> WithClock<C> for TokenBucket<C> {
    fn with_clock(self, clock: C) -> Self {
        TokenBucket {
            capacity: self.capacity,
            tokens_per_second: self.tokens_per_second,
            ms_per_token: self.ms_per_token,
            clock,
            tokens: self.tokens,
            last_update: self.last_update,
        }
    }
}

impl<C> Default for TokenBucket<C>
where
    C: Clock + Default,
{
    fn default() -> Self {
        Self::with_clock(1, 1.0, C::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_token_bucket_acquire() {
        let bucket = TokenBucket::new(10, 1.0);

        // Should be able to acquire up to capacity
        assert!(bucket.try_acquire(10).is_ok());

        // Should not be able to acquire more than capacity
        assert!(bucket.try_acquire(1).is_err());

        // After 1 second, should be able to acquire 1 more token
        std::thread::sleep(Duration::from_millis(1100));
        assert!(bucket.try_acquire(1).is_ok());
    }

    #[test]
    fn test_token_bucket_update_config() {
        let bucket = TokenBucket::new(10, 1.0);

        // Should start with 10 tokens
        assert_eq!(bucket.available_tokens(), 10);

        // Update to higher capacity and rate
        assert!(bucket.update_config(20, 2.0).is_ok());

        // Should now have 20 tokens available
        assert_eq!(bucket.available_tokens(), 20);

        // Should be able to acquire up to new capacity
        assert!(bucket.try_acquire(20).is_ok());

        // After 1 second, should be able to acquire 2 more tokens (2 tokens/sec)
        std::thread::sleep(Duration::from_millis(1100));
        assert_eq!(bucket.available_tokens(), 2);
        assert!(bucket.try_acquire(2).is_ok());
    }
}
