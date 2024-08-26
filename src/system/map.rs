use std::borrow::Cow;
use std::marker::PhantomData;

use crate::ReadOnlySystemInput;
pub(crate) use crate::{ReadOnlySystem, System, SystemInput};

/// A [`System`] that maps the output of another system.
#[derive(Clone, Copy)]
pub struct MapSystem<S, F, O> {
    pub(super) system: S,
    pub(super) f: F,
    pub(super) _marker: PhantomData<O>,
}

unsafe impl<S, F, I, O, O_> System<I, O_> for MapSystem<S, F, O>
where
    S: System<I, O>,
    I: SystemInput,
    F: FnMut(O) -> O_,
{
    unsafe fn run(&mut self, input: I::Output<'_, '_>) -> O_ {
        (self.f)(unsafe { self.system.run(input) })
    }

    fn name(&self) -> Cow<'static, str> {
        self.system.name()
    }
}

unsafe impl<S, F, I, O, O_> ReadOnlySystem<I, O_> for MapSystem<S, F, O>
where
    S: System<I, O>,
    I: ReadOnlySystemInput,
    F: FnMut(O) -> O_,
{
}

#[cfg(test)]
mod tests {
    use crate::{ReadOnlySystem, SystemExt, World};

    #[test]
    fn map_system_output() {
        fn system(_world: &World) -> Option<usize> {
            Some(123)
        }

        let world = World::new();
        let mut mapped = system.map(Option::unwrap);

        assert_eq!(unsafe { mapped.run_from_ref_once(&world) }, 123);
    }
}
