use super::{known, shown};
use crate::booth::Booth;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use keel::{Operator, Wire};
use serde_json::{Map, Value};

type Seat<W> = State<Booth<W>>;
type Op = Option<Extension<Operator>>;
type Body = Json<Map<String, Value>>;

impl<W: Wire + 'static> Booth<W> {
    pub(crate) fn plate(self) -> Router {
        Router::new()
            .route(
                "/invite",
                post(
                    |State(booth): Seat<W>, headers: HeaderMap, op: Op, body: Body| async move {
                        booth.invite(headers, op, body).await
                    },
                ),
            )
            .route(
                "/join",
                post(|State(booth): Seat<W>, body: Body| async move { booth.join(body).await }),
            )
            .route(
                "/login",
                post(|State(booth): Seat<W>, body: Body| async move { booth.login(body).await }),
            )
            .route(
                "/logout",
                post(|State(booth): Seat<W>, headers: HeaderMap| async move {
                    booth.logout(headers).await
                }),
            )
            .route(
                "/whoami",
                get(|State(booth): Seat<W>, op: Op| async move { booth.who(op).await }),
            )
            .route("/profile/{sub}", get(shown::<W>))
            .route("/handle/{handle}", get(known::<W>))
            .route(
                "/auth",
                get(|State(booth): Seat<W>, op: Op| async move { booth.auth(op).await }),
            )
            .route(
                "/mint",
                post(|State(booth): Seat<W>, op: Op| async move { booth.mint(op).await }),
            )
            .route(
                "/repass",
                post(|State(booth): Seat<W>, op: Op, body: Body| async move {
                    booth.repass(op, body).await
                }),
            )
            .route(
                "/revive",
                post(|State(booth): Seat<W>, body: Body| async move { booth.revive(body).await }),
            )
            .route(
                "/bearer",
                post(|State(booth): Seat<W>, body: Body| async move { booth.token(body).await })
                    .delete(|State(booth): Seat<W>, headers: HeaderMap| async move {
                        booth.untoken(headers).await
                    }),
            )
            .with_state(self)
    }
}
