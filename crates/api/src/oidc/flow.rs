use crate::oidc::plain::{bearer, sour};
use crate::oidc::swap::SWAP;
use crate::oidc::{Ask, Grant, Oidc};
use crate::util::pct;
use axum::Extension;
use axum::Json;
use axum::Router;
use axum::extract::{Form, OriginalUri, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use keel::{Operator, Wire};
use serde_json::{Value, json};

impl<W: Wire + 'static> Oidc<W> {
    pub(crate) fn plate(self) -> Router {
        Router::new()
            .route(
                "/.well-known/openid-configuration",
                get(|State(oidc): State<Self>| async move { oidc.disco().await }),
            )
            .route(
                "/.well-known/jwks.json",
                get(|State(oidc): State<Self>| async move { oidc.jwks().await }),
            )
            .route(
                "/authorize",
                get(
                    |State(oidc): State<Self>,
                     uri: OriginalUri,
                     ask: Query<Ask>,
                     op: Option<Extension<Operator>>| async move {
                        oidc.authorize(uri, ask, op).await
                    },
                ),
            )
            .route(
                "/token",
                post(|State(oidc): State<Self>, grant: Form<Grant>| async move {
                    oidc.token(grant).await
                }),
            )
            .route(
                "/userinfo",
                get(|State(oidc): State<Self>, headers: HeaderMap| async move {
                    oidc.userinfo(headers).await
                }),
            )
            .with_state(self)
    }

    pub(crate) async fn disco(&self) -> Json<Value> {
        let iss = &self.iss;
        Json(json!({
            "issuer": iss,
            "authorization_endpoint": format!("{iss}/authorize"),
            "token_endpoint": format!("{iss}/token"),
            "userinfo_endpoint": format!("{iss}/userinfo"),
            "jwks_uri": format!("{iss}/.well-known/jwks.json"),
            "response_types_supported": ["code"],
            "grant_types_supported": ["authorization_code", "refresh_token", SWAP],
            "subject_types_supported": ["public"],
            "id_token_signing_alg_values_supported": ["ES256"],
            "scopes_supported": ["openid", "profile"],
            "code_challenge_methods_supported": ["S256"],
            "token_endpoint_auth_methods_supported": ["none"],
        }))
    }

    pub(crate) async fn jwks(&self) -> Json<Value> {
        Json(json!({ "keys": [self.keys.jwk] }))
    }

    pub(crate) async fn authorize(
        &self,
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
        self.open(ask, actor).await
    }

    pub(crate) async fn token(&self, Form(grant): Form<Grant>) -> Response {
        let grip = match grant.kind.as_str() {
            "authorization_code" => self.trade(grant).await,
            "refresh_token" => self.renew(grant).await,
            SWAP => self.swap(grant).await,
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

    pub(crate) async fn userinfo(&self, headers: HeaderMap) -> Result<Json<Value>, StatusCode> {
        let token = bearer(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
        self.look(&token).await
    }
}
