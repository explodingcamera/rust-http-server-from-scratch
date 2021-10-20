use anyhow::{anyhow, Result};
use bytes::{Bytes, BytesMut};
use futures::channel::mpsc;
use futures::channel::oneshot;

use http_request::Request;
use router::{Handler, Route, Router};
// helpers for zero-copy
use socket2::{Domain, Socket, Type};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use std::vec;
use thiserror::Error;
use tokio::runtime;

// TODO: tokio is temporary and will be replaced by a custom implementation late on
use tokio;
use tokio::io::{AsyncReadExt, AsyncWriteExt}; // this implements async operations on buffers
use tokio::net::{TcpListener, TcpStream};

pub use httpstatus::{StatusClass, StatusCode};

use crate::router::MiddlewareContext;
use crate::router::{middleware_matches_request, RequestPath};

pub mod http_request;
pub mod http_response;
pub mod router;
pub mod tokens;

const REQUEST_BUFFER_SIZE: usize = 30000;

#[derive(Error, Debug)]
pub enum ServerError {}

pub struct HTTPServer {
    handler_count: u64,
    handlers: HashMap<u64, Handler>,
    routes_mut: Vec<Route>,
    routes: Arc<Vec<Route>>,
}

struct NewRequest {
    pub request: Arc<Request>,
    pub handlers: Vec<(Route, Option<RequestPath>)>,
    pub resp: oneshot::Sender<Result<Vec<u8>>>,
}

trait HTTPFramework: Router {}
impl HTTPFramework for HTTPServer {}
impl HTTPServer {
    pub fn new() -> Self {
        HTTPServer {
            handler_count: 0,
            handlers: HashMap::with_capacity(100),
            routes: Arc::new(vec![]),
            routes_mut: Vec::with_capacity(100),
        }
    }

    fn add_route(&mut self, route: Route) {
        self.routes_mut.push(route)
    }

    fn add_handler(&mut self, id: u64, handler: Handler) {
        self.handlers.insert(id, handler);
    }

    // start listening on a new socket/port
    pub fn listen_blocking(&mut self, address: SocketAddr) -> Result<()> {
        let rt = runtime::Runtime::new()?;
        rt.block_on(self.listen(address))
    }

    async fn listen(&mut self, address: SocketAddr) -> Result<()> {
        let (tx, mut rx) = mpsc::channel::<NewRequest>(1000);

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
        //
        // This reliance on tokio is mostly temporary and will be later replaced by a custom implementation
        let listener = TcpListener::from_std(socket.into())?;

        println!("started server on {}", address);

        // Process incoming requests
        // A separate thread processes all incoming requests
        // -> This thread then creates a new green thread for each of these
        // -> -> This thread then matches the correct middlewares
        // -> -> These are then send to the main thread using the mpsc queue (tx)
        // New requests are then processed in the while loop below
        // -> The correct middlewares are called and a respons is build
        // -> -> The thread then returns the body to it's client
        let routes = self.routes.clone();
        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((socket, addr)) => {
                        // Spawn a new non-blocking, multithreaded task for each request
                        // (A task is essentially a green thread)

                        let routes = routes.clone();
                        let tx = tx.clone();

                        tokio::spawn(async move {
                            HTTPServer::process_request(routes, socket, addr, tx)
                                .await
                                .unwrap_or_else(|e| {
                                    println!("{}", e);
                                })
                        });
                    }
                    Err(e) => println!("couldn't get client: {:?}", e),
                }
            }
        });

        loop {
            while let Some(req) = rx.try_next().ok() {
                match req {
                    None => return Ok(()),
                    Some(req) => {
                        let resp_channel = req.resp;
                        let mut response = http_response::ResponseBuilder::default();
                        response.set_header("x-powered-by", "webserver-from-scratch");
                        let mut ctx = MiddlewareContext::new(req.request.clone(), response);

                        let mut err = false;
                        for (route, _request_path) in req.handlers.iter() {
                            // TODO: add route and request_path to the context

                            if let Some(handler) = self.handlers.get_mut(&route.handler) {
                                handler(&mut ctx);
                                continue;
                            }
                            err = true;
                            break;
                        }

                        if err {
                            resp_channel
                                .send(Err(anyhow!("invalid handler id")))
                                .expect("response channel should be open");
                        } else {
                            resp_channel
                                .send(Ok(ctx.response.build()))
                                .expect("response channel should be open");
                        }
                    }
                }
            }
        }
    }

    // process incoming sockets
    async fn process_request(
        routes: Arc<Vec<Route>>,
        mut socket: TcpStream,
        _addr: SocketAddr,
        mut tx: mpsc::Sender<NewRequest>,
    ) -> Result<()> {
        // println!("received request from {}", addr);

        // read request
        // NOTE: readable might give a false positive, maybe add retry logic in the future
        socket.readable().await?;
        let mut buffer = BytesMut::with_capacity(REQUEST_BUFFER_SIZE);

        let request_length = socket.read_buf(&mut buffer).await?;
        println!("got request:\n  length: {}", request_length);

        // parse requests
        let mut request = http_request::Request::new();
        request.parse(Bytes::from(buffer))?;
        let request = Arc::new(request);

        println!(
            "  method: {}\n  path: {}\n  version: HTTP/1.{}",
            request.method.clone().unwrap(),
            request.path.clone().unwrap(),
            request.version.clone().unwrap()
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
                String::from_utf8(request.body.clone()).unwrap_or("(not valid utf8)".to_string())
            );
        }

        // for middleware in self
        let mut apply_middlewares: Vec<(Route, Option<RequestPath>)> = vec![];
        for route in routes.iter() {
            match middleware_matches_request(&request, route) {
                Ok(request_path) => apply_middlewares.push((route.to_owned(), request_path)),
                Err(_) => {}
            }
        }

        println!("{:?}", apply_middlewares);

        let (resp_tx, resp_rx) = oneshot::channel();
        tx.try_send(NewRequest {
            handlers: apply_middlewares,
            request,
            resp: resp_tx,
        })
        .expect("channel should be open");

        // write response
        socket.writable().await?;
        socket.write_all(&resp_rx.await??).await?;

        Ok(())
    }
}
