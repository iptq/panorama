use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use anyhow::Result;
use futures::future::{Future, FutureExt};
use panorama_strings::{StringEntry, StringStore};
use parking_lot::{Mutex, RwLock};
use tokio::{
    io::{
        self, AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader,
        WriteHalf,
    },
    task::JoinHandle,
};

use crate::command::Command;
use crate::response::Response;

pub type BoxedFunc = Box<dyn Fn()>;

/// The private Client struct, that is shared by all of the exported structs in the state machine.
pub struct Client<C> {
    conn: WriteHalf<C>,
    symbols: StringStore,

    id: usize,
    results: ResultMap,

    /// Cached capabilities that shouldn't change between
    caps: Vec<StringEntry>,
    handle: JoinHandle<Result<()>>,
}

impl<C> Client<C>
where
    C: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    /// Creates a new client that wraps a connection
    pub fn new(conn: C) -> Self {
        let (read_half, write_half) = io::split(conn);
        let results = Arc::new(RwLock::new(HashMap::new()));
        let listen_fut = tokio::spawn(listen(read_half, results.clone()));

        Client {
            conn: write_half,
            symbols: StringStore::new(256),
            id: 0,
            results,
            caps: Vec::new(),
            handle: listen_fut,
        }
    }

    /// Sends a command to the server and returns a handle to retrieve the result
    pub async fn execute(&mut self, cmd: Command) -> Result<Response> {
        debug!("executing command {:?}", cmd);
        let id = self.id;
        self.id += 1;
        {
            let mut handlers = self.results.write();
            handlers.insert(id, (None, None));
        }

        let cmd_str = format!("pano{} {}\n", id, cmd);
        debug!("[{}] writing to socket: {:?}", id, cmd_str);
        self.conn.write_all(cmd_str.as_bytes()).await?;
        debug!("[{}] written.", id);

        ExecHandle(self, id).await;
        let resp = {
            let mut handlers = self.results.write();
            handlers.remove(&id).unwrap().0.unwrap()
        };
        Ok(Response(resp))
    }

    /// Executes the CAPABILITY command
    pub async fn supports(&mut self) -> Result<()> {
        let cmd = Command::Capability;
        debug!("sending: {:?} {:?}", cmd, cmd.to_string());
        let result = self.execute(cmd).await?;
        debug!("result from supports: {:?}", result);
        Ok(())
    }
}

pub struct ExecHandle<'a, C>(&'a Client<C>, usize);

impl<'a, C> Future for ExecHandle<'a, C> {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let mut handlers = self.0.results.write();
        let mut state = handlers.get_mut(&self.1);

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

use std::task::Waker;
pub type ResultMap = Arc<RwLock<HashMap<usize, (Option<String>, Option<Waker>)>>>;

async fn listen(conn: impl AsyncRead + Unpin, results: ResultMap) -> Result<()> {
    debug!("amogus");
    let mut reader = BufReader::new(conn);
    loop {
        let mut next_line = String::new();
        reader.read_line(&mut next_line).await?;

        // debug!("line: {:?}", next_line);
        let parts = next_line.split(" ").collect::<Vec<_>>();
        let tag = parts[0];
        if tag == "*" {
            debug!("UNTAGGED {:?}", next_line);
        } else if tag.starts_with("pano") {
            let id = tag.trim_start_matches("pano").parse::<usize>()?;
            debug!("set {} to {:?}", id, next_line);
            let mut results = results.write();
            if let Some((c, w)) = results.get_mut(&id) {
                *c = Some(next_line);
                let w = w.take().unwrap();
                w.wake();
            }
        }
    }
}
