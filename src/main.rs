use anyhow::Result;
use socket2::{Domain, Socket, Type};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};

fn main() -> Result<()> {
    let socket = Socket::new(Domain::IPV6, Type::STREAM, None)?;
    let address: SocketAddr = "[::1]:8080".parse()?;
    socket.bind(&address.into())?;

    socket.listen(128)?;
    let listener: TcpListener = socket.into();

    println!("starting server");
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
    let mut buffer = [0; 10];
    socket.read(&mut buffer[..])?;
    println!("The bytes: {:?}", &buffer[..]);

    // write
    socket.write(b"hi")?;

    Ok(())
}
