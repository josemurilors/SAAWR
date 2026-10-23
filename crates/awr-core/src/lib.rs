//! SA-AWR core: all business logic, no GUI.

pub mod audio;
pub mod cargrp;
pub mod checker;
pub mod config;
pub mod error;
pub mod fla;
pub mod fsutil;
pub mod handling;
pub mod ide;
pub mod ids;
pub mod injector;
pub mod inventory;
pub mod meta;
pub mod paths;
pub mod references;
#[cfg(test)]
mod testutil;
pub mod text;
pub mod validators;

pub use error::{AwrError, Result};
