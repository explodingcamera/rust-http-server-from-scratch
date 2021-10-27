#![feature(async_closure)]

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::Mutex;
use webserver_from_scratch::{
    router::{HandlerFn, HandlerFut, MiddlewareContext, Router},
    HTTPServer, LogLevel, StatusCode,
};

fn main() -> Result<()> {
    let mut server = HTTPServer::new();
    server.loglevel(LogLevel::Off);

    server
        .get("/", |ctx: Arc<Mutex<MiddlewareContext>>| -> HandlerFut {
            Box::pin(async move {
                let mut ctx = ctx.lock().await;
                let resp = b"<h1>Hello World</h1>";
                ctx.response.content_type("text/html");
                ctx.response.write(resp);
                ctx.end();
                Ok(())
            })
        })
        .get(
            "/:name",
            |ctx: Arc<Mutex<MiddlewareContext>>| -> HandlerFut {
                Box::pin(async move {
                    let mut _ctx = ctx.lock().await;
                    _ctx.response.content_type("text/html");
                    _ctx.response.write(b"<h1>Hello ");

                    let params = _ctx.params.clone();
                    let name = params.get(":name");
                    let name = if let Some(name) = name {
                        name.value.as_bytes()
                    } else {
                        b"World"
                    };

                    _ctx.response.write(&name);
                    _ctx.response.write(b"</h1>");
                    _ctx.end();
                    Ok(())
                })
            },
        );

    server.any("*", |ctx: Arc<Mutex<MiddlewareContext>>| -> HandlerFut {
        Box::pin(async move {
            let mut ctx = ctx.lock().await;

            let resp = b"404";
            ctx.response.status_code(StatusCode::NotFound);
            ctx.response.write(resp);
            Ok(())
        })
    });

    server.listen_blocking("[::1]:8080".parse().unwrap())
}
