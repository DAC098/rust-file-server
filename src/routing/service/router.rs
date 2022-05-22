use futures::future::BoxFuture;
use tower::{Service, util::BoxCloneService};

use crate::routing::params::Params;

pub trait RouterExt {
    fn get_path(&self) -> String;

    fn add_params(&mut self, params: Params) -> ();
}

impl<B> RouterExt for hyper::Request<B>
where
    B: hyper::body::HttpBody
{
    fn get_path(&self) -> String {
        self.uri().path().into()
    }

    fn add_params(&mut self, params: Params) -> () {
        self.extensions_mut().insert(params);
    }
}

#[derive(Clone)]
pub struct Router<Req, Res, Err> {
    router: matchit::Router<BoxCloneService<Req, Res, Err>>,
    no_match: BoxCloneService<Req, Res, Err>
}

impl<Req, Res, Err> Router<Req, Res, Err> {

    pub fn new(
        no_match: BoxCloneService<Req, Res, Err>
    ) -> Router<Req, Res, Err> {
        Router { 
            router: matchit::Router::new(),
            no_match
        }
    }

    pub fn insert<R, S>(
        &mut self, 
        route: R, 
        value: S,
    ) -> std::result::Result<(), matchit::InsertError>
    where
        R: Into<String>,
        S: Service<Req, Response = Res, Error = Err> + Clone + Send + 'static,
        S::Future: Send + 'static,
    {
        self.router.insert(route, BoxCloneService::new(value))
    }
}

impl<Req, Res, Err> Service<Req> for Router<Req, Res, Err>
where
    Req: RouterExt
{
    type Response = Res;
    type Error = Err;
    type Future = BoxFuture<'static, Result<Res, Err>>;

    fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, mut req: Req) -> Self::Future {
        let path = req.get_path();
        let result = self.router.at_mut(path.as_str());

        if let Ok(matches) = result {
            let mut map = Params::with_capacity(matches.params.len());

            for (key, value) in matches.params.iter() {
                map.insert(key.to_owned(), value.to_owned());
            }

            req.add_params(map);

            matches.value.call(req)
        } else {
            self.no_match.call(req)
        }
    }
}