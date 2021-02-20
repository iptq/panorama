// let's try this again

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use futures::{
    future::{self, BoxFuture, Future, FutureExt, TryFuture},
    sink::{Sink, SinkExt},
    stream::{Stream, StreamExt},
};
use panorama_imap::builders::command::Command;
use parking_lot::Mutex;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
    sync::{oneshot, Notify},
};
use tokio_rustls::{rustls::ClientConfig, webpki::DNSNameRef, TlsConnector};
use tokio_util::codec::{Decoder, Framed, FramedRead, FramedWrite, LinesCodec, LinesCodecError};

use crate::config::{ImapConfig, TlsMethod};

pub async fn open_imap_connection(config: ImapConfig) -> Result<()> {
    let server = config.server.as_str();
    let port = config.port;

    let stream = TcpStream::connect((server, port)).await?;

    debug!("hellosu");
    match config.tls {
        TlsMethod::Off => begin_authentication(config, stream).await,
        TlsMethod::On => {
            let stream = perform_tls_negotiation(server.to_owned(), stream).await?;
            begin_authentication(config, stream).await
        }
        TlsMethod::Starttls => {
            let (stream, cmd_mgr) = CommandManager::new(stream);
            let flights = cmd_mgr.flights();

            // listen(stream, flights).await?;
            // async move {
            //     let mut cmd_mgr = cmd_mgr;
            //     cmd_mgr.capabilities().await;
            // }
            // .await;

            todo!()
        }
    }
}

/// Performs TLS negotiation, using the webpki_roots and verifying the server name
#[instrument(skip(server_name, stream))]
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

    todo!()
}

#[instrument(skip(config, stream))]
async fn begin_authentication(
    config: ImapConfig,
    stream: impl AsyncRead + AsyncWrite,
) -> Result<()> {
    Ok(())
}

pub async fn listen(
    mut stream: impl Stream<Item = Result<String, LinesCodecError>> + Unpin,
    in_flight: InFlight,
) -> Result<()> {
    debug!("listening for messages from server");
    loop {
        let line = match stream.next().await {
            Some(v) => v?,
            None => break,
        };
        debug!("line: {:?}", line);

        let mut parts = line.split(' ');
        let tag = parts.next().unwrap().parse()?; // TODO: handle empty

        {
            let mut in_flight = in_flight.lock();
            if let Some(sender) = in_flight.remove(&tag) {
                sender.send(()).unwrap();
            }
        }
    }

    Ok(())
}

// trait ImapStream: AsyncRead + AsyncWrite + Send + Unpin {}
// impl<T: AsyncRead + AsyncWrite + Send + Unpin> ImapStream for T {}

trait ImapSink: Sink<String, Error = LinesCodecError> + Unpin {}
impl<T: Sink<String, Error = LinesCodecError> + Unpin> ImapSink for T {}

type InFlightMap = HashMap<usize, oneshot::Sender<()>>;
type InFlight = Arc<Mutex<InFlightMap>>;

struct CommandManager<'a> {
    id: usize,
    in_flight: Arc<Mutex<HashMap<usize, oneshot::Sender<()>>>>,
    sink: Box<dyn ImapSink + 'a>,
}

impl<'a> CommandManager<'a> {
    pub fn new(
        stream: impl AsyncRead + AsyncWrite + 'a,
    ) -> (impl Stream<Item = Result<String, LinesCodecError>>, Self) {
        let codec = LinesCodec::new();
        let framed = codec.framed(stream);
        let (framed_sink, framed_stream) = framed.split();

        let cmd_mgr = CommandManager {
            id: 0,
            in_flight: Arc::new(Mutex::new(HashMap::new())),
            sink: Box::new(framed_sink),
        };
        (framed_stream, cmd_mgr)
    }

    pub fn flights(&self) -> Arc<Mutex<HashMap<usize, oneshot::Sender<()>>>> {
        self.in_flight.clone()
    }

    pub fn decompose(self) -> impl ImapSink + Unpin + 'a {
        self.sink
    }

    pub async fn capabilities(&mut self) -> Result<Vec<String>> {
        self.exec(Command {
            args: b"CAPABILITY".to_vec(),
            next_state: None,
        })
        .await?;
        Ok(vec![])
    }

    pub async fn exec(&mut self, command: Command) -> Result<()> {
        let id = self.id;
        self.id += 1;

        let cmd_str = String::from_utf8(command.args)?;
        self.sink.send(cmd_str).await?;

        let (tx, rx) = oneshot::channel();
        {
            let mut in_flight = self.in_flight.lock();
            in_flight.insert(id, tx);
        }

        rx.await?;
        Ok(())
    }
}
