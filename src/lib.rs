use anyhow::Result;
use socket2::{Domain, Socket, Type};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::Duration;

pub mod http_response;
pub use httpstatus::{StatusClass, StatusCode};

pub fn start_server() -> Result<()> {
    let socket = Socket::new(Domain::IPV6, Type::STREAM, None)?;
    let address: SocketAddr = "[::1]:8080".parse()?;
    socket.bind(&address.into())?;
    socket.set_linger(Some(Duration::new(0, 0)))?;
    socket.listen(128)?;
    let listener: TcpListener = socket.into();

    println!("starting server on {}", address);
    loop {
        match listener.accept() {
            Ok((socket, addr)) => process_request(socket, addr)
                .map_err(|e| println!("Failed with: {}", e))
                .unwrap(),
            Err(e) => println!("couldn't get client: {:?}", e),
        }
    }
}

fn process_request(mut socket: TcpStream, addr: SocketAddr) -> Result<()> {
    println!("received request from {}", addr);

    // read
    let mut buffer = [0; 30000];
    socket.read(&mut buffer[..])?;

    let _req = std::str::from_utf8(&buffer[..])?;
    // println!("Got Request:\n\n{}", req);

    let mut response = http_response::ResponseBuilder::default();
    response.write(b"tee");

    // write
    socket.write(&response.build())?;

    Ok(())
}
