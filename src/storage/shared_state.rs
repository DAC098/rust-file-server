use std::collections::HashMap;
use std::sync::Arc;
use std::path::PathBuf;

use crate::config::StorageConfig;

pub struct StaticResources {
    pub directories: HashMap<String, PathBuf>,
    pub files: HashMap<String, PathBuf>
}

pub struct StorageState {
    pub directory: PathBuf,
    pub temporary: PathBuf,
    pub web_static: Option<PathBuf>,
    
    pub static_resources: StaticResources
}

pub type ArcStorageState = Arc<StorageState>;

impl StorageState {

    pub fn get_tmp_file(&self, ext: &str) -> Option<PathBuf> {
        let mut count: u64 = 0;
        let mut tmp_file = self.temporary.clone();
        let now = chrono::Utc::now().timestamp().to_string();

        loop {
            let file_name = format!("{}_{}", now, count);
            tmp_file.set_file_name(file_name);
            tmp_file.set_extension(ext);

            if !tmp_file.exists() {
                return Some(tmp_file)
            } else {
                if count == u64::MAX {
                    return None;
                } else {
                    count += 1;
                }
            }
        }
    }
}

impl From<StorageConfig> for ArcStorageState {
    fn from(storage: StorageConfig) -> ArcStorageState {
        Arc::new(StorageState {
            directory: storage.directory,
            temporary: storage.temporary,
            web_static: storage.web_static,
            static_resources: StaticResources {
                directories: storage.static_.directories,
                files: storage.static_.files
            }
        })
    }
}