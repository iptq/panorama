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
//! - [ClientBuilder][self::ClientBuilder] : Constructs the config for the IMAP client

mod inner;

use std::sync::Arc;

use anyhow::Result;
use tokio::net::TcpStream;
use tokio_rustls::{
    client::TlsStream, rustls::ClientConfig as RustlsConfig, webpki::DNSNameRef, TlsConnector,
};

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
            return Ok(ClientUnauthenticated::Encrypted(
                ClientUnauthenticatedEncrypted { inner },
            ));
        } else {
            let inner = Client::new(conn, self);
            inner.wait_for_greeting().await;
            return Ok(ClientUnauthenticated::Unencrypted(
                ClientUnauthenticatedUnencrypted { inner },
            ));
        }
    }
}

pub enum ClientUnauthenticated {
    Encrypted(ClientUnauthenticatedEncrypted),
    Unencrypted(ClientUnauthenticatedUnencrypted),
}

impl ClientUnauthenticated {
    pub async fn upgrade(self) -> Result<ClientUnauthenticated> {
        match self {
            // this is a no-op, we don't need to upgrade
            ClientUnauthenticated::Encrypted(_) => Ok(self),
            ClientUnauthenticated::Unencrypted(e) => {
                let client = ClientUnauthenticatedEncrypted {
                    inner: e.inner.upgrade().await?,
                };
                Ok(ClientUnauthenticated::Encrypted(client))
            }
        }
    }

    pub async fn capabilities(&mut self) -> Result<()> {
        match self {
            ClientUnauthenticated::Encrypted(e) => e.inner.capabilities().await?,
            ClientUnauthenticated::Unencrypted(e) => e.inner.capabilities().await?,
        }
        Ok(())
    }
}

pub struct ClientUnauthenticatedUnencrypted {
    /// Connection to the remote server
    inner: Client<TcpStream>,
}

impl ClientUnauthenticatedUnencrypted {
    pub async fn upgrade(&self) {}
}

/// An IMAP client that isn't authenticated.
pub struct ClientUnauthenticatedEncrypted {
    /// Connection to the remote server
    inner: Client<TlsStream<TcpStream>>,
}

impl ClientUnauthenticatedEncrypted {}
