use std::time::Duration;

use archetypal_ecs::World;
use criterion::{criterion_group, criterion_main, Criterion};

#[allow(unused)]
struct A(u32);
#[allow(unused)]
struct B(u64);

fn benchmark(c: &mut Criterion) {
    c.benchmark_group("bulk_spawn").bench_function("spawn_many_in_for_loop", |bencher| {
        bencher.iter(|| {
            let mut world = World::new();

            for _ in 0..10000 {
                world.spawn((A(123), B(321)));
            }
        })
    });
}

criterion_group!(
    name = this;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(4));
    targets = benchmark,
);
criterion_main!(this);
