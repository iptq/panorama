// let's try this again

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use futures::{
    future::{Future, TryFuture},
    stream::Stream,
};
use panorama_imap::builders::command::Command;
use parking_lot::Mutex;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
    sync::{oneshot, Notify},
};
use tokio_rustls::{rustls::ClientConfig, webpki::DNSNameRef, TlsConnector};
use tokio_util::codec::{Framed, Decoder, LinesCodec};

use crate::config::{ImapConfig, TlsMethod};

pub async fn open_imap_connection(config: ImapConfig) -> Result<()> {
    let server = config.server.as_str();
    let port = config.port;

    let stream = TcpStream::connect((server, port)).await?;

    match config.tls {
        TlsMethod::Off => begin_authentication(config, stream).await,
        TlsMethod::On => {
            let stream = perform_tls_negotiation(server.to_owned(), stream).await?;
            begin_authentication(config, stream).await
        }
        TlsMethod::Starttls => {
            let cmd_mgr = CommandManager::new(stream);
            todo!()
        }
    }
}

/// Performs TLS negotiation, using the webpki_roots and verifying the server name
async fn perform_tls_negotiation(
    server_name: impl AsRef<str>,
    stream: impl AsyncRead + AsyncWrite + Unpin,
) -> Result<impl AsyncRead + AsyncWrite> {
    let server_name = server_name.as_ref();

    let mut tls_config = ClientConfig::new();
    tls_config
        .root_store
        .add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
    let tls_config = TlsConnector::from(Arc::new(tls_config));
    let dnsname = DNSNameRef::try_from_ascii_str(server_name).unwrap();
    let stream = tls_config.connect(dnsname, stream).await?;

    Ok(stream)
}

async fn fetch_capabilities(stream: impl AsyncRead + AsyncWrite) -> Result<Vec<String>> {
    let codec = LinesCodec::new();
    let framed = codec.framed(stream);

    // framed.send("a0 CAPABILITY");
    todo!()
}

async fn begin_authentication(
    config: ImapConfig,
    stream: impl AsyncRead + AsyncWrite,
) -> Result<()> {
    Ok(())
}

trait ImapStream: AsyncRead + AsyncWrite + Unpin {}
impl<T: AsyncRead + AsyncWrite + Unpin> ImapStream for T {}

struct CommandManager<'a> {
    id: usize,
    in_flight: Arc<Mutex<HashMap<usize, oneshot::Sender<()>>>>,
    stream: Framed<Box<dyn ImapStream + 'a>, LinesCodec>,
}

impl<'a> CommandManager<'a> {
    pub fn new(stream: impl ImapStream + 'a) -> Self {
        let codec = LinesCodec::new();
        let framed = codec.framed(Box::new(stream) as Box<_>);

        CommandManager {
            id: 0,
            in_flight: Arc::new(Mutex::new(HashMap::new())),
            stream: framed,
        }
    }

    pub fn decompose(self) -> impl ImapStream + 'a {
        let parts = self.stream.into_parts();
        parts.io
    }

    pub async fn listen(&self) {
        loop {
        }
    }

    pub fn run(&mut self, command: Command) -> impl TryFuture {
        let id = self.id;
        self.id += 1;

        let (tx, rx) = oneshot::channel();
        {
            let mut in_flight = self.in_flight.lock();
            in_flight.insert(id, tx);
        }

        async { rx.await }
    }
}
