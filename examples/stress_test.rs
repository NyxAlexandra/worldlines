use std::hint::black_box;

use worldlines::component::Component;
use worldlines::world::World;

#[derive(Component)]
struct A(u32);

#[derive(Component)]
struct B(u64);

#[derive(Component)]
struct C(#[expect(unused)] u128);

fn main() {
    const COUNT: usize = 1_000_000;

    let mut world = World::new();
    let entities: Vec<_> = world
        .spawn_iter((0..COUNT).map(|_| black_box((A(123), B(321)))))
        .collect();

    assert_eq!(world.len(), COUNT);

    for entity in entities {
        let mut entity = world.entity_mut(entity).unwrap();

        {
            let a: &A = entity.get().unwrap();
            let b: &B = entity.get().unwrap();

            assert_eq!(a.0, 123);
            assert_eq!(b.0, 321);
        }

        entity.insert(C(999));
        entity.despawn();
    }
}
