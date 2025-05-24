//! Benchmarks for the Leaky Bucket rate limiter.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Barrier;
use std::thread;
use std::time::Duration;

use bucketboss::{LeakyBucket, RateLimiter, ReconfigurableRateLimiter};

// A simple mock clock for benchmarking
#[derive(Default, Clone)]
struct MockClock(Arc<AtomicU64>);

impl bucketboss::clock::Clock for MockClock {
    fn now(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

fn leaky_bucket_acquire_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("leaky_bucket_acquire");

    // Test with different rates and burst sizes
    let test_cases = [
        (10, 1.0),     // 1 request per second, burst of 10
        (100, 10.0),   // 10 requests per second, burst of 100
        (1000, 100.0), // 100 requests per second, burst of 1000
    ];

    for (burst_size, rate) in test_cases.iter() {
        group.bench_with_input(
            format!("burst_{}_rate_{}", burst_size, rate),
            &(burst_size, rate),
            |b, &(burst_size, rate)| {
                let clock = MockClock::default();
                let bucket = LeakyBucket::with_clock(*rate, Some(*burst_size), clock);

                b.iter(|| {
                    let _ = black_box(bucket.try_acquire(1));
                });
            },
        );
    }

    group.finish();
}

fn leaky_bucket_contention_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("leaky_bucket_contention");

    // Test with different thread counts
    let thread_counts = [1, 2, 4, 8];

    for &num_threads in thread_counts.iter() {
        group.bench_function(format!("{}_threads", num_threads), |b| {
            b.iter_custom(|iters| {
                let clock = Arc::new(MockClock::default());
                let clock = MockClock(clock.0.clone());
                let bucket = Arc::new(LeakyBucket::with_clock(
                    1_000_000.0,     // High rate
                    Some(1_000_000), // Large burst size
                    clock,
                ));

                let barrier = Arc::new(Barrier::new(num_threads + 1));
                let mut handles = vec![];

                for _ in 0..num_threads {
                    let bucket = bucket.clone();
                    let barrier = barrier.clone();

                    let handle = thread::spawn(move || {
                        barrier.wait();
                        for _ in 0..(iters / num_threads as u64) {
                            bucket.try_acquire(1).unwrap();
                            black_box(());
                        }
                    });

                    handles.push(handle);
                }

                let start = std::time::Instant::now();
                barrier.wait();

                for handle in handles {
                    handle.join().unwrap();
                }

                start.elapsed()
            });
        });
    }

    group.finish();
}

fn leaky_bucket_update_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("leaky_bucket_update");

    group.bench_function("update_config", |b| {
        let clock = MockClock::default();
        let bucket = LeakyBucket::with_clock(10.0, Some(100), clock);

        b.iter(|| {
            bucket.update_config(200, 20.0).unwrap();
            black_box(());
            bucket.update_config(100, 10.0).unwrap();
            black_box(());
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    leaky_bucket_acquire_benchmark,
    leaky_bucket_contention_benchmark,
    leaky_bucket_update_benchmark
);
criterion_main!(benches);
