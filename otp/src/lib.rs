use ring::hmac::{self, Algorithm, Tag};

pub const DEFAULT_STEP: u64 = 30;
pub const DEFAULT_DIGITS: u32 = 8;

fn hmac_one_off(algorithm: Algorithm, secret: &[u8], data: &[u8]) -> Tag {
    let key = hmac::Key::new(algorithm, secret);
    let mut context = hmac::Context::with_key(&key);
    context.update(&data);
    context.sign()
}

fn pad_string(uint_string: String, digits: usize) -> String {
    if uint_string.len() < digits {
        let mut rtn = String::with_capacity(digits);

        for _ in 0..(digits - uint_string.len()) {
            rtn.push('0');
        }

        rtn.push_str(&uint_string);
        rtn
    } else {
        uint_string
    }
}

fn generate_integer_string(algorithm: Algorithm, secret: &[u8], digits: u32, data: &[u8]) -> String {
    let hash = hmac_one_off(algorithm, secret, data);

    let hash_ref = hash.as_ref();
    let offset = (hash_ref[hash_ref.len() - 1] & 0xf) as usize;
    let binary = 
        ((hash_ref[offset] & 0x7f) as u64) << 24 |
        (hash_ref[offset + 1] as u64) << 16 |
        (hash_ref[offset + 2] as u64) <<  8 |
        (hash_ref[offset + 3] as u64);

    let uint_string = (binary % 10u64.pow(digits)).to_string();
    let digits = digits as usize;

    pad_string(uint_string, digits)
}

pub fn htop(secret: &[u8], digits: u32, counter: u64) -> String {
    let counter_bytes = counter.to_be_bytes();
    
    generate_integer_string(hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY, secret, digits, &counter_bytes)
}

pub fn totp(algorithm: Algorithm, secret: &[u8], digits: u32, step: u64, time: u64) -> String {
    let data = (time / step).to_be_bytes();

    generate_integer_string(algorithm, secret, digits, &data)
}

#[cfg(test)]
mod tests {
    use ring::hmac;

    use crate::{htop, totp, DEFAULT_DIGITS, DEFAULT_STEP};

    #[test]
    fn htop_test() {
        let secret = b"12345678901234567890";
        let results = vec![
            "755224",
            "287082",
            "359152",
            "969429",
            "338314",
            "254676",
            "287922",
            "162583",
            "399871",
            "520489",
        ];

        for count in 0..results.len() {
            let check = htop(secret, 6, count as u64);

            assert_eq!(
                check.as_str(), 
                results[count], 
                "count: {} received: {} expected: {}", 
                count, 
                check, 
                results[count]
            );
        }
    }

    #[test]
    fn totp_sha1_test() {
        let secret = b"12345678901234567890";

        let pairs = vec![
            ("94287082", 59),
            ("07081804", 1111111109),
            ("14050471", 1111111111),
            ("89005924", 1234567890),
            ("69279037", 2000000000),
            ("65353130", 20000000000),
        ];

        for (expected, time) in pairs {
            let check = totp(hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY, secret, DEFAULT_DIGITS, DEFAULT_STEP, time);

            assert_eq!(
                check.as_str(),
                expected,
                "time: {} check: {} expected: {}",
                time,
                check,
                expected
            );
        }
    }

    #[test]
    fn totp_sha256_test() {
        let secret = b"12345678901234567890123456789012";

        let pairs = vec![
            ("46119246", 59),
            ("68084774", 1111111109),
            ("67062674", 1111111111),
            ("91819424", 1234567890),
            ("90698825", 2000000000),
            ("77737706", 20000000000),
        ];

        for (expected, time) in pairs {
            let check = totp(hmac::HMAC_SHA256, secret, DEFAULT_DIGITS, DEFAULT_STEP, time);

            assert_eq!(
                check.as_str(),
                expected,
                "time: {} check: {} expected: {}",
                time,
                check,
                expected
            );
        }
    }

    #[test]
    fn totp_sha512_test() {
        let secret = b"1234567890123456789012345678901234567890123456789012345678901234";

        let pairs = vec![
            ("90693936", 59),
            ("25091201", 1111111109),
            ("99943326", 1111111111),
            ("93441116", 1234567890),
            ("38618901", 2000000000),
            ("47863826", 20000000000),
        ];

        for (expected, time) in pairs {
            let check = totp(hmac::HMAC_SHA512, secret, DEFAULT_DIGITS, DEFAULT_STEP, time);

            assert_eq!(
                check.as_str(), 
                expected,
                "time: {} check: {} expected: {}",
                time,
                check,
                expected
            );
        }
    }
}
