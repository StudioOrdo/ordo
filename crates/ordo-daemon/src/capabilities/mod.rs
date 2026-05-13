//! System capabilities registry and validation.
//! Manages governed operations, MCP export policies, and catalog resolution.

pub mod db;
pub(crate) mod registry;
pub mod types;

pub use db::*;
pub use registry::built_in_capabilities;
pub use types::*;
