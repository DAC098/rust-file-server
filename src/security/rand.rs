use ring::rand::{SystemRandom, SecureRandom};

pub fn rand_bytes(size: usize) -> Option<Vec<u8>> {
    let mut rtn: Vec<u8> = vec!(0; size);

    if let Ok(()) = SystemRandom::new().fill(rtn.as_mut_slice()) {
        Some(rtn)
    } else {
        None
    }
}