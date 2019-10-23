use std::io;

mod webserver;

use webserver::WebServer;
use config::Config;
use log::*;

lazy_static::lazy_static! {
    pub static ref CONFIG: Config = {
        let mut conf = Config::default();
        conf.set_default("port", 8080).unwrap();
        conf.set_default("addr", "0.0.0.0").unwrap();
        conf.set_default("templates", "templates/").unwrap();
        conf.set_default("indexes", vec![""; 0]).unwrap();

        conf.set_default("read_timeout", 5000).unwrap();
        conf.set_default("write_timeout", 5000).unwrap();
        conf.set_default("max_request_size", 8192).unwrap();

        let root = std::env::current_dir();
        let root = root
            .unwrap()
            .into_os_string();

        conf.set_default("root", root.to_str().unwrap_or("")).unwrap();

        conf
            .merge(config::File::with_name("config.yml")).unwrap()
            .merge(config::Environment::with_prefix("SERV")).unwrap();

        conf
    };
}

fn main() -> io::Result<()> {
    use std::collections::HashMap;

    pretty_env_logger::init_custom_env("SERV_LOG");

    debug!(
        "initialized with config: \n{:#?}\n",
        CONFIG
            .clone()
            .try_into::<HashMap<String, config::Value>>()
            .unwrap()
    );

    let mut server = WebServer::new()?;

    server.listen()?;
    Ok(())
}
