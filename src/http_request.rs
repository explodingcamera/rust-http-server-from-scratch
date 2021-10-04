use bytes::{BufMut, BytesMut};
use std::collections::HashMap;

struct Headers {
    headers: HashMap<String, &[u8]>,
}

pub struct Request {
    /// The request method, such as `GET`.
    pub method: Option<&'buf str>,
    /// The request path, such as `/about-us`.
    pub path: Option<&'buf str>,
    /// The request version, such as `HTTP/1.1`.
    pub version: Option<u8>,
    /// The request headers.
    pub headers: Headers,
}
