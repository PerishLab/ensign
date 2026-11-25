use super::{known, shown};
use crate::booth::Booth;
use axum::extract::{Path as Route, State};
use axum::http::HeaderMap;
use axum::routing::{delete, get, patch, post};
use axum::{Extension, Json, Router};
use keel::{Operator, Wire};
use serde_json::{Map, Value};

type Seat<W> = State<Booth<W>>;
type Op = Option<Extension<Operator>>;
type Body = Json<Map<String, Value>>;
type Raw = Json<Value>;
type Key = Route<i64>;
type Pair = Route<(i64, i64)>;

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
            .route(
                "/profile",
                patch(|State(booth): Seat<W>, op: Op, raw: Raw| async move {
                    booth.rename(op, raw).await
                }),
            )
            .route("/profile/{sub}", get(shown::<W>))
            .route("/handle/{handle}", get(known::<W>))
            .route(
                "/auth",
                get(|State(booth): Seat<W>, op: Op| async move { booth.auth(op).await }),
            )
            .route(
                "/sessions",
                get(
                    |State(booth): Seat<W>, op: Op, headers: HeaderMap| async move {
                        booth.sessions(op, headers).await
                    },
                ),
            )
            .route(
                "/sessions/{id}",
                delete(
                    |State(booth): Seat<W>, op: Op, headers: HeaderMap, id: Key| async move {
                        booth.evict(op, headers, id).await
                    },
                ),
            )
            .route(
                "/tokens/{id}",
                delete(|State(booth): Seat<W>, op: Op, id: Key| async move {
                    booth.revoke(op, id).await
                }),
            )
            .route(
                "/rescue",
                get(|State(booth): Seat<W>, op: Op| async move { booth.rescue(op).await }),
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
            .merge(admin::<W>())
            .with_state(self)
    }
}

fn admin<W: Wire + 'static>() -> Router<Booth<W>> {
    Router::new()
        .route(
            "/admin/actors",
            get(|State(booth): Seat<W>, op: Op| async move { booth.actors(op).await }),
        )
        .route(
            "/admin/actors/{id}",
            get(|State(booth): Seat<W>, op: Op, id: Key| async move { booth.actor(op, id).await })
                .patch(
                    |State(booth): Seat<W>, op: Op, id: Key, raw: Raw| async move {
                        booth.bar(op, id, raw).await
                    },
                ),
        )
        .route(
            "/admin/invites",
            get(|State(booth): Seat<W>, op: Op| async move { booth.invites(op).await }).post(
                |State(booth): Seat<W>, op: Op, raw: Raw| async move { booth.issue(op, raw).await },
            ),
        )
        .route(
            "/admin/invites/{id}",
            delete(
                |State(booth): Seat<W>, op: Op, id: Key| async move { booth.cancel(op, id).await },
            ),
        )
        .route(
            "/admin/teams",
            get(|State(booth): Seat<W>, op: Op| async move { booth.teams(op).await }).post(
                |State(booth): Seat<W>, op: Op, raw: Raw| async move { booth.forge(op, raw).await },
            ),
        )
        .route(
            "/admin/teams/{id}/members",
            post(
                |State(booth): Seat<W>, op: Op, id: Key, raw: Raw| async move {
                    booth.enrol(op, id, raw).await
                },
            ),
        )
        .route(
            "/admin/teams/{id}/members/{tie}",
            delete(|State(booth): Seat<W>, op: Op, pair: Pair| async move {
                booth.expel(op, pair).await
            }),
        )
        .route(
            "/admin/apps",
            get(|State(booth): Seat<W>, op: Op| async move { booth.apps(op).await }).post(
                |State(booth): Seat<W>, op: Op, raw: Raw| async move {
                    booth.register(op, raw).await
                },
            ),
        )
        .route(
            "/admin/apps/{id}",
            patch(
                |State(booth): Seat<W>, op: Op, id: Key, raw: Raw| async move {
                    booth.amend(op, id, raw).await
                },
            ),
        )
}
