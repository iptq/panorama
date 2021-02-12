use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;

use anyhow::Result;
use futures::{
    future::{self, Either, FutureExt},
    pin_mut, select,
    sink::{Sink, SinkExt},
    stream::{Stream, StreamExt, TryStream},
};
use imap::{
    builders::command::{Command, CommandBuilder},
    parser::parse_response,
    types::{Capability, RequestId, Response, ResponseCode, State, Status},
};
use tokio::{
    net::TcpStream,
    sync::{
        mpsc::{self, Receiver},
        oneshot,
    },
};
use tokio_rustls::{rustls::ClientConfig, webpki::DNSNameRef, TlsConnector};
use tokio_util::codec::{Decoder, LinesCodec, LinesCodecError};

pub async fn run_mail(server: impl AsRef<str>, port: u16) -> Result<()> {
    let server = server.as_ref();
    let client = TcpStream::connect((server, port)).await?;
    let codec = LinesCodec::new();
    let mut framed = codec.framed(client);
    let mut state = State::NotAuthenticated;
    let (sink, stream) = framed.split::<String>();

    let result = listen_loop(&mut state, sink, stream).await?;
    if let LoopExit::NegotiateTls(stream, sink) = result {
        debug!("negotiating tls");
        let mut config = ClientConfig::new();
        config
            .root_store
            .add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
        let config = TlsConnector::from(Arc::new(config));
        let dnsname = DNSNameRef::try_from_ascii_str(server).unwrap();

        // reconstruct the original stream
        let stream = stream.reunite(sink)?.into_inner();
        // let stream = TcpStream::connect((server, port)).await?;
        let stream = config.connect(dnsname, stream).await?;

        let codec = LinesCodec::new();
        let mut framed = codec.framed(stream);
        let (sink, stream) = framed.split::<String>();

        listen_loop(&mut state, sink, stream).await?;
    }

    Ok(())
}

enum LoopExit<S, S2> {
    NegotiateTls(S, S2),
    Closed,
}

async fn listen_loop<S, S2>(st: &mut State, mut sink: S2, mut stream: S) -> Result<LoopExit<S, S2>>
where
    S: Stream<Item = Result<String, LinesCodecError>> + Unpin,
    S2: Sink<String> + Unpin,
    S2::Error: Display,
{
    let (tx, mut rx) = mpsc::unbounded_channel::<()>();
    let mut cmd_mgr = CommandManager::new(sink);

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
