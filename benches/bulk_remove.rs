use std::hint::black_box;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, Criterion};
use worldlines::component::Component;
use worldlines::world::World;

#[derive(Component)]
struct A(#[expect(unused)] u32);

#[derive(Component)]
struct B(#[expect(unused)] u64);

fn benchmark(c: &mut Criterion) {
    c.bench_function("bulk_remove", |bencher| {
        const COUNT: usize = 10_000;

        bencher.iter(|| {
            let mut world = World::new();
            let entities: Vec<_> = world
                .spawn_iter((0..COUNT).map(|_| black_box((A(123), B(321)))))
                .collect();

            for entity in entities {
                _ = world.entity_mut(entity).unwrap().remove::<B>();
            }
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
