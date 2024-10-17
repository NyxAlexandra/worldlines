use std::time::Duration;

use archetypal_ecs::{Component, World};
use criterion::{criterion_group, criterion_main, Criterion};

#[derive(Component)]
struct A(#[expect(unused)] u32);

#[derive(Component)]
struct B(#[expect(unused)] u64);

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
