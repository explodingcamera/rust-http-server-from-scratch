use anyhow::Result;
use socket2::{Domain, Socket, Type};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};

pub mod http_response;
pub use httpstatus::{StatusClass, StatusCode};

#[derive(Default, Debug)]
pub struct HTTPServer {}

impl HTTPServer {
    pub fn new() -> Self {
        HTTPServer {}
    }

    pub async fn listen(self, address: SocketAddr) -> Result<()> {
        let socket = Socket::new(Domain::IPV6, Type::STREAM, None)?;
        socket.bind(&address.into())?;
        socket.set_linger(Some(Duration::new(0, 0)))?;
        socket.listen(128)?;
        let listener = TcpListener::from_std(socket.into())?;

        println!("starting server on {}", address);
        loop {
            match listener.accept().await {
                Ok((socket, addr)) => Self::process_request(socket, addr)
                    .await
                    .map_err(|e| println!("Failed with: {}", e))
                    .unwrap(),
                Err(e) => println!("couldn't get client: {:?}", e),
            }
        }
    }

    async fn process_request(socket: TcpStream, addr: SocketAddr) -> Result<()> {
        println!("received request from {}", addr);

        // read
        let mut buffer = [0; 30000];

        socket.readable().await?;
        socket.try_read(&mut buffer[..])?;

        let req = std::str::from_utf8(&buffer[..])?;
        println!("Got Request:\n\n{}", req);

        let mut response = http_response::ResponseBuilder::default();
        response.write(b"tee");

        // write
        socket.writable().await?;
        socket.try_write(&response.build())?;

        Ok(())
    }
}
