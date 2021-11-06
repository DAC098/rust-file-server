use serde::{Deserialize};

use hyper::{Request, Response, Body, Result};

pub async fn handle_get(
    req: Request<Body>
) -> Result<Response<Body>> {
    let (_, fs_path) = req.uri().path().split_at(3);

    if fs_path == "" {
        println!("redirecting empty root");

        Ok(Response::builder()
            .status(302)
            .header("Location", "/fs/")
            .body(Body::empty())
            .unwrap())
    } else {
        println!("file path: \"{}\"", fs_path);

        Ok(Response::builder()
            .status(200)
            .header("content-type", "text/plain")
            .body("okay".into())
            .unwrap())
    }
}