use anyhow::Result;
use futures::stream::StreamExt;

use crate::command::Command;
use crate::response::{Response, ResponseDone, Status};

use super::{ClientAuthenticated, ClientUnauthenticated};

#[async_trait]
pub trait Auth {
    /// Performs authentication, consuming the client
    // TODO: return the unauthed client if failed?
    async fn perform_auth(self, client: ClientUnauthenticated) -> Result<ClientAuthenticated>;

    /// Converts the wrappers around the client once the authentication has happened. Should only
    /// be called by the `perform_auth` function.
    fn convert_client(client: ClientUnauthenticated) -> ClientAuthenticated {
        match client {
            ClientUnauthenticated::Encrypted(e) => ClientAuthenticated::Encrypted(e),
            ClientUnauthenticated::Unencrypted(e) => ClientAuthenticated::Unencrypted(e),
        }
    }
}

pub struct Plain {
    pub username: String,
    pub password: String,
}

#[async_trait]
impl Auth for Plain {
    async fn perform_auth(self, mut client: ClientUnauthenticated) -> Result<ClientAuthenticated> {
        let command = Command::Login {
            username: self.username,
            password: self.password,
        };

        let result = client.execute(command).await?;
        let done = result.done().await?;

        assert!(done.is_some());
        let done = done.unwrap();

        if done.status != Status::Ok {
            bail!("unable to login: {:?}", done);
        }

        // if !matches!(
        //     result,
        //     Response::Done(ResponseDone {
        //         status: Status::Ok,
        //         ..
        //     })
        // ) {
        //     bail!("unable to login: {:?}", result);
        // }

        Ok(<Self as Auth>::convert_client(client))
    }
}
