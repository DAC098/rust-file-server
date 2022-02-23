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

pub fn make_pad_string(len: usize, pad_char: char) -> String {
    let mut rtn = String::with_capacity(len * pad_char.len_utf8());

    for _ in 0..len {
        rtn.push(pad_char);
    }

    rtn
}

pub fn get_directory_and_basename<T>(path: T, mut no_leading: bool) -> (String, String)
where
    T: AsRef<str>
{
    let mut directory = String::new();
    let mut working = String::new();

    for ch in path.as_ref().chars() {
        if ch == '/' {
            if no_leading {
                no_leading = false;
            } else {
                directory.push('/');
            }

            directory.push_str(working.as_str());
            working.clear();
        } else {
            working.push(ch);
        }
    }

    working.shrink_to_fit();

    (directory, working)
}