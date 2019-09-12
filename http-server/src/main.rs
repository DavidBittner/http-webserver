use std::net::{TcpListener, TcpStream};
use log::*;

mod config;
mod webserver;

lazy_static::lazy_static! {
    pub static ref CONFIG: config::Config = {
        use std::fs::File;
        use std::io::Read;

        let file = File::open("config.yml");

        match file {
            Ok(mut file) => {
                let mut contents = String::new();
                file.read_to_string(&mut contents)
                    .expect("failed to read config file.");

                serde_yaml::from_str(&contents)
                    .expect("fhe config file is malformed yaml.")
            },
            Err(_) => {
                config::Config {
                    port: 8080
                }
            }
        }
    };
}

use std::io::{Write, Read, BufRead, BufReader};
use std::io;

fn parse_request(stream: &mut TcpStream) -> io::Result<String> {
    let reader = BufReader::new(stream);

    let cont: String = reader.lines()
        .map(Result::unwrap)
        .take_while(|line| !line.is_empty())
        .fold(String::new(), |tot, at| format!("{}{}", tot, at));

    Ok(cont)
}

fn handle_request(stream: &mut TcpStream, mut req: String) -> io::Result<()> {
    stream.write(resp.as_bytes())?;
    stream.flush()?;

    Ok(())
}

fn main() -> std::io::Result<()> {
    pretty_env_logger::init();

    let addr = format!("127.0.0.1:{}", CONFIG.port);
    info!("listening at addr: {}", addr);

    let listener = TcpListener::bind(addr)?;

    for stream in listener.incoming() {
        let mut stream = stream?;

        let string = parse_request(&mut stream)?;
        let addr = stream.peer_addr()?;
        info!("Request from: {}", addr);
        info!("{}", string);

        info!("handling...");
        handle_request(&mut stream, string)?;
    }

    Ok(())
}
