use std::any::Any;
use std::collections::VecDeque;

use crate::{ComponentId, Event, EventReader, SparseMap};

/// Stores and manages events.
#[derive(Debug)]
pub struct Events {
    events: SparseMap<ComponentId, Box<dyn Any>>,
}

pub(super) struct EventQueue<E: Event> {
    inner: VecDeque<Option<E>>,
}

impl Events {
    pub fn new() -> Self {
        let events = SparseMap::new();

        Self { events }
    }

    pub fn read<E: Event>(&mut self) -> Option<EventReader<'_, '_, E>> {
        todo!()
    }

    pub fn push<E: Event>(&mut self, event: E) {
        let any = self.events.get_or_insert_with(ComponentId::of::<E>(), || {
            Box::new(EventQueue::<E>::new())
        });
        let queue: &mut EventQueue<_> = unsafe { any.downcast_mut().unwrap_unchecked() };

        queue.push(event);
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }
}

impl<E: Event> EventQueue<E> {
    pub const fn new() -> Self {
        Self { inner: VecDeque::new() }
    }

    pub fn push(&mut self, event: E) {
        self.inner.push_back(Some(event));
    }

    pub fn next(&mut self) -> Option<E> {
        self.inner.pop_front()
    }
}
