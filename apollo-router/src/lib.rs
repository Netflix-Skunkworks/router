//! Components of a federated GraphQL Server.
//!
//! Most of these modules are of varying interest to different audiences.
//!
//! If your interests are confined to developing plugins, then the following modules
//! are likely to be of most interest to you:
//!
//! * [`self`] - this module (apollo_router) contains high level building blocks for a federated GraphQL router
//!
//! * [`graphql`] - graphql specific functionality for requests, responses, errors
//!
//! * [`layers`] - examples of tower layers used to implement plugins
//!
//! * [`plugin`] - various APIs for implementing a plugin
//!
//! * [`services`] - the various services handling a GraphQL requests,
//!   and APIs for plugins to intercept them

#![cfg_attr(feature = "failfast", allow(unreachable_code))]
#![warn(unreachable_pub)]
#![warn(missing_docs)]

macro_rules! failfast_debug {
    ($($tokens:tt)+) => {{
        tracing::debug!($($tokens)+);
        #[cfg(feature = "failfast")]
        panic!(
            "failfast triggered. \
            Please remove the feature failfast if you don't want to see these panics"
        );
    }};
}

macro_rules! failfast_error {
    ($($tokens:tt)+) => {{
        tracing::error!($($tokens)+);
        #[cfg(feature = "failfast")]
        panic!(
            "failfast triggered. \
            Please remove the feature failfast if you don't want to see these panics"
        );
    }};
}

#[macro_use]
mod json_ext;
#[macro_use]
pub mod plugin;

#[macro_use]
pub(crate) mod metrics;

pub(crate) mod axum_factory;
mod cache;
mod configuration;
mod context;
mod error;
mod executable;
mod files;
pub mod graphql;
mod http_ext;
mod http_server_factory;
mod introspection;
pub mod layers;
pub(crate) mod notification;
mod orbiter;
mod plugins;
pub(crate) mod protocols;
mod query_planner;
mod request;
mod response;
mod router;
mod router_factory;
pub mod services;
pub(crate) mod spec;
mod state_machine;
pub mod test_harness;
pub mod tracer;
mod uplink;

#[cfg(feature = "apollo_unsupported")]
pub use axum_factory::set_add_extra_endpoints_layer;
#[cfg(feature = "apollo_unsupported")]
pub use axum_factory::set_add_main_endpoint_layer;
#[cfg(feature = "apollo_unsupported")]
pub use error::set_into_graphql_errors;
#[cfg(feature = "apollo_unsupported")]
pub use error::set_to_graphql_error;
#[cfg(feature = "apollo_unsupported")]
pub use error::CacheResolverError;
#[cfg(feature = "apollo_unsupported")]
pub use error::FetchError;
#[cfg(feature = "apollo_unsupported")]
pub use error::PlannerErrors;
#[cfg(feature = "apollo_unsupported")]
pub use error::QueryPlannerError;
#[cfg(feature = "apollo_unsupported")]
pub use error::RouterError;
#[cfg(feature = "apollo_unsupported")]
pub use json_ext::Path;
#[cfg(feature = "apollo_unsupported")]
pub use spec::operation_limits::OperationLimits;

pub use crate::configuration::Configuration;
pub use crate::configuration::ListenAddr;
pub use crate::context::Context;
pub use crate::executable::main;
pub use crate::executable::Executable;
pub use crate::notification::{HandleStream, Notify, NotifyError};
pub use crate::router::ApolloRouterError;
pub use crate::router::ConfigurationSource;
pub use crate::router::LicenseSource;
pub use crate::router::RouterHttpServer;
pub use crate::router::SchemaSource;
pub use crate::router::ShutdownSource;
pub use crate::router_factory::Endpoint;
pub use crate::test_harness::MockedSubgraphs;
pub use crate::test_harness::TestHarness;
pub use crate::uplink::UplinkConfig;

/// Not part of the public API
#[doc(hidden)]
pub mod _private {
    // Reexports for macros
    pub use linkme;
    pub use once_cell;
    pub use router_bridge;
    pub use serde_json;

    pub use crate::plugin::PluginFactory;
    pub use crate::plugin::PLUGINS;
    // For tests
    pub use crate::router_factory::create_test_service_factory_from_yaml;
}
