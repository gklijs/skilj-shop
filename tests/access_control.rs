//! Real Postgres + real signed JWTs + the demo's HTTP router.
//! This test fails if Postgres cannot start; authorization checks never skip.
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use chrono::Utc;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use skilj::local::{LocalConfig, LocalPostgres};
use skilj_core::{access_control::AccessLevel, db};
use skilj_shop::{
    demo::{graphql, sample_orders, Demo, FINANCE_QUERY, PLACE_ORDER, SELLER_QUERY},
    Error,
};
use tower::ServiceExt;

fn success(response: &Value) {
    assert!(
        response.get("errors").is_none(),
        "unexpected GraphQL error: {response}"
    );
}

fn denied(response: &Value) {
    assert!(
        !response["errors"]
            .as_array()
            .expect("expected GraphQL errors")
            .is_empty(),
        "unexpected success: {response}"
    );
}

async fn assert_storefront_serves_catalog_without_private_data(demo: &Demo) -> Result<(), Error> {
    let orders = sample_orders();
    for (path, content_type) in [
        ("/", "text/html; charset=utf-8"),
        ("/walkthrough", "text/html; charset=utf-8"),
        ("/assets/storefront.css", "text/css; charset=utf-8"),
        ("/assets/keyboard.webp", "image/webp"),
        ("/assets/dock.webp", "image/webp"),
        ("/assets/mat.webp", "image/webp"),
    ] {
        let response = demo
            .router
            .clone()
            .oneshot(Request::get(path).body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        assert_eq!(response.headers()[header::CONTENT_TYPE], content_type);
        let body = response.into_body().collect().await?.to_bytes();
        assert!(!body.is_empty(), "{path} returned an empty body");
        if content_type == "image/webp" {
            assert!(body.starts_with(b"RIFF") && body.get(8..12) == Some(b"WEBP"));
        }

        let body = String::from_utf8_lossy(&body);
        if path == "/" {
            for expected in [
                "Mechanical Keyboard",
                "USB-C Dock",
                "Felt Desk Mat",
                "€139.00",
                "€89.00",
                "€34.00",
                r#"href="/walkthrough""#,
                r#"href="/assets/storefront.css""#,
                r#"src="/assets/keyboard.webp""#,
                r#"src="/assets/dock.webp""#,
                r#"src="/assets/mat.webp""#,
            ] {
                assert!(body.contains(expected), "{path} is missing {expected}");
            }
        }
        if path == "/walkthrough" {
            assert!(body.contains(
                r#"href="https://github.com/gklijs/skilj-shop/blob/main/README.md#set-the-demo-tokens""#
            ));
        }

        for identity in [
            &demo.seller,
            &demo.other_seller,
            &demo.finance,
            &demo.operator,
        ] {
            assert!(!body.contains(&identity.token), "{path} leaked a JWT");
        }
        for order in &orders {
            for value in [
                &order.order_id,
                &order.seller_id,
                &order.email,
                &order.address,
            ] {
                assert!(!body.contains(value), "{path} leaked private order data");
            }
            let cents = order.price_paid_cents;
            assert!(
                !body.contains(&cents.to_string()),
                "{path} leaked a payment amount"
            );
            let formatted = format!("€{}.{:02}", cents / 100, cents % 100);
            assert!(!body.contains(&formatted), "{path} leaked a payment amount");
        }
    }

    Ok(())
}

#[tokio::test]
async fn roles_cannot_cross_payment_fulfillment_or_seller_boundaries() -> Result<(), Error> {
    let root = std::env::temp_dir().join(format!(
        "skilj-shop-test-{}",
        skilj_core::shared::try_generate_token_id()?
    ));
    let postgres = LocalPostgres::start(LocalConfig {
        data_dir: root.join("postgres"),
        database: "shop_test".into(),
        cleanup: true,
        port: None,
    })
    .await?;
    let demo = Demo::start(postgres.database_url(), &root.join("keys.json")).await?;
    assert_storefront_serves_catalog_without_private_data(&demo).await?;
    let variables = json!({"order": "order-1001"});

    let seller = graphql(
        &demo.router,
        Some(&demo.seller.token),
        SELLER_QUERY,
        variables.clone(),
    )
    .await?;
    success(&seller);
    assert_eq!(
        seller["data"]["projection"],
        json!({"orderId": "order-1001",
        "description": "Mechanical keyboard, ISO layout", "quantity": 1,
        "address": "Demo Street 12, 1234 AB Amsterdam"})
    );
    let finance = graphql(
        &demo.router,
        Some(&demo.finance.token),
        FINANCE_QUERY,
        variables.clone(),
    )
    .await?;
    success(&finance);
    assert_eq!(
        finance["data"]["projection"],
        json!({"orderId": "order-1001",
        "email": "alice@example.test", "pricePaidCents": 12995, "currency": "EUR"})
    );
    let finance_other_order = graphql(
        &demo.router,
        Some(&demo.finance.token),
        FINANCE_QUERY,
        json!({"order": "order-1002"}),
    )
    .await?;
    success(&finance_other_order);
    assert_eq!(
        finance_other_order["data"]["projection"]["orderId"],
        "order-1002"
    );

    // Cross-role projections, aliases, and nonexistent fields all fail.
    for (token, query) in [
        (&demo.seller.token, FINANCE_QUERY.to_string()),
        (&demo.finance.token, SELLER_QUERY.to_string()),
        (
            &demo.seller.token,
            SELLER_QUERY.replace("quantity", "quantity stolenEmail: email"),
        ),
        (
            &demo.seller.token,
            SELLER_QUERY.replace("quantity", "quantity pricePaidCents"),
        ),
        (
            &demo.finance.token,
            FINANCE_QUERY.replace("currency", "currency address description"),
        ),
    ] {
        denied(&graphql(&demo.router, Some(token), &query, variables.clone()).await?);
    }
    // Even a fragment-free request cannot bypass the projection's gate.
    for (token, name) in [
        (&demo.seller.token, "FinanceOrder"),
        (&demo.finance.token, "FulfillmentOrder"),
    ] {
        let query = format!(
            r#"{{ projection(boundedContext: "shop", name: "{name}", key: "order-1001") {{ __typename }} }}"#
        );
        denied(&graphql(&demo.router, Some(token), &query, json!({})).await?);
    }

    // Owner scope still applies when the order ID is known.
    denied(
        &graphql(
            &demo.router,
            Some(&demo.seller.token),
            SELLER_QUERY,
            json!({"order": "order-1002"}),
        )
        .await?,
    );
    denied(
        &graphql(
            &demo.router,
            Some(&demo.other_seller.token),
            SELLER_QUERY,
            variables.clone(),
        )
        .await?,
    );
    let other_seller = graphql(
        &demo.router,
        Some(&demo.other_seller.token),
        SELLER_QUERY,
        json!({"order": "order-1002"}),
    )
    .await?;
    success(&other_seller);
    assert_eq!(other_seller["data"]["projection"]["orderId"], "order-1002");

    // Introspection itself hides the other group's projection and fields.
    let schema_query = include_str!("../queries/introspection.graphql");
    for (token, visible, hidden, forbidden) in [
        (
            &demo.seller.token,
            "shop_FulfillmentOrder",
            "shop_FinanceOrder",
            vec!["email", "pricePaidCents"],
        ),
        (
            &demo.finance.token,
            "shop_FinanceOrder",
            "shop_FulfillmentOrder",
            vec!["address", "description", "quantity"],
        ),
    ] {
        let schema = graphql(&demo.router, Some(token), schema_query, json!({})).await?;
        success(&schema);
        let types = schema["data"]["__schema"]["types"].as_array().unwrap();
        let visible = types
            .iter()
            .find(|ty| ty["name"] == visible)
            .expect("own projection missing");
        assert!(!types.iter().any(|ty| ty["name"] == hidden));
        for field in forbidden {
            assert!(!visible["fields"]
                .as_array()
                .unwrap()
                .iter()
                .any(|f| f["name"] == field));
        }
    }

    // Neither group can submit commands or widen its capabilities.
    let payload = json!({"payload": serde_json::to_string(&sample_orders()[0])?});
    for identity in [&demo.seller, &demo.finance] {
        denied(
            &graphql(
                &demo.router,
                Some(&identity.token),
                PLACE_ORDER,
                payload.clone(),
            )
            .await?,
        );
        let query = format!(
            r#"mutation {{ grantCapability(roleId: "{}", boundedContext: "shop", capability: "finance") {{ capabilities }} }}"#,
            identity.role.id
        );
        denied(&graphql(&demo.router, Some(&identity.token), &query, json!({})).await?);
        for query in [
            r#"{ queryEvents(boundedContext: "shop", eventTypes: ["OrderPlaced"]) { payload } }"#,
            r#"{ fetchCommands(boundedContext: "shop", commandTypes: ["PlaceOrder"]) { payload } }"#,
        ] {
            denied(&graphql(&demo.router, Some(&identity.token), query, json!({})).await?);
        }
    }
    denied(&graphql(&demo.router, None, SELLER_QUERY, variables.clone()).await?);
    denied(
        &graphql(
            &demo.router,
            Some("finance"),
            FINANCE_QUERY,
            variables.clone(),
        )
        .await?,
    );

    // Elevate only the test identities' access levels. Even admin access
    // must not bypass field capabilities on either raw payload surface.
    for identity in [&demo.seller, &demo.finance] {
        let mut mapping = db::get_active_role_access_mapping(&demo.pool, &identity.role.id, "shop")
            .await?
            .unwrap();
        db::revoke_active_role_access_mapping(&demo.pool, &identity.role.id, "shop", Utc::now())
            .await?;
        mapping.level = AccessLevel::Admin;
        mapping.created_at = Utc::now();
        db::insert_role_access_mapping(&demo.pool, &mapping).await?;
    }
    for (token, allowed, blocked) in [
        (
            &demo.seller.token,
            vec!["address", "description", "quantity"],
            vec!["email", "price_paid_cents", "currency"],
        ),
        (
            &demo.finance.token,
            vec!["email", "price_paid_cents", "currency"],
            vec!["address", "description", "quantity"],
        ),
        (
            &demo.operator.token,
            vec![],
            vec![
                "email",
                "price_paid_cents",
                "currency",
                "address",
                "description",
                "quantity",
            ],
        ),
    ] {
        for (field, query) in [
            (
                "queryEvents",
                r#"{ queryEvents(boundedContext: "shop", eventTypes: ["OrderPlaced"]) { payload } }"#,
            ),
            (
                "fetchCommands",
                r#"{ fetchCommands(boundedContext: "shop", commandTypes: ["PlaceOrder"]) { payload } }"#,
            ),
        ] {
            let response = graphql(&demo.router, Some(token), query, json!({})).await?;
            success(&response);
            let records = response["data"][field].as_array().unwrap();
            assert!(
                !records.is_empty(),
                "raw payload test must exercise stored data"
            );
            for record in records {
                let payload: Value = serde_json::from_str(record["payload"].as_str().unwrap())?;
                for field in &allowed {
                    assert!(
                        !payload[field].is_null(),
                        "missing allowed {field}: {payload}"
                    );
                }
                for field in &blocked {
                    assert!(payload[field].is_null(), "leaked {field}: {payload}");
                }
                if token == &demo.seller.token {
                    assert_eq!(payload["seller_id"], "seller-a");
                }
            }
        }
    }
    let response = demo
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/commands/trigger")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), 404, "REST must not be mounted");

    // A capability revoke must take effect on the very next request.
    assert!(
        db::remove_mapping_capability(&demo.pool, &demo.seller.role.id, "shop", "fulfillment")
            .await?
    );
    denied(
        &graphql(
            &demo.router,
            Some(&demo.seller.token),
            SELLER_QUERY,
            variables,
        )
        .await?,
    );

    demo.shutdown().await;
    postgres.shutdown().await?;
    std::fs::remove_dir_all(root)?;
    Ok(())
}
