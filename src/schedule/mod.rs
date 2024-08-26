use crate::{SystemNode, World};

/// A label for a [`Schedule`].
pub trait Label: 'static {
    type Schedule: Schedule;

    /// Initialize the schedule assosciated with this label.
    fn get(self) -> Self::Schedule;
}

impl<S: Schedule> Label for S {
    type Schedule = Self;

    fn get(self) -> Self::Schedule {
        self
    }
}

/// A runtime in an [`App`](crate::App).
pub trait Schedule: Send + Sync + 'static {
    /// Run this schedule.
    fn run(&mut self, world: &mut World, systems: &mut [SystemNode]) {
        for system in systems {
            system.run_from_mut(world);
            system.try_apply(world);
        }
    }
}

impl<F> Schedule for F
where
    F: FnMut(&mut World, &mut [SystemNode]) + Send + Sync + 'static,
{
    fn run(&mut self, world: &mut World, systems: &mut [SystemNode]) {
        self(world, systems)
    }
}
