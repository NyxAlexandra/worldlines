use std::time::Duration;

use criterion::{criterion_group, criterion_main, Bencher, Criterion};
use worldlines::prelude::*;

#[derive(Component)]
struct Position {
    x: f32,
    y: f32,
}

#[derive(Component)]
struct Velocity {
    x: f32,
    y: f32,
}

fn benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("query");

    group
        .bench_function("simple", simple)
        .bench_function("fragmented", fragmented);
}

fn simple(bencher: &mut Bencher<'_>) {
    const COUNT: usize = 10_000;

    let mut world = World::new();

    world.spawn_iter(
        (0..COUNT).map(|_| {
            (Position { x: 1.0, y: -1.0 }, Velocity { x: 1.0, y: -1.0 })
        }),
    );
    bencher.iter(|| {
        for (position, velocity) in
            &mut world.query_mut::<(&mut Position, &Velocity)>().unwrap()
        {
            position.x += velocity.x;
            position.y += velocity.y;
        }
    });
}

// from [Bevy](https://github.com/bevyengine/bevy/blob/60b2c7ce7755a49381c5265021ff175d3624218c/benches/benches/bevy_ecs/iteration/iter_frag.rs).
fn fragmented(bencher: &mut Bencher<'_>) {
    const COUNT: usize = 10_000;

    #[derive(Component)]
    struct Data(f32);

    macro_rules! create_entities {
        ($world:ident, ($($t:ident),*)) => {
            $(
                #[derive(Component)]
                struct $t(#[expect(unused)] f32);

                for _ in 0..COUNT {
                    $world.spawn(($t(0.0), Data(1.0)));
                }
            )*
        };
    }

    let mut world = World::new();

    create_entities!(
        world,
        (
            C0, C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11, C12, C13, C14,
            C15, C16, C17, C18, C19, C20, C21, C22, C23, C24, C25, C26, C27,
            C28, C29, C30, C31
        )
    );

    bencher.iter(|| {
        for Data(data) in &mut world.query_mut::<&mut Data>().unwrap() {
            *data *= 2.0;
        }
    });
}

criterion_group!(
    name = this;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(4));
    targets = benchmark,
);
criterion_main!(this);
