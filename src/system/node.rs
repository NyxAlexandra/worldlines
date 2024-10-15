use std::fmt;

use crate::{
    AnySystem,
    ReadOnlySystem,
    ReadOnlySystemInput,
    System,
    SystemInput,
    World,
    WorldAccess,
    WorldPtr,
};

/// A [`System`] in an [`App`](crate::App) and its run conditions.
pub struct SystemNode<O = ()> {
    inner: AnySystem<O>,
    is_valid: Option<bool>,
    conditions: Vec<AnySystem<bool>>,
}

/// Traits for collections of systems.
///
/// Used to easily insert systems into an [`App`](crate::App).
pub trait IntoSystemNodes<I: SystemInput, O = ()> {
    /// Returns the systems of this object.
    fn into_system_nodes(self) -> impl Iterator<Item = SystemNode<O>>;
}

impl<O> SystemNode<O> {
    /// Creates a new system node.
    pub fn new<I: SystemInput + 'static>(
        system: impl System<I, O> + 'static,
    ) -> Self {
        let inner = AnySystem::new(system);
        let is_valid = None;
        let conditions = Vec::new();

        Self { inner, is_valid, conditions }
    }

    /// Returns `true` if the system has pending deferred operations.
    pub fn should_apply(&self) -> bool {
        self.inner.should_apply()
    }

    /// Inserts a run condition into this system node.
    pub fn insert_condition<I: ReadOnlySystemInput + 'static>(
        &mut self,
        condition: impl ReadOnlySystem<I, bool> + 'static,
    ) {
        self.conditions.push(AnySystem::new(condition));
    }

    /// Inserts a run condition into this system node and returns `self`.
    pub fn and_insert_condition<I: ReadOnlySystemInput + 'static>(
        mut self,
        condition: impl ReadOnlySystem<I, bool> + 'static,
    ) -> Self {
        self.insert_condition(condition);

        self
    }

    /// Updates a [`WorldAccess`] with the access of this system.
    pub fn access(&self, access: &mut WorldAccess) {
        self.inner.access(access);
    }

    /// Run this system from a pointer.
    ///
    /// # Safety
    ///
    /// The pointer must be valid for the access of this system.
    pub unsafe fn run_from(&mut self, world: WorldPtr<'_>) -> Option<O> {
        self.validate();

        if self.is_valid != Some(true)
            || self
                .conditions
                .iter_mut()
                .any(|condition| unsafe { !condition.run(world) })
        {
            return None;
        }

        Some(unsafe { self.inner.run(world) })
    }

    /// Run this system from a mutable reference.
    ///
    /// Only runs if the conditions are met.
    pub fn run_from_mut(&mut self, world: &mut World) -> Option<O> {
        unsafe { self.run_from(world.as_ptr_mut()) }
    }

    /// Run this system from a mutable reference.
    ///
    /// Will run even if conditions aren't met.
    pub fn force_run_from_mut(&mut self, world: &mut World) -> O {
        unsafe { self.inner.run_from_mut(world) }
    }

    /// Applies deffered operations.
    pub fn apply(&mut self, world: &mut World) {
        self.inner.apply(world);
    }

    /// Apply deferred operations if [`SystemInput::should_apply`].
    pub fn try_apply(&mut self, world: &mut World) -> Option<()> {
        self.inner.try_apply(world)
    }

    fn validate(&mut self) {
        if self.is_valid.is_none() {
            let mut access = WorldAccess::new();

            self.access(&mut access);

            self.is_valid = Some(access.is_valid());
        }
    }
}

impl<O> fmt::Debug for SystemNode<O> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

macro_rules! impl_into_system_nodes {
    ($(($s:ident, $i:ident)),*) => {
        impl_into_system_nodes!([] [$(($s, $i))*]);
    };

    ([$(($s:ident, $i:ident))*] []) => {
        impl<$($s, $i,)* O> IntoSystemNodes<($($i,)*), O> for ($($s,)*)
        where
            $(
                $s: System<$i, O> + 'static,
                $i: SystemInput + 'static,
            )*
        {
            #[allow(unused_variables, non_snake_case)]
            fn into_system_nodes(self) -> impl Iterator<Item = SystemNode<O>> {
                let ($($s,)*) = self;

                let iter = ::std::iter::empty();

                $(
                    let iter = iter.chain(::std::iter::once(SystemNode::new($s)));
                )*

                iter
            }
        }
    };

    (
        [$(($rest_s:ident, $rest_i:ident))*]
        [($head_s:ident, $head_i:ident) $(($tail_s:ident, $tail_i:ident))*]
    ) => {
        impl_into_system_nodes!([$(($rest_s, $rest_i))*] []);
        impl_into_system_nodes!([$(($rest_s, $rest_i))* ($head_s, $head_i)] [$(($tail_s, $tail_i))*]);
    };
}

impl_into_system_nodes!(
    (S0, I0),
    (S1, I1),
    (S2, I2),
    (S3, I3),
    (S4, I4),
    (S5, I5),
    (S6, I6),
    (S7, I7),
    (S8, I8),
    (S9, I9),
    (S10, I10),
    (S11, I11),
    (S12, I12),
    (S13, I13),
    (S14, I14),
    (S15, I15)
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dont_run_if_condition_not_fulfilled() {
        fn don_run_this_system() {
            panic!("I told you... :(");
        }

        let mut world = World::new();
        let mut system =
            SystemNode::new(don_run_this_system).and_insert_condition(|| false);

        _ = system.run_from_mut(&mut world);
    }
}
