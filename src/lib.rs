//! A simplistic archetypal ECS implementation.

#![forbid(unsafe_op_in_unsafe_fn)]

// allows referencing this crate as `worldlines` in derives
extern crate self as worldlines;

pub mod access;
pub mod commands;
pub mod component;
pub mod entity;
pub mod query;
pub mod resource;
mod storage;
pub mod system;
pub mod world;
/// Re-export of all items in this crate.
pub mod prelude {
    pub use crate::access::*;
    pub use crate::commands::*;
    pub use crate::component::*;
    pub use crate::entity::*;
    pub use crate::query::*;
    pub use crate::resource::*;
    pub use crate::system::*;
    pub use crate::world::*;
}
