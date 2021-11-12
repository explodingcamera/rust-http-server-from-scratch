#![feature(async_closure)]

use anyhow::Result;
use webserver_from_scratch::{middleware, router::Router, HTTPServer, LogLevel, StatusCode};

fn main() -> Result<()> {
    let mut server = HTTPServer::new();
    server.loglevel(LogLevel::Off);

    let hello_world_handler = middleware!(|ctx| {
        let resp = b"<h1>Hello World</h1>";
        ctx.response.content_type("text/html");
        ctx.response.write(resp);
        ctx.end();
    });

    let hello_handler = middleware!(|ctx| {
        ctx.response.content_type("text/html");
        ctx.response.write(b"<h1>Hello ");

        let params = ctx.params.clone();
        let name = params.get(":name");
        let name = if let Some(name) = name {
            name.value.as_bytes()
        } else {
            b"World"
        };

        ctx.response.write(name);
        ctx.response.write(b"</h1>");
        ctx.end();
    });

    server
        .get("/", hello_world_handler)
        .get("/:name", hello_handler);

    server.get(
        "*",
        middleware!(|ctx| {
            webserver_from_scratch::websocket::accept_websocket(&mut ctx).await?;
        }),
    );

    server.any(
        "*",
        middleware!(|ctx| {
            let resp = b"404";
            ctx.response.status_code(StatusCode::NotFound);
            ctx.response.write(resp);
        }),
    );

    server.listen_blocking("[::1]:8080".parse().unwrap())
}
