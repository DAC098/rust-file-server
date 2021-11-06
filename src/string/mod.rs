use std::path::{PathBuf};

pub fn name_from_pathbuf(path: &PathBuf) -> Option<String> {
    if let Some(name) = path.file_name() {
        if let Ok(rtn) = name.to_os_string().into_string() {
            Some(rtn)
        } else {
            None
        }
    } else {
        None
    }
}