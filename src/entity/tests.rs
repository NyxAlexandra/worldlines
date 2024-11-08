use crate::component::Component;
use crate::world::World;

#[derive(Component)]
struct Name(&'static str);

#[derive(Component)]
struct Age(u32);

#[test]
fn spawned_entity_contains_initial_components() {
    let mut world = World::new();
    let entity = world.spawn((Name("Alexandra"), Age(u32::MAX)));

    let name = entity.get::<Name>().unwrap();
    let age = entity.get::<Age>().unwrap();

    assert_eq!(name.0, "Alexandra");
    assert_eq!(age.0, u32::MAX);
}
