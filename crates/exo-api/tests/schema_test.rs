use async_graphql::{Request, Variables};
use exo_api::create_schema;

#[tokio::test]
async fn test_schema_health_query() {
    let schema = create_schema();
    let query = "{ health }";
    let res = schema.execute(query).await;
    assert_eq!(
        res.data.into_json().unwrap(),
        serde_json::json!({ "health": "OK" })
    );
}

#[tokio::test]
async fn test_event_query_stub() {
    let schema = create_schema();
    let query = r#"
        query {
            event(id: "1234") {
                id
                author
            }
        }
    "#;
    let res = schema.execute(query).await;
    // Should be null for now
    let json = res.data.into_json().unwrap();
    assert_eq!(json, serde_json::json!({ "event": null }));
}
