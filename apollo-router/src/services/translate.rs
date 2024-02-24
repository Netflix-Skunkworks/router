use std::task::Poll;
use futures::future::BoxFuture;
use tower::Service;

use super::supergraph::Request;
use super::supergraph::Response;

pub type BoxService = tower::util::BoxService<Request, Request, Response>;
pub type BoxCloneService = tower::util::BoxCloneService<Request, Request, Response>;
pub type ServiceResult = Result<Request, Response>;

pub struct TranslateService;

impl Service<Request> for TranslateService {
    type Response = Request;
    type Error = Response;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        Box::pin(async { Ok(req) })
    }
}
