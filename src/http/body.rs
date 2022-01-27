use std::path::Path;

use futures::StreamExt;
use hyper::Body;
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
    let mut data = String::new();

    while let Some(chunk) = body.next().await {
        let bytes = chunk?;

        if let Ok(str_chunk) = std::str::from_utf8(&bytes) {
            data.push_str(str_chunk);
        } else {
            return Err(Error {
                status: 400,
                name: "NonUTF8Body".into(),
                msg: "given body contains invalid utf-8 characters".into(),
                source: None
            })
        }
    }

    let rtn = match serde_json::from_str::<T>(data.as_str()) {
        Ok(value) => Ok(value),
        Err(err) => {
            match err.classify() {
                serde_json::error::Category::Syntax => {
                    Err(Error {
                        status: 400,
                        name: "InvalidJsonBody".into(),
                        msg: "given invalid json body".into(),
                        source: None
                    })
                },
                serde_json::error::Category::Data => {
                    Err(Error {
                        status: 400,
                        name: "InvalidJson".into(),
                        msg: "given json does not meet the requirements".into(),
                        source: None
                    })
                },
                _ => {
                    Err(Error {
                        status: 500,
                        name: "ErrorParsingJson".into(),
                        msg: "server error when parsing json".into(),
                        source: None
                    })
                }
            }
        }
    };

    rtn
}

pub async fn file_from_body<T>(path: T, open: bool, mut body: Body) -> Result<File>
where
    T: AsRef<Path> 
{
    let mut options = OpenOptions::new();
    options.write(true);
    options.create(!open);
    let mut file = options.open(path).await?;

    while let Some(chunk) = body.next().await {
        let bytes = chunk?;
        file.write(&bytes).await?;
    }

    Ok(file)
}