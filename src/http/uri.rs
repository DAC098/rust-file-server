use std::collections::HashMap;

use hyper::Uri;

const EMPTY_QUERY: &str = "";

pub fn query_iter(uri: &Uri) -> impl Iterator<Item=(&str,Option<&str>)> {
    uri.query()
        .unwrap_or(EMPTY_QUERY)
        .split("&")
        .filter(|v| !v.is_empty())
        .map(|v| {
            if let Some(pair) = v.split_once("=") {
                (pair.0, Some(pair.1))
            } else {
                (v, None)
            }
        })
}

pub struct QueryMap(
    HashMap<String, Vec<Option<String>>>
);

impl QueryMap {

    pub fn new(uri: &Uri) -> QueryMap {
        let mut map: HashMap<String, Vec<Option<String>>> = HashMap::new();

        for (key, value) in query_iter(uri) {
            let key_owned = key.to_owned();

            if let Some(v) = map.get_mut(&key_owned) {
                v.push(value.map(|s| s.to_owned()));
            } else {
                map.insert(key_owned, vec!(value.map(|s| s.to_owned())));
            }
        }

        QueryMap(map)
    }

    pub fn has_key<K>(&self, key: K) -> bool
    where
        K: Into<String>
    {
        self.0.contains_key(&key.into())
    }

    pub fn get_value<K>(&self, key: K) -> Option<Option<String>>
    where
        K: Into<String>
    {
        if let Some(list) = self.0.get(&key.into()) {
            Some(list.first()
                .unwrap()
                .as_ref()
                .map(|v| v.clone()))
        } else {
            None
        }
    }

    pub fn get_value_ref<K>(&self, key: K) -> Option<&Option<String>>
    where
        K: Into<String>
    {
        if let Some(list) = self.0.get(&key.into()) {
            list.first()
        } else {
            None
        }
    }

    // pub fn get_all<K>(&self, key: K) -> Option<&Vec<Option<String>>>
    // where
    //     K: Into<String>
    // {
    //     self.0.get(&key.into())
    // }
}