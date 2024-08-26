use std::collections::{vec_deque, VecDeque};

pub use self::storage::*;
use crate::{Component, SystemInput, World, WorldAccess};

mod storage;

/// An ECS event.
pub trait Event: Component {}

impl<E: Component> Event for E {}

/// An iterator of [`Event`]s.
pub struct EventReader<'w, 's, E: Event> {
    events: &'w mut EventQueue<E>,
    index: &'s mut usize,
}

unsafe impl<E: Event> SystemInput for EventReader<'_, E> {
    type Output<'w, 's> = EventReader<'w, 's, E>;
    type State = usize;

    fn init(_world: &World) -> Self::State {
        0
    }

    fn access(access: &mut WorldAccess) {
        access.events::<E>();
    }

    unsafe fn get<'w, 's>(
        world: crate::WorldPtr<'w>,
        state: &'s mut Self::State,
    ) -> Self::Output<'w, 's> {
        todo!()
    }

    fn should_apply(state: &Self::State) -> bool {}

    fn apply(world: &mut World, state: &mut Self::State) {}
}
