use std::io;

mod config;
mod webserver;

use webserver::WebServer;

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
                    port: 8080,
                    addr: "0.0.0.0".parse().unwrap()
                }
            }
        }
    };
}

fn main() -> io::Result<()> {
    pretty_env_logger::init();
    let mut server = WebServer::new(&CONFIG)?;
    server.listen()?;

    Ok(())
}
