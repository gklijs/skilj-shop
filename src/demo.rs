//! Local talk setup, not a production identity provider. JWTs are genuinely
//! signed and validated by skilj; roles/capabilities are stored in Postgres.
use std::path::Path;

use axum::{body::Body, http::Request, routing::get, Json, Router};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::Utc;
use http_body_util::BodyExt;
use jsonwebtoken::{EncodingKey, Header};
use rsa::{pkcs1::EncodeRsaPrivateKey, traits::PublicKeyParts, RsaPrivateKey};
use serde_json::{json, Value};
use skilj::{IdpConfig, SigningAlgorithm, Skilj};
use skilj_core::{
    access_control::{AccessLevel, Role, RoleAccessMapping, RoleStatus},
    db::{self, Pool},
    shared::try_generate_token_id,
};
use tower::ServiceExt;

use crate::{shop, Error};

const ISSUER: &str = "https://skilj-shop.example.test";
const AUDIENCE: &str = "skilj-shop-talk";

pub const PLACE_ORDER: &str = r#"
mutation PlaceOrder($payload: String!) {
  submitCommand(boundedContext: "shop", commandTypeName: "PlaceOrder", payload: $payload) {
    accepted rejectionReason triggeredEventSequences
  }
}"#;

pub const SELLER_QUERY: &str = include_str!("../queries/seller.graphql");
pub const FINANCE_QUERY: &str = include_str!("../queries/finance.graphql");

pub struct Identity {
    pub role: Role,
    pub token: String,
}

pub struct Demo {
    pub skilj: Skilj,
    pub router: Router,
    pub seller: Identity,
    pub other_seller: Identity,
    pub finance: Identity,
    pub operator: Identity,
    pub pool: Pool,
    jwks_task: tokio::task::JoinHandle<()>,
}

impl Demo {
    pub async fn start(database_url: &str, key_file: &Path) -> Result<Self, Error> {
        let keys = LocalKeys::generate()?;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let jwks_url = format!("http://{}/jwks.json", listener.local_addr()?);
        let jwks = keys.jwks.clone();
        let jwks_router =
            Router::new().route("/jwks.json", get(move || async move { Json(jwks.clone()) }));
        let jwks_task = tokio::spawn(async move {
            if let Err(error) = axum::serve(listener, jwks_router).await {
                eprintln!("demo JWKS server failed: {error}");
            }
        });

        let admin =
            skilj::dev::seed_admin(database_url, "Demo operator", &[shop::BOUNDED_CONTEXT]).await?;
        let (skilj, report) = crate::register(Skilj::builder(database_url))
            .reconciliation_role(admin.external_subject())
            .identity_provider(IdpConfig::new(
                jwks_url.parse()?,
                ISSUER,
                AUDIENCE,
                SigningAlgorithm::Rs256,
            ))
            .key_provider(skilj::KeyFileProvider::open(key_file)?)
            .build()
            .await?;
        if !report.skipped_no_access.is_empty() {
            return Err(format!("registration denied: {:?}", report.skipped_no_access).into());
        }
        let pool =
            db::connect_with(database_url, db::PgPoolOptions::new().max_connections(2)).await?;
        let operator = Identity {
            role: admin.role().clone(),
            token: keys.sign(admin.external_subject())?,
        };
        // Access levels and capabilities are independent: these two roles
        // only read, with no administration or capability delegation rights.
        let seller =
            seed_identity(&pool, &keys, "Seller", Some("seller-a"), shop::FULFILLMENT).await?;
        let other_seller = seed_identity(
            &pool,
            &keys,
            "Other seller",
            Some("seller-b"),
            shop::FULFILLMENT,
        )
        .await?;
        let finance = seed_identity(&pool, &keys, "Finance", None, shop::FINANCE).await?;
        admin.close().await;

        // Deliberately mount only the GraphQL router; no REST routes or tokens.
        let router = skilj.graphql_router().await?;
        let demo = Self {
            skilj,
            router,
            seller,
            other_seller,
            finance,
            operator,
            pool,
            jwks_task,
        };
        for order in sample_orders() {
            let response = graphql(
                &demo.router,
                Some(&demo.operator.token),
                PLACE_ORDER,
                json!({"payload": serde_json::to_string(&order)?}),
            )
            .await?;
            let result = &response["data"]["submitCommand"];
            if response.get("errors").is_some()
                || (result["accepted"] != true
                    && result["rejectionReason"] != "order already exists")
            {
                return Err(format!("could not seed demo order: {response}").into());
            }
        }
        Ok(demo)
    }

    pub async fn shutdown(self) {
        self.skilj
            .shutdown(std::time::Duration::from_secs(10))
            .await;
        self.pool.close().await;
        self.jwks_task.abort();
    }
}

async fn seed_identity(
    pool: &Pool,
    keys: &LocalKeys,
    name: &str,
    scope: Option<&str>,
    capability: &str,
) -> Result<Identity, Error> {
    let id = try_generate_token_id()?;
    let role = Role {
        external_subject: format!("shop-demo-{id}"),
        id,
        name: name.into(),
        superadmin: false,
        status: RoleStatus::Active,
        created_at: Utc::now(),
        revoked_at: None,
    };
    db::insert_role(pool, &role).await?;
    let mapping = RoleAccessMapping {
        role: role.clone(),
        bounded_context: db::get_bounded_context(pool, shop::BOUNDED_CONTEXT)
            .await?
            .ok_or("shop context missing")?,
        level: AccessLevel::Read,
        can_read_sensitive: false,
        scope: scope.map(str::to_string),
        capabilities: [capability.to_string()].into_iter().collect(),
        delegable: Default::default(),
        status: RoleStatus::Active,
        created_at: Utc::now(),
        revoked_at: None,
    };
    db::insert_role_access_mapping(pool, &mapping).await?;
    Ok(Identity {
        token: keys.sign(&role.external_subject)?,
        role,
    })
}

/// Send an actual HTTP request through the same router used by the server.
/// GraphQL errors stay in the returned body so tests can assert denials.
pub async fn graphql(
    router: &Router,
    token: Option<&str>,
    query: &str,
    variables: Value,
) -> Result<Value, Error> {
    let mut request = Request::builder()
        .method("POST")
        .uri("/graphql")
        .header("content-type", "application/json");
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let response = router
        .clone()
        .oneshot(request.body(Body::from(
            json!({"query": query, "variables": variables}).to_string(),
        ))?)
        .await?;
    let status = response.status();
    let body = response.into_body().collect().await?.to_bytes();
    if !status.is_success() {
        return Err(format!("GraphQL HTTP {status}: {}", String::from_utf8_lossy(&body)).into());
    }
    Ok(serde_json::from_slice(&body)?)
}

pub fn sample_orders() -> Vec<shop::OrderPayload> {
    vec![
        shop::OrderPayload {
            order_id: "order-1001".into(),
            seller_id: "seller-a".into(),
            email: "alice@example.test".into(),
            price_paid_cents: 12995,
            currency: "EUR".into(),
            address: "Demo Street 12, 1234 AB Amsterdam".into(),
            description: "Mechanical keyboard, ISO layout".into(),
            quantity: 1,
        },
        shop::OrderPayload {
            order_id: "order-1002".into(),
            seller_id: "seller-b".into(),
            email: "bob@example.test".into(),
            price_paid_cents: 7450,
            currency: "EUR".into(),
            address: "Example Lane 8, 5678 CD Utrecht".into(),
            description: "USB-C docking station".into(),
            quantity: 2,
        },
    ]
}

struct LocalKeys {
    encoding: EncodingKey,
    jwks: Value,
}

impl LocalKeys {
    fn generate() -> Result<Self, Error> {
        let private = RsaPrivateKey::new(&mut rsa::rand_core::OsRng, 2048)?;
        let pem = private.to_pkcs1_pem(rsa::pkcs1::LineEnding::LF)?;
        let public = private.to_public_key();
        Ok(Self {
            encoding: EncodingKey::from_rsa_pem(pem.as_bytes())?,
            jwks: json!({"keys": [{"kty": "RSA", "use": "sig", "alg": "RS256", "kid": "demo",
                "n": URL_SAFE_NO_PAD.encode(public.n().to_bytes_be()), "e": URL_SAFE_NO_PAD.encode(public.e().to_bytes_be())}]}),
        })
    }
    fn sign(&self, subject: &str) -> Result<String, Error> {
        let mut header = Header::new(jsonwebtoken::Algorithm::RS256);
        header.kid = Some("demo".into());
        Ok(jsonwebtoken::encode(
            &header,
            &json!({"sub": subject, "iss": ISSUER, "aud": AUDIENCE,
            "exp": (Utc::now() + chrono::Duration::hours(8)).timestamp()}),
            &self.encoding,
        )?)
    }
}
