use bytes::{Buf, Bytes};
use std::{
    collections::{btree_map, BTreeMap},
    convert::{TryFrom, TryInto},
    fmt::{self, Debug},
};
use thiserror::Error;

use crate::tokens;

#[derive(Error, Debug)]
pub enum RequestError {
    #[error("invalid method")]
    Method,
    #[error("invalid header name")]
    HeaderName,
    #[error("invalid header value")]
    HeaderValue,
    #[error("invalid status")]
    Status,
    #[error("invalid version")]
    Version,
    #[error("expected newline")]
    NewLine,
    #[error("expected space")]
    Space,
    #[error("invalid token")]
    Token,
    #[error("invalid uri")]
    URI,
    #[error("too many headers")]
    TooManyHeaders,
}

#[derive(Debug, Clone)]
pub struct Headers {
    headers: BTreeMap<String, Vec<u8>>,
}

impl Headers {
    pub fn iter(&self) -> btree_map::Iter<String, Vec<u8>> {
        self.headers.iter()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Method {
    GET,
    HEAD,
    POST,
    PUT,
    DELETE,
    CONNECT,
    OPTIONS,
    TRACE,
    PATCH,
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl TryFrom<&str> for Method {
    type Error = &'static str;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_ascii_uppercase().as_str() {
            "GET" => Ok(Method::GET),
            "HEAD" => Ok(Method::HEAD),
            "POST" => Ok(Method::POST),
            "PUT" => Ok(Method::PUT),
            "DELETE" => Ok(Method::DELETE),
            "CONNECT" => Ok(Method::CONNECT),
            "OPTIONS" => Ok(Method::OPTIONS),
            "TRACE" => Ok(Method::TRACE),
            "PATCH" => Ok(Method::PATCH),
            _ => Err("invalid method"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Request {
    /// The request method, such as `GET`.
    pub method: Option<Method>,
    /// The request path, such as `/about-us`.
    pub path: Option<String>,
    /// The request version, such as `HTTP/1.1`.
    pub version: Option<u8>,
    /// The request headers.
    pub headers: Headers,
    /// The request body.
    pub body: Vec<u8>,
}

impl Request {
    // Creates a new Request
    pub fn new() -> Self {
        Request {
            method: None,
            path: None,
            version: None,
            headers: Headers {
                headers: BTreeMap::new(),
            },
            body: vec![],
        }
    }

    pub fn parse(&mut self, buf: Bytes) -> Result<(), RequestError> {
        let mut bytes = Bytes::from(buf);

        self.method = Some(
            Request::parse_token(&mut bytes)?
                .as_str()
                .try_into()
                .map_err(|_| RequestError::Method)?,
        );

        self.path = Some(Request::parse_uri(&mut bytes)?);
        self.version = Some(Request::parse_version(&mut bytes)?);

        Request::parse_new_line(&mut bytes)?;
        Request::parse_headers(&mut bytes, &mut self.headers)?;
        Request::parse_new_line(&mut bytes)?;

        if bytes.remaining() != 0 {
            self.body = bytes.to_vec();
        }

        Ok(())
    }

    pub fn parse_headers(bytes: &mut Bytes, headers: &mut Headers) -> Result<(), RequestError> {
        let mut parse_header = || -> Result<(), RequestError> {
            let header_name = Request::parse_header_name(bytes)?;
            Request::parse_space(bytes)?;
            let header_value = Request::parse_header_value(bytes)?;

            headers.headers.insert(header_name, header_value);
            Ok(())
        };

        loop {
            if parse_header().is_err() {
                break;
            }
        }

        Ok(())
    }

    pub fn parse_space(bytes: &mut Bytes) -> Result<(), RequestError> {
        if !bytes.has_remaining() {
            return Err(RequestError::NewLine);
        }

        if bytes.get_u8() == b' ' && bytes.has_remaining() {
            Ok(())
        } else {
            Err(RequestError::Space)
        }
    }

    pub fn parse_new_line(bytes: &mut Bytes) -> Result<(), RequestError> {
        if !bytes.has_remaining() {
            return Err(RequestError::NewLine);
        }

        match bytes.get_u8() {
            b'\r' => {
                if bytes.has_remaining() && bytes.get_u8() == b'\n' {
                    Ok(())
                } else {
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
                break;
            }
        }
        return Err(RequestError::Token);
    }

    pub fn parse_header_name(bytes: &mut Bytes) -> Result<String, RequestError> {
        for (i, b) in bytes.iter().enumerate() {
            if b == &b':' {
                let token = &bytes.slice(0..i)[..];
                bytes.advance(i + 1);
                return Ok(std::str::from_utf8(&token)
                    .map_err(|_| RequestError::Token)?
                    .to_string());
            } else if !tokens::is_header_name_token(*b) {
                break;
            }
        }
        return Err(RequestError::Token);
    }

    pub fn parse_header_value(bytes: &mut Bytes) -> Result<Vec<u8>, RequestError> {
        for (i, b) in bytes.iter().enumerate() {
            if b == &b'\r' || b == &b'\n' {
                let token = &bytes.slice(0..i)[..];
                bytes.advance(i);
                Request::parse_new_line(bytes)?;
                return Ok(token.to_vec());
            } else if !tokens::is_header_value_token(*b) {
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
        assert_eq!(request.method, Some(Method::GET));
        assert_eq!(request.path, Some(String::from("/test")));
    }

    #[test]
    fn accept_only_newline() {
        let mut request = Request::new();

        request
            .parse(Bytes::from_static(b"GET /test HTTP/1.1\n"))
            .expect("parsing request");

        assert_eq!(request.version, Some(1));
        assert_eq!(request.method, Some(Method::GET));
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
