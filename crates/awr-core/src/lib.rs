//! SA-AWR core: all business logic, no GUI.

pub mod config;
pub mod error;
pub mod fsutil;
pub mod inventory;
pub mod meta;
pub mod paths;

pub use error::{AwrError, Result};
