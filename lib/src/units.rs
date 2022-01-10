pub const VALUE_LIST: [u64; 7] = [
    1u64,
    1024u64,
    1048576u64,
    1073741824u64,
    1099511627776u64,
    1125899906842624u64,
    1152921504606846976u64
];

pub const PREFIX_LIST: [&str; 7] = [
    "B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB"
];

/*
pub enum FileSize {
    BYTE = 0,
    KIBIBYTE = 1,
    MEBIBYTE = 2,
    GIBIBYTE = 3,
    TEBIBYTE = 4,
    PEBIBYTE = 5,
    EXBIBYTE = 6
}

#[inline]
pub fn get_file_unit_int(unit: FileSize) -> u64 {
    VALUE_LIST[unit as usize]
}

#[inline]
pub fn get_file_unit_prefix(unit: FileSize) -> &'static str {
    PREFIX_LIST[unit as usize]
}

pub fn bytes_to_given_unit(size: u64, unit: FileSize) -> String {
    let index = unit as usize;
    format!("{} {}", size / VALUE_LIST[index], PREFIX_LIST[index])
}
*/
pub fn bytes_to_unit(size: u64) -> String {
    let mut index = 0;

    while index < 6 {
        if size / VALUE_LIST[index + 1] == 0 {
            break;
        } else {
            index += 1;
        }
    }

    format!("{} {}", size / VALUE_LIST[index], PREFIX_LIST[index])
}