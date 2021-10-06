use bytes::{Buf, Bytes};
use std::collections::HashMap;
use thiserror::Error;

use crate::tokens;

#[derive(Error, Debug)]
pub enum RequestError {
    #[error("invalid header name")]
    HeaderName,
    #[error("invalid header value")]
    HeaderValue,
    #[error("invalid status")]
    Status,
    #[error("invalid version")]
    Version,
    #[error("invalid newline")]
    NewLine,
    #[error("invalid token")]
    Token,
    #[error("invalid uri")]
    URI,
    #[error("too many headers")]
    TooManyHeaders,
}

#[derive(Debug)]
pub struct Headers<'a> {
    headers: HashMap<&'a str, &'a [u8]>,
}

#[derive(Debug)]
pub struct Request<'a> {
    /// The request method, such as `GET`.
    pub method: Option<String>,
    /// The request path, such as `/about-us`.
    pub path: Option<String>,
    /// The request version, such as `HTTP/1.1`.
    pub version: Option<u8>,
    /// The request headers.
    pub headers: Headers<'a>,
}

impl<'a> Request<'a> {
    // Creates a new Request
    pub fn new() -> Self {
        Request {
            method: None,
            path: None,
            version: None,
            headers: Headers {
                headers: HashMap::new(),
            },
        }
    }

    pub fn parse(&mut self, buf: Bytes) -> Result<(), RequestError> {
        let mut bytes = Bytes::from(buf);
        self.method = Some(Request::parse_token(&mut bytes)?);
        self.path = Some(Request::parse_uri(&mut bytes)?);
        self.version = Some(Request::parse_version(&mut bytes)?);

        Ok(())
    }

    pub fn parse_version(bytes: &mut Bytes) -> Result<u8, RequestError> {
        match &bytes.slice(0..8)[..] {
            b"HTTP/1.0" => Ok(0),
            b"HTTP/1.1" => Ok(1),
            _ => Err(RequestError::Version),
        }
    }

    pub fn parse_uri(bytes: &mut Bytes) -> Result<String, RequestError> {
        for (i, b) in bytes.iter().enumerate() {
            if b == &b' ' {
                let token = &bytes.slice(0..i)[..];
                bytes.advance(i + 1);
                return Ok(std::str::from_utf8(&token)
                    .map_err(|_| RequestError::URI)?
                    .to_string());
            } else if !tokens::is_uri_token(*b) {
                break;
            }
        }
        return Err(RequestError::URI);
    }

    pub fn parse_token(bytes: &mut Bytes) -> Result<String, RequestError> {
        for (i, b) in bytes.iter().enumerate() {
            if b == &b' ' {
                let token = &bytes.slice(0..i)[..];
                bytes.advance(i + 1);
                return Ok(std::str::from_utf8(&token)
                    .map_err(|_| RequestError::Token)?
                    .to_string());
            } else if !tokens::is_token(*b) {
                println!("{}", b);
                break;
            }
        }
        return Err(RequestError::Token);
    }
}
