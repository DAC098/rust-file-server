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

pub fn get_directory_and_basename<T>(path: T, no_leading: bool) -> (String, String)
where
    T: AsRef<str>
{
    let mut directory = String::with_capacity(10);
    let mut working = String::with_capacity(10);
    let mut chars = path.as_ref().chars();

    if no_leading {
        if let Some(ch) = chars.next() {
            if ch != '/' {
                working.push(ch);
            }
        }
    }

    while let Some(ch) = chars.next() {
        if ch == '/' {
            let remaining = directory.capacity() - directory.len();

            if remaining < working.len() + 1 {
                directory.reserve(working.len() + 1);
            }

            directory.push('/');
            directory.push_str(&working);
            working.clear();
        } else {
            if working.capacity() == working.len() {
                working.reserve(10);
            }

            working.push(ch);
        }
    }

    directory.shrink_to_fit();
    working.shrink_to_fit();

    (directory, working)
}