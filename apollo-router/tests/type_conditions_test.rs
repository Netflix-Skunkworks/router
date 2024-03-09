//!
//! Please ensure that any tests added to this file use the tokio multi-threaded test executor.
//!

use apollo_compiler::execution::JsonMap;
use apollo_router::plugin::test::MockSubgraph;
use apollo_router::services::supergraph;
use apollo_router::MockedSubgraphs;
use apollo_router::TestHarness;
use serde_json::json;
use tower::ServiceExt;

mod integration;

#[tokio::test(flavor = "multi_thread")]
async fn test_type_conditions_enabled() {
    let harness = setup(json! {{
        "experimental_type_conditioned_fetching": true,
        // will make debugging easier
        "plugins": {
            "experimental.expose_query_plan": true
        }
    }});
    let supergraph_service = harness.build_supergraph().await.unwrap();
    let mut variables = JsonMap::new();
    variables.insert("movieResultParam", "movieResultEnabled".into());
    variables.insert("articleResultParam", "articleResultEnabled".into());
    let request = supergraph::Request::fake_builder()
        .query(QUERY.to_string())
        .header("Apollo-Expose-Query-Plan", "true")
        .variables(variables)
        .build()
        .expect("expecting valid request");

    let response = supergraph_service
        .oneshot(request)
        .await
        .unwrap()
        .next_response()
        .await
        .unwrap();

    insta::assert_json_snapshot!(response);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_type_conditions_disabled() {
    let harness = setup(json! {{
        "experimental_type_conditioned_fetching": false
    }});
    let supergraph_service = harness.build_supergraph().await.unwrap();
    let mut variables = JsonMap::new();
    variables.insert("movieResultParam", "movieResultDisabled".into());
    variables.insert("articleResultParam", "articleResultDisabled".into());
    let request = supergraph::Request::fake_builder()
        .query(QUERY.to_string())
        .build()
        .expect("expecting valid request");

    let response = supergraph_service
        .oneshot(request)
        .await
        .unwrap()
        .next_response()
        .await
        .unwrap();

    insta::assert_json_snapshot!(response);
}

fn setup(configuration: serde_json::Value) -> TestHarness<'static> {
    let search_service =  MockSubgraph::builder().with_json(json!{{
        "query":"query Search__searchSubgraph__0{search{__typename ...on MovieResult{sections{__typename ...on EntityCollectionSection{__typename id}...on GallerySection{__typename id}}id}...on ArticleResult{id sections{__typename ...on GallerySection{__typename id}...on EntityCollectionSection{__typename id}}}}}",
        "operationName":"Search__searchSubgraph__0"
    }},
json!{{
        "data":{
            "search":[
                {
                    "__typename":"ArticleResult",
                    "id":"a7052397-b605-414a-aba4-408d51c8eef0",
                    "sections":[
                        {
                            "__typename":"EntityCollectionSection",
                            "id":"d0182b8a-a671-4244-ba1c-905274b0d198"
                        },
                        {
                            "__typename":"EntityCollectionSection","id":"e6eec2fc-05ce-40a2-956b-f1335e615204"
                        }
                    ]
                },
                {
                    "__typename":"ArticleResult",
                    "id":"3a7b08c9-d8c0-4c55-b55d-596a272392e0",
                    "sections":[
                        {
                            "__typename":"EntityCollectionSection",
                            "id":"f44f584e-5d3d-4466-96f5-9afc3f5d5a54"
                        },
                        {
                            "__typename":"GallerySection",
                            "id":"e065e2b1-8454-4db9-89c8-48e66ec838c4"
                        }
                    ]
                }
            ]
        }
    }}).build();

    let artwork_service = MockSubgraph::builder()
    // Enabled has 2 queries: first one...
    .with_json(json!{{
        "query":"query Search__artworkSubgraph__1($representations:[_Any!]!$movieResultParam:String){_entities(representations:$representations){...on EntityCollectionSection{title artwork(params:$movieResultParam)}...on GallerySection{artwork(params:$movieResultParam)}}}",
        "operationName":"Search__artworkSubgraph__1",
        "variables":{
            "movieResultParam":"movieResultEnabled",
            "representations":[
                {
                    "__typename":"EntityCollectionSection",
                    "id":"d0182b8a-a671-4244-ba1c-905274b0d198"
                },
                {
                    "__typename":"EntityCollectionSection",
                    "id":"e6eec2fc-05ce-40a2-956b-f1335e615204"
                },
                {
                    "__typename":"EntityCollectionSection",
                    "id":"f44f584e-5d3d-4466-96f5-9afc3f5d5a54"
                },
                {
                    "__typename":"GallerySection",
                    "id":"e065e2b1-8454-4db9-89c8-48e66ec838c4"
                }
            ]
        }
    }},
json!{{
    "data":{
        "_entities":[
            {"artwork":"movieResultEnabled artwork"},
            {"artwork":"movieResultEnabled artwork"}
        ]
    }
    }})
    // ... and second one!
    .with_json(json!{{
        "query": "query Search__artworkSubgraph__2($representations:[_Any!]!$articleResultParam:String){_entities(representations:$representations){...on GallerySection{artwork(params:$articleResultParam)}...on EntityCollectionSection{artwork(params:$articleResultParam)}}}","operationName":"Search__artworkSubgraph__2",
        "variables":{
            "articleResultParam":"articleResultEnabled",
            "representations":[
                {
                    "__typename":"EntityCollectionSection",
                    "id":"d0182b8a-a671-4244-ba1c-905274b0d198"
                },
                {
                    "__typename":"EntityCollectionSection",
                    "id":"e6eec2fc-05ce-40a2-956b-f1335e615204"
                },
                {
                    "__typename":"EntityCollectionSection",
                    "id":"f44f584e-5d3d-4466-96f5-9afc3f5d5a54"
                },
                {
                    "__typename":"GallerySection",
                    "id":"e065e2b1-8454-4db9-89c8-48e66ec838c4"
                }
            ]
        }
        }},
        json!{{
            "data":{
                "_entities":[
                    {"artwork":"articleResultEnabled artwork"},
                    {"artwork":"articleResultEnabled artwork"}
                ]
            }
        }}
    )
    // Disabled, not great
    .with_json(json!{{
            "query":"query Search__artworkSubgraph__1($representations:[_Any!]!$movieResultParam:String){_entities(representations:$representations){...on EntityCollectionSection{title artwork(params:$movieResultParam)}...on GallerySection{artwork(params:$movieResultParam)}}}",
            "operationName":"Search__artworkSubgraph__1",
            "variables":{
                "representations":[
                    {
                        "__typename":"EntityCollectionSection",
                        "id":"d0182b8a-a671-4244-ba1c-905274b0d198"
                    },
                    {
                        "__typename":"EntityCollectionSection",
                        "id":"e6eec2fc-05ce-40a2-956b-f1335e615204"
                    },
                    {
                        "__typename":"EntityCollectionSection",
                        "id":"f44f584e-5d3d-4466-96f5-9afc3f5d5a54"
                    },
                    {
                        "__typename":"GallerySection",
                        "id":"e065e2b1-8454-4db9-89c8-48e66ec838c4"
                    }
                ]
            }
        }},
    json!{
        {"data":{
            "_entities":[
                {
                    "artwork":"Hello World"
                },
                {
                    "artwork":"Hello World"
                }
            ]
        }
    }}).build();

    let mut mocks = MockedSubgraphs::default();
    mocks.insert("searchSubgraph", search_service);
    mocks.insert("artworkSubgraph", artwork_service);

    let schema = include_str!("fixtures/type_conditions.graphql");
    TestHarness::builder()
        .try_log_level("info")
        .configuration_json(configuration)
        .unwrap()
        .schema(schema)
        .extra_plugin(mocks)
}

static QUERY: &str = r#"
query Search($movieResultParam: String, $articleResultParam: String) {
    search {
      ... on MovieResult {
        sections {
          ... on EntityCollectionSection {
            id
            title
            artwork(params: $movieResultParam)
          }
          ... on GallerySection {
            artwork(params: $movieResultParam)
            id
          }
        }
        id
      }
      ... on ArticleResult {
        id
        sections {
          ... on GallerySection {
            artwork(params: $articleResultParam)
          }
          ... on EntityCollectionSection {
            artwork(params: $articleResultParam)
          }
        }
      }
    }
}"#;
