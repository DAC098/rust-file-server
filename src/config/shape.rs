use std::fmt::Write;
use std::{fs::canonicalize, collections::HashMap};
use std::path::{PathBuf, Path};
use std::convert::TryFrom;
use std::io::ErrorKind as IoErrorKind;

use serde::Deserialize;
use shape_rs::{assign_map_struct, MapShape};

use crate::config::error;

#[derive(Debug,Deserialize)]
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

#[derive(Debug,Deserialize)]
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

#[derive(Debug,Deserialize)]
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

#[derive(Debug,Deserialize)]
pub struct TemplateShape {
    pub directory: Option<PathBuf>,
    pub dev_mode: Option<bool>,
    pub index_path: Option<PathBuf>,
}

impl MapShape for TemplateShape {
    fn map_shape(&mut self, rhs: Self) {
        self.directory.map_shape(rhs.directory);
        self.dev_mode.map_shape(rhs.dev_mode);
        self.index_path.map_shape(rhs.index_path);
    }
}

#[derive(Debug,Deserialize)]
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

#[derive(Debug,Deserialize)]
pub struct WatcherShape {
    pub enable: Option<bool>
}

impl MapShape for WatcherShape {
    fn map_shape(&mut self, rhs: Self) {
        self.enable.map_shape(rhs.enable);
    }
}

#[derive(Debug,Deserialize)]
pub struct BindInterfaceShape {
    pub host: String,
    pub port: Option<u16>
}

#[derive(Debug,Deserialize)]
pub struct StorageStaticShape {
    pub directories: Option<HashMap<String, PathBuf>>,
    pub files: Option<HashMap<String, PathBuf>>
}

impl MapShape for StorageStaticShape {
    fn map_shape(&mut self, rhs: Self) {
        if let Some(map) = self.directories.as_mut() {
            if let Some(rhs_map) = rhs.directories {
                for (key, path) in rhs_map {
                    map.insert(key, path);
                }
            }
        } else if let Some(rhs_map) = rhs.directories {
            self.directories = Some(rhs_map);
        }

        if let Some(map) = self.files.as_mut() {
            if let Some(rhs_map) = rhs.files {
                for (key, path) in rhs_map {
                    map.insert(key, path);
                }
            }
        } else if let Some(rhs_map) = rhs.files {
            self.files = Some(rhs_map)
        }
    }
}

#[derive(Debug,Deserialize)]
pub struct StorageShape {
    pub directory: Option<PathBuf>,
    pub temporary: Option<PathBuf>,
    pub web_static: Option<PathBuf>,

    #[serde(rename(deserialize = "static"))]
    pub static_: Option<StorageStaticShape>
}

impl MapShape for StorageShape {
    fn map_shape(&mut self, rhs: Self) {
        self.directory.map_shape(rhs.directory);
        self.temporary.map_shape(rhs.temporary);
        self.web_static.map_shape(rhs.web_static);

        assign_map_struct(&mut self.static_, rhs.static_);
    }
}

#[derive(Debug,Deserialize)]
pub struct SecurityShape {
    pub secret: Option<String>,
}

impl MapShape for SecurityShape {
    fn map_shape(&mut self, rhs: Self) {
        self.secret.map_shape(rhs.secret);
    }
}

#[derive(Debug,Deserialize)]
pub struct ServerShape {
    pub storage: Option<StorageShape>,
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
    pub security: Option<SecurityShape>,
}

impl MapShape for ServerShape {
    fn map_shape(&mut self, rhs: Self) {
        self.bind.map_shape(rhs.bind);
        self.port.map_shape(rhs.port);
        self.threads.map_shape(rhs.threads);
        self.backlog.map_shape(rhs.backlog);
        self.max_connections.map_shape(rhs.max_connections);
        self.max_connection_rate.map_shape(rhs.max_connection_rate);

        assign_map_struct(&mut self.storage, rhs.storage);
        assign_map_struct(&mut self.db, rhs.db);
        assign_map_struct(&mut self.email, rhs.email);
        assign_map_struct(&mut self.info, rhs.info);
        assign_map_struct(&mut self.ssl, rhs.ssl);
        assign_map_struct(&mut self.template, rhs.template);
        assign_map_struct(&mut self.watcher, rhs.watcher);
        assign_map_struct(&mut self.security, rhs.security);
    }
}

impl Default for ServerShape {
    fn default() -> ServerShape {
        ServerShape {
            storage: None,
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
            watcher: None,
            security: None
        }
    }
}

impl TryFrom<&PathBuf> for ServerShape {
    type Error = error::Error;

    fn try_from(config_file: &PathBuf) -> Result<ServerShape, Self::Error> {
        if let Some(ext) = config_file.extension() {
            let ext = ext.to_ascii_lowercase();

            if ext.eq("yaml") || ext.eq("yml") {
                Ok(serde_yaml::from_reader::<
                    std::io::BufReader<std::fs::File>,
                    ServerShape
                    >(std::io::BufReader::new(
                        std::fs::File::open(config_file)?
                    ))?)
            } else if ext.eq("json") {
                Ok(serde_json::from_reader::<
                    std::io::BufReader<std::fs::File>,
                    ServerShape
                    >(std::io::BufReader::new(
                        std::fs::File::open(config_file)?
                    ))?)
            } else {
                Err(error::Error::InvalidExtension(ext.to_os_string()))
            }
        } else {
            Err(error::Error::UnknownExtension)
        }
    }
}

fn validate_path_buf(conf_dir: &Path, name: &str, is_dir: bool, directory: PathBuf) -> error::Result<PathBuf> {
    let to_canonicalize = if directory.has_root() {
        directory
    } else {
        let mut with_root = conf_dir.clone().to_owned();
        with_root.push(directory);
        with_root
    };

    match canonicalize(&to_canonicalize) {
        Ok(path) => {
            if is_dir {
                if !path.is_dir() {
                    Err(error::Error::InvalidConfig(
                        format!(
                            "requested {} is not a directory.\nconfig file: {}\ngiven value: {}\nreal path: {}",
                            name,
                            conf_dir.display(), 
                            to_canonicalize.display(),
                            path.display()
                        )
                    ))
                } else {
                    Ok(path)
                }
            } else {
                if !path.is_file() {
                    Err(error::Error::InvalidConfig(
                        format!(
                            "requested {} is not a file.\nconfig file: {}\ngiven value: {}\nreal path: {}",
                            name,
                            conf_dir.display(),
                            to_canonicalize.display(),
                            path.display()
                        )
                    ))
                } else {
                    Ok(path)
                }
            }
        },
        Err(error) => match error.kind() {
            IoErrorKind::NotFound => Err(error::Error::InvalidConfig(
                format!(
                    "requested {} directory was not found.\nconfig file: {}\ngive value: {}",
                    name,
                    conf_dir.display(), 
                    to_canonicalize.display()
                )
            )),
            _ => Err(error.into())
        }
    }
}

pub fn validate_server_shape(conf_dir: &Path, mut conf: ServerShape) -> error::Result<ServerShape> {
    conf.storage = if let Some(mut storage) = conf.storage {
        storage.directory = if let Some(directory) = storage.directory {
            Some(validate_path_buf(conf_dir, "file storage (conf.storage.directory)", true, directory)?)
        } else {
            None
        };
        storage.temporary = if let Some(tmp_directory) = storage.temporary {
            Some(validate_path_buf(conf_dir, "temporary (conf.storage.temporary)", true, tmp_directory)?)
        } else {
            None
        };
        storage.web_static = if let Some(web_static) = storage.web_static {
            Some(validate_path_buf(conf_dir, "web static (conf.storage.web_static)", true, web_static)?)
        } else {
            None
        };
        storage.static_ = if let Some(mut static_) = storage.static_ {
            static_.directories = if let Some(directories) = static_.directories {
                let mut verified_map = HashMap::with_capacity(directories.len());

                for (mut key, value) in directories {
                    if !key.ends_with("/") {
                        key.write_char('/')?;
                    }

                    let name = format!("static directory map (conf.storage.static.directories.\"{}\"", key);

                    verified_map.insert(key, validate_path_buf(
                        conf_dir, name.as_str(), true, value
                    )?);
                }

                Some(verified_map)
            } else {
                None
            };
            static_.files = if let Some(files) = static_.files {
                let mut verified_map = HashMap::with_capacity(files.len());

                for (key, value) in files {
                    let name = format!("static file map (conf.storage.static.files.\"{}\"", key);

                    verified_map.insert(key, validate_path_buf(
                        conf_dir, name.as_str(), false, value
                    )?);
                }

                Some(verified_map)
            } else {
                None
            };

            Some(static_)
        } else {
            None
        };

        Some(storage)
    } else {
        None
    };

    conf.template = if let Some(mut template) = conf.template {
        template.directory = if let Some(directory) = template.directory {
            Some(validate_path_buf(conf_dir, "templates directory (conf.template.directory)", true, directory)?)
        } else {
            None
        };

        template.index_path = if let Some(index_path) = template.index_path {
            Some(validate_path_buf(conf_dir, "template index file (conf.template.index_path", false, index_path)?)
        } else {
            None
        };

        Some(template)
    } else {
        None
    };

    Ok(conf)
}