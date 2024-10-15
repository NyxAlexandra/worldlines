// TODO: optimize

pub use self::entity::*;
pub use self::world::*;
use crate::World;

mod entity;
mod world;

/// A queue of [`Command`]s to be performed on a [`World`].
#[repr(transparent)]
pub struct CommandQueue {
    inner: Vec<CommandBox>,
}

// hack to avoid error on "mutable references in const fn" (bc `&mut World`)
#[repr(transparent)]
struct CommandBox {
    inner: Box<dyn FnOnce(&mut World) + Send>,
}

// TODO?: fallible commands

/// An operation to be performed on a [`World`].
///
/// See [`WorldQueue`].
pub trait Command: Send + 'static {
    /// Perform this command on the world.
    fn apply(self, world: &mut World);
}

impl<F: FnOnce(&mut World) + Send + 'static> Command for F {
    fn apply(self, world: &mut World) {
        self(world);
    }
}

impl CommandQueue {
    /// Create a new empty queue.
    pub const fn new() -> Self {
        Self { inner: Vec::new() }
    }

    /// Borrow this command queue as a [`WorldQueue`].
    pub fn as_world_queue<'w, 's>(
        &'s mut self,
        world: &'w World,
    ) -> WorldQueue<'w, 's> {
        WorldQueue { entities: &world.entities, queue: self }
    }

    /// Add a [`Command`] to the end of the queue.
    pub fn push<C: Command>(&mut self, command: C) {
        self.push_fn(move |world| command.apply(world));
    }

    /// Push a [`Command`] function to the end of the queue.
    pub fn push_fn(&mut self, f: impl FnOnce(&mut World) + Send + 'static) {
        self.inner.push(CommandBox { inner: Box::new(f) });
    }

    /// Apply the [`Command`]s in the queue to the [`World`].
    pub fn apply(&mut self, world: &mut World) {
        for command in self.inner.drain(..) {
            (command.inner)(world);
        }
    }
}

unsafe impl Sync for CommandQueue {}

impl Default for CommandQueue {
    fn default() -> Self {
        Self::new()
    }
}
