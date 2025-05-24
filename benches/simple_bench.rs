use bucketboss::{LeakyBucket, RateLimiter, ReconfigurableRateLimiter, TokenBucket};
use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;

fn bench_token_bucket_single_thread(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("token_bucket_single_thread");

    group.bench_function("try_acquire", |b| {
        b.iter_batched(
            || TokenBucket::new(1_000_000, 1_000_000.0),
            |bucket| {
                rt.block_on(async {
                    for _ in 0..1000 {
                        let _ = black_box(bucket.try_acquire(1));
                    }
                });
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_leaky_bucket_single_thread(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("leaky_bucket_single_thread");

    group.bench_function("try_acquire", |b| {
        b.iter_batched(
            || LeakyBucket::new(1_000_000.0, Some(1_000_000)),
            |bucket| {
                rt.block_on(async {
                    for _ in 0..1000 {
                        let _ = black_box(bucket.try_acquire(1));
                    }
                });
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_token_bucket_concurrent(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("token_bucket_concurrent");

    for &num_threads in &[2, 4, 8] {
        group.bench_with_input(
            format!("{}_threads", num_threads),
            &num_threads,
            |b, &nt| {
                b.iter_batched(
                    || Arc::new(TokenBucket::new(1_000_000, 1_000_000.0)),
                    |bucket| {
                        rt.block_on(async {
                            let mut handles = Vec::with_capacity(nt);

                            for _ in 0..nt {
                                let bucket = bucket.clone();
                                let h = tokio::spawn(async move {
                                    for _ in 0..1000 {
                                        let _ = black_box(bucket.try_acquire(1));
                                    }
                                });
                                handles.push(h);
                            }

                            for h in handles {
                                h.await.unwrap();
                            }
                        });
                    },
                    BatchSize::SmallInput,
                )
            },
        );
    }

    group.finish();
}

fn bench_leaky_bucket_concurrent(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("leaky_bucket_concurrent");

    for &num_threads in &[2, 4, 8] {
        group.bench_with_input(
            format!("{}_threads", num_threads),
            &num_threads,
            |b, &nt| {
                b.iter_batched(
                    || Arc::new(LeakyBucket::new(1_000_000.0, Some(1_000_000))),
                    |bucket| {
                        rt.block_on(async {
                            let mut handles = Vec::with_capacity(nt);

                            for _ in 0..nt {
                                let bucket = bucket.clone();
                                let h = tokio::spawn(async move {
                                    for _ in 0..1000 {
                                        let _ = black_box(bucket.try_acquire(1));
                                    }
                                });
                                handles.push(h);
                            }

                            for h in handles {
                                h.await.unwrap();
                            }
                        });
                    },
                    BatchSize::SmallInput,
                )
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_token_bucket_single_thread,
    bench_leaky_bucket_single_thread,
    bench_token_bucket_concurrent,
    bench_leaky_bucket_concurrent,
);
criterion_main!(benches);
