use anyhow::Result;
use webserver_from_scratch::{http_request::Method, router::Router, HTTPServer, StatusCode};

fn main() -> Result<()> {
    let mut server = HTTPServer::new();

    let server = server
        .handle(
            Method::GET,
            "/",
            Box::new(|ctx| {
                let resp = b"Hello World! :)";
                ctx.response.write(resp);
                ctx.end();
            }),
        )
        .any(
            "*",
            Box::new(|ctx| {
                let resp = b"404";
                ctx.response.status_code(StatusCode::NotFound);
                ctx.response.write(resp);
            }),
        );

    server.listen_blocking("[::1]:8080".parse().unwrap())
}
