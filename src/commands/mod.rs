//! Deferred operations to be performed on the world.

use std::any::type_name;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ptr::NonNull;
use std::{fmt, ptr};

pub use self::entity::*;
pub use self::world::*;
use crate::entity::EntityWorld;
use crate::world::World;

mod entity;
mod world;

/// A command to be performed on the world.
pub trait Command: Send + 'static {
    /// Returns the name of this commands for debugging purposes.
    ///
    /// Defaults to the [`type_name`].
    fn name() -> &'static str {
        type_name::<Self>()
    }

    /// Apply this command on a world.
    fn apply(self, world: &mut World);
}

/// A command to be performed on an entity.
pub trait EntityCommand: Send + 'static {
    /// Applies this command to an entity.
    fn apply(self, entity: EntityWorld<'_>);
}

impl<F: FnOnce(&mut World) + Send + 'static> Command for F {
    fn apply(self, world: &mut World) {
        self(world);
    }
}

impl<F: FnOnce(EntityWorld<'_>) + Send + 'static> EntityCommand for F {
    fn apply(self, entity: EntityWorld<'_>) {
        self(entity);
    }
}

/// A buffer of [commands](Command) to be performed on a world.
#[derive(Default)]
pub struct Commands {
    commands: Vec<&'static dyn CommandInfo>,
    bytes: Vec<MaybeUninit<u8>>,
}

/// # Safety
///
/// The value returned by [`CommandInfo::size`] must equal the size of the
/// represented command. The function returned by [`CommandInfo::drop`] must
/// only call the type's drop implementation.
unsafe trait CommandInfo {
    /// [`Command::name`].
    fn name(&self) -> &'static str;

    /// Size in bytes.
    fn size(&self) -> usize;

    /// A function that can drop the command.
    fn drop(&self) -> unsafe fn(*mut u8);

    /// Call [`Command::apply`] on a pointer to a command.
    unsafe fn call(&self, ptr: NonNull<u8>, world: &mut World);
}

fn command_info_of_val<C: Command>(_: &C) -> &'static dyn CommandInfo {
    &PhantomData::<C>
}

unsafe impl<C: Command> CommandInfo for PhantomData<C> {
    fn name(&self) -> &'static str {
        C::name()
    }

    fn size(&self) -> usize {
        size_of::<C>()
    }

    fn drop(&self) -> unsafe fn(*mut u8) {
        |ptr| unsafe { ptr::drop_in_place(ptr.cast::<C>()) }
    }

    unsafe fn call(&self, ptr: NonNull<u8>, world: &mut World) {
        let command = unsafe { ptr.cast().read_unaligned() };

        C::apply(command, world);
    }
}

impl Commands {
    /// Creates a new empty command buffer.
    pub const fn new() -> Self {
        let commands = Vec::new();
        let bytes = Vec::new();

        Self { bytes, commands }
    }

    /// Returns the amount of commands in the queue.
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Returns `true` if this queue is empty.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Pushes a command to the buffer.
    pub fn push(&mut self, command: impl Command) {
        // hack to not specify the type of `command` bc type elision doesn't
        // like pointer casting
        #[inline(always)]
        unsafe fn write_unaligned<T>(ptr: *mut u8, value: T) {
            unsafe { ptr.cast::<T>().write_unaligned(value) };
        }

        let info = command_info_of_val(&command);

        self.commands.push(info);
        self.bytes.reserve(info.size());

        let byte_index = self.bytes.len();
        let ptr =
            unsafe { self.bytes.as_mut_ptr().byte_add(byte_index).cast() };

        unsafe { write_unaligned(ptr, command) };
    }

    /// Pushes a function command to the queue.
    ///
    /// Helpful as using [`Commands::push`] on a closure fails type
    /// elision.
    pub fn push_fn(&mut self, f: impl FnOnce(&mut World) + Send + 'static) {
        self.push(f);
    }

    /// Applies stored commands to the world.
    #[track_caller]
    pub fn apply(&mut self, world: &mut World) {
        self.for_each(|info, ptr| {
            // SAFETY: the pointer is to a valid instance of the command as it
            // resides at the current index
            unsafe { info.call(ptr, world) };
        });
    }

    /// Borrows this buffer as a [`WorldQueue`].
    pub fn as_world_queue<'w, 's>(
        &'s mut self,
        world: &'w World,
    ) -> WorldQueue<'w, 's> {
        WorldQueue::new(world, self)
    }

    #[inline]
    fn for_each(
        &mut self,
        mut f: impl FnMut(&'static dyn CommandInfo, NonNull<u8>),
    ) {
        self.commands
            .drain(..)
            .scan(0, |byte_index, info| {
                // less-than-or-equal-to, as the command could be a ZST
                let ptr = (*byte_index <= self.bytes.len()).then(|| unsafe {
                    NonNull::new_unchecked(
                        self.bytes.as_mut_ptr().byte_add(*byte_index).cast(),
                    )
                });

                if ptr.is_some() {
                    *byte_index += info.size();
                }

                ptr.map(|ptr| (info, ptr))
            })
            .for_each(|(info, ptr)| f(info, ptr));
    }
}

/// # Safety
///
/// All commands are [`Send`].
unsafe impl Send for Commands {}

/// # Safety
///
/// [`Commands`] does not expose references to internal commands.
unsafe impl Sync for Commands {}

impl Drop for Commands {
    fn drop(&mut self) {
        self.for_each(|info, ptr| unsafe {
            info.drop()(ptr.as_ptr());
        });
    }
}

impl fmt::Debug for Commands {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Commands ")?;

        f.debug_list()
            .entries(self.commands.iter().copied().map(CommandInfo::name))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{self, AtomicBool};

    use super::*;
    use crate::entity::EntityId;
    use crate::prelude::{Bundle, Component};

    #[derive(Component)]
    struct Name(&'static str);

    #[derive(Component)]
    struct Age(u32);

    #[test]
    fn apply() {
        struct Spawn<B: Bundle>(B);

        impl<B: Bundle> Command for Spawn<B> {
            fn apply(self, world: &mut World) {
                world.spawn(self.0);
            }
        }

        let mut world = World::new();
        let mut commands = Commands::new();

        commands.push(Spawn((Name("Alexandra"), Age(u32::MAX))));
        commands.apply(&mut world);

        let query = world.query::<EntityId>().unwrap();
        let mut iter = query.iter();

        let alexandra = {
            let entity = iter.next().unwrap();

            world.entity(entity).unwrap()
        };
        let Name(name) = alexandra.get::<Name>().unwrap();
        let Age(age) = alexandra.get::<Age>().unwrap();

        assert_eq!(*name, "Alexandra");
        assert_eq!(*age, u32::MAX);
    }

    #[test]
    fn queue_drops_all_commands() {
        struct HasToDrop;

        static HAS_DROPPED: AtomicBool = AtomicBool::new(false);

        impl Command for HasToDrop {
            fn apply(self, _world: &mut World) {}
        }

        impl Drop for HasToDrop {
            fn drop(&mut self) {
                HAS_DROPPED.store(true, atomic::Ordering::Relaxed);
            }
        }

        let mut commands = Commands::new();

        commands.push(HasToDrop);
        drop(commands);

        assert!(HAS_DROPPED.load(atomic::Ordering::Relaxed));
    }
}
