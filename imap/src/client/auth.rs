use anyhow::Result;

use crate::command::Command;
use crate::response::{Response, ResponseDone, Status};

use super::{ClientAuthenticated, ClientUnauthenticated};

#[async_trait]
pub trait Auth {
    /// Performs authentication, consuming the client
    // TODO: return the unauthed client if failed?
    async fn perform_auth(self, client: ClientUnauthenticated) -> Result<ClientAuthenticated>;

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

        let (result, _) = client.execute(command).await?;
        let result = result.await?;

        if !matches!(
            result,
            Response::Done(ResponseDone {
                status: Status::Ok,
                ..
            })
        ) {
            bail!("unable to login: {:?}", result);
        }

        Ok(<Self as Auth>::convert_client(client))
    }
}
