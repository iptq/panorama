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
#[macro_use]
extern crate tracing;

pub mod config;
pub mod mail;
pub mod ui;

/// A cloneable type that allows sending an exit-"signal" to stop the application.
pub type ExitSender = tokio::sync::mpsc::Sender<()>;

/// Consumes any error and dumps it to the logger.
pub fn report_err(err: anyhow::Error) {
    error!("error: {:?}", err);
}
