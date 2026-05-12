//! Conversation Gateway module.
//! Handles WebSocket connections, protocol frame processing, and internal dispatching.

pub mod api;
pub(crate) mod handlers;
pub mod types;

pub use api::*;
pub use types::*;
