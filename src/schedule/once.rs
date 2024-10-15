use super::{Label, Schedule};
use crate::{SystemNode, World};

/// A schedule that runs its systems once.
pub struct Once;

/// The schedule implementation for [`Once`].
#[doc(hidden)]
#[derive(Default)]
pub struct _Once {
    ran: bool,
}

impl Label for Once {
    type Schedule = _Once;

    fn get(self) -> Self::Schedule {
        Default::default()
    }
}

impl Schedule for _Once {
    fn run(&mut self, world: &mut World, systems: &mut [SystemNode]) {
        if !self.ran {
            self.ran = true;

            for system in systems {
                system.run_from_mut(world);
                system.try_apply(world);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{App, ResMut};

    #[test]
    fn once_system_run_only_once() {
        struct Counter(usize);

        fn increment_counter(mut counter: ResMut<Counter>) {
            counter.0 += 1;
        }

        let mut app = App::new().and_insert(Once, (increment_counter,));

        app.world_mut().create(Counter(0));

        app.tick();
        app.tick();

        let counter = app.world().resource::<Counter>().unwrap();

        assert_eq!(counter.0, 1);
    }
}
