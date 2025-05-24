//! # BucketBoss
//!
//! A high-performance, flexible rate limiting library for Rust.
//!
//! ## Features
//! - **Token Bucket** - Classic token bucket algorithm with burst support
//! - **Leaky Bucket** - Precise rate limiting with leaky bucket algorithm
//! - **No-std support** - Works in `no_std` environments with `alloc`
//! - **Async ready** - Optional async support via feature flags
//! - **Distributed** - Optional Redis backend for distributed rate limiting

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]
#![forbid(unsafe_code)]
#![deny(
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_qualifications,
    unused_results,
    clippy::all
)]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod clock;
pub mod error;
pub mod leaky_bucket;
pub mod token_bucket;
pub mod traits;

pub use clock::*;
pub use error::*;
pub use leaky_bucket::*;
pub use token_bucket::*;
pub use traits::*;

/// Re-export for use in tests and examples
#[cfg(feature = "std")]
pub mod testing {
    pub use super::clock::MockClock;
}

#[cfg(test)]
mod tests {
    //! Integration tests for the bucketboss crate.
    //!
    //! These tests verify the interaction between different modules
    //! and ensure the public API works as expected.

    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    /// A simple test clock that can be advanced manually
    #[derive(Debug, Clone)]
    pub struct TestClock {
        now: Arc<AtomicU64>,
    }

    impl TestClock {
        pub fn new(initial_time: u64) -> Self {
            Self {
                now: Arc::new(AtomicU64::new(initial_time)),
            }
        }

        pub fn advance(&self, ms: u64) {
            let _ = self.now.fetch_add(ms, Ordering::SeqCst);
        }
    }

    impl Clock for TestClock {
        fn now(&self) -> u64 {
            self.now.load(Ordering::Relaxed)
        }
    }

    /// Helper function to run a test with a test clock
    pub fn with_test_clock<F>(test: F)
    where
        F: FnOnce(TestClock) -> (),
    {
        let clock = TestClock::new(0);
        test(clock.clone());
    }
}

#[cfg(test)]
mod property_tests {
    //! Property-based tests for the bucketboss crate.
    //!
    //! These tests use the proptest crate to generate random inputs
    //! and verify that properties hold across a wide range of scenarios.

    use super::*;
    use proptest::prelude::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    /// A mock clock for property tests
    #[derive(Debug, Clone)]
    struct PropTestClock {
        now: Arc<AtomicU64>,
    }

    impl PropTestClock {
        fn new(initial_time: u64) -> Self {
            Self {
                now: Arc::new(AtomicU64::new(initial_time)),
            }
        }

        fn advance(&self, ms: u64) {
            let _ = self.now.fetch_add(ms, Ordering::SeqCst);
        }
    }

    impl Clock for PropTestClock {
        fn now(&self) -> u64 {
            self.now.load(Ordering::Relaxed)
        }
    }

    /// Generate a strategy for rate values
    fn rate_strategy() -> impl Strategy<Value = f64> {
        (0.1f64..1000.0f64)
    }

    /// Generate a strategy for capacity/burst values
    fn capacity_strategy() -> impl Strategy<Value = u32> {
        1u32..1000u32
    }
}
