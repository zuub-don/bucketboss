//! Example of using `bucketboss` in an embedded environment.
//!
//! This example demonstrates how to use the TokenBucket rate limiter in a `no_std` environment
//! with a custom clock implementation.

// Only enable no_std and no_main for actual embedded targets
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

use core::sync::atomic::{AtomicU64, Ordering};

// Only use panic-halt in no_std environments
#[cfg(not(test))]
use panic_halt as _;

use bucketboss::{
    clock::Clock,
    RateLimiter,
    TokenBucket,
};

#[cfg(not(test))]
use core::panic::PanicInfo;

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

// A simple mock hardware timer that increments every millisecond
struct HardwareTimer {
    counter: AtomicU64,
}

impl HardwareTimer {
    const fn new() -> Self {
        Self {
            counter: AtomicU64::new(0),
        }
    }

    fn increment(&self, ms: u64) {
        self.counter.fetch_add(ms, Ordering::SeqCst);
    }

    fn read(&self) -> u64 {
        self.counter.load(Ordering::Relaxed)
    }
}

// A clock that uses our hardware timer
struct HardwareClock {
    timer: &'static HardwareTimer,
}

impl Clock for HardwareClock {
    fn now(&self) -> u64 {
        self.timer.read()
    }
}

// Global hardware timer
static TIMER: HardwareTimer = HardwareTimer::new();

#[no_mangle]
pub fn timer_tick() {
    // This would be called by the hardware timer interrupt handler
    TIMER.increment(1);
}

// Main function for embedded targets
#[no_mangle]
pub extern "C" fn embedded_main() -> ! {
    // Create a clock that uses our hardware timer
    let clock = HardwareClock { timer: &TIMER };

    // Create a rate limiter that allows 10 operations per second with a burst of 5
    let rate_limiter = TokenBucket::with_clock(5, 10.0, clock);

    // Simulate some operations
    for i in 0..20 {
        // Simulate time passing (in a real system, this would happen naturally)
        if i % 2 == 0 {
            // Simulate a delay of 50ms between operations
            TIMER.increment(50);
        }

        // Try to perform an operation
        match rate_limiter.try_acquire(1) {
            Ok(_) => {
                // Operation allowed
                log::info("Operation allowed", format_args!("Operation {}: Allowed", i));
            },
            Err(_) => {
                // Rate limited
                log::warn("Operation limited", format_args!("Operation {}: Rate limited", i));
            }
        }
    }

    // The program should never reach here in a real embedded system
    loop {}
}

// Mock logging for the example
mod log {
    use core::fmt;

    pub fn info(_msg: &str, _args: fmt::Arguments) {
        // In a real embedded system, this would write to a UART or other output
        // For testing purposes, we'll use core::hint::black_box to prevent optimization
        #[cfg(test)]
        core::hint::black_box((_msg, _args));
    }

    pub fn warn(_msg: &str, _args: fmt::Arguments) {
        // In a real embedded system, this would write to a UART or other output
        // For testing purposes, we'll use core::hint::black_box to prevent optimization
    }
}

// Test entry point
#[cfg(test)]
mod tests {
    #[test]
    fn test_embedded() {
        // This is a test function for the embedded example
    }
}

// Main entry point for embedded targets
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Call our embedded main function
    embedded_main()
}
