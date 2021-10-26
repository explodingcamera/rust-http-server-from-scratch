use anyhow::Result;
use webserver_from_scratch::{HTTPServer, LogLevel, StatusCode};

fn main() -> Result<()> {
    let mut server = HTTPServer::new();
    server.loglevel(LogLevel::Off);

    // server
    //     .get(
    //         "/",
    //         Box::new(|ctx| {
    //             let resp = b"<h1>Hello World</h1>";
    //             ctx.response.content_type("text/html");
    //             ctx.response.write(resp);
    //             ctx.end();
    //         }),
    //     )
    //     .get(
    //         "/:name",
    //         Box::new(|ctx| {
    //             let name = ctx.params.get(":name");

    //             ctx.response.content_type("text/html");
    //             ctx.response.write(b"<h1>Hello ");

    //             if let Some(name) = name {
    //                 ctx.response.write(name.value.as_bytes());
    //             } else {
    //                 ctx.response.write(b"World");
    //             }

    //             ctx.response.write(b"</h1>");
    //             ctx.end();
    //         }),
    //     );

    // server.any(
    //     "*",
    //     Box::new(|ctx| {
    //         let resp = b"404";
    //         ctx.response.status_code(StatusCode::NotFound);
    //         ctx.response.write(resp);
    //     }),
    // );

    server.listen_blocking("[::1]:8080".parse().unwrap())
}
