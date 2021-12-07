# Creating a Webserver from scratch (in rust)

# Getting Started

## 1. Install Dependencies

- [rust >= 1.56.0](https://rustup.rs/)

## 2. Run

```bash
$ cargo run --bin client
$ cargo run --bin server
```

# TODO:

- More Security Checks (Request Size limits)
- Websockets
  - [x] websocket middleware
  - [x] websocket upgrade
  - [x] websocket frame parser
  - [x] websocket masking
  - [ ] websocket chunked messages
  - [ ] websocket frame builder
- Partial request parsing
- Stream Abstraction (Chunked encoding)
- Revisit low level parallel processing of incoming sockets

# Assessment

- Which areas still need some focus? Other network protocols?

# Full Example

```rust
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
```

# Macro

## Usage

```rs
middleware!(|ctx| {
   // User Code (can use .await)
})
```

## Resulting Code

```rs
{
    let closure = |ctx: MiddlewareCtx| -> HandlerFut {
        Box::pin(async move {
            // We have to use a mutex for the request context since the
            // borrow checker doesn't recognize that an async function has finished running and
            let mut ctx = ctx.lock(); // will be locked after `ctx` is dropped at the end of this block
            /// {User code}
            Ok(()) // enable error catching by returning a result (enables using `?` for catching errors)
        })
    };
    closure // return closure from this block
}
```

# Inspirations

- Websocket
  - https://github.com/1tgr/rust-websocket-lite
  - https://github.com/snapview/tungstenite-rs
- Http
  - https://expressjs.com
  - https://github.com/nickel-org/nickel.rs
  - https://github.com/seanmonstar/httparse
  - https://github.com/magic003/http-parser-rs
