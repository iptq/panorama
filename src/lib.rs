//! Panorama
//! ===

#![deny(unsafe_code)]
#![deny(missing_docs)]
// TODO: get rid of this before any kind of public release
#![allow(unused_imports, unused_variables)]

#[macro_use]
extern crate anyhow;
#[macro_use]
extern crate crossterm;
#[macro_use]
extern crate format_bytes;
#[macro_use]
extern crate serde;
#[macro_use]
extern crate sqlx;
#[macro_use]
extern crate log;

pub mod config;
pub mod mail;
pub mod script;
pub mod ui;

/// Consumes any error and dumps it to the logger.
pub fn report_err(err: anyhow::Error) {
    error!("error: {}", err);
}
