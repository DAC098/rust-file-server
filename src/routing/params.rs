use std::collections::HashMap;

pub struct Params(HashMap<String, String>);

impl Params {

    pub fn with_capacity(size: usize) -> Params {
        Params(HashMap::with_capacity(size))
    }

    pub fn with<const N: usize>(list: [(String, String); N]) -> Params {
        Params (HashMap::from(list))
    }

    pub fn insert<K,V>(&mut self, key: K, value: V) -> Option<String>
    where
        K: Into<String>,
        V: Into<String>
    {
        self.0.insert(key.into(), value.into())
    }

    pub fn has_key<'a, K>(&self, key: K) -> bool
    where
        K: Into<&'a String>
    {
        self.0.contains_key(key.into())
    }

    pub fn get_value<'a, K>(&self, key: K) -> Option<String>
    where
        K: Into<&'a String>
    {
        self.0.get(key.into()).map(|v| v.clone())
    }

    pub fn get_value_ref<'a, K>(&self, key: K) -> Option<&String>
    where
        K: Into<&'a String>
    {
        self.0.get(key.into())
    }
}