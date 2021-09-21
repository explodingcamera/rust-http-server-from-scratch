use httpstatus::StatusCode;

pub struct ResponseBuilder {
    status_code: StatusCode,
    content_type: String,
    content_length: u64,
}

const HELLO_RESPONSE: &[u8] =
    b"HTTP/1.1 200 OK\nContent-Type: text/plain\nContent-Length: 12\n\nHello world!";

impl Default for ResponseBuilder {
    fn default() -> Self {
        Self {
            status_code: StatusCode::Ok,
            content_type: "text/plain".to_string(),
            content_length: 0,
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

    pub fn build(&self) -> Vec<u8> {
        // http version
        let mut response = b"HTTP/1.1 ".to_vec();

        // status code
        response.extend(self.status_code.as_u16().to_string().as_bytes());
        response.extend(b" ");
        response.extend(self.status_code.reason_phrase().as_bytes());
        response.extend(b"\n");

        // add headers

        // add body

        response
    }
}
