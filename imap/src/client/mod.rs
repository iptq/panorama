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

use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use anyhow::Result;
use futures::{
    future::{self, FutureExt},
    stream::{Stream, StreamExt},
};
use tokio::{
    net::TcpStream,
    sync::{mpsc, oneshot},
    task::JoinHandle,
};
use tokio_rustls::{
    client::TlsStream, rustls::ClientConfig as RustlsConfig, webpki::DNSNameRef, TlsConnector,
};

use crate::command::{Command, FetchItems, SearchCriteria};
use crate::response::{
    AttributeValue, Envelope, MailboxData, MailboxFlag, Response, ResponseCode, ResponseData,
    ResponseDone, Status,
};

pub use self::inner::{Client, ResponseStream};

/// Struct used to start building the config for a client.
///
/// Call [`.build`][1] to _build_ the config, then run [`.open`][2] to actually start opening
/// the connection to the server.
///
/// [1]: self::ClientConfigBuilder::build
/// [2]: self::ClientConfig::open
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

            let mut inner = Client::new(conn, self);
            inner.wait_for_greeting().await?;
            return Ok(ClientUnauthenticated::Encrypted(inner));
        } else {
            let mut inner = Client::new(conn, self);
            inner.wait_for_greeting().await?;
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
    async fn execute(&mut self, cmd: Command) -> Result<ResponseStream> {
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
    async fn execute(&mut self, cmd: Command) -> Result<ResponseStream> {
        match self {
            ClientAuthenticated::Encrypted(e) => e.execute(cmd).await,
            ClientAuthenticated::Unencrypted(e) => e.execute(cmd).await,
        }
    }

    fn sender(&self) -> mpsc::UnboundedSender<String> {
        match self {
            ClientAuthenticated::Encrypted(e) => e.write_tx.clone(),
            ClientAuthenticated::Unencrypted(e) => e.write_tx.clone(),
        }
    }

    /// Checks if the server that the client is talking to has support for the given capability.
    pub async fn has_capability(&mut self, cap: impl AsRef<str>) -> Result<bool> {
        match self {
            ClientAuthenticated::Encrypted(e) => e.has_capability(cap).await,
            ClientAuthenticated::Unencrypted(e) => e.has_capability(cap).await,
        }
    }

    /// Runs the LIST command
    pub async fn list(&mut self) -> Result<Vec<String>> {
        let cmd = Command::List {
            reference: "".to_owned(),
            mailbox: "*".to_owned(),
        };

        let res = self.execute(cmd).await?;
        let (_, data) = res.wait().await?;

        let mut folders = Vec::new();
        for resp in data {
            if let Response::MailboxData(MailboxData::List { name, .. }) = resp {
                folders.push(name.to_owned());
            }
        }

        Ok(folders)
    }

    /// Runs the SELECT command
    pub async fn select(&mut self, mailbox: impl AsRef<str>) -> Result<SelectResponse> {
        let cmd = Command::Select {
            mailbox: mailbox.as_ref().to_owned(),
        };

        let stream = self.execute(cmd).await?;
        let (_, data) = stream.wait().await?;

        let mut select = SelectResponse::default();
        for resp in data {
            match resp {
                Response::MailboxData(MailboxData::Flags(flags)) => select.flags = flags,
                Response::MailboxData(MailboxData::Exists(exists)) => select.exists = Some(exists),
                Response::MailboxData(MailboxData::Recent(recent)) => select.recent = Some(recent),
                Response::Data(ResponseData {
                    status: Status::Ok,
                    code: Some(code),
                    ..
                }) => match code {
                    ResponseCode::Unseen(value) => select.unseen = Some(value),
                    ResponseCode::UidNext(value) => select.uid_next = Some(value),
                    ResponseCode::UidValidity(value) => select.uid_validity = Some(value),
                    _ => {}
                },
                _ => {}
            }
        }

        Ok(select)
    }

    /// Runs the SEARCH command
    pub async fn uid_search(&mut self) -> Result<Vec<u32>> {
        let cmd = Command::UidSearch {
            criteria: SearchCriteria::All,
        };
        let stream = self.execute(cmd).await?;
        let (_, data) = stream.wait().await?;
        for resp in data {
            if let Response::MailboxData(MailboxData::Search(uids)) = resp {
                return Ok(uids);
            }
        }
        bail!("could not find the SEARCH response")
    }

    /// Runs the FETCH command
    pub async fn fetch(
        &mut self,
        uids: &[u32],
        items: FetchItems,
    ) -> Result<impl Stream<Item = (u32, Vec<AttributeValue>)>> {
        let cmd = Command::Fetch {
            uids: uids.to_vec(),
            items,
        };
        debug!("fetch: {}", cmd);
        let stream = self.execute(cmd).await?;
        // let (done, data) = stream.wait().await?;
        Ok(stream.filter_map(|resp| match resp {
            Response::Fetch(n, attrs) => future::ready(Some((n, attrs))).boxed(),
            Response::Done(_) => future::ready(None).boxed(),
            _ => future::pending().boxed(),
        }))
    }

    /// Runs the UID FETCH command
    pub async fn uid_fetch(
        &mut self,
        uids: &[u32],
        items: FetchItems,
    ) -> Result<impl Stream<Item = (u32, Vec<AttributeValue>)>> {
        let cmd = Command::UidFetch {
            uids: uids.to_vec(),
            items,
        };
        debug!("uid fetch: {}", cmd);
        let stream = self.execute(cmd).await?;
        // let (done, data) = stream.wait().await?;
        Ok(stream.filter_map(|resp| match resp {
            Response::Fetch(n, attrs) => future::ready(Some((n, attrs))).boxed(),
            Response::Done(_) => future::ready(None).boxed(),
            _ => future::pending().boxed(),
        }))
    }

    /// Runs the IDLE command
    #[cfg(feature = "rfc2177-idle")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rfc2177-idle")))]
    pub async fn idle(&mut self) -> Result<IdleToken> {
        let cmd = Command::Idle;
        let stream = self.execute(cmd).await?;
        let sender = self.sender();
        Ok(IdleToken { stream, sender })
    }
}

#[derive(Debug, Default)]
pub struct SelectResponse {
    pub flags: Vec<MailboxFlag>,
    pub exists: Option<u32>,
    pub recent: Option<u32>,
    pub uid_next: Option<u32>,
    pub uid_validity: Option<u32>,
    pub unseen: Option<u32>,
}

/// A token that represents an idling connection.
///
/// Dropping this token indicates that the idling should be completed, and the DONE command will be
/// sent to the server as a result.
#[cfg(feature = "rfc2177-idle")]
#[cfg_attr(docsrs, doc(cfg(feature = "rfc2177-idle")))]
pub struct IdleToken {
    pub stream: ResponseStream,
    sender: mpsc::UnboundedSender<String>,
}

#[cfg(feature = "rfc2177-idle")]
#[cfg_attr(docsrs, doc(cfg(feature = "rfc2177-idle")))]
impl Drop for IdleToken {
    fn drop(&mut self) {
        // TODO: should ignore this?
        self.sender.send(format!("DONE\r\n")).unwrap();
    }
}

#[cfg(feature = "rfc2177-idle")]
#[cfg_attr(docsrs, doc(cfg(feature = "rfc2177-idle")))]
impl Stream for IdleToken {
    type Item = Response;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let stream = Pin::new(&mut self.stream);
        Stream::poll_next(stream, cx)
    }
}
