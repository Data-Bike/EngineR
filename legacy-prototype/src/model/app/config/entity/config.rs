use lazy_static::lazy_static;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub host: &'static str,
    pub user: &'static str,
    pub password: &'static str,
    pub database: &'static str,
}

lazy_static! {
    static ref CONTENT:String = match std::fs::read_to_string("config.toml") {
        Ok(x) => {x}
        Err(_) => {
            if !std::path::Path::new("config.toml").exists() {
                let the_config = Config {
                    host: "localhost",
                    user: "enginer",
                    password: "testpassword",
                    database: "enginer",
                };
                let str = toml::to_string(&the_config).unwrap();
                std::fs::write("config.toml", str).unwrap();
            }
            std::fs::read_to_string("config.toml").unwrap()
        }
    };
    pub static ref CONFIG: Config = toml::from_str(&CONTENT).unwrap();
}