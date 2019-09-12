use std::net::{TcpStream, TcpListener};
use std::io::{Read, Write};
use std::io;

static PORT_ENV: &'static str = "ECHO_PORT";

fn echo(mut stream: TcpStream) -> io::Result<()> {
    let addr = stream.peer_addr()?;

    loop {
        let mut buff = [0; 1024];
        let size = stream.read(&mut buff)?;

        if size == 0 {
            println!("--- {} terminated the connection.", addr);
            return Ok(());
        }else{
            println!("--> {} bytes received from {}...", size, addr);

            stream.write(&buff[0..size])?;
            println!("<-- {} bytes sent to {}.", size, addr);
        }
    }
}

fn main() -> io::Result<()> {
    let port: u16 = std::env::var(PORT_ENV)
        .unwrap_or("8080".to_owned())
        .parse()
        .unwrap();

    let addr = format!("{}:{}", "0.0.0.0", port);
    println!("--- started listening on {}.", addr); 

    let listener = TcpListener::bind(addr)?;
    for stream in listener.incoming() {
        let stream = stream?;

        println!("--- user connected: {}", stream.peer_addr()?);
        echo(stream)?;
    }

    Ok(())
}
