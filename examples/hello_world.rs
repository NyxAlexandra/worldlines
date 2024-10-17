use std::error::Error;

use archetypal_ecs::*;

#[derive(Component)]
struct Person;

#[derive(Component)]
struct Dog;

#[derive(Component)]
struct Name(&'static str);

fn main() -> Result<(), Box<dyn Error>> {
    let mut world = World::new();

    // hi
    world.spawn((Person, Name("Alexandra")));
    world.spawn((Dog, Name("Hiro")));

    for Name(name) in world.query::<&Name, ()>()? {
        println!("hello {}!", name);
    }

    Ok(())
}
