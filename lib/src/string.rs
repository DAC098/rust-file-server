use std::path::PathBuf;

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

pub fn get_directory_and_basename<T>(path: T) -> (String, String)
where
    T: AsRef<str>
{
    let mut directory = String::new();
    let mut working = String::new();

    for ch in path.as_ref().chars() {
        if ch == '/' {
            directory.push('/');
            directory.push_str(working.as_str());
            working.clear();
        } else {
            working.push(ch);
        }
    }

    working.shrink_to_fit();

    (directory, working)
}