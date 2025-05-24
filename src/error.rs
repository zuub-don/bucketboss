//! Error types for the rate limiter.
//!
//! This module defines the error types used throughout the crate, including
//! rate limit exceeded errors and configuration errors.

use core::fmt;

/// The error type for rate limiting operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitError {
    /// The rate limit has been exceeded.
    RateLimitExceeded {
        /// The number of tokens that were requested.
        requested: u32,
        /// The number of tokens currently available.
        available: u32,
        /// The time in milliseconds until the next token becomes available.
        retry_after_ms: u64,
    },
    /// The requested configuration is invalid.
    InvalidConfiguration {
        /// A description of what made the configuration invalid.
        reason: &'static str,
    },
}

impl RateLimitError {
    /// Creates a new `RateLimitExceeded` error.
    pub fn rate_limit_exceeded(requested: u32, available: u32, retry_after_ms: u64) -> Self {
        Self::RateLimitExceeded {
            requested,
            available,
            retry_after_ms,
        }
    }

    /// Creates a new `InvalidConfiguration` error.
    pub fn invalid_config(reason: &'static str) -> Self {
        Self::InvalidConfiguration { reason }
    }

    /// Returns whether this error indicates a rate limit was exceeded.
    pub fn is_rate_limit_exceeded(&self) -> bool {
        matches!(self, Self::RateLimitExceeded { .. })
    }

    /// Returns whether this error indicates an invalid configuration.
    pub fn is_invalid_config(&self) -> bool {
        matches!(self, Self::InvalidConfiguration { .. })
    }

    /// If this is a `RateLimitExceeded` error, returns the retry-after duration in milliseconds.
    pub fn retry_after_ms(&self) -> Option<u64> {
        match self {
            Self::RateLimitExceeded { retry_after_ms, .. } => Some(*retry_after_ms),
            _ => None,
        }
    }
}

impl fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RateLimitExceeded {
                requested,
                available,
                retry_after_ms,
            } => write!(
                f,
                "rate limit exceeded: requested {} tokens, but only {} available (retry after {}ms)",
                requested, available, retry_after_ms
            ),
            Self::InvalidConfiguration { reason } => write!(f, "invalid configuration: {}", reason),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for RateLimitError {}

/// A specialized `Result` type for rate limiting operations.
pub type Result<T> = core::result::Result<T, RateLimitError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_exceeded() {
        let err = RateLimitError::rate_limit_exceeded(5, 2, 1000);
        assert!(err.is_rate_limit_exceeded());
        assert!(!err.is_invalid_config());
        assert_eq!(err.retry_after_ms(), Some(1000));
        assert_eq!(
            err.to_string(),
            "rate limit exceeded: requested 5 tokens, but only 2 available (retry after 1000ms)"
        );
    }

    #[test]
    fn test_invalid_config() {
        let err = RateLimitError::invalid_config("capacity must be greater than 0");
        assert!(!err.is_rate_limit_exceeded());
        assert!(err.is_invalid_config());
        assert_eq!(err.retry_after_ms(), None);
        assert_eq!(
            err.to_string(),
            "invalid configuration: capacity must be greater than 0"
        );
    }
}
