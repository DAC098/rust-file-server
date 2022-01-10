use argon2::{Variant, Version, ThreadMode, Config};

use crate::security::error;
use crate::security::rand::rand_bytes;

pub fn default_argon2_config() -> Config<'static> {
    Config {
        variant: Variant::Argon2i,
        version: Version::Version13,
        mem_cost: 65536,
        time_cost: 10,
        lanes: 4,
        thread_mode: ThreadMode::Parallel,
        secret: &[],
        ad: &[],
        hash_length: 32
    }
}

/*
pub fn hash_with_config(
    password: &String,
    config: &Config
) -> error::Result<String> {
    if let Some(bytes) = rand_bytes(64) {
        Ok(argon2::hash_encoded(password.as_bytes(), bytes.as_slice(), &config)?)
    } else {
        Err(error::Error::General)
    }
}
*/

pub fn hash_with_default(
    password: &String
) -> error::Result<String> {
    if let Some(bytes) = rand_bytes(64) {
        let default = default_argon2_config();
        Ok(argon2::hash_encoded(password.as_bytes(), bytes.as_slice(), &default)?)
    } else {
        Err(error::Error::General)
    }
}