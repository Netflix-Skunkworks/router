pub(crate) mod service;

use super::supergraph::{Request, Response};

pub type BoxService = tower::util::BoxService<Request, Request, Response>;
