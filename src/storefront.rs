use axum::{http::header::CONTENT_TYPE, response::Html, routing::get, Router};

pub(crate) fn router() -> Router {
    Router::new()
        .route(
            "/",
            get(|| async { Html(include_str!("../storefront/index.html")) }),
        )
        .route(
            "/walkthrough",
            get(|| async { Html(include_str!("../storefront/walkthrough.html")) }),
        )
        .route(
            "/assets/storefront.css",
            get(|| async {
                (
                    [(CONTENT_TYPE, "text/css; charset=utf-8")],
                    include_str!("../storefront/storefront.css"),
                )
            }),
        )
        .route(
            "/assets/keyboard.webp",
            get(|| async {
                (
                    [(CONTENT_TYPE, "image/webp")],
                    include_bytes!("../storefront/keyboard.webp").as_slice(),
                )
            }),
        )
        .route(
            "/assets/dock.webp",
            get(|| async {
                (
                    [(CONTENT_TYPE, "image/webp")],
                    include_bytes!("../storefront/dock.webp").as_slice(),
                )
            }),
        )
        .route(
            "/assets/mat.webp",
            get(|| async {
                (
                    [(CONTENT_TYPE, "image/webp")],
                    include_bytes!("../storefront/mat.webp").as_slice(),
                )
            }),
        )
}
