# BucketBoss

[![Crates.io](https://img.shields.io/crates/v/bucketboss)](https://crates.io/crates/bucketboss)
[![Documentation](https://docs.rs/bucketboss/badge.svg)](https://docs.rs/bucketboss)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/copyleftdev/bucketboss)
[![Build Status](https://github.com/copyleftdev/bucketboss/actions/workflows/rust.yml/badge.svg)](https://github.com/copyleftdev/bucketboss/actions)

A high-performance, flexible rate limiting library for Rust.

## Features

- **Token Bucket** - Classic token bucket rate limiting algorithm
- **Leaky Bucket** - Leaky bucket rate limiting algorithm
- **Thread-Safe** - Safe for use in concurrent environments
- **Async Support** - Built with async/await in mind
- **No-std Support** - Can be used in `no_std` environments (with `default-features = false`)
- **Distributed Rate Limiting** - Optional Redis backend for distributed rate limiting

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
bucketboss = "0.1"
```

## Examples

### Basic Usage

```rust
use bucketboss::{TokenBucket, LeakyBucket, RateLimiter};

// Create a token bucket that allows 10 requests per second
let bucket = TokenBucket::new(10, 10.0);

// Try to acquire a token
assert!(bucket.try_acquire(1).is_ok());

// Try to acquire more tokens than available
assert!(bucket.try_acquire(20).is_err());

// Create a leaky bucket that allows 10 requests per second
let leaky = LeakyBucket::new(10.0, Some(10));
assert!(leaky.try_acquire(5).is_ok());
```

### Async Example

```rust
use bucketboss::{TokenBucket, RateLimiter};
use std::time::Duration;

#[tokio::main]
async fn main() {
    let bucket = TokenBucket::new(10, 10.0);
    
    // Try to acquire a token asynchronously
    if let Ok(_) = bucket.try_acquire_async(1).await {
        // Token acquired, process request
    } else {
        // Rate limited
    }
}
```

## Benchmarks

Run the benchmarks with:

```bash
cargo bench
```

### Single-Threaded Performance
- **TokenBucket try_acquire**: ~34.90µs (mean)
- **LeakyBucket try_acquire**: ~34.94µs (mean)

### Concurrent Performance (8 threads)
- **TokenBucket**: ~235.87µs (mean)
- **LeakyBucket**: ~424.82µs (mean)

## License

Licensed under either of:

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.etBoss 🚀

[![Crates.io](https://img.shields.io/crates/v/bucketboss)](https://crates.io/crates/bucketboss)
[![Documentation](https://docs.rs/bucketboss/badge.svg)](https://docs.rs/bucketboss)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/yourusername/bucketboss)
[![Build Status](https://github.com/yourusername/bucketboss/actions/workflows/rust.yml/badge.svg)](https://github.com/yourusername/bucketboss/actions)

**The last rate limiter you'll ever need.**

BucketBoss is a high-performance, flexible rate limiting library for Rust that provides both Token Bucket and Leaky Bucket algorithms. It's designed to be:

- **Fast**: Uses atomic operations for thread-safe, lock-free operation in the hot path
- **Flexible**: Works in `std`, `no_std`, and `wasm32` environments
- **Embedded-friendly**: Zero allocations during operation
- **Async-ready**: Optional async support via feature flags
- **Distributed**: Optional Redis backend for distributed rate limiting

## Features

- 🪣 **Token Bucket** - Classic token bucket algorithm with burst support
- 💧 **Leaky Bucket** - Precise rate limiting with leaky bucket algorithm
- 🚀 **High Performance** - Lock-free operations in the hot path
- 🧩 **Modular** - Trait-based design for extensibility
- 🕒 **Testable** - Mock clock for deterministic testing
- 🔌 **Integrations** - Works with Axum, Actix, and more

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
bucketboss = "0.1"
```

## Feature Flags

- `std` (enabled by default): Enables standard library support
- `async`: Enables async support (requires `tokio`)
- `distributed`: Enables distributed rate limiting with Redis

## Examples

### Basic Usage

```rust
use bucketboss::{TokenBucket, RateLimiter};

// Create a rate limiter that allows 10 operations per second with a burst of 5
let mut limiter = TokenBucket::new(5, 10.0);

// Try to perform an operation
if limiter.try_acquire(1).is_ok() {
    println!("Operation allowed!");
} else {
    println!("Rate limited!");
}
```

### With Axum Middleware

```rust
use axum::{
    routing::get,
    Router,
    extract::State,
    http::StatusCode,
};
use bucketboss::{TokenBucket, RateLimiter};
use std::sync::Arc;
use tokio::sync::Mutex;

struct AppState {
    rate_limiter: Arc<Mutex<TokenBucket>>,
}

async fn handler(State(state): State<AppState>) -> Result<String, StatusCode> {
    let mut limiter = state.rate_limiter.lock().await;
    if limiter.try_acquire(1).is_ok() {
        Ok("Hello, World!".to_string())
    } else {
        Err(StatusCode::TOO_MANY_REQUESTS)
    }
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(handler))
        .with_state(AppState {
            rate_limiter: Arc::new(Mutex::new(TokenBucket::new(5, 10.0))),
        });
    
    // Run the server...
}
```

### In a `no_std` Environment

```rust
#![no_std]

use bucketboss::{TokenBucket, RateLimiter};

// In a real embedded system, you would implement the Clock trait
// for your hardware timer
struct DummyClock;

impl bucketboss::clock::Clock for DummyClock {
    fn now(&self) -> u64 {
        // Return current time in milliseconds
        0
    }
}

fn main() {
    let clock = DummyClock;
    let mut limiter = TokenBucket::with_clock(5, 10.0, clock);
    
    // Use the rate limiter...
}
```

## Benchmarks

Run the benchmarks with:

```bash
cargo bench
```

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

## Acknowledgements

- Inspired by various rate limiting implementations and research papers
- Special thanks to the Rust community for amazing libraries and tools
