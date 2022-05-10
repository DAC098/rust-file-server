use std::marker::{Sync};
use std::convert::{From, Into};

use tokio_postgres::types::{ToSql};

pub struct QueryParams<'a> {
    params: Vec<&'a (dyn ToSql + Sync)>
}

impl<'a> QueryParams<'a> {

    pub fn new() -> QueryParams<'a> {
        QueryParams {
            params: Vec::new()
        }
    }

    pub fn with_capacity(size: usize) -> QueryParams<'a> {
        QueryParams {
            params: Vec::with_capacity(size)
        }
    }
    
    pub fn push<T>(&mut self, param: &'a T) -> usize
    where
        T: ToSql + Sync
    {
        self.params.push(param);
        self.params.len()
    }

    pub fn next(&self) -> usize {
        self.params.len() + 1
    }

    pub fn slice(&self) -> &[&'a(dyn ToSql + Sync)] {
        &self.params[..]
    }
}

impl<'a> From<&'a QueryParams<'a>> for &[&'a(dyn ToSql + Sync)] {
    fn from(v: &'a QueryParams<'a>) -> Self {
        v.slice()
    }
}

impl<'a> From<Vec<&'a(dyn ToSql + Sync)>> for QueryParams<'a> {
    
    fn from(vec: Vec<&'a(dyn ToSql + Sync)>) -> QueryParams<'a> {
        QueryParams {params: vec}
    }
}

impl<'a> From<&[&'a(dyn ToSql + Sync)]> for QueryParams<'a> {

    fn from(slice: &[&'a(dyn ToSql + Sync)]) -> QueryParams<'a> {
        QueryParams {params: slice.to_vec()}
    }
    
}

impl<'a> Into<Vec<&'a(dyn ToSql + Sync)>> for QueryParams<'a> {

    fn into(self) -> Vec<&'a(dyn ToSql + Sync)> {
        self.params
    }
    
}
