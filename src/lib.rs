use anyhow::Result;
use bytes::{Bytes, BytesMut};
use router::{Route, Router};
// helpers for zero-copy
use socket2::{Domain, Socket, Type};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use std::vec;
use thiserror::Error;

// TODO: tokio is temporary and will be replaced by a custom implementation late on
use tokio;
use tokio::io::{AsyncReadExt, AsyncWriteExt}; // this implements async operations on buffers
use tokio::net::{TcpListener, TcpStream};

pub use httpstatus::{StatusClass, StatusCode};

pub mod http_request;
pub mod http_response;
pub mod router;
pub mod tokens;

const REQUEST_BUFFER_SIZE: usize = 30000;

#[derive(Error, Debug)]
pub enum ServerError {}

pub struct HTTPServer<D: Send + 'static + Sync> {
    routes: Arc<Vec<Route>>,
}

trait HTTPFramework<D: Sync + Send + 'static>: Router<D> {}
impl<D: Sync + Send + 'static> HTTPFramework<D> for HTTPServer<D> {}
impl<D: Sync + Send + 'static> HTTPServer<D> {
    pub fn new() -> Self {
        HTTPServer {
            routes: Arc::new(vec![]),
        }
    }

    // start listening on a new socket/port
    pub async fn listen(self, address: SocketAddr) -> Result<()> {
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
        loop {
            match listener.accept().await {
                Ok((socket, addr)) => {
                    // Spawn a new non-blocking, multithreaded task for each request
                    // (A task is essentially a green thread)

                    tokio::spawn(async {
                        HTTPServer::process_request(self.routes, socket)
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
    async fn process_request(routes: Arc<Vec<Route>>, mut socket: TcpStream) -> Result<()> {
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

        println!(
            "  method: {}\n  path: {}\n  version: HTTP/1.{}",
            request.method.unwrap(),
            request.path.unwrap(),
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
                String::from_utf8(request.body).unwrap_or("(not valid utf8)".to_string())
            );
        }

        // for middleware in self.
        // middleware_matches_request

        // build response
        let mut response = http_response::ResponseBuilder::default();
        response.set_header("x-powered-by", "webserver-from-scratch");
        response.write(b"hello world");

        // write response
        socket.writable().await?;
        let response = response.build();
        socket.write_all(&response).await?;

        Ok(())
    }
}
