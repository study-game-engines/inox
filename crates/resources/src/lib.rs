#![warn(clippy::all)]

pub use crate::buffer::*;
pub use crate::hash_buffer::*;

pub use crate::config::*;
pub use crate::data::*;
pub use crate::event_handlers::*;
pub use crate::events::*;
pub use crate::resource::*;
pub use crate::shared_data::*;
pub use crate::singleton::*;
pub use crate::storage::*;

pub mod buffer;
pub mod hash_buffer;

pub mod config;
pub mod data;
pub mod event_handlers;
pub mod events;
pub mod platform;
pub mod resource;
pub mod shared_data;
pub mod singleton;
pub mod storage;
