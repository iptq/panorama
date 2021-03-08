use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use anyhow::Result;
use futures::{
    future::{self, FutureExt, TryFutureExt},
    stream::{Stream, StreamExt},
};
use tokio::{
    io::{
        self, AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, ReadHalf, WriteHalf,
    },
    sync::{
        mpsc,
        oneshot::{self, error::TryRecvError},
    },
    task::JoinHandle,
};
use tokio_rustls::{
    client::TlsStream, rustls::ClientConfig as RustlsConfig, webpki::DNSNameRef, TlsConnector,
};
use tokio_util::codec::FramedRead;

use crate::codec::ImapCodec;
use crate::command::Command;
use crate::parser::{parse_capability, parse_response};
use crate::response::{Response, ResponseDone};

use super::ClientConfig;

pub const TAG_PREFIX: &str = "ptag";
type Command2 = (Command, mpsc::UnboundedSender<Response>);

pub struct Client<C> {
    ctr: usize,
    config: ClientConfig,
    conn: WriteHalf<C>,
    cmd_tx: mpsc::UnboundedSender<Command2>,
    greeting_rx: Option<oneshot::Receiver<()>>,
    exit_tx: oneshot::Sender<()>,
    listener_handle: JoinHandle<Result<ReadHalf<C>>>,
}

impl<C> Client<C>
where
    C: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    pub fn new(conn: C, config: ClientConfig) -> Self {
        let (read_half, write_half) = io::split(conn);
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let (greeting_tx, greeting_rx) = oneshot::channel();
        let (exit_tx, exit_rx) = oneshot::channel();
        let handle = tokio::spawn(
            listen(read_half, cmd_rx, greeting_tx, exit_rx).map_err(|err| {
                error!("Help, the listener loop died: {}", err);
                err
            }),
        );

        Client {
            ctr: 0,
            conn: write_half,
            config,
            cmd_tx,
            greeting_rx: Some(greeting_rx),
            exit_tx,
            listener_handle: handle,
        }
    }

    pub async fn wait_for_greeting(&mut self) -> Result<()> {
        if let Some(greeting_rx) = self.greeting_rx.take() {
            greeting_rx.await?;
        }
        Ok(())
    }

    pub async fn execute(&mut self, cmd: Command) -> Result<ResponseStream> {
        let id = self.ctr;
        self.ctr += 1;

        let cmd_str = format!("{}{} {}\r\n", TAG_PREFIX, id, cmd);
        self.conn.write_all(cmd_str.as_bytes()).await?;
        self.conn.flush().await?;

        let (tx, rx) = mpsc::unbounded_channel();
        self.cmd_tx.send((cmd, tx))?;

        Ok(ResponseStream { inner: rx })
    }

    pub async fn has_capability(&mut self, cap: impl AsRef<str>) -> Result<bool> {
        // TODO: cache capabilities if needed?
        let cap = cap.as_ref();
        let cap = parse_capability(cap)?;

        let resp = self.execute(Command::Capability).await?;
        let (_, data) = resp.wait().await?;

        for resp in data {
            if let Response::Capabilities(caps) = resp {
                return Ok(caps.contains(&cap));
            }
            // debug!("cap: {:?}", resp);
        }

        Ok(false)
    }

    pub async fn upgrade(mut self) -> Result<Client<TlsStream<C>>> {
        // TODO: make sure STARTTLS is in the capability list
        if !self.has_capability("STARTTLS").await? {
            bail!("server doesn't support this capability");
        }

        // first, send the STARTTLS command
        let mut resp = self.execute(Command::Starttls).await?;
        let resp = resp.next().await.unwrap();
        debug!("server response to starttls: {:?}", resp);

        debug!("sending exit for upgrade");
        // TODO: check that the channel is still open?
        self.exit_tx.send(()).unwrap();
        let reader = self.listener_handle.await??;
        let writer = self.conn;

        let conn = reader.unsplit(writer);
        let server_name = &self.config.hostname;

        let mut tls_config = RustlsConfig::new();
        tls_config
            .root_store
            .add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
        let tls_config = TlsConnector::from(Arc::new(tls_config));
        let dnsname = DNSNameRef::try_from_ascii_str(server_name).unwrap();
        let stream = tls_config.connect(dnsname, conn).await?;
        debug!("upgraded, stream is using TLS now");

        Ok(Client::new(stream, self.config))
    }
}

pub struct ResponseStream {
    inner: mpsc::UnboundedReceiver<Response>,
}

impl ResponseStream {
    /// Retrieves just the DONE item in the stream, discarding the rest
    pub async fn done(mut self) -> Result<Option<ResponseDone>> {
        while let Some(resp) = self.inner.recv().await {
            if let Response::Done(done) = resp {
                return Ok(Some(done));
            }
        }
        Ok(None)
    }

    /// Waits for the entire stream to finish, returning the DONE status and the stream
    pub async fn wait(mut self) -> Result<(Option<ResponseDone>, Vec<Response>)> {
        let mut done = None;
        let mut vec = Vec::new();
        while let Some(resp) = self.inner.recv().await {
            if let Response::Done(d) = resp {
                done = Some(d);
                break;
            } else {
                vec.push(resp);
            }
        }
        Ok((done, vec))
    }
}

impl Stream for ResponseStream {
    type Item = Response;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        self.inner.poll_recv(cx)
    }
}

#[allow(unreachable_code)]
async fn listen<C>(
    conn: ReadHalf<C>,
    mut cmd_rx: mpsc::UnboundedReceiver<Command2>,
    greeting_tx: oneshot::Sender<()>,
    mut exit_rx: oneshot::Receiver<()>,
) -> Result<ReadHalf<C>>
where
    C: AsyncRead + Unpin,
{
    let codec = ImapCodec::default();
    let mut framed = FramedRead::new(conn, codec);
    // let mut reader = BufReader::new(conn);
    let mut greeting_tx = Some(greeting_tx);
    let mut curr_cmd: Option<Command2> = None;
    let mut exit_rx = exit_rx.map_err(|_| ()).shared();
    // let mut exit_fut = Some(exit_rx.fuse());
    // let mut fut1 = None;
    let mut cache = String::new();

    loop {
        // let mut next_line = String::new();
        // let read_fut = reader.read_line(&mut next_line).fuse();
        let read_fut = framed.next().fuse();
        pin_mut!(read_fut);

        // only listen for a new command if there isn't one already
        let mut cmd_fut = if let Some(_) = curr_cmd {
            // if there is one, just make a future that never resolves so it'll always pick the
            // other options in the select.
            future::pending().boxed().fuse()
        } else {
            cmd_rx.recv().boxed().fuse()
        };

        select! {
            _ = exit_rx => {
                debug!("exiting the loop");
                break;
            }

            cmd = cmd_fut => {
                if curr_cmd.is_none() {
                    curr_cmd = cmd;
                }
            }

            resp = read_fut => {
                // trace!("read line {:?}", next_line);
                // res should not be None here
                // cache += &next_line;
                // let resp = match parse_response(&cache) {
                //     Ok(v) => {
                //         cache.clear();
                //         v
                //     }
                //     Err(e) => {
                //         error!("parse error: {}", e);
                //         continue;
                //     }
                // };
                let resp = match resp {
                    Some(Ok(v)) => v,
                    a => { error!("failed: {:?}", a); bail!("fuck"); },
                };

                // if this is the very first response, then it's a greeting
                if let Some(greeting_tx) = greeting_tx.take() {
                    greeting_tx.send(());
                }

                if let Response::Done(_) = resp {
                    // since this is the DONE message, clear curr_cmd so another one can be sent
                    if let Some((_, cmd_tx)) = curr_cmd.take() {
                        cmd_tx.send(resp)?;
                    }
                } else if let Some((ref cmd, ref mut cmd_tx)) = curr_cmd {
                    cmd_tx.send(resp)?;
                }
            }
        }
    }

    let conn = framed.into_inner();
    Ok(conn)
}
