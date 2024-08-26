#![forbid(unsafe_op_in_unsafe_fn)]

// allows using derive macros that reference `archetypal_ecs` internally
extern crate self as archetypal_ecs;

pub use self::access::*;
pub use self::app::*;
pub use self::bundle::*;
pub use self::component::*;
pub use self::entity::*;
pub use self::query::*;
pub use self::queue::*;
pub use self::resource::*;
pub use self::schedule::*;
pub use self::system::*;
pub use self::util::*;
pub use self::world::*;

mod access;
mod app;
mod bundle;
mod component;
mod entity;
mod query;
mod queue;
mod resource;
mod schedule;
mod system;
mod util;
mod world;
