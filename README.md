# `archetypal-ecs`

A simple archetypal ECS.

(I'm still looking for a better name...)

## Usage

A simple example:

```rust
use std::error::Error;

use archetypal_ecs::*;

struct Person;
struct Dog;
struct Name(&'static str);

fn main() -> Result<(), Box<dyn Error>> {
    let mut world = World::new();

    // `World::spawn` takes a `Bundle`: a group of components. `Bundle` is
    // implemented for tuples `T0..Tn` where `n = 15` and can be derived
    world.spawn((Person, Name("Alexandra")));
    world.spawn((Dog, Name("Hiro")));

    // will print both names, even though they don't have the same components
    for Name(name) in world.query::<&Name, ()>()? {
        println!("hello {}!", name);
    }

    Ok(())
}
```
