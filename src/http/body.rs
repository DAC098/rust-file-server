use std::path::Path;

use futures::StreamExt;
use hyper::{Body, body::Buf};
use serde::de::DeserializeOwned;
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;

use crate::http::{
    error::Result,
    error::Error
};

pub async fn json_from_body<T>(mut body: Body) -> Result<T>
where
    T: DeserializeOwned
{
    let mut data = Vec::new();

    while let Some(chunk) = body.next().await {
        let bytes = chunk?;

        data.extend_from_slice(&bytes);
    }

    match serde_json::from_slice::<T>(&data) {
        Ok(value) => Ok(value),
        Err(err) => {
            match err.classify() {
                serde_json::error::Category::Syntax => {
                    Err(Error::new(400, "InvalidJsonBody", "given invalid json body"))
                },
                serde_json::error::Category::Data => {
                    Err(Error::new(400, "InvalidJson", "given json does not meet the requirements"))
                },
                _ => {
                    Err(Error::new_source(
                        500, 
                        "ErrorParsingJson", 
                        "server error when parsing json",
                        err
                    ))
                }
            }
        }
    }
}

pub async fn file_from_body<T>(path: T, open: bool, mut body: Body) -> Result<(File, usize)>
where
    T: AsRef<Path> 
{
    let mut written = 0;
    let mut file = OpenOptions::new()
        .write(true)
        .create(!open)
        .open(path)
        .await?;

    while let Some(chunk) = body.next().await {
        let mut bytes = chunk?;
        written += bytes.len();

        while bytes.has_remaining() {
            file.write_buf(&mut bytes).await?;
        }
    }

    Ok((file, written))
}