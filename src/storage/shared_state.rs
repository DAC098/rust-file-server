use std::sync::{Arc};
use std::path::{PathBuf, Path};

pub struct StorageState {
    directory: PathBuf
}

pub type ArcStorageState = Arc<StorageState>;

impl StorageState {

    pub fn get_dir(&self) -> PathBuf {
        self.directory.clone()
    }
    
    pub fn get_dir_ref(&self) -> &PathBuf {
        &self.directory
    }

    pub fn get_dir_with<P>(&self, path: P) -> PathBuf
    where
        P: AsRef<Path> 
    {
        let mut rtn = self.directory.clone();
        rtn.push(path);
        rtn
    }
}

pub fn build_shared_state(directory: PathBuf) -> ArcStorageState {
    Arc::new(StorageState {
        directory
    })
}