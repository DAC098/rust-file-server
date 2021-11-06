use std::path::{PathBuf};
use std::convert::{TryFrom};

use serde::{Deserialize};
use shape_rs::{assign_map_struct, MapShape};

use crate::config::{error};

#[derive(Deserialize)]
pub struct DBShape {
    pub username: Option<String>,
    pub password: Option<String>,

    pub database: Option<String>,

    pub hostname: Option<String>,
    pub port: Option<u16>
}

impl MapShape for DBShape {
    fn map_shape(&mut self, rhs: Self) {
        self.username.map_shape(rhs.username);
        self.password.map_shape(rhs.password);
        self.database.map_shape(rhs.database);
        self.hostname.map_shape(rhs.hostname);
        self.port.map_shape(rhs.port);
    }
}

#[derive(Deserialize)]
pub struct EmailShape {
    pub enable: Option<bool>,
    pub from: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub relay: Option<String>
}

impl MapShape for EmailShape {
    fn map_shape(&mut self, rhs: Self) {
        self.enable.map_shape(rhs.enable);
        self.from.map_shape(rhs.from);
        self.username.map_shape(rhs.username);
        self.password.map_shape(rhs.password);
        self.relay.map_shape(rhs.relay);
    }
}

#[derive(Deserialize)]
pub struct ServerInfoShape {
    pub secure: Option<bool>,
    pub origin: Option<String>,
    pub name: Option<String>
}

impl MapShape for ServerInfoShape {
    fn map_shape(&mut self, rhs: Self) {
        self.secure.map_shape(rhs.secure);
        self.origin.map_shape(rhs.origin);
        self.name.map_shape(rhs.name);
    }
}

#[derive(Deserialize)]
pub struct TemplateShape {
    pub directory: Option<PathBuf>,
    pub dev_mode: Option<bool>
}

impl MapShape for TemplateShape {
    fn map_shape(&mut self, rhs: Self) {
        self.directory.map_shape(rhs.directory);
        self.dev_mode.map_shape(rhs.dev_mode);
    }
}

#[derive(Deserialize)]
pub struct SslShape {
    pub enable: Option<bool>,
    pub key: Option<PathBuf>,
    pub cert: Option<PathBuf>
}

impl MapShape for SslShape {
    fn map_shape(&mut self, rhs: Self) {
        self.enable.map_shape(rhs.enable);
        self.key.map_shape(rhs.key);
        self.cert.map_shape(rhs.cert);
    }
}

#[derive(Deserialize)]
pub struct WatcherShape {
    pub enable: Option<bool>
}

impl MapShape for WatcherShape {
    fn map_shape(&mut self, rhs: Self) {
        self.enable.map_shape(rhs.enable);
    }
}

#[derive(Deserialize)]
pub struct BindInterfaceShape {
    pub host: String,
    pub port: Option<u16>
}

#[derive(Deserialize)]
pub struct ServerShape {
    pub directory: Option<PathBuf>,
    pub bind: Option<Vec<BindInterfaceShape>>,
    pub port: Option<u16>,

    pub threads: Option<usize>,
    pub backlog: Option<u32>,
    pub max_connections: Option<usize>,
    pub max_connection_rate: Option<usize>,

    pub db: Option<DBShape>,
    pub email: Option<EmailShape>,
    pub info: Option<ServerInfoShape>,
    pub ssl: Option<SslShape>,
    pub template: Option<TemplateShape>,
    pub watcher: Option<WatcherShape>,
}

impl MapShape for ServerShape {
    fn map_shape(&mut self, rhs: Self) {
        self.directory.map_shape(rhs.directory);
        self.bind.map_shape(rhs.bind);
        self.port.map_shape(rhs.port);
        self.threads.map_shape(rhs.threads);
        self.backlog.map_shape(rhs.backlog);
        self.max_connections.map_shape(rhs.max_connections);
        self.max_connection_rate.map_shape(rhs.max_connection_rate);

        assign_map_struct(&mut self.db, rhs.db);
        assign_map_struct(&mut self.email, rhs.email);
        assign_map_struct(&mut self.info, rhs.info);
        assign_map_struct(&mut self.ssl, rhs.ssl);
        assign_map_struct(&mut self.template, rhs.template);
        assign_map_struct(&mut self.watcher, rhs.watcher);
    }
}

impl Default for ServerShape {
    fn default() -> ServerShape {
        ServerShape {
            directory: None,
            bind: None,
            port: None,
            threads: None,
            backlog: None,
            max_connections: None,
            max_connection_rate: None,

            db: None,
            email: None,
            info: None,
            ssl: None,
            template: None,
            watcher: None
        }
    }
}

impl TryFrom<PathBuf> for ServerShape {
    type Error = error::Error;

    fn try_from(config_file: PathBuf) -> Result<ServerShape, Self::Error> {
        if let Some(ext) = config_file.extension() {
            let ext = ext.to_ascii_lowercase();
    
            if ext.eq("yaml") || ext.eq("yml") {
                Ok(serde_yaml::from_reader::<
                    std::io::BufReader<std::fs::File>,
                    ServerShape
                    >(std::io::BufReader::new(
                        std::fs::File::open(&config_file)?
                    ))?)
            } else if ext.eq("json") {
                Ok(serde_json::from_reader::<
                    std::io::BufReader<std::fs::File>,
                    ServerShape
                    >(std::io::BufReader::new(
                        std::fs::File::open(&config_file)?
                    ))?)
            } else {
                Err(error::Error::InvalidExtension(ext.to_os_string()))
            }
        } else {
            Err(error::Error::UnknownExtension)
        }
    }
}