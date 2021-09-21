use std::collections::HashMap;

use bytes::{Buf, BufMut, BytesMut};
use httpstatus::StatusCode;

pub struct ResponseBuilder {
    status_code: StatusCode,
    content_type: String,
    headers: HashMap<String, String>,
    body: BytesMut,
}

impl Default for ResponseBuilder {
    fn default() -> Self {
        Self {
            status_code: StatusCode::Ok,
            content_type: "text/plain".to_string(),
            headers: HashMap::new(),
            body: BytesMut::new(),
        }
    }
}

impl From<ResponseBuilder> for Vec<u8> {
    fn from(builder: ResponseBuilder) -> Self {
        builder.build()
    }
}

impl ResponseBuilder {
    pub fn new() -> Self {
        ResponseBuilder {
            ..Default::default()
        }
    }

    pub fn status_code(&mut self, status: StatusCode) -> &mut Self {
        self.status_code = status;
        self
    }

    pub fn content_type(&mut self, content_type: String) -> &mut Self {
        self.content_type = content_type;
        self
    }

    pub fn write(&mut self, src: &[u8]) {
        self.body.put_slice(src)
    }

    pub fn build(&self) -> Vec<u8> {
        // http version
        let mut response = b"HTTP/1.1 ".to_vec();

        // status code
        response.put_slice(self.status_code.as_u16().to_string().as_bytes());
        response.put_slice(b" ");
        response.put(self.status_code.reason_phrase().as_bytes());

        // parse body
        let body = self.body.clone();
        let content_length = self.body.len();
        let content_type = if self.content_type != "" {
            self.content_type.clone()
        } else {
            "text/plain".to_string()
        };
        response.put_slice(b"\n");

        // add headers
        let mut headers = self.headers.clone();
        headers.insert("Content-Type".to_string(), content_type);
        headers.insert("Content-Length".to_string(), content_length.to_string());

        for (key, val) in &headers {
            response.put_slice(key.as_bytes());
            response.put_slice(b": ");
            response.put_slice(val.as_bytes());
            response.put_slice(b"\n");
        }
        response.put_slice(b"\n");

        // add body
        response.put(body);
        response
    }
}
