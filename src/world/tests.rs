use crate::prelude::*;

#[test]
fn clear_despawns_all_entities() {
    let mut world = World::new();
    let entities: Vec<_> = world.spawn_iter((0..10).map(|_| ())).collect();

    world.clear();

    for entity in entities {
        assert!(!world.contains(entity));
    }
}

/// Tests that the world can handle large amounts of entities.
#[test]
// takes forever on miri
#[cfg_attr(miri, ignore)]
fn spawn_many() {
    #[derive(Component)]
    struct A(#[expect(unused)] u32);

    #[derive(Component)]
    struct B(#[expect(unused)] u64);

    let mut world = World::new();
    let iter = (0..1_000_000).map(|_| (A(123), B(321)));

    world.spawn_iter(iter);
}
