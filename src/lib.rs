//! Panorama
//! ===

#![deny(missing_docs)]

#[macro_use]
extern crate anyhow;
#[macro_use]
extern crate crossterm;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde;

pub mod config;
pub mod mail;
pub mod ui;

/// A cloneable type that allows sending an exit-"signal" to stop the application.
pub type ExitSender = tokio::sync::oneshot::Sender<()>;
