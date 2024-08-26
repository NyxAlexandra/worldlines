use std::marker::PhantomData;

use crate::{EntityRef, QueryFilter};

/// A [`QueryFilter`] that inverts another filter.
pub struct Not<F: QueryFilter>(PhantomData<F>);

impl<F: QueryFilter> QueryFilter for Not<F> {
    fn include(entity: EntityRef<'_>) -> bool {
        !F::include(entity)
    }
}
