//! Panorama/IMAP
//! ===
//!
//! This is a library that implements the IMAP protocol according to RFC 3501 and several
//! extensions. Although its primary purpose is to be used in panorama, it should be usable for
//! general-purpose IMAP usage. See the [client][crate::client] module for more information on how
//! to get started with a client quickly.

#[macro_use]
extern crate anyhow;
#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate derive_builder;
#[macro_use]
extern crate futures;
#[macro_use]
extern crate log;
#[macro_use]
extern crate pest_derive;

pub mod client;
pub mod codec;
pub mod command;
pub mod parser;
pub mod response;
