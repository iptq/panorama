pub mod core;
pub mod types;

pub mod bodystructure;
pub mod rfc3501;
pub mod rfc4315;
pub mod rfc4551;
pub mod rfc5161;
pub mod rfc5464;
pub mod rfc7162;

use anyhow::{Error, Result};
use nom::{branch::alt, IResult};

use self::types::{Capability, Response};

#[cfg(test)]
mod tests;

pub fn parse_capability(msg: &[u8]) -> Result<Capability> {
    rfc3501::capability(msg)
        .map(|(_, resp)| resp)
        .map_err(|err| anyhow!("error: {}", err))
}

pub fn parse_response(msg: &[u8]) -> Result<Response> {
    alt((
        rfc3501::continue_req,
        rfc3501::response_data,
        rfc3501::response_tagged,
    ))(msg)
    .map(|(_, resp)| resp)
    .map_err(|err| anyhow!("error: {}", err))
}

pub type ParseResult<'a> = IResult<&'a [u8], Response>;
