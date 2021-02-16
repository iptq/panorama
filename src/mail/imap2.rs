// let's try this again

use std::sync::Arc;

use anyhow::Result;
use futures::stream::Stream;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};
use tokio_rustls::{rustls::ClientConfig, webpki::DNSNameRef, TlsConnector};

use crate::config::{ImapConfig, TlsMethod};

pub async fn open_imap_connection(config: ImapConfig) -> Result<()> {
    let server = config.server.as_str();
    let port = config.port;

    let stream = TcpStream::connect((server, port)).await?;

    match config.tls {
        TlsMethod::Off => begin_authentication(config, stream).await,
        TlsMethod::On => {
            let stream = perform_tls_negotiation(server.to_owned(), stream).await?;
            begin_authentication(config, stream).await
        }
        TlsMethod::Starttls => {
            todo!()
        }
    }
}

/// Performs TLS negotiation, using the webpki_roots and verifying the server name
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

async fn begin_authentication(
    config: ImapConfig,
    stream: impl AsyncRead + AsyncWrite,
) -> Result<()> {
    Ok(())
}
