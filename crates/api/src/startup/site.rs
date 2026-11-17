use axum::Router;
use axum::http::header;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;

const PAGE: &str = include_str!("../../../../apps/web/index.html");
const SCRIPT: &str = include_str!("../../../../apps/web/client.mjs");
const STYLE: &str = include_str!("../../../../apps/web/style.css");

pub(crate) fn plate() -> Router {
    Router::new()
        .route("/", get(|| async { Redirect::to("/login") }))
        .route(
            "/login",
            get(|| async { reply(PAGE, "text/html; charset=utf-8") }),
        )
        .route(
            "/logout",
            get(|| async { reply(PAGE, "text/html; charset=utf-8") }),
        )
        .route(
            "/assets/client.mjs",
            get(|| async { reply(SCRIPT, "text/javascript; charset=utf-8") }),
        )
        .route(
            "/assets/style.css",
            get(|| async { reply(STYLE, "text/css; charset=utf-8") }),
        )
}

fn reply(body: &'static str, kind: &'static str) -> Response {
    (
        [
            (header::CONTENT_TYPE, kind),
            (header::CACHE_CONTROL, "no-store"),
            (header::CONTENT_SECURITY_POLICY, "default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            (header::REFERRER_POLICY, "no-referrer"),
        ],
        body,
    ).into_response()
}
