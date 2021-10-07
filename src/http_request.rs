use bytes::{Buf, Bytes};
use std::collections::BTreeMap;
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
    headers: BTreeMap<&'a str, &'a [u8]>,
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
                headers: BTreeMap::new(),
            },
        }
    }

    pub fn parse(&mut self, buf: Bytes) -> Result<(), RequestError> {
        let mut bytes = Bytes::from(buf);
        self.method = Some(Request::parse_token(&mut bytes)?);
        self.path = Some(Request::parse_uri(&mut bytes)?);
        self.version = Some(Request::parse_version(&mut bytes)?);
        Request::parse_new_line(&mut bytes)?;
        Request::parse_headers(&mut bytes, &mut self.headers)?;

        Ok(())
    }

    pub fn parse_headers(
        _bytes: &mut Bytes,
        _headers: &'a mut Headers,
    ) -> Result<(), RequestError> {
        Ok(())
    }

    pub fn parse_new_line(bytes: &mut Bytes) -> Result<(), RequestError> {
        if !bytes.has_remaining() {
            return Err(RequestError::NewLine);
        }

        match bytes.get_u8() {
            b'\r' => {
                print!("1");
                if bytes.has_remaining() && bytes.get_u8() == b'\n' {
                    print!("2");
                    Ok(())
                } else {
                    print!("3");
                    Err(RequestError::NewLine)
                }
            }
            b'\n' => Ok(()),
            _ => Err(RequestError::NewLine),
        }
    }

    pub fn parse_version(bytes: &mut Bytes) -> Result<u8, RequestError> {
        let res = match &bytes.slice(0..8)[..] {
            b"HTTP/1.0" => Ok(0),
            b"HTTP/1.1" => Ok(1),
            _ => return Err(RequestError::Version),
        };
        bytes.advance(8);
        res
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_requests() {
        let mut request = Request::new();

        request
            .parse(Bytes::from_static(b"GET /test HTTP/1.1\r\n\r\n"))
            .expect("parsing request");

        assert_eq!(request.version, Some(1));
        assert_eq!(request.method, Some(String::from("GET")));
        assert_eq!(request.path, Some(String::from("/test")));
    }

    #[test]
    fn accept_only_newline() {
        let mut request = Request::new();

        request
            .parse(Bytes::from_static(b"GET /test HTTP/1.1\n"))
            .expect("parsing request");

        assert_eq!(request.version, Some(1));
        assert_eq!(request.method, Some(String::from("GET")));
        assert_eq!(request.path, Some(String::from("/test")));
    }

    #[test]
    fn do_not_accept_only_cr() {
        let mut request = Request::new();

        request
            .parse(Bytes::from_static(b"GET /test HTTP/1.1\r"))
            .expect_err("parsing request");
    }
}
