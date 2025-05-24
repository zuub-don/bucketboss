//! Core traits for the rate limiter.
//!
//! This module defines the core traits that implement different rate limiting algorithms.
//! The main trait is `RateLimiter`, which provides the basic interface for all rate limiters.

use core::time::Duration;

use crate::error::Result;

/// A trait for rate limiting algorithms.
///
/// This trait defines the core functionality that all rate limiters must implement.
/// It provides methods for checking if a request is allowed and for updating the rate limiter state.
pub trait RateLimiter: Send + Sync + 'static {
    /// Attempts to acquire the specified number of tokens.
    ///
    /// Returns `Ok(())` if the tokens were successfully acquired, or an error if the rate limit
    /// would be exceeded. The error will contain information about when the next token will be
    /// available.
    ///
    /// # Arguments
    ///
    /// * `tokens` - The number of tokens to acquire. This is typically 1 for simple rate limiting,
    ///   but could be higher for operations that consume multiple tokens.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the tokens were successfully acquired
    /// * `Err(RateLimitError::RateLimitExceeded)` if the rate limit would be exceeded
    /// * `Err(RateLimitError::InvalidConfiguration)` if the rate limiter is misconfigured
    fn try_acquire(&self, tokens: u32) -> Result<()>;

    /// Returns the number of tokens currently available.
    ///
    /// This is a non-consuming operation that doesn't affect the rate limiter state.
    /// It can be used to check the current rate limit status without consuming any tokens.
    fn available_tokens(&self) -> u32;

    /// Returns the maximum number of tokens that can be held in the bucket.
    ///
    /// This represents the burst capacity of the rate limiter.
    fn capacity(&self) -> u32;

    /// Returns the rate at which tokens are replenished, in tokens per second.
    fn rate_per_second(&self) -> f64;

    /// Returns the time until the next token will be available, in milliseconds.
    ///
    /// Returns `None` if tokens are currently available or if the rate limiter is empty.
    fn time_until_next_token_ms(&self) -> Option<u64>;

    /// Returns the time until the next token will be available as a `Duration`.
    ///
    /// Returns `None` if tokens are currently available or if the rate limiter is empty.
    fn time_until_next_token(&self) -> Option<Duration> {
        self.time_until_next_token_ms().map(Duration::from_millis)
    }
}

/// A trait for rate limiters that can be configured with a custom clock.
///
/// This is useful for testing or for environments where the system clock is not available.
pub trait WithClock<C>: Sized {
    /// Sets the clock implementation to use.
    ///
    /// # Arguments
    ///
    /// * `clock` - The clock implementation to use
    ///
    /// # Returns
    ///
    /// A new instance of the rate limiter with the specified clock.
    fn with_clock(self, clock: C) -> Self;
}

/// A trait for rate limiters that can be reconfigured.
pub trait ReconfigurableRateLimiter: RateLimiter {
    /// Updates the rate limiter configuration.
    ///
    /// # Arguments
    ///
    /// * `capacity` - The new maximum number of tokens the bucket can hold
    /// * `tokens_per_second` - The new rate at which tokens are replenished
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the configuration was successfully updated
    /// * `Err(RateLimitError::InvalidConfiguration)` if the new configuration is invalid
    fn update_config(&self, capacity: u32, tokens_per_second: f64) -> Result<()>;
}

/// A builder trait for creating rate limiters with a fluent interface.
pub trait RateLimiterBuilder: Sized {
    /// The type of rate limiter that will be built.
    type Limiter: RateLimiter;

    /// Sets the capacity of the rate limiter.
    ///
    /// The capacity is the maximum number of tokens the bucket can hold.
    /// This determines the maximum burst size.
    fn capacity(self, capacity: u32) -> Self;

    /// Sets the rate of the rate limiter in tokens per second.
    ///
    /// This determines how quickly tokens are replenished.
    fn tokens_per_second(self, tokens_per_second: f64) -> Self;

    /// Builds the rate limiter with the current configuration.
    ///
    /// # Returns
    ///
    /// A new instance of the rate limiter with the specified configuration.
    fn build(self) -> Result<Self::Limiter>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // A test implementation of RateLimiter for testing the traits
    struct TestRateLimiter {
        available: u32,
        capacity: u32,
        rate: f64,
    }

    impl RateLimiter for TestRateLimiter {
        fn try_acquire(&self, tokens: u32) -> Result<()> {
            if tokens <= self.available {
                Ok(())
            } else {
                Err(crate::error::RateLimitError::rate_limit_exceeded(
                    tokens,
                    self.available,
                    1000,
                ))
            }
        }

        fn available_tokens(&self) -> u32 {
            self.available
        }

        fn capacity(&self) -> u32 {
            self.capacity
        }

        fn rate_per_second(&self) -> f64 {
            self.rate
        }

        fn time_until_next_token_ms(&self) -> Option<u64> {
            if self.available > 0 {
                None
            } else {
                Some(1000)
            }
        }
    }

    #[test]
    fn test_rate_limiter_trait() {
        let limiter = TestRateLimiter {
            available: 5,
            capacity: 10,
            rate: 1.0,
        };

        assert_eq!(limiter.available_tokens(), 5);
        assert_eq!(limiter.capacity(), 10);
        assert_eq!(limiter.rate_per_second(), 1.0);

        // The TestRateLimiter doesn't track state, so all calls should work as long as tokens <= available
        assert!(limiter.try_acquire(3).is_ok());
        assert!(limiter.try_acquire(3).is_ok());
        // This should pass because we're not tracking state
        assert!(limiter.try_acquire(3).is_ok());

        // Verify that requesting more than available fails
        assert!(limiter.try_acquire(6).is_err());

        assert_eq!(limiter.time_until_next_token_ms(), None);
    }
}
