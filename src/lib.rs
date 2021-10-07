use anyhow::Result;
use bytes::{Bytes, BytesMut};
use socket2::{Domain, Socket, Type};
use std::net::SocketAddr;
use std::time::Duration;
use thiserror::Error;

// TODO: tokio is temporary and will be replaced by a custom implementation
use tokio;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

pub use httpstatus::{StatusClass, StatusCode};

pub mod http_request;
pub mod http_response;
pub mod tokens;

const REQUEST_BUFFER_SIZE: usize = 30000;

#[derive(Error, Debug)]
pub enum ServerError {}

#[derive(Default, Debug)]
pub struct HTTPServer {}

impl HTTPServer {
    pub fn new() -> Self {
        HTTPServer {}
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

        // We convert the socket into a tokio::net::TcpListener which includes a handy way to check if a socket is ready (since we use non blocking sockets).
        // This is only temporary and will be later replaced by a custom implementation
        let listener = TcpListener::from_std(socket.into())?;

        println!("started server on {}", address);
        loop {
            match listener.accept().await {
                Ok((socket, addr)) => {
                    // Spawn a new task for each request
                    // (A task is essentially a green thread)
                    tokio::spawn(async move {
                        Self::process_request(socket, addr)
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
    async fn process_request(mut socket: TcpStream, _addr: SocketAddr) -> Result<()> {
        // println!("received request from {}", addr);

        // read
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

        // build response
        let mut response = http_response::ResponseBuilder::default();
        response.set_header("x-powered-by", "webserver-from-scratch");
        response.write(b"hello world");

        // write
        socket.writable().await?;
        let response = response.build();
        socket.write_all(&response).await?;

        Ok(())
    }
}
