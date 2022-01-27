use std::collections::HashMap;
use std::path::PathBuf;
use std::convert::{TryFrom, TryInto};
use std::net::{SocketAddr, IpAddr};

use shape_rs::MapShape;

mod shape;
pub mod error;

#[derive(Debug)]
pub struct DBConfig {
    pub username: String,
    pub password: String,

    pub database: String,

    pub hostname: String,
    pub port: u16
}

impl TryFrom<Option<shape::DBShape>> for DBConfig {
    type Error = error::Error;

    fn try_from(value: Option<shape::DBShape>) -> error::Result<DBConfig> {
        if let Some(v) = value {
            Ok(DBConfig {
                username: v.username.unwrap_or("postgres".to_owned()),
                password: v.password.unwrap_or("".to_owned()),
                database: v.database.unwrap_or("file_server".to_owned()),
                hostname: v.hostname.unwrap_or("localhost".to_owned()),
                port: v.port.unwrap_or(5432)
            })
        } else {
            Ok(DBConfig {
                username: "postgres".to_owned(),
                password: "".to_owned(),
                database: "file_server".to_owned(),
                hostname: "localhost".to_owned(),
                port: 5432
            })
        }
    }
}

#[derive(Debug)]
pub struct EmailConfig {
    pub enable: bool,
    pub from: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub relay: Option<String>
}

impl TryFrom<Option<shape::EmailShape>> for EmailConfig {
    type Error = error::Error;

    fn try_from(value: Option<shape::EmailShape>) -> error::Result<EmailConfig> {
        if let Some(v) = value {
            Ok(EmailConfig {
                enable: v.enable.unwrap_or(false),
                from: v.from,
                username: v.username,
                password: v.password,
                relay: v.relay
            })
        } else {
            Ok(EmailConfig {
                enable: false,
                from: None,
                username: None,
                password: None,
                relay: None
            })
        }
    }
}

#[derive(Debug)]
pub struct ServerInfoConfig {
    pub secure: bool,
    pub origin: String,
    pub name: String
}

impl TryFrom<Option<shape::ServerInfoShape>> for ServerInfoConfig {
    type Error = error::Error;

    fn try_from(value: Option<shape::ServerInfoShape>) -> error::Result<ServerInfoConfig> {
        if let Some(v) = value {
            Ok(ServerInfoConfig {
                secure: v.secure.unwrap_or(false),
                origin: v.origin.unwrap_or("".to_owned()),
                name: v.name.unwrap_or("File Server".to_owned())
            })
        } else {
            Ok(ServerInfoConfig {
                secure: false,
                origin: "".to_owned(),
                name: "File Server".to_owned()
            })
        }
    }
}

#[derive(Debug)]
pub struct TemplateConfig {
    pub directory: PathBuf,
    pub dev_mode: bool,
    pub index_path: Option<PathBuf>
}

impl TryFrom<Option<shape::TemplateShape>> for TemplateConfig {
    type Error = error::Error;

    fn try_from(value: Option<shape::TemplateShape>) -> error::Result<TemplateConfig> {
        let default_dir = std::env::current_dir().unwrap_or(PathBuf::new());

        if let Some(v) = value {
            Ok(TemplateConfig {
                directory: v.directory.unwrap_or(default_dir),
                dev_mode: v.dev_mode.unwrap_or(false),
                index_path: v.index_path
            })
        } else {
            Ok(TemplateConfig {
                directory: default_dir,
                dev_mode: false,
                index_path: None
            })
        }
    }
}

#[derive(Debug)]
pub struct SslConfig {
    pub enable: bool,
    pub key: Option<PathBuf>,
    pub cert: Option<PathBuf>
}

impl TryFrom<Option<shape::SslShape>> for SslConfig {
    type Error = error::Error;

    fn try_from(value: Option<shape::SslShape>) -> error::Result<SslConfig> {
        if let Some(v) = value {
            Ok(SslConfig {
                enable: v.enable.unwrap_or(false),
                key: v.key,
                cert: v.cert
            })
        } else {
            Ok(SslConfig {
                enable: false,
                key: None,
                cert: None
            })
        }
    }
}

#[derive(Debug)]
pub struct WatcherConfig {
    pub enable: bool
}

impl TryFrom<Option<shape::WatcherShape>> for WatcherConfig {
    type Error = error::Error;

    fn try_from(value: Option<shape::WatcherShape>) -> error::Result<WatcherConfig> {
        if let Some(v) = value {
            Ok(WatcherConfig {
                enable: v.enable.unwrap_or(false)
            })
        } else {
            Ok(WatcherConfig {
                enable: false
            })
        }
    }
}

#[derive(Debug)]
pub struct BindInterfaceConfig {
    pub host: String,
    pub port: u16
}

impl BindInterfaceConfig {

    pub fn to_sockaddr(self) -> error::Result<SocketAddr> {
        let host = self.host;
        let ip: IpAddr = host.parse().map_err(
            |_err| error::Error::InvalidIpAddr(host)
        )?;

        Ok(SocketAddr::new(ip, self.port))
    }
    
}

#[derive(Debug)]
pub struct StorageStaticConfig {
    pub directories: HashMap<String, PathBuf>,
    pub files: HashMap<String, PathBuf>
}

impl TryFrom<Option<shape::StorageStaticShape>> for StorageStaticConfig {
    type Error = error::Error;

    fn try_from(value: Option<shape::StorageStaticShape>) -> error::Result<StorageStaticConfig> {
        if let Some(v) = value {
            Ok(StorageStaticConfig {
                directories: v.directories.unwrap_or_default(),
                files: v.files.unwrap_or_default()
            })
        } else {
            Ok(StorageStaticConfig {
                directories: Default::default(),
                files: Default::default()
            })
        }
    }
}

#[derive(Debug)]
pub struct StorageConfig {
    pub directory: PathBuf,
    pub temporary: PathBuf,
    pub web_static: Option<PathBuf>,
    pub static_: StorageStaticConfig
}

impl TryFrom<Option<shape::StorageShape>> for StorageConfig {
    type Error = error::Error;

    fn try_from(value: Option<shape::StorageShape>) -> error::Result<StorageConfig> {
        if let Some(v) = value {
            if v.directory.is_none() {
                return Err(error::Error::InvalidConfig(
                    format!("missing conf.storage.directory")
                ))
            }

            if v.temporary.is_none() {
                return Err(error::Error::InvalidConfig(
                    format!("missing conf.storage.temporary")
                ))
            }

            Ok(StorageConfig {
                directory: v.directory.unwrap(),
                temporary: v.temporary.unwrap(),
                web_static: v.web_static,
                static_: v.static_.try_into()?
            })
        } else {
            Err(error::Error::InvalidConfig(
                format!("missing conf.storage.directory and conf.storage.temporary")
            ))
        }
    }
}

#[derive(Debug)]
pub struct SecurityConfig {
    pub secret: String
}

impl TryFrom<Option<shape::SecurityShape>> for SecurityConfig {
    type Error = error::Error;

    fn try_from(value: Option<shape::SecurityShape>) -> error::Result<SecurityConfig> {
        if let Some(v) = value {
            Ok(SecurityConfig {
                secret: v.secret.unwrap_or(String::new())
            })
        } else {
            Ok(SecurityConfig {
                secret: String::new()
            })
        }
    }
}

#[derive(Debug)]
pub struct ServerConfig {
    pub storage: StorageConfig,
    pub bind: Vec<BindInterfaceConfig>,

    pub backlog: u32,
    pub threads: usize,
    pub max_connections: usize,
    pub max_connection_rate: usize,

    pub db: DBConfig,
    pub email: EmailConfig,
    pub info: ServerInfoConfig,
    pub ssl: SslConfig,
    pub template: TemplateConfig,
    pub watcher: WatcherConfig,
    pub security: SecurityConfig,
}

impl TryFrom<shape::ServerShape> for ServerConfig {
    type Error = error::Error;

    fn try_from(server_shape: shape::ServerShape) -> error::Result<ServerConfig> {
        let mut bind: Vec<BindInterfaceConfig>;

        if let Some(interfaces) = server_shape.bind {
            bind = Vec::with_capacity(interfaces.len());
            let port = server_shape.port.unwrap_or(8080);

            for inter in interfaces {
                bind.push(BindInterfaceConfig {
                    host: inter.host,
                    port: inter.port.unwrap_or(port)
                })
            }
        } else {
            bind = Vec::new();
        }

        Ok(ServerConfig {
            storage: server_shape.storage.try_into()?,
            bind,
            threads: server_shape.threads.unwrap_or(num_cpus::get()),
            backlog: server_shape.backlog.unwrap_or(2048),
            max_connections: server_shape.max_connections.unwrap_or(25000),
            max_connection_rate: server_shape.max_connection_rate.unwrap_or(256),
            db: server_shape.db.try_into()?,
            email: server_shape.email.try_into()?,
            info: server_shape.info.try_into()?,
            ssl: server_shape.ssl.try_into()?,
            template: server_shape.template.try_into()?,
            watcher: server_shape.watcher.try_into()?,
            security: server_shape.security.try_into()?
        })
    }
}

pub fn load_server_config(files: Vec<PathBuf>) -> error::Result<ServerConfig> {
    let mut base_shape = shape::ServerShape::default();

    for file in files {
        let shape = shape::ServerShape::try_from(&file)?;
        let parent = file.parent().unwrap();

        base_shape.map_shape(shape::validate_server_shape(
            &parent,
            shape
        )?);
    }

    base_shape.try_into()
}

pub fn get_config_file(path: &String) -> error::Result<PathBuf> {
    if let Ok(cannonical_path) = std::fs::canonicalize(path) {
        if !cannonical_path.is_file() {
            Err(error::Error::InvalidFile(cannonical_path.into_os_string()))
        } else {
            Ok(cannonical_path)
        }
    } else {
        Err(error::Error::FileNotFound(path.clone()))
    }
}