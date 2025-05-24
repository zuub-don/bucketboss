//! Example of using `bucketboss` in an embedded environment.
//!
//! This example demonstrates how to use the TokenBucket rate limiter in a `no_std` environment
//! with a custom clock implementation.

#![no_std]
#![no_main]

use core::sync::atomic::{AtomicU64, Ordering};
use panic_halt as _;

use bucketboss::{
    clock::{Clock, MockClock},
    error::Result,
    RateLimiter, TokenBucket,
};

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

#[no_mangle]
pub fn main() -> ! {
    // Create a clock that uses our hardware timer
    let clock = HardwareClock { timer: &TIMER };

    // Create a rate limiter that allows 10 operations per second with a burst of 5
    let mut rate_limiter = TokenBucket::with_clock(5, 10.0, clock);

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
                log::info!("Operation {}: Allowed", i);
            }
            Err(_) => {
                // Rate limited
                log::warn!("Operation {}: Rate limited", i);
            }
        }
    }

    // The program should never reach here in a real embedded system
    loop {}
}

// Mock logging for the example
mod log {
    use core::fmt::Arguments;

    pub fn info(_: &str, _: core::fmt::Arguments) {
        // In a real system, this would log to a serial port or similar
    }

    pub fn warn(_: &str, _: core::fmt::Arguments) {
        // In a real system, this would log to a serial port or similar
    }
}

// Required for no_std binaries
#[no_mangle]
fn _start() -> ! {
    main()
}
