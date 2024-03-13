use std::num::NonZeroUsize;
use std::sync::Arc;

use futures::future::BoxFuture;
use router_bridge::planner::Planner;
use tokio::task::JoinSet;
use tower::balance::p2c::Balance;
use tower::buffer::Buffer;
use tower::discover::ServiceList;
use tower::load::CompleteOnResponse;
use tower::load::PendingRequestsDiscover;
use tower::ServiceExt;

use super::bridge_query_planner::BridgeQueryPlanner;
use super::QueryPlanResult;
use crate::error::ServiceBuildError;
use crate::services::QueryPlannerRequest;
use crate::services::QueryPlannerResponse;
use crate::spec::Schema;
use crate::Configuration;
use crate::QueryPlannerError;

type ServiceType = Buffer<
    Balance<PendingRequestsDiscover<ServiceList<Vec<BridgeQueryPlanner>>>, QueryPlannerRequest>,
    QueryPlannerRequest,
>;

#[derive(Clone)]
pub(crate) struct BridgeQueryPlannerPool {
    planners: Vec<Arc<Planner<QueryPlanResult>>>,
    service: ServiceType,
    schema: Arc<Schema>
}

impl BridgeQueryPlannerPool {
    pub(crate) async fn new(
        sdl: String,
        configuration: Arc<Configuration>,
        size: NonZeroUsize,
    ) -> Result<Self, ServiceBuildError> {
        let mut join_set = JoinSet::new();

        (0..size.into()).for_each(|_| {
            let sdl = sdl.clone();
            let configuration = configuration.clone();

            join_set.spawn(async move { BridgeQueryPlanner::new(sdl, configuration).await });
        });

        let mut services = Vec::new();
        let mut planners = Vec::new();

        while let Some(task_result) = join_set.join_next().await {
            // TODO: Error Type
            let new_result = task_result.map_err(|_e| {
                ServiceBuildError::QueryPlannerError(
                    crate::QueryPlannerError::UnhandledPlannerResult,
                )
            })?;

            let service = new_result?;

            planners.push(service.planner());
            services.push(service);
        }

        let schema = services
            .first()
            .expect("There should be at least 1 service in pool")
            .schema();

        let discover = ServiceList::new(services);
        let discover = PendingRequestsDiscover::new(discover, CompleteOnResponse::default());
        let balance_service = Balance::new(discover);

        // TODO: Bounds? Using 10_000 just like the bridge query planner
        let service = Buffer::new(balance_service, 10_000);

        Ok(Self { planners, service, schema })
    }

    pub(crate) async fn new_from_planners(
        old_planners: Vec<Arc<Planner<QueryPlanResult>>>,
        schema: String,
        configuration: Arc<Configuration>,
    ) -> Result<Self, ServiceBuildError> {
        let mut join_set = JoinSet::new();

        old_planners.into_iter().for_each(|old_planner| {
            let sdl = schema.clone();
            let configuration = configuration.clone();

            join_set.spawn(async move {
                BridgeQueryPlanner::new_from_planner(old_planner, sdl, configuration).await
            });
        });

        let mut services = Vec::new();
        let mut planners = Vec::new();

        while let Some(task_result) = join_set.join_next().await {
            // TODO: Error Type
            let new_result = task_result.map_err(|_e| {
                ServiceBuildError::QueryPlannerError(
                    crate::QueryPlannerError::UnhandledPlannerResult,
                )
            })?;

            let service = new_result?;

            planners.push(service.planner());
            services.push(service);
        }

        let schema = services
            .first()
            .expect("There should be at least 1 service in pool")
            .schema();

        let discover = ServiceList::new(services);
        let discover = PendingRequestsDiscover::new(discover, CompleteOnResponse::default());
        let balance_service = Balance::new(discover);

        // TODO: Bounds? Using 10_000 just like the bridge query planner
        let service = Buffer::new(balance_service, 10_000);

        Ok(Self { planners, service, schema })
    }

    pub(crate) fn planners(&self) -> Vec<Arc<Planner<QueryPlanResult>>> {
        self.planners.clone()
    }

    pub(crate) fn schema(&self) -> Arc<Schema> {
        self.schema.clone()
    }
}

impl tower::Service<QueryPlannerRequest> for BridgeQueryPlannerPool {
    type Response = QueryPlannerResponse;

    type Error = QueryPlannerError;

    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: QueryPlannerRequest) -> Self::Future {
        let clone = self.service.clone();
        let service = std::mem::replace(&mut self.service, clone);

        Box::pin(async move {
            let res = service.oneshot(req).await;

            res.map_err(|e| match e.downcast::<Self::Error>() {
                Ok(e) => *e,
                Err(_) => QueryPlannerError::UnhandledPlannerResult,
            })
        })
    }
}
