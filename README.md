# `archetypal-ecs`

A simple archetypal ECS.

(I'm still looking for a better name...)

---

A simple example:

```rust
use std::error::Error;

use archetypal_ecs::*;

struct Person;
struct Dog;
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
```
