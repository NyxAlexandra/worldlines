use std::time::Duration;

use archetypal_ecs::World;
use criterion::{criterion_group, criterion_main, Criterion};

#[allow(unused)]
struct A(u32);
#[allow(unused)]
struct B(u64);

fn benchmark(c: &mut Criterion) {
    c.benchmark_group("bulk_spawn").bench_function("spawn_iter", |bencher| {
        bencher.iter(|| {
            let mut world = World::new();

            world.spawn_iter((0..10000).map(|_| (A(123), B(321))));
        })
    });
}

criterion_group!(
    name = this;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(5));
    targets = benchmark,
);
criterion_main!(this);