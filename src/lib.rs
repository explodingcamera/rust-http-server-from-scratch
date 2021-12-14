#![feature(trait_alias)]
#![feature(fn_traits)]
#![feature(associated_type_bounds)]
#![feature(async_closure)]

use anyhow::Result;
use bytes::{Bytes, BytesMut};
use parking_lot::Mutex;
use router::Route;
// helpers for zero-copy
use socket2::{Domain, Socket, Type};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use std::vec;
use thiserror::Error;
use tokio::runtime;

use tokio::io::{AsyncReadExt, AsyncWriteExt}; // this implements async operations on buffers
use tokio::net::{TcpListener, TcpStream};

use crate::router::{middleware_matches_request, MiddlewareContext, RequestPath};
pub use httpstatus::{StatusClass, StatusCode};

pub mod http_request;
pub mod http_response;
mod macros;
pub mod router;
pub mod tokens;
pub mod websocket;

const REQUEST_BUFFER_SIZE: usize = 30000;

#[derive(Error, Debug)]
pub enum ServerError {}

pub struct HTTPServer {
    routes_mut: Vec<Route>,
    routes: Arc<Vec<Route>>,
    loglevel: LogLevel,
}

#[repr(usize)]
#[derive(Clone)]
pub enum LogLevel {
    Off = 1,
    Debug,
    Normal,
    Critical,
}

trait HTTPFramework {}
impl<'a> HTTPFramework for HTTPServer {}

impl<'a> Default for HTTPServer {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> HTTPServer {
    pub fn new() -> Self {
        HTTPServer {
            routes: Arc::new(vec![]),
            routes_mut: Vec::with_capacity(100),
            loglevel: LogLevel::Off,
        }
    }

    fn add_route(&mut self, route: Route) {
        self.routes_mut.push(route)
    }

    // start listening on a new socket/port
    pub fn listen_blocking(&mut self, address: SocketAddr) -> Result<()> {
        let rt = runtime::Runtime::new()?;
        rt.block_on(self.listen(address))
    }

    pub fn loglevel(&mut self, loglevel: LogLevel) -> &mut Self {
        self.loglevel = loglevel;
        self
    }

    async fn listen(&mut self, address: SocketAddr) -> Result<()> {
        self.routes = Arc::new(self.routes_mut.clone());

        // Create and bind a TCP listener
        // Protocol is None/0 since tcp is implied by Type::STREAM)
        let socket = Socket::new(Domain::IPV6, Type::STREAM, None)?;

        // Enable processing of both ipv6 and ipv4 packets
        socket.set_only_v6(false)?;

        // Stop processing requests right when a close/shutdown request is received
        socket.set_linger(Some(Duration::new(0, 0)))?;

        // Set our socket as non-blocking, which will result in
        // `read`, `write`, `recv` and `send` operations immediately
        // returning from their calls.
        // We want this to enable multiple threads to process sockets concurrently.
        socket.set_nonblocking(true)?;

        // Finally bind the socket to the correct interface/port and start to listen for new connection
        socket.bind(&address.into())?;
        socket.listen(128)?;

        // We convert the socket into a tokio::net::TcpListener, since this
        // includes a handy way to check if a socket is ready (since we use non blocking sockets)
        // and async functions for reading from/writing to a socket (since we use non-blocking green threads).
        let listener = TcpListener::from_std(socket.into())?;

        println!("started server on {}", address);

        // Process incoming requests
        // A separate thread processes all incoming requests
        // -> This thread then creates a new green thread for each of these
        // -> -> This thread then matches the correct middlewares and calls them in the correct order
        let routes = self.routes.clone();
        let loglevel = self.loglevel.clone();

        loop {
            match listener.accept().await {
                // non-blocking equivalent to socket.accept
                Ok((socket, addr)) => {
                    // Spawn a new non-blocking, multithreaded task for each request
                    // (A task is essentially a green thread)

                    let loglevel = loglevel.clone();
                    let routes = routes.clone();

                    tokio::spawn(async move {
                        HTTPServer::process_request(routes, socket, addr, loglevel)
                            .await
                            .unwrap_or_else(|e| {
                                println!("{}", e);
                            })
                    });
                }
                Err(e) => println!("couldn't get client: {:?}", e),
            }
        }
    }

    // process incoming sockets
    async fn process_request(
        routes: Arc<Vec<Route>>,
        mut socket: TcpStream, // Equivalent to Socket with some extra async methods
        _addr: SocketAddr,
        loglevel: LogLevel,
    ) -> Result<()> {
        let loglevel = loglevel as usize;

        // read request
        // NOTE: readable might give a false positive
        socket.readable().await?;
        let mut buffer = BytesMut::with_capacity(REQUEST_BUFFER_SIZE);

        let request_length = socket.read_buf(&mut buffer).await?;

        if loglevel > 1 {
            println!("got request:\n  length: {}", request_length);
        }

        // parse requests
        let mut request = http_request::Request::new();
        request.parse(Bytes::from(buffer))?;

        if loglevel > 1 {
            HTTPServer::print_debug_request(&request.clone());
        }

        let relevant_middlewares: &mut Vec<(Route, RequestPath)> = &mut vec![];
        for route in routes.iter() {
            if route.method.is_some() && route.method != request.method {
                continue;
            }

            if let Ok(Some(request_path)) = middleware_matches_request(&request, route) {
                relevant_middlewares.push((route.clone(), request_path.clone()))
            }
        }

        let mut response = http_response::ResponseBuilder::default();
        response.set_header("x-powered-by", "webserver-from-scratch");

        // Since the borrow checker doesn't know that the ownership is given up inside the middleware, we sadly need to use a mutes.
        // Theoretically we could use unsafe code instead (with safety guarantees) however I want to avoid that.
        let ctx = Arc::new(Mutex::new(MiddlewareContext::new(
            request, response, socket,
        )));

        let mut err = false;
        for (middleware_route, middleware_path) in relevant_middlewares {
            {
                let mut x = ctx.lock();
                x.params = middleware_path.params.clone();
            }
            let handler = middleware_route.handler.clone();

            let fut = handler(ctx.clone());

            if let Err(e) = fut.await {
                err = true;
                println!("An error occurred on a middleware: {}", e);
                break;
            }

            if ctx.lock().has_ended() {
                break;
            }
        }

        if err {
            let mut ctx = ctx.lock();
            ctx.response.clear();
            ctx.response.write(b"internal server error");
            ctx.response.status_code(StatusCode::InternalServerError);
        }

        // write response
        let mut ctx = ctx.lock();
        if !ctx.is_raw() {
            let resp = &ctx.response.build();
            ctx.socket.writable().await?;
            ctx.socket.write_all(resp).await?;
        }

        Ok(())
    }

    fn print_debug_request(request: &http_request::Request) {
        println!(
            "  method: {}\n  path: {}\n  version: HTTP/1.{}",
            request.method.clone().unwrap(),
            request.path.clone().unwrap(),
            request.version.unwrap()
        );

        for (header, value) in request.headers.iter() {
            println!(
                "  header: name=`{}` value=`{}`",
                header,
                String::from_utf8(value.to_vec()).expect("header to be string")
            );
        }

        if !request.body.is_empty() {
            println!(
                "  body: {}",
                String::from_utf8(request.body.clone())
                    .unwrap_or_else(|_| "(not valid utf8)".to_string())
            );
        }
    }
}
