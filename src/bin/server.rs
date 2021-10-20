use anyhow::Result;
use webserver_from_scratch::{http_request::Method, router::Router, HTTPServer};

// TODO: fix routing

fn main() -> Result<()> {
    let mut server = HTTPServer::new();

    let server = server.handle(Method::GET, "/", |ctx| async {
        let resp = b"Hello World! :)";
        ctx.response.write(resp);
    });

    server.listen_blocking("[::1]:8080".parse().unwrap())
}
