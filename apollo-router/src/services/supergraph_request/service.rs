use std::sync::Arc;
use std::task::Poll;

use futures::future::BoxFuture;

use crate::services::layers::query_analysis::QueryAnalysisLayer;
use crate::services::layers::{apq::APQLayer, persisted_queries::PersistedQueryLayer};

use crate::services::supergraph::{Request as SupergraphRequest, Response as SupergraphResponse};

#[derive(Clone)]
pub(crate) struct SupergraphRequestService {
    apq_layer: APQLayer,
    persisted_query_layer: Arc<PersistedQueryLayer>,
    query_analysis_layer: QueryAnalysisLayer,
}

impl SupergraphRequestService {
    pub(crate) fn new(
        apq_layer: APQLayer,
        persisted_query_layer: Arc<PersistedQueryLayer>,
        query_analysis_layer: QueryAnalysisLayer,
    ) -> Self {
        Self {
            apq_layer,
            persisted_query_layer,
            query_analysis_layer,
        }
    }
}

impl tower::Service<SupergraphRequest> for SupergraphRequestService {
    type Response = SupergraphRequest;
    type Error = SupergraphResponse;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: SupergraphRequest) -> Self::Future {
        let pq_layer = self.persisted_query_layer.clone();
        let apq_layer = self.apq_layer.clone();
        let qa_layer = self.query_analysis_layer.clone();

        Box::pin(async move {
            let mut request_res = pq_layer.supergraph_request(req);

            if let Ok(supergraph_request) = request_res {
                request_res = apq_layer.supergraph_request(supergraph_request).await;
            }

            match request_res {
                Err(response) => Err(response),
                Ok(request) => match qa_layer.supergraph_request(request).await {
                    Err(response) => Err(response),
                    Ok(request) => {
                        pq_layer
                            .supergraph_request_with_analyzed_query(request)
                            .await
                    }
                },
            }
        })
    }
}
