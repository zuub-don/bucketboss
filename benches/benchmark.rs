use bucketboss::{LeakyBucket, RateLimiter, TokenBucket};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

fn bench_token_bucket_acquire(c: &mut Criterion) {
    let mut group = c.benchmark_group("token_bucket_acquire");

    for &capacity in &[10, 100, 1000] {
        for &rate in &[10.0, 100.0, 1000.0] {
            group.bench_function(format!("capacity_{}_rate_{}", capacity, rate as u32), |b| {
                let bucket = TokenBucket::new(capacity, rate);
                b.iter(|| {
                    let _ = black_box(bucket.try_acquire(1));
                })
            });
        }
    }
    group.finish();
}

fn bench_leaky_bucket_acquire(c: &mut Criterion) {
    let mut group = c.benchmark_group("leaky_bucket_acquire");

    for &capacity in &[10, 100, 1000] {
        for &rate in &[10.0, 100.0, 1000.0] {
            group.bench_function(format!("capacity_{}_rate_{}", capacity, rate as u32), |b| {
                let bucket = LeakyBucket::new(rate, Some(capacity));
                b.iter(|| {
                    let _ = black_box(bucket.try_acquire(1));
                })
            });
        }
    }
    group.finish();
}

fn bench_token_bucket_concurrent(c: &mut Criterion) {
    let mut group = c.benchmark_group("token_bucket_concurrent");

    for &num_threads in &[2, 4, 8] {
        group.bench_function(format!("{}_threads", num_threads), |b| {
            b.iter_custom(|iters| {
                let bucket = Arc::new(TokenBucket::new(1_000_000, 1_000_000.0));
                let start = Instant::now();

                thread::scope(|s| {
                    for _ in 0..num_threads {
                        let bucket = bucket.clone();
                        s.spawn(move || {
                            for _ in 0..(iters / num_threads as u64) {
                                let _ = black_box(bucket.try_acquire(1));
                            }
                        });
                    }
                });

                start.elapsed()
            })
        });
    }
    group.finish();
}

fn bench_leaky_bucket_concurrent(c: &mut Criterion) {
    let mut group = c.benchmark_group("leaky_bucket_concurrent");

    for &num_threads in &[2, 4, 8] {
        group.bench_function(format!("{}_threads", num_threads), |b| {
            b.iter_custom(|iters| {
                let bucket = Arc::new(LeakyBucket::new(1_000_000.0, Some(1_000_000)));
                let start = Instant::now();

                thread::scope(|s| {
                    for _ in 0..num_threads {
                        let bucket = bucket.clone();
                        s.spawn(move || {
                            for _ in 0..(iters / num_threads as u64) {
                                let _ = black_box(bucket.try_acquire(1));
                            }
                        });
                    }
                });

                start.elapsed()
            })
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_token_bucket_acquire,
    bench_leaky_bucket_acquire,
    bench_token_bucket_concurrent,
    bench_leaky_bucket_concurrent,
);
criterion_main!(benches);
