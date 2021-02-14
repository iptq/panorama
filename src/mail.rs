//! Mail

use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;

use anyhow::Result;
use futures::{
    future::{self, Either},
    pin_mut,
    sink::{Sink, SinkExt},
    stream::{Stream, StreamExt},
};
use imap::{
    builders::command::Command,
    parser::parse_response,
    types::{Capability, RequestId, Response, ResponseCode, State, Status},
};
use tokio::{
    net::TcpStream,
    sync::mpsc::{self, UnboundedReceiver},
};
use tokio_rustls::{rustls::ClientConfig, webpki::DNSNameRef, TlsConnector};
use tokio_stream::wrappers::WatchStream;
use tokio_util::codec::{Decoder, LinesCodec, LinesCodecError};

use crate::config::{MailConfig, ConfigWatcher};

/// Command sent to the mail thread by something else (i.e. UI)
pub enum MailCommand {
    /// Refresh the list
    Refresh,

    /// Send a raw command
    Raw(Command),
}

/// Main entrypoint for the mail listener.
pub async fn run_mail(
    config_watcher: ConfigWatcher,
    cmd_in: UnboundedReceiver<MailCommand>,
) -> Result<()> {
    let mut curr_conn = None;

    let mut config_watcher = WatchStream::new(config_watcher);
    loop {
        let config: MailConfig = match config_watcher.next().await {
            Some(Some(v)) => v,
            _ => break,
        };

        let handle = tokio::spawn(open_imap_connection(config));
        curr_conn = Some(handle);
    }

    Ok(())
}

async fn open_imap_connection(config: MailConfig) -> Result<()> {
    debug!(
        "Opening imap connection to {}:{}",
        config.server, config.port
    );
    let server = config.server.as_str();
    let port = config.port;

    let client = TcpStream::connect((server, port)).await?;
    let codec = LinesCodec::new();
    let framed = codec.framed(client);
    let mut state = State::NotAuthenticated;
    let (sink, stream) = framed.split::<String>();

    let result = listen_loop(config.clone(), &mut state, sink, stream, false).await?;
    if let LoopExit::NegotiateTls(stream, sink) = result {
        debug!("negotiating tls");
        let mut tls_config = ClientConfig::new();
        tls_config
            .root_store
            .add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
        let tls_config = TlsConnector::from(Arc::new(tls_config));
        let dnsname = DNSNameRef::try_from_ascii_str(server).unwrap();

        // reconstruct the original stream
        let stream = stream.reunite(sink)?.into_inner();
        // let stream = TcpStream::connect((server, port)).await?;
        let stream = tls_config.connect(dnsname, stream).await?;

        let codec = LinesCodec::new();
        let framed = codec.framed(stream);
        let (sink, stream) = framed.split::<String>();
        listen_loop(config.clone(), &mut state, sink, stream, true).await?;
    }

    Ok(())
}

/// Action that should be taken after the connection loop quits.
enum LoopExit<S, S2> {
    /// Used in case the STARTTLS command is issued; perform TLS negotiation on top of the current
    /// stream
    NegotiateTls(S, S2),
    Closed,
}

async fn listen_loop<S, S2>(
    config: MailConfig,
    st: &mut State,
    sink: S2,
    mut stream: S,
    with_ssl: bool,
) -> Result<LoopExit<S, S2>>
where
    S: Stream<Item = Result<String, LinesCodecError>> + Unpin,
    S2: Sink<String> + Unpin,
    S2::Error: Display,
{
    let (tx, mut rx) = mpsc::unbounded_channel::<()>();
    let mut cmd_mgr = CommandManager::new(sink);

    if with_ssl {
        let cmd = Command {
            args: b"CAPABILITY".to_vec(),
            next_state: Some(State::Authenticated),
        };
        cmd_mgr.send(cmd, |_| {}).await?;
    }

    loop {
        let fut1 = stream.next();
        let fut2 = rx.recv();
        pin_mut!(fut1);
        pin_mut!(fut2);

        debug!("waiting for next select");
        match future::select(fut1, fut2).await {
            Either::Left((line, _)) => {
                let mut line = match line {
                    Some(v) => v?,
                    None => break,
                };
                line += "\r\n";
                let (_, resp) = match parse_response(line.as_bytes()) {
                    Ok(v) => v,
                    Err(e) => bail!(e.to_string()),
                };
                debug!("<<< {:?}", resp);

                match st {
                    State::NotAuthenticated => match resp {
                        Response::Data {
                            status: Status::Ok,
                            code: Some(ResponseCode::Capabilities(caps)),
                            ..
                        } => {
                            if !with_ssl {
                                // prepare to do TLS negotiation
                                let mut has_starttls = false;
                                for cap in caps {
                                    if let Capability::Atom("STARTTLS") = cap {
                                        has_starttls = true;
                                    }
                                }
                                if has_starttls {
                                    let cmd = Command {
                                        args: b"STARTTLS".to_vec(),
                                        next_state: None,
                                    };
                                    let tx = tx.clone();
                                    cmd_mgr
                                        .send(cmd, move |_| {
                                            tx.send(()).unwrap();
                                        })
                                        .await?;
                                }
                            }
                        }

                        Response::Capabilities(caps) => {
                            if with_ssl {
                                // send authentication information
                                let cmd = Command {
                                    args: format!("LOGIN {} {}", config.username, config.password)
                                        .as_bytes()
                                        .to_vec(),
                                    next_state: Some(State::Authenticated),
                                };
                                cmd_mgr.send(cmd, |_| {}).await?;
                            }
                        }

                        Response::Done { tag, code, .. } => {
                            cmd_mgr.process_done(tag, code)?;
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
            Either::Right((_, _)) => {
                debug!("ENCOUNTERED EXIT");
                let sink = cmd_mgr.decompose();
                return Ok(LoopExit::NegotiateTls(stream, sink));
            }
        }
    }

    Ok(LoopExit::Closed)
}

/// A struct in charge of managing multiple in-flight commands.
struct CommandManager<S> {
    tag_idx: usize,
    in_flight: HashMap<String, Box<dyn Fn(Option<ResponseCode>) + Send>>,
    sink: S,
}

impl<S> CommandManager<S>
where
    S: Sink<String> + Unpin,
{
    pub fn new(sink: S) -> Self {
        CommandManager {
            tag_idx: 0,
            in_flight: HashMap::new(),
            sink,
        }
    }

    pub fn decompose(self) -> S {
        self.sink
    }

    pub async fn send(
        &mut self,
        cmd: Command,
        cb: impl Fn(Option<ResponseCode>) + Send + 'static,
    ) -> Result<()> {
        let tag_idx = self.tag_idx;
        self.tag_idx += 1;
        let cb = Box::new(cb);
        let tag_str = format!("t{}", tag_idx);
        let cmd_str = std::str::from_utf8(&cmd.args)?;
        let full_str = format!("{} {}", tag_str, cmd_str);
        self.in_flight.insert(tag_str.clone(), cb);

        debug!(">>> {:?}", full_str);
        self.sink
            .send(full_str)
            .await
            .map_err(|_| anyhow!("failed to send command"))
    }

    pub fn process_done(&mut self, id: RequestId, code: Option<ResponseCode>) -> Result<()> {
        let name = std::str::from_utf8(id.as_bytes())?;
        if let Some(cb) = self.in_flight.remove(name) {
            cb(code);
        }
        Ok(())
    }
}
