use std::convert::TryFrom;

use serde::Serialize;
use tokio_postgres::GenericClient;

use crate::http::error::Result;

#[derive(Debug)]
pub struct InvalidAlgorithm(i16);

#[repr(i16)]
#[derive(Debug, Clone)]
pub enum TotpAlgorithm {
    SHA1 = 1,
    SHA256 = 2,
    SHA512 = 3,
}

impl TryFrom<i16> for TotpAlgorithm {
    type Error = InvalidAlgorithm;

    fn try_from(v: i16) -> std::result::Result<Self, Self::Error> {
        match v {
            1 => Ok(Self::SHA1),
            2 => Ok(Self::SHA256),
            3 => Ok(Self::SHA512),
            _ => Err(InvalidAlgorithm(v))
        }
    }
}

#[derive(Debug, Serialize)]
pub struct User {
    pub id: i64,
    pub username: String,

    #[serde(skip_serializing)]
    pub hash: String,

    pub email: Option<String>,
    pub email_verified: bool,
    pub totp_enabled: bool,

    #[serde(skip_serializing)]
    pub totp_algorithm: Option<TotpAlgorithm>,
    #[serde(skip_serializing)]
    pub totp_secret: Option<String>,
    #[serde(skip_serializing)]
    pub totp_step: Option<u16>,
    #[serde(skip_serializing)]
    pub totp_digits: Option<u16>
}

impl User {

    pub async fn find_username(conn: &impl GenericClient, username: &String) -> Result<Option<User>> {
        if let Some(record) = conn.query_opt(
            "\
            select id, \
                   hash, \
                   email, \
                   email_verified, \
                   totp_enabled, \
                   totp_algorithm, \
                   totp_secret, \
                   totp_step, \
                   totp_digits \
            from users \
            where username = $1",
            &[username]
        ).await? {
            let totp_algorithm = record.get::<usize, Option<i16>>(5)
                .map(|v| TotpAlgorithm::try_from(v).unwrap());
            let totp_step = record.get::<usize, Option<i16>>(7)
                .map(|v| u16::try_from(v).unwrap());
            let totp_digits = record.get::<usize, Option<i16>>(8)
                .map(|v| u16::try_from(v).unwrap());

            Ok(Some(User {
                id: record.get(0),
                username: username.clone(),
                hash: record.get(1),
                email: record.get(2),
                email_verified: record.get(3),
                totp_enabled: record.get(4),
                totp_algorithm,
                totp_secret: record.get(6),
                totp_step,
                totp_digits,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn find_id(conn: &impl GenericClient, id: &i64) -> Result<Option<User>> {
        if let Some(record) = conn.query_opt(
            "\
            select username, \
                   hash, \
                   email, \
                   email_verified, \
                   totp_enabled, \
                   totp_algorithm, \
                   totp_secret, \
                   totp_step, \
                   totp_digits \
            from users \
            where id = $1",
            &[id]
        ).await? {
            let totp_algorithm = record.get::<usize, Option<i16>>(5)
                .map(|v| TotpAlgorithm::try_from(v).unwrap());
            let totp_step = record.get::<usize, Option<i16>>(7)
                .map(|v| u16::try_from(v).unwrap());
            let totp_digits = record.get::<usize, Option<i16>>(8)
                .map(|v| u16::try_from(v).unwrap());

            Ok(Some(User {
                id: id.clone(),
                username: record.get(0),
                hash: record.get(1),
                email: record.get(2),
                email_verified: record.get(3),
                totp_enabled: record.get(4),
                totp_algorithm,
                totp_secret: record.get(6),
                totp_step,
                totp_digits,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn find_username_or_optional_email(conn: &impl GenericClient, username: &String, email: &Option<String>) -> Result<Vec<User>> {
        Ok(conn.query(
            "\
            select id, \
                   username, \
                   hash, \
                   email, \
                   email_verified, \
                   totp_enabled, \
                   totp_algorithm, \
                   totp_secret, \
                   totp_step, \
                   totp_digits \
            from users \
            where username = $1 or \
                  email = $2",
            &[username, email]
        ).await?.into_iter().map(|record| {
            let totp_algorithm = record.get::<usize, Option<i16>>(5)
                .map(|v| TotpAlgorithm::try_from(v).unwrap());
            let totp_step = record.get::<usize, Option<i16>>(8)
                .map(|v| u16::try_from(v).unwrap());
            let totp_digits = record.get::<usize, Option<i16>>(9)
                .map(|v| u16::try_from(v).unwrap());

            User {
                id: record.get(0),
                username: record.get(1),
                hash: record.get(2),
                email: record.get(3),
                email_verified: record.get(4),
                totp_enabled: record.get(5),
                totp_algorithm,
                totp_secret: record.get(7),
                totp_step,
                totp_digits
            }
        }).collect())
    }
}