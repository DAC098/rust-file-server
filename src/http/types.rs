pub type Request = hyper::Request<hyper::Body>;
pub type Response = hyper::Response<hyper::Body>;
pub type RequestTuple = (hyper::http::request::Parts, hyper::Body);