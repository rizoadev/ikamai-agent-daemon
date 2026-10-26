//! IKAMAI Agent Daemon — core library.
//!
//! Contains domain models, agent registry, task router,
//! event bus, scheduler, and health monitor.

pub mod config;
pub mod error;
pub mod models;

pub use error::{Error, Result};
