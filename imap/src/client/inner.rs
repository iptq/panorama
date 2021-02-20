use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use anyhow::Result;
use futures::future::{Future, FutureExt};
use panorama_strings::{StringEntry, StringStore};
use parking_lot::RwLock;
use tokio::{
    io::{
        self, AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader,
        WriteHalf,
    },
    task::JoinHandle,
};

use crate::command::Command;

pub type BoxedFunc = Box<dyn Fn()>;

/// The private Client struct, that is shared by all of the exported structs in the state machine.
pub struct Client<C> {
    conn: WriteHalf<C>,
    symbols: StringStore,

    id: usize,
    handlers: Arc<RwLock<HashMap<usize, bool>>>,

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
        let listen_fut = tokio::spawn(listen(read_half));

        Client {
            conn: write_half,
            symbols: StringStore::new(256),
            id: 0,
            handlers: Arc::new(RwLock::new(HashMap::new())),
            caps: Vec::new(),
            handle: listen_fut,
        }
    }

    /// Sends a command to the server and returns a handle to retrieve the result
    pub async fn execute(&mut self, cmd: Command) -> Result<()> {
        let id = self.id;
        self.id += 1;

        {
            let mut handlers = self.handlers.write();
            handlers.insert(id, false);
        }

        let cmd_str = cmd.to_string();
        self.conn.write_all(cmd_str.as_bytes()).await?;

        ExecHandle(self, id).await;
        Ok(())
    }

    /// Executes the CAPABILITY command
    pub async fn supports(&mut self) {
        let cmd = Command::Capability;
        let result = self.execute(cmd).await;
        debug!("poggers {:?}", result);
    }
}

pub struct ExecHandle<'a, C>(&'a Client<C>, usize);

impl<'a, C> Future for ExecHandle<'a, C> {
    type Output = ();
    fn poll(self: Pin<&mut Self>, _: &mut Context) -> Poll<Self::Output> {
        let state = {
            let handlers = self.0.handlers.read();
            handlers.get(&self.1).cloned()
        };

        // TODO: handle the None case here
        let state = state.unwrap();

        match state {
            true => Poll::Ready(()),
            false => Poll::Pending,
        }
    }
}

async fn listen(conn: impl AsyncRead + Unpin) -> Result<()> {
    let mut reader = BufReader::new(conn);
    loop {
        let mut next_line = String::new();
        reader.read_line(&mut next_line).await?;
    }
}
