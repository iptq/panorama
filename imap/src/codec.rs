use bytes::{Buf, BytesMut};
use tokio_util::codec::Decoder;

use crate::parser::parse_streamed_response;
use crate::response::Response;

#[derive(Default)]
pub struct ImapCodec;

impl Decoder for ImapCodec {
    type Item = Response;
    type Error = anyhow::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let s = std::str::from_utf8(src)?;
        match parse_streamed_response(s) {
            Ok((resp, len)) => {
                src.advance(len);
                return Ok(Some(resp));
            }
            Err(e) => {}
        };

        Ok(None)
    }
}
