use std::fs::{read_dir};
use std::ffi::{OsString};
use std::fmt::{Write};
use std::path::{PathBuf};

use serde::{Deserialize, Serialize};

use hyper::{Request, Response, Body, Result};
use hyper::header::{HeaderValue};

use crate::storage::shared_state::{ArcStorageState};
use crate::string::{name_from_pathbuf};

#[derive(Serialize)]
pub enum DirItem {
    Directory {
        name: String
    },
    File {
        name: String
    },
    Unknown {}
}

pub async fn handle_get(
    req: Request<Body>
) -> Result<Response<Body>> {
    let storage = req.extensions().get::<ArcStorageState>().unwrap();
    let (_, root_check) = req.uri().path().split_at(3);

    if root_check == "" {
        println!("redirecting empty root");

        return Ok(Response::builder()
            .status(302)
            .header("Location", "/fs/")
            .body(Body::empty())
            .unwrap())
    }

    let (_, req_fs_path) = root_check.split_at(1);
    let fs_path = storage.get_dir_with(req_fs_path);

    println!("file path: \"{}\"", fs_path.display());

    if fs_path.exists() {
        if fs_path.is_dir() {
            let mut item_list: Vec<DirItem> = Vec::new();
            
            match read_dir(fs_path) {
                Ok(iter) => {
                    let (_min, max_opt) = iter.size_hint();

                    if let Some(max) = max_opt {
                        item_list.reserve(max);
                    }

                    for entry in iter {
                        if let Ok(entry) = entry {
                            let entry_path = entry.path();

                            if entry_path.is_dir() {
                                if let Some(name) = name_from_pathbuf(&entry_path) {
                                    item_list.push(DirItem::Directory {name})
                                } else {
                                    item_list.push(DirItem::Unknown {});
                                }
                            } else if entry_path.is_file() {
                                if let Some(name) = name_from_pathbuf(&entry_path) {
                                    item_list.push(DirItem::File {name})
                                } else {
                                    item_list.push(DirItem::Unknown {})
                                }
                            } else {
                                item_list.push(DirItem::Unknown {})
                            }
                        }
                    }

                    handle_get_dir_response(req, item_list).await
                },
                Err(e) => {
                    Ok(Response::builder()
                        .status(500)
                        .header("content-type", "text/plain")
                        .body("error when reading directory".into())
                        .unwrap())
                }
            }
        } else if fs_path.is_file() {

            Ok(Response::builder()
                .status(200)
                .header("content-type", "text/plain")
                .body("okay".into())
                .unwrap())
        } else {
            Ok(Response::builder()
                .status(200)
                .header("content-type", "text/plain")
                .body("okay".into())
                .unwrap())
        }
    } else {
        Ok(Response::builder()
            .status(404)
            .header("content-type", "text/plain")
            .body("path not found".into())
            .unwrap())
    }
}

async fn handle_get_dir_response(req: Request<Body>, item_list: Vec<DirItem>) -> Result<Response<Body>> {
    let pretty = true;
    let headers = req.headers();
    let accepting: mime::Mime = {
        if let Some(accept) = headers.get("accept") {
            accept.to_str().unwrap_or("text/plain").parse().unwrap()
        } else {
            "text/plain".parse().unwrap()
        }
    };

    if accepting.type_() == "text" || accepting.type_() == "*" {
        if accepting.subtype() == "plain" || accepting.subtype() == "*" {
            let mut first = true;
            let sep = if pretty { "\n" } else { "," };
            let mut rtn = String::new();

            for item in item_list {
                match item {
                    DirItem::Directory {name} => {
                        rtn.write_str(name.as_str()).unwrap();

                        if first {
                            first = false;
                        } else {
                            rtn.write_str(sep).unwrap();
                        }
                    },
                    DirItem::File {name} => {
                        rtn.write_str(name.as_str()).unwrap();

                        if first {
                            first = false;
                        } else {
                            rtn.write_str(sep).unwrap();
                        }
                    },
                    DirItem::Unknown {} => {}
                }
            }

            return Ok(Response::builder()
                .status(200)
                .header("content-type", "text/plain")
                .body(rtn.into())
                .unwrap())
        }
    } else if accepting.type_() == "application" {
        if accepting.subtype() == "json" {
            if let Ok(json_value) = serde_json::to_value(item_list) {
                let to_string = if pretty { 
                    serde_json::to_string_pretty(&json_value) 
                } else {
                    serde_json::to_string(&json_value)
                };

                if let Ok(body) = to_string {
                    return Ok(Response::builder()
                        .status(200)
                        .header("content-type", "application/json")
                        .body(body.into())
                        .unwrap())
                } else {
                    return Ok(Response::builder()
                        .status(200)
                        .header("content-type", "text/plain")
                        .body("json encoding error".into())
                        .unwrap())
                }
            } else {
                return Ok(Response::builder()
                    .status(200)
                    .header("content-type", "text/plain")
                    .body("json serialization error".into())
                    .unwrap())
            }
        }
    }

    Ok(Response::builder()
        .status(400)
        .header("content-type", "text/plain")
        .body("unknown accept header value".into())
        .unwrap())
}

async fn handle_get_file_response(req: Request<Body>, path: PathBuf) -> Result<Response<Body>> {
    Ok(Response::builder()
        .status(200)
        .header("content-type", "text/plain")
        .body("okay".into())
        .unwrap())
}