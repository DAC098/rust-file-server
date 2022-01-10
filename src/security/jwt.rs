use chrono::Duration;

use crate::security::error;

pub struct Claim {
    // expiration timestamp
    pub exp: u64,
    // issued at timestamp
    pub iat: u64,
    // issuer
    pub iss: String,
    // audience
    //pub aud: String,
    // subject
    pub sub: String
}

impl Claim {
    pub fn new(iss: String, sub: String) -> error::Result<Claim> {
        let now = chrono::Utc::now();
        let iat = now.timestamp() as u64;
        let exp = {
            if let Some(check) = now.checked_add_signed(Duration::days(3)) {
                check.timestamp() as u64
            } else {
                return Err(error::Error::TimestampOverflow)
            }
        };

        Ok(Claim { exp, iat, iss , sub })
    }
}