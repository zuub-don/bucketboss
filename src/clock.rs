//! Clock abstraction for time-based operations.
//!
//! Provides a trait-based clock interface to allow for deterministic testing
//! and platform-specific time implementations.

use core::time::Duration;

/// A trait representing a monotonic clock, used for rate limiting operations.
///
/// This trait abstracts over different time sources to enable testing and
/// platform-specific implementations. The clock is expected to be monotonic,
/// meaning that subsequent calls to `now()` should never return a value
/// less than a previous call.
pub trait Clock: Send + Sync + 'static {
    /// Returns the current time in milliseconds since an arbitrary epoch.
    ///
    /// The epoch could be the Unix epoch, system boot, or any other fixed
    /// point in time, as long as it's consistent for the lifetime of the clock.
    fn now(&self) -> u64;

    /// Returns the current time as a `Duration` since the clock's epoch.
    fn now_duration(&self) -> Duration {
        Duration::from_millis(self.now())
    }
}

/// A clock that uses the system's monotonic clock.
///
/// This is the default production clock that should be used in most cases.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    #[inline]
    fn now(&self) -> u64 {
        #[cfg(feature = "std")]
        {
            use std::time::{SystemTime, UNIX_EPOCH};
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("SystemTime before UNIX EPOCH!")
                .as_millis() as u64
        }

        #[cfg(not(feature = "std"))]
        {
            // In no_std environments, you'll need to provide a suitable implementation
            // This is a placeholder that will cause a compilation error if used without std
            // and no custom clock implementation
            compile_error!("std feature is required for SystemClock");
        }
    }
}

/// A mock clock for testing purposes.
///
/// This clock allows manual control of the current time, making it ideal for
/// deterministic testing of time-based functionality.
#[derive(Debug, Default)]
pub struct MockClock {
    now: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl Clone for MockClock {
    fn clone(&self) -> Self {
        Self {
            now: std::sync::Arc::clone(&self.now),
        }
    }
}

#[cfg(feature = "std")]
impl MockClock {
    /// Creates a new `MockClock` starting at the given time in milliseconds.
    pub fn new(initial_time: u64) -> Self {
        Self {
            now: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(initial_time)),
        }
    }

    /// Advances the clock by the specified number of milliseconds.
    pub fn advance(&self, ms: u64) {
        let _ = self.now.fetch_add(ms, std::sync::atomic::Ordering::SeqCst);
    }

    /// Sets the clock to the specified time in milliseconds.
    pub fn set(&self, ms: u64) {
        self.now.store(ms, std::sync::atomic::Ordering::SeqCst);
    }
}

#[cfg(feature = "std")]
impl Clock for MockClock {
    fn now(&self) -> u64 {
        self.now.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_clock() {
        let clock = MockClock::new(1000);
        assert_eq!(clock.now(), 1000);

        clock.advance(500);
        assert_eq!(clock.now(), 1500);

        clock.set(2000);
        assert_eq!(clock.now(), 2000);
    }

    #[test]
    fn test_system_clock() {
        let clock = SystemClock;
        let t1 = clock.now();
        let t2 = clock.now();
        assert!(t2 >= t1, "System clock should be monotonic");
    }
}
