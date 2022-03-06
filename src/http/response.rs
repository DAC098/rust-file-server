use std::convert::TryFrom;

use chrono::{Utc, DateTime};
use hyper::Body;
use hyper::header::{HeaderValue, HeaderName};
use hyper::{Response as HyperResponse, http::response::Builder};
use hyper::StatusCode;
use serde::Serialize;
use serde_json::json;

use super::error;
use super::types::{Response, Request};
use super::mime::get_accepting_default;

#[inline]
pub fn build() -> Builder {
    HyperResponse::builder()
}

pub fn okay_text_response() -> error::Result<Response> {
    build()
        .status(200)
        .header("content-type", "text/plain")
        .body("okay".into())
        .map_err(Into::into)
}

pub fn okay_html_response() -> error::Result<Response> {
    build()
        .status(200)
        .header("content-type", "text/html")
        .body("<!DOCTYPE html>\
        <html>\
            <head>\
                <title>okay</title>\
            </head>\
            <body>okay</body>\
        </html>".into())
        .map_err(Into::into)
}

pub struct JsonResponseBuilder {
    builder: Builder,
    message: String,
    error: Option<String>,
    timestamp: DateTime<Utc>
}

impl JsonResponseBuilder {

    pub fn new<S>(status: S) -> JsonResponseBuilder
    where
        StatusCode: TryFrom<S>,
        <StatusCode as TryFrom<S>>::Error: Into<hyper::http::Error>,
    {
        JsonResponseBuilder {
            builder: build().status(status),
            message: "successful".into(),
            error: None,
            timestamp: Utc::now()
        }
    }

    pub fn set_message<M>(mut self, message: M) -> JsonResponseBuilder
    where
        M: Into<String>
    {
        self.message = message.into();
        self
    }

    pub fn set_error<E>(mut self, error: E) -> JsonResponseBuilder
    where
        E: Into<String>
    {
        self.error = Some(error.into());
        self
    }

    // pub fn set_timestamp(mut self, timestamp: DateTime<Utc>) -> JsonResponseBuilder {
    //     self.timestamp = timestamp;
    //     self
    // }

    pub fn add_header<H, V>(mut self, header: H, value: V) -> JsonResponseBuilder
    where
        // not really happy about this, could probably done better.
        // this pulled straight from the
        HeaderName: TryFrom<H>,
        <HeaderName as TryFrom<H>>::Error: Into<hyper::http::Error>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<hyper::http::Error>,
    {
        self.builder = self.builder.header(header, value);
        self
    }

    pub fn payload_response<T>(self, payload: T) -> error::Result<Response>
    where
        T: Serialize
    {
        let mut json = json!({
            "message": self.message,
            "timestamp": self.timestamp.timestamp(),
            "payload": payload
        });

        if let Some(err) = self.error {
            json["error"] = err.into();
        }

        self.builder.header("content-type", "application/json")
            .body(serde_json::to_vec(&json)?.into())
            .map_err(Into::into)
    }

    pub fn response(self) -> error::Result<Response> {
        let mut json = json!({
            "message": self.message,
            "timestamp": self.timestamp.timestamp()
        });

        if let Some(err) = self.error {
            json["error"] = err.into();
        }

        self.builder.header("content-type", "application/json")
            .body(serde_json::to_vec(&json)?.into())
            .map_err(Into::into)
    }
}

pub fn okay_response(req: Request) -> error::Result<Response> {
    let accept = get_accepting_default(req.headers(), "text/plain")?;

    for mime in accept {
        if mime.type_() == "text" || mime.type_() == "*" {
            if mime.subtype() == "plain" || mime.subtype() == "*" {
                return okay_text_response();
            } else if mime.subtype() == "html" {
                return okay_html_response();
            }
        } else if mime.type_() == "application" {
            if mime.subtype() == "json" {
                return JsonResponseBuilder::new(200)
                    .response()
            }
        }
    }

    okay_text_response()
}

pub fn redirect_response<P>(new_path: P) -> error::Result<Response>
where
    HeaderValue: TryFrom<P>,
    <HeaderValue as TryFrom<P>>::Error: Into<hyper::http::Error>,
{
    build()
        .status(302)
        .header("location", new_path)
        .body(Body::empty())
        .map_err(Into::into)
}