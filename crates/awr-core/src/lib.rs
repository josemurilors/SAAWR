//! SA-AWR core: all business logic, no GUI.

pub mod adopt;
pub mod audio;
pub mod batch;
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
pub mod pack;
pub mod paths;
pub mod pickups;
pub mod readme;
pub mod references;
pub mod shop;
#[cfg(test)]
mod testutil;
pub mod text;
pub mod trainer;
pub mod tuning;
pub mod validators;
pub mod weapons;

pub use error::{AwrError, Result};
