use std::collections::{HashMap, HashSet, VecDeque};
use std::mem;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use anyhow::{Context as AnyhowContext, Result};
use futures::future::{self, Either, Future, FutureExt};
use parking_lot::{RwLock, RwLockWriteGuard};
use tokio::{
    io::{
        self, AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, ReadHalf, WriteHalf,
    },
    sync::mpsc,
    task::JoinHandle,
};
use tokio_rustls::{
    client::TlsStream, rustls::ClientConfig as RustlsConfig, webpki::DNSNameRef, TlsConnector,
};

use crate::command::Command;
use crate::parser::{parse_capability, parse_response};
use crate::response::{Capability, Response, ResponseCode, Status};
// use crate::types::{Capability as Capability_, Status};

use super::ClientConfig;

pub type CapsLock = Arc<RwLock<Option<HashSet<Capability>>>>;
pub type ResultMap = Arc<RwLock<VecDeque<(usize, Option<Response>, Vec<Response>, Option<Waker>)>>>;
pub type GreetingState = Arc<RwLock<(Option<Response>, Option<Waker>)>>;
pub const TAG_PREFIX: &str = "panorama";

/// The lower-level Client struct, that is shared by all of the exported structs in the state machine.
pub struct Client<C> {
    config: ClientConfig,
    conn: WriteHalf<C>,

    id: usize,
    results: ResultMap,

    /// cached set of capabilities
    caps: CapsLock,

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
        let results = Arc::new(RwLock::new(VecDeque::new()));
        let (exit_tx, exit_rx) = mpsc::channel(1);
        let greeting = Arc::new(RwLock::new((None, None)));
        let caps: CapsLock = Arc::new(RwLock::new(None));

        let listener_handle = tokio::spawn(listen(
            read_half,
            caps.clone(),
            results.clone(),
            exit_rx,
            greeting.clone(),
        ));

        Client {
            config,
            conn: write_half,
            id: 0,
            results,
            listener_handle,
            caps,
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
    pub async fn execute(&mut self, cmd: Command) -> Result<(Response, Vec<Response>)> {
        debug!("executing command {:?}", cmd);
        let id = self.id;
        self.id += 1;
        {
            let mut handlers = self.results.write();
            handlers.push_back((id, None, vec![], None));
        }

        let cmd_str = format!("{}{} {}\r\n", TAG_PREFIX, id, cmd);
        // debug!("[{}] writing to socket: {:?}", id, cmd_str);
        self.conn.write_all(cmd_str.as_bytes()).await?;
        self.conn.flush().await?;
        // debug!("[{}] written.", id);

        let resp = ExecWaiter(self, id, false).await;
        // let resp = {
        //     let mut handlers = self.results.write();
        //     handlers.remove(&id).unwrap().0.unwrap()
        // };
        Ok(resp)
    }

    /// Executes the CAPABILITY command
    pub async fn capabilities(&mut self, force: bool) -> Result<()> {
        {
            let caps = self.caps.read();
            if caps.is_some() && !force {
                return Ok(());
            }
        }

        let cmd = Command::Capability;
        debug!("sending: {:?} {:?}", cmd, cmd.to_string());
        let (result, intermediate) = self
            .execute(cmd)
            .await
            .context("error executing CAPABILITY command")?;
        debug!("cap resp: {:?}", result);

        if let Some(Response::Capabilities(new_caps)) = intermediate
            .iter()
            .find(|resp| matches!(resp, Response::Capabilities(_)))
        {
            let mut caps = self.caps.write();
            *caps = Some(new_caps.iter().cloned().collect());
        }

        Ok(())
    }

    /// Attempts to upgrade this connection using STARTTLS
    pub async fn upgrade(mut self) -> Result<Client<TlsStream<C>>> {
        // TODO: make sure STARTTLS is in the capability list
        if !self.has_capability("STARTTLS").await? {
            bail!("server doesn't support this capability");
        }

        // first, send the STARTTLS command
        let resp = self.execute(Command::Starttls).await?;
        debug!("server response to starttls: {:?}", resp);

        debug!("sending exit for upgrade");
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
        debug!("upgraded, stream is using TLS now");

        Ok(Client::new(stream, self.config))
    }

    /// Check if this client has a particular capability
    pub async fn has_capability(&mut self, cap: impl AsRef<str>) -> Result<bool> {
        let cap = cap.as_ref().to_owned();
        debug!("checking for the capability: {:?}", cap);
        let cap = parse_capability(cap)?;

        self.capabilities(false).await?;
        let caps = self.caps.read();
        // TODO: refresh caps

        let caps = caps.as_ref().unwrap();
        let result = caps.contains(&cap);
        debug!("cap result: {:?}", result);
        Ok(result)
    }
}

pub struct GreetingWaiter(GreetingState);

impl Future for GreetingWaiter {
    type Output = Response;
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let (state, waker) = &mut *self.0.write();
        debug!("g {:?}", state);
        if waker.is_none() {
            *waker = Some(cx.waker().clone());
        }

        match state.take() {
            Some(v) => Poll::Ready(v),
            None => Poll::Pending,
        }
    }
}

pub struct ExecWaiter<'a, C>(&'a Client<C>, usize, bool);

impl<'a, C> Future for ExecWaiter<'a, C> {
    type Output = (Response, Vec<Response>);
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        // add the waker
        let mut results = self.0.results.write();
        if !self.2 {
            if let Some((_, _, _, waker_ref)) =
                results.iter_mut().find(|(id, _, _, _)| *id == self.1)
            {
                let waker = cx.waker().clone();
                *waker_ref = Some(waker);
                self.2 = true;
            }
        }

        // if this struct exists then there's definitely at least one entry
        let (id, last_response, _, _) = &results[0];
        if *id != self.1 || last_response.is_none() {
            return Poll::Pending;
        }

        let (_, last_response, intermediate_responses, _) = results.pop_front().unwrap();
        mem::drop(results);

        Poll::Ready((
            last_response.expect("already checked"),
            intermediate_responses,
        ))
    }
}

/// Main listen loop for the application
async fn listen<C>(
    conn: C,
    caps: CapsLock,
    results: ResultMap,
    mut exit: mpsc::Receiver<()>,
    greeting: GreetingState,
) -> Result<C>
where
    C: AsyncRead + Unpin,
{
    // debug!("amogus");
    let mut reader = BufReader::new(conn);
    let mut greeting = Some(greeting);

    loop {
        let mut next_line = String::new();
        let fut = reader.read_line(&mut next_line).fuse();
        pin_mut!(fut);
        let fut2 = exit.recv().fuse();
        pin_mut!(fut2);

        match future::select(fut, fut2).await {
            Either::Left((res, _)) => {
                let bytes = res.context("read failed")?;
                if bytes == 0 {
                    bail!("connection probably died");
                }

                debug!("got a new line {:?}", next_line);
                let resp = parse_response(next_line)?;

                // if this is the very first message, treat it as a greeting
                if let Some(greeting) = greeting.take() {
                    let (greeting, waker) = &mut *greeting.write();
                    debug!("received greeting!");
                    *greeting = Some(resp.clone());
                    if let Some(waker) = waker.take() {
                        waker.wake();
                    }
                }

                // update capabilities list
                // TODO: probably not really necessary here (done somewhere else)?
                if let Response::Capabilities(new_caps)
                | Response::Data {
                    status: Status::Ok,
                    code: Some(ResponseCode::Capabilities(new_caps)),
                    ..
                } = &resp
                {
                    let caps = &mut *caps.write();
                    *caps = Some(new_caps.iter().cloned().collect());
                    debug!("new caps: {:?}", caps);
                }

                match &resp {
                    Response::Data {
                        status: Status::Ok, ..
                    } => {
                        let mut results = results.write();
                        if let Some((_, _, intermediate, _)) = results.iter_mut().next() {
                            debug!("pushed to intermediate: {:?}", resp);
                            intermediate.push(resp);
                        }
                    }

                    // bye
                    Response::Data {
                        status: Status::Bye,
                        ..
                    } => {
                        bail!("disconnected");
                    }

                    Response::Done { tag, .. } => {
                        if tag.starts_with(TAG_PREFIX) {
                            // let id = tag.trim_start_matches(TAG_PREFIX).parse::<usize>()?;
                            let mut results = results.write();
                            if let Some((_, opt, _, waker)) = results.iter_mut().next() {
                                *opt = Some(resp);
                                if let Some(waker) = waker.take() {
                                    waker.wake();
                                }
                            }
                        }
                    }

                    _ => {}
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
