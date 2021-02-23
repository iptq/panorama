//! IMAP Client
//! ===
//!
//! The IMAP client in this module is implemented as a state machine in the type system: methods
//! that are not supported in a particular state (ex. fetch in an unauthenticated state) cannot be
//! expressed in the type system entirely.
//!
//! Because there's many client types for the different types of clients, you'll want to start
//! here:
//!
//! - [`ClientBuilder`][self::ClientBuilder] : Constructs the config for the IMAP client
//!
//! If you choose not to use the high-level type-safe features of `ClientBuilder`, then you can
//! also choose to access the lower level [`Client`][self::inner::Client] directly.
//!
//! Example
//! ---
//!
//! The following example connects to `mywebsite.com:143` using STARTTLS.
//!
//! ```no_run
//! # use anyhow::Result;
//! # use panorama_imap::client::ClientConfigBuilder;
//! # async fn test() -> Result<()> {
//! let config = ClientConfigBuilder::default()
//!     .hostname("mywebsite.com".to_owned())
//!     .port(143)
//!     .tls(false)
//!     .build().unwrap();
//! let insecure = config.open().await?;
//! let unauth = insecure.upgrade().await?;
//! # Ok(())
//! # }
//! ```

pub mod auth;
mod inner;

use std::sync::Arc;

use anyhow::Result;
use tokio::net::TcpStream;
use tokio_rustls::{
    client::TlsStream, rustls::ClientConfig as RustlsConfig, webpki::DNSNameRef, TlsConnector,
};

use crate::command::Command;
use crate::response::Response;

pub use self::inner::Client;

/// Struct used to start building the config for a client.
///
/// Call [`.build`][1] to _build_ the config, then run [`.connect`][2] to actually start opening
/// the connection to the server.
///
/// [1]: self::ClientConfigBuilder::build
/// [2]: self::ClientConfig::connect
pub type ClientBuilder = ClientConfigBuilder;

/// An IMAP client that hasn't been connected yet.
#[derive(Builder, Clone, Debug)]
pub struct ClientConfig {
    /// The hostname of the IMAP server. If using TLS, must be an address
    hostname: String,

    /// The port of the IMAP server.
    port: u16,

    /// Whether or not the client is using an encrypted stream.
    ///
    /// To upgrade the connection later, use the upgrade method.
    tls: bool,
}

impl ClientConfig {
    pub async fn open(self) -> Result<ClientUnauthenticated> {
        let hostname = self.hostname.as_ref();
        let port = self.port;
        let conn = TcpStream::connect((hostname, port)).await?;

        if self.tls {
            let mut tls_config = RustlsConfig::new();
            tls_config
                .root_store
                .add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
            let tls_config = TlsConnector::from(Arc::new(tls_config));
            let dnsname = DNSNameRef::try_from_ascii_str(hostname).unwrap();
            let conn = tls_config.connect(dnsname, conn).await?;

            let inner = Client::new(conn, self);
            inner.wait_for_greeting().await;
            return Ok(ClientUnauthenticated::Encrypted(inner));
        } else {
            let inner = Client::new(conn, self);
            inner.wait_for_greeting().await;
            return Ok(ClientUnauthenticated::Unencrypted(inner));
        }
    }
}

pub enum ClientUnauthenticated {
    Encrypted(Client<TlsStream<TcpStream>>),
    Unencrypted(Client<TcpStream>),
}

impl ClientUnauthenticated {
    pub async fn upgrade(self) -> Result<ClientUnauthenticated> {
        match self {
            // this is a no-op, we don't need to upgrade
            ClientUnauthenticated::Encrypted(_) => Ok(self),
            ClientUnauthenticated::Unencrypted(e) => {
                Ok(ClientUnauthenticated::Encrypted(e.upgrade().await?))
            }
        }
    }

    /// Exposing low-level execute
    async fn execute(&mut self, cmd: Command) -> Result<(Response, Vec<Response>)> {
        match self {
            ClientUnauthenticated::Encrypted(e) => e.execute(cmd).await,
            ClientUnauthenticated::Unencrypted(e) => e.execute(cmd).await,
        }
    }

    /// Checks if the server that the client is talking to has support for the given capability.
    pub async fn has_capability(&mut self, cap: impl AsRef<str>) -> Result<bool> {
        match self {
            ClientUnauthenticated::Encrypted(e) => e.has_capability(cap).await,
            ClientUnauthenticated::Unencrypted(e) => e.has_capability(cap).await,
        }
    }
}

pub enum ClientAuthenticated {
    Encrypted(Client<TlsStream<TcpStream>>),
    Unencrypted(Client<TcpStream>),
}

impl ClientAuthenticated {
    /// Exposing low-level execute
    async fn execute(&mut self, cmd: Command) -> Result<(Response, Vec<Response>)> {
        match self {
            ClientAuthenticated::Encrypted(e) => e.execute(cmd).await,
            ClientAuthenticated::Unencrypted(e) => e.execute(cmd).await,
        }
    }

    pub async fn list(&mut self) -> Result<()> {
        let cmd = Command::List {
            reference: "".to_owned(),
            mailbox: "*".to_owned(),
        };
        let resp = self.execute(cmd).await?;
        debug!("list response: {:?}", resp);
        Ok(())
    }
}
