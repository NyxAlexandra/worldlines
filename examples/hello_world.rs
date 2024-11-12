use worldlines::prelude::*;

#[derive(Component)]
struct Person;

#[derive(Component)]
struct Dog;

#[derive(Component)]
struct Name(&'static str);

fn main() -> Result<(), AccessError> {
    let mut world = World::new();

    // `World::spawn` takes a `Bundle`: a group of components. `Bundle` is
    // implemented for tuples `T0..Tn` where `n = 15` and can be derived
    world.spawn((Person, Name("Alexandra")));
    world.spawn((Dog, Name("Hiro")));

    // will print both names, even though they don't have the same components
    for Name(name) in &mut world.query::<&Name>()? {
        println!("hello {}!", name);
    }

    Ok(())
}
