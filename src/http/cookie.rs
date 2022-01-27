use std::{collections::HashMap, convert::TryFrom};

use chrono::{DateTime, Utc, Duration};
use hyper::{HeaderMap, header::{COOKIE, HeaderValue, InvalidHeaderValue}};

pub fn get_cookie_map(headers: &HeaderMap) -> HashMap<String, Vec<String>> {
    let mut rtn: HashMap<String, Vec<String>> = HashMap::new();

    for cookies in headers.get_all(COOKIE) {
        let to_str = cookies.to_str();

        if to_str.is_err() {
            continue;
        }

        for value in to_str.unwrap().split("; ") {
            if let Some((k, v)) = value.split_once('=') {
                let k_string = k.to_owned();

                if let Some(list) = rtn.get_mut(&k_string) {
                    list.push(v.to_owned());
                } else {
                    rtn.insert(k_string, vec![v.to_owned()]);
                }
            }
        }
    }

    rtn
}

pub enum SameSite {
    Strict,
    Lax,
    None
}

impl SameSite {
    pub fn as_str(&self) -> &str {
        match self {
            SameSite::Strict => "Strict",
            SameSite::Lax => "Lax",
            SameSite::None => "None"
        }
    }
}

pub struct SetCookie {
    pub key: String,
    pub value: String,

    pub expires: Option<DateTime<Utc>>,
    pub max_age: Option<Duration>,
    pub domain: Option<String>,
    pub path: Option<String>,
    pub secure: bool,
    pub http_only: bool,
    pub same_site: Option<SameSite>
}

impl SetCookie {
    pub fn new(key: String, value: String) -> SetCookie {
        SetCookie {
            key, value,
            expires: None,
            max_age: None,
            domain: None,
            path: None,
            secure: false,
            http_only: false,
            same_site: None
        }
    }

    pub fn into_header_value(self) -> std::result::Result<HeaderValue, InvalidHeaderValue> {
        let mut rtn = format!("{}={}", self.key, self.value);

        if let Some(expire) = self.expires {
            let date = expire.format("%a, %d %b %Y %H:%M:%S GMT").to_string();
            rtn.push_str("; Expires=");
            rtn.push_str(date.as_str());
        }

        if let Some(duration) = self.max_age {
            let seconds = duration.num_seconds().to_string();
            rtn.push_str("; Max-Age=");
            rtn.push_str(seconds.as_str());
        }

        if let Some(domain) = self.domain {
            rtn.push_str("; Domain=");
            rtn.push_str(domain.as_str());
        }

        if let Some(path) = self.path {
            rtn.push_str("; Path=");
            rtn.push_str(path.as_str());
        }

        if self.secure {
            rtn.push_str("; Secure");
        }

        if self.http_only {
            rtn.push_str("; HttpOnly");
        }

        if let Some(same_site) = self.same_site {
            rtn.push_str("; SameSite=");
            rtn.push_str(same_site.as_str());
        }

        HeaderValue::from_str(rtn.as_str())
    }
}

impl TryFrom<SetCookie> for HeaderValue {
    type Error = InvalidHeaderValue;

    fn try_from(value: SetCookie) -> Result<Self, Self::Error> {
        value.into_header_value()
    }
}