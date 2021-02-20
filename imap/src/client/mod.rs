//! IMAP Client
//! ===
//!
//! The IMAP client in this module is implemented as a state machine in the type system: methods
//! that are not supported in a particular state (ex. fetch in an unauthenticated state) cannot be
//! expressed in the type system entirely.

mod inner;

use std::sync::Arc;

use anyhow::Result;
use tokio::{
    io::{self, AsyncRead, AsyncWrite, ReadHalf, WriteHalf},
    net::TcpStream,
};
use tokio_rustls::{client::TlsStream, rustls::ClientConfig, webpki::DNSNameRef, TlsConnector};

use self::inner::Client;

pub type ClientBuilder = ClientNotConnectedBuilder;

/// An IMAP client that hasn't been connected yet.
#[derive(Builder, Clone, Debug)]
pub struct ClientNotConnected {
    /// The hostname of the IMAP server. If using TLS, must be an address
    hostname: String,

    /// The port of the IMAP server.
    port: u16,

    /// Whether or not the client is using an encrypted stream.
    ///
    /// To upgrade the connection later, use the upgrade method.
    tls: bool,
}

impl ClientNotConnected {
    pub async fn connect(self) -> Result<ClientUnauthenticated> {
        let hostname = self.hostname.as_ref();
        let port = self.port;
        let conn = TcpStream::connect((hostname, port)).await?;

        if self.tls {
            let mut tls_config = ClientConfig::new();
            tls_config
                .root_store
                .add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
            let tls_config = TlsConnector::from(Arc::new(tls_config));
            let dnsname = DNSNameRef::try_from_ascii_str(hostname).unwrap();
            let conn = tls_config.connect(dnsname, conn).await?;

            let inner = Client::new(conn);
            return Ok(ClientUnauthenticated::Encrypted(
                ClientEncryptedUnauthenticated { inner },
            ));
        }

        let inner = Client::new(conn);
        return Ok(ClientUnauthenticated::Unencrypted(
            ClientUnencryptedUnauthenticated { inner },
        ));
    }
}

pub enum ClientUnauthenticated {
    Encrypted(ClientEncryptedUnauthenticated),
    Unencrypted(ClientUnencryptedUnauthenticated),
}

impl ClientUnauthenticated {}

pub struct ClientUnencryptedUnauthenticated {
    /// Connection to the remote server
    inner: Client<TcpStream>,
}

impl ClientUnencryptedUnauthenticated {
    pub async fn upgrade(&self) {}
}

/// An IMAP client that isn't authenticated.
pub struct ClientEncryptedUnauthenticated {
    /// Connection to the remote server
    inner: Client<TlsStream<TcpStream>>,
}

impl ClientEncryptedUnauthenticated {}
