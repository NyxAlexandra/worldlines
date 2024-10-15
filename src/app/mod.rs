pub use self::plugin::*;
use crate::{
    IndexTypeMap,
    IntoSystemNodes,
    Label,
    Schedule,
    SystemInput,
    SystemNode,
    TypeData,
    World,
};

mod plugin;

/// A runtime for an ECS.
pub struct App {
    world: World,
    schedules: IndexTypeMap<ScheduleBox>,
    runner: Option<Box<dyn FnOnce(Self)>>,
}

/// A [`Schedule`] and systems that are a part of it.
pub struct ScheduleBox {
    schedule: Box<dyn Schedule>,
    systems: Vec<SystemNode>,
}

impl App {
    /// Creates an empty app.
    pub fn new() -> Self {
        let world = World::new();
        let schedules = IndexTypeMap::default();
        let runner = None;

        Self { world, schedules, runner }
    }

    /// Returns the world of this app.
    pub fn world(&self) -> &World {
        &self.world
    }

    /// Returns the world of this app.
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// Loads a [`Plugin`] into this app.
    pub fn load<P: Plugin>(&mut self, plugin: P) -> Result<(), P::Err> {
        plugin.load(self)
    }

    /// Calls [`App::load`] and returns `self`.
    pub fn and_load<P: Plugin>(mut self, plugin: P) -> Result<Self, P::Err> {
        self.load(plugin).map(|_| self)
    }

    /// Inserts systems into a schedule.
    pub fn schedule<I: SystemInput>(
        &mut self,
        label: impl Label,
        systems: impl IntoSystemNodes<I>,
    ) {
        self.schedules
            .entry(TypeData::of_val(&label).type_id())
            .or_insert_with(|| ScheduleBox {
                schedule: Box::new(label.get()),
                systems: Vec::new(),
            })
            .systems
            .extend(systems.into_system_nodes());
    }

    /// Calls [`App::schedule`] and returns `self`.
    pub fn and_schedule<I: SystemInput>(
        mut self,
        label: impl Label,
        systems: impl IntoSystemNodes<I>,
    ) -> Self {
        self.schedule(label, systems);

        self
    }

    /// Set the runner for this app.
    pub fn set_runner(&mut self, runner: impl FnOnce(Self) + 'static) {
        self.runner = Some(Box::new(runner));
    }

    /// Calls [`App::set_runner`] and returns `self`.
    pub fn and_set_runner(mut self, f: impl FnOnce(Self) + 'static) -> Self {
        self.set_runner(f);

        self
    }

    /// Run all schedules.
    pub fn tick(&mut self) {
        for ScheduleBox { schedule, systems } in self.schedules.values_mut() {
            schedule.run(&mut self.world, systems);
        }
    }

    /// Invokes the runner on this app if present, otherwise calls [`App::tick`]
    /// in a loop.
    pub fn run(mut self) {
        if let Some(runner) = self.runner.take() {
            runner(self);
        } else {
            loop {
                self.tick();
            }
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WorldAccess;

    #[test]
    fn basic_schedule() {
        struct Main;

        struct A;
        struct B;

        impl Schedule for Main {
            fn run(&mut self, world: &mut World, systems: &mut [SystemNode]) {
                // TODO: cache access
                let mut access = WorldAccess::new();

                for system in systems {
                    access.clear();
                    system.access(&mut access);

                    if !access.is_valid() {
                        continue;
                    }

                    system.run_from_mut(world).unwrap();
                    system.try_apply(world);
                }
            }
        }

        fn system_a(world: &mut World) {
            world.create(A);
        }

        fn system_b(world: &mut World) {
            world.create(B);
        }

        let mut app = App::new();

        app.schedule(Main, (system_a, system_b));
        app.tick();

        assert!(app.world.has::<A>());
        assert!(app.world.has::<B>());
    }
}
