use std::path::{PathBuf};
use std::convert::{From, TryFrom};
use std::net::{SocketAddr, IpAddr};

use shape_rs::{MapShape};

mod shape;
pub mod error;

pub struct DBConfig {
    pub username: String,
    pub password: String,

    pub database: String,

    pub hostname: String,
    pub port: u16
}

impl From<Option<shape::DBShape>> for DBConfig {
    fn from(value: Option<shape::DBShape>) -> DBConfig {
        if let Some(v) = value {
            DBConfig {
                username: v.username.unwrap_or("postgres".to_owned()),
                password: v.password.unwrap_or("".to_owned()),
                database: v.database.unwrap_or("image_server".to_owned()),
                hostname: v.hostname.unwrap_or("localhost".to_owned()),
                port: v.port.unwrap_or(5432)
            }
        } else {
            DBConfig {
                username: "postgres".to_owned(),
                password: "".to_owned(),
                database: "image_server".to_owned(),
                hostname: "localhost".to_owned(),
                port: 5432
            }
        }
    }
}

pub struct EmailConfig {
    pub enable: bool,
    pub from: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub relay: Option<String>
}

impl From<Option<shape::EmailShape>> for EmailConfig {
    fn from(value: Option<shape::EmailShape>) -> EmailConfig {
        if let Some(v) = value {
            EmailConfig {
                enable: v.enable.unwrap_or(false),
                from: v.from,
                username: v.username,
                password: v.password,
                relay: v.relay
            }
        } else {
            EmailConfig {
                enable: false,
                from: None,
                username: None,
                password: None,
                relay: None
            }
        }
    }
}

pub struct ServerInfoConfig {
    pub secure: bool,
    pub origin: String,
    pub name: String
}

impl From<Option<shape::ServerInfoShape>> for ServerInfoConfig {
    fn from(value: Option<shape::ServerInfoShape>) -> ServerInfoConfig {
        if let Some(v) = value {
            ServerInfoConfig {
                secure: v.secure.unwrap_or(false),
                origin: v.origin.unwrap_or("".to_owned()),
                name: v.name.unwrap_or("Image Server".to_owned())
            }
        } else {
            ServerInfoConfig {
                secure: false,
                origin: "".to_owned(),
                name: "Image Server".to_owned()
            }
        }
    }
}

pub struct TemplateConfig {
    pub directory: PathBuf,
    pub dev_mode: bool,
}

impl From<Option<shape::TemplateShape>> for TemplateConfig {
    fn from(value: Option<shape::TemplateShape>) -> TemplateConfig {
        let default_dir = std::env::current_dir().unwrap_or(PathBuf::new());

        if let Some(v) = value {
            TemplateConfig {
                directory: v.directory.unwrap_or(default_dir),
                dev_mode: v.dev_mode.unwrap_or(false)
            }
        } else {
            TemplateConfig {
                directory: default_dir,
                dev_mode: false
            }
        }
    }
}

pub struct SslConfig {
    pub enable: bool,
    pub key: Option<PathBuf>,
    pub cert: Option<PathBuf>
}

impl From<Option<shape::SslShape>> for SslConfig {
    fn from(value: Option<shape::SslShape>) -> SslConfig {
        if let Some(v) = value {
            SslConfig {
                enable: v.enable.unwrap_or(false),
                key: v.key,
                cert: v.cert
            }
        } else {
            SslConfig {
                enable: false,
                key: None,
                cert: None
            }
        }
    }
}

pub struct WatcherConfig {
    pub enable: bool
}

impl From<Option<shape::WatcherShape>> for WatcherConfig {
    fn from(value: Option<shape::WatcherShape>) -> WatcherConfig {
        if let Some(v) = value {
            WatcherConfig {
                enable: v.enable.unwrap_or(false)
            }
        } else {
            WatcherConfig {
                enable: false
            }
        }
    }
}

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

pub struct ServerConfig {
    pub directory: PathBuf,
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
    pub watcher: WatcherConfig
}

impl From<shape::ServerShape> for ServerConfig {
    fn from(server_shape: shape::ServerShape) -> ServerConfig {
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

        ServerConfig {
            directory: server_shape.directory.unwrap_or(PathBuf::new()),
            bind,
            threads: server_shape.threads.unwrap_or(num_cpus::get()),
            backlog: server_shape.backlog.unwrap_or(2048),
            max_connections: server_shape.max_connections.unwrap_or(25000),
            max_connection_rate: server_shape.max_connection_rate.unwrap_or(256),
            db: server_shape.db.into(),
            email: server_shape.email.into(),
            info: server_shape.info.into(),
            ssl: server_shape.ssl.into(),
            template: server_shape.template.into(),
            watcher: server_shape.watcher.into(),
        }
    }
}

pub fn load_server_config(files: Vec<PathBuf>) -> error::Result<ServerConfig> {
    let mut base_shape = shape::ServerShape::default();

    for file in files {
        base_shape.map_shape(shape::ServerShape::try_from(file)?);
    }

    Ok(base_shape.into())
}

pub fn validate_server_config(conf: &ServerConfig) -> error::Result<()> {
    if !conf.directory.exists() {
        return Err(error::Error::InvalidConfig(
            format!("requested image directory does not exist. given: \"{}\"", conf.directory.display())
        ));
    } else if !conf.directory.is_dir() {
        return Err(error::Error::InvalidConfig(
            format!("requested image directory is not a directory. given: \"{}\"", conf.directory.display())
        ));
    }

    if conf.bind.is_empty() {
        return Err(error::Error::InvalidConfig(
            format!("no bind interfaces where specified")
        ));
    }

    Ok(())
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