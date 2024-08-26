use std::borrow::Cow;

use crate::{System, SystemInput};

/// A [`System`] with a new [name](System::name).
#[derive(Clone)]
pub struct NamedSystem<S> {
    pub(super) system: S,
    pub(super) name: Cow<'static, str>,
}

unsafe impl<S, I, O> System<I, O> for NamedSystem<S>
where
    S: System<I, O>,
    I: SystemInput,
{
    unsafe fn run(&mut self, input: I::Output<'_, '_>) -> O {
        unsafe { self.system.run(input) }
    }

    fn name(&self) -> Cow<'static, str> {
        self.name.clone()
    }

    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.system.debug(f)
    }
}
