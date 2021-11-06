use std::sync::{Arc};
use std::path::{Path};

pub struct StorageState {
    pub directory: Path
}

pub type ArcStorageState = Arc<StorageState>;