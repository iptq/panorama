use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use anyhow::{Context as AnyhowContext, Result};
use futures::future::{self, Either, Future, FutureExt};
use panorama_strings::{StringEntry, StringStore};
use parking_lot::{Mutex, RwLock};
use tokio::{
    io::{
        self, AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader,
        ReadHalf, WriteHalf,
    },
    sync::{mpsc, oneshot},
    task::JoinHandle,
};
use tokio_rustls::{
    client::TlsStream, rustls::ClientConfig as RustlsConfig, webpki::DNSNameRef, TlsConnector,
};

use crate::command::Command;
use crate::types::Response;

use super::ClientConfig;

pub type BoxedFunc = Box<dyn Fn()>;
pub type ResultMap = Arc<RwLock<HashMap<usize, (Option<String>, Option<Waker>)>>>;
pub type GreetingState = Arc<RwLock<(bool, Option<Waker>)>>;
pub const TAG_PREFIX: &str = "panorama";

/// The lower-level Client struct, that is shared by all of the exported structs in the state machine.
pub struct Client<C> {
    config: ClientConfig,
    conn: WriteHalf<C>,
    symbols: StringStore,

    id: usize,
    results: ResultMap,

    /// cached set of capabilities
    caps: Vec<StringEntry>,

    /// join handle for the listener thread
    listener_handle: JoinHandle<Result<ReadHalf<C>>>,

    /// used for telling the listener thread to stop and return the read half
    exit_tx: mpsc::Sender<()>,

    /// used for receiving the greeting
    greeting: GreetingState,
}

impl<C> Client<C>
where
    C: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    /// Creates a new client that wraps a connection
    pub fn new(conn: C, config: ClientConfig) -> Self {
        let (read_half, write_half) = io::split(conn);
        let results = Arc::new(RwLock::new(HashMap::new()));
        let (exit_tx, exit_rx) = mpsc::channel(1);
        let greeting = Arc::new(RwLock::new((false, None)));
        let listen_fut = tokio::spawn(listen(
            read_half,
            results.clone(),
            exit_rx,
            greeting.clone(),
        ));

        Client {
            config,
            conn: write_half,
            symbols: StringStore::new(256),
            id: 0,
            results,
            caps: Vec::new(),
            listener_handle: listen_fut,
            exit_tx,
            greeting,
        }
    }

    /// Returns a future that doesn't resolve until we receive a greeting from the server.
    pub fn wait_for_greeting(&self) -> GreetingWaiter {
        debug!("waiting for greeting");
        GreetingWaiter(self.greeting.clone())
    }

    /// Sends a command to the server and returns a handle to retrieve the result
    pub async fn execute(&mut self, cmd: Command) -> Result<String> {
        debug!("executing command {:?}", cmd);
        let id = self.id;
        self.id += 1;
        {
            let mut handlers = self.results.write();
            handlers.insert(id, (None, None));
        }

        let cmd_str = format!("{}{} {}\r\n", TAG_PREFIX, id, cmd);
        debug!("[{}] writing to socket: {:?}", id, cmd_str);
        self.conn.write_all(cmd_str.as_bytes()).await?;
        self.conn.flush().await?;
        debug!("[{}] written.", id);

        ExecWaiter(self, id).await;
        let resp = {
            let mut handlers = self.results.write();
            handlers.remove(&id).unwrap().0.unwrap()
        };
        Ok(resp)
    }

    /// Executes the CAPABILITY command
    pub async fn capabilities(&mut self) -> Result<()> {
        let cmd = Command::Capability;
        debug!("sending: {:?} {:?}", cmd, cmd.to_string());
        let result = self
            .execute(cmd)
            .await
            .context("error executing CAPABILITY command")?;
        let (_, resp) = Response::from_bytes(result.as_bytes())
            .map_err(|err| anyhow!(""))
            .context("error parsing response from CAPABILITY")?;
        debug!("cap resp: {:?}", resp);
        if let Response::Capabilities(caps) = resp {
            debug!("capabilities: {:?}", caps);
        }
        Ok(())
    }

    /// Attempts to upgrade this connection using STARTTLS
    pub async fn upgrade(mut self) -> Result<Client<TlsStream<C>>> {
        // TODO: make sure STARTTLS is in the capability list
        // first, send the STARTTLS command
        let resp = self.execute(Command::Starttls).await?;
        debug!("server response to starttls: {:?}", resp);

        debug!("sending exit ()");
        self.exit_tx.send(()).await?;
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

        Ok(Client::new(stream, self.config))
    }
}

pub struct GreetingWaiter(GreetingState);

impl Future for GreetingWaiter {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let (state, waker) = &mut *self.0.write();
        if waker.is_none() {
            *waker = Some(cx.waker().clone());
        }

        match state {
            true => Poll::Ready(()),
            false => Poll::Pending,
        }
    }
}

pub struct ExecWaiter<'a, C>(&'a Client<C>, usize);

impl<'a, C> Future for ExecWaiter<'a, C> {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let mut handlers = self.0.results.write();
        let state = handlers.get_mut(&self.1);

        // TODO: handle the None case here
        debug!("f[{}] {:?}", self.1, state);
        let (result, waker) = state.unwrap();

        match result {
            Some(_) => Poll::Ready(()),
            None => {
                *waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

/// Main listen loop for the application
async fn listen<C>(
    conn: C,
    results: ResultMap,
    mut exit: mpsc::Receiver<()>,
    greeting: GreetingState,
) -> Result<C>
where
    C: AsyncRead + Unpin,
{
    debug!("amogus");
    let mut reader = BufReader::new(conn);
    let mut greeting = Some(greeting);

    loop {
        let mut next_line = String::new();
        let fut = reader.read_line(&mut next_line).fuse();
        pin_mut!(fut);
        let fut2 = exit.recv().fuse();
        pin_mut!(fut2);

        match future::select(fut, fut2).await {
            Either::Left((_, _)) => {
                debug!("got a new line");
                let next_line = next_line.trim_end_matches('\n').trim_end_matches('\r');

                let mut parts = next_line.split(" ");
                let tag = parts.next().unwrap();
                let rest = parts.collect::<Vec<_>>().join(" ");

                if tag == "*" {
                    debug!("UNTAGGED {:?}", rest);

                    // TODO: verify that the greeting is actually an OK
                    if let Some(greeting) = greeting.take() {
                        let (greeting, waker) = &mut *greeting.write();
                        debug!("got greeting");
                        *greeting = true;
                        if let Some(waker) = waker.take() {
                            waker.wake();
                        }
                    }
                } else if tag.starts_with(TAG_PREFIX) {
                    let id = tag.trim_start_matches(TAG_PREFIX).parse::<usize>()?;
                    debug!("set {} to {:?}", id, rest);
                    let mut results = results.write();
                    if let Some((c, w)) = results.get_mut(&id) {
                        // *c = Some(rest.to_string());
                        *c = Some(next_line.to_owned());
                        if let Some(waker) = w.take() {
                            waker.wake();
                        }
                    }
                }
            }
            Either::Right((_, _)) => {
                debug!("exiting read loop");
                break;
            }
        }
    }

    let conn = reader.into_inner();
    Ok(conn)
}
