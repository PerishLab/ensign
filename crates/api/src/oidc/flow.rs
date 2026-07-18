use crate::oidc::plain::{bearer, pct, sour};
use crate::oidc::{Ask, Grant, Oidc};
use axum::Extension;
use axum::Json;
use axum::extract::{Form, OriginalUri, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use keel::{Operator, Wire};
use serde_json::{Value, json};

pub(crate) async fn disco<W: Wire>(State(oidc): State<Oidc<W>>) -> Json<Value> {
    let iss = &oidc.iss;
    Json(json!({
        "issuer": iss,
        "authorization_endpoint": format!("{iss}/authorize"),
        "token_endpoint": format!("{iss}/token"),
        "userinfo_endpoint": format!("{iss}/userinfo"),
        "jwks_uri": format!("{iss}/.well-known/jwks.json"),
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code", "refresh_token"],
        "subject_types_supported": ["public"],
        "id_token_signing_alg_values_supported": ["ES256"],
        "scopes_supported": ["openid", "profile"],
        "code_challenge_methods_supported": ["S256"],
        "token_endpoint_auth_methods_supported": ["none"],
    }))
}

pub(crate) async fn jwks<W: Wire>(State(oidc): State<Oidc<W>>) -> Json<Value> {
    Json(json!({ "keys": [oidc.keys.jwk] }))
}

pub(crate) async fn authorize<W: Wire + 'static>(
    State(oidc): State<Oidc<W>>,
    OriginalUri(uri): OriginalUri,
    Query(ask): Query<Ask>,
    op: Option<Extension<Operator>>,
) -> Response {
    let Some(Extension(Operator(actor))) = op else {
        let seek = uri
            .path_and_query()
            .map(|part| part.as_str())
            .unwrap_or("/authorize");
        let back = format!("/login?return={}", pct(seek));
        return Redirect::to(&back).into_response();
    };
    oidc.open(ask, actor).await
}

pub(crate) async fn token<W: Wire + 'static>(
    State(oidc): State<Oidc<W>>,
    Form(grant): Form<Grant>,
) -> Response {
    let grip = match grant.kind.as_str() {
        "authorization_code" => oidc.trade(grant).await,
        "refresh_token" => oidc.renew(grant).await,
        _ => Err(sour("unsupported_grant_type")),
    };
    let mut out = match grip {
        Ok(json) => json.into_response(),
        Err((code, json)) => (code, json).into_response(),
    };
    let heads = out.headers_mut();
    heads.insert("cache-control", HeaderValue::from_static("no-store"));
    heads.insert("pragma", HeaderValue::from_static("no-cache"));
    out
}

pub(crate) async fn userinfo<W: Wire + 'static>(
    State(oidc): State<Oidc<W>>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    let token = bearer(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    oidc.look(&token).await
}
