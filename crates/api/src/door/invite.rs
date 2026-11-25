use crate::booth::Booth;
use crate::util::{Sound, fault, parse, private};
use axum::extract::Path as Route;
use axum::http::{HeaderMap, StatusCode};
use axum::{Extension, Json};
use keel::life::tick;
use keel::{Op, Operator, Wire, form};
use keel_gate::{digest, wild};
use serde::Deserialize;
use serde_json::{Value, json};

const MAX: i64 = 2_592_000;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Card {
    note: Option<String>,
    ttl: Option<i64>,
}

impl<W: Wire + 'static> Booth<W> {
    pub(crate) async fn invites(
        &self,
        op: Option<Extension<Operator>>,
    ) -> Result<(StatusCode, HeaderMap, Json<Value>), StatusCode> {
        let Some(Extension(Operator(me))) = op else {
            return Err(StatusCode::UNAUTHORIZED);
        };
        let rows = self.core.of(me).live("Invite").await.sound()?;
        let out: Vec<Value> = rows
            .iter()
            .map(|row| {
                json!({
                    "id": row.key(),
                    "note": row.text("note").unwrap_or_default(),
                    "created": row.created(),
                    "expires": row.expires(),
                })
            })
            .collect();
        Ok((StatusCode::OK, private(), Json(json!(out))))
    }

    pub(crate) async fn issue(
        &self,
        op: Option<Extension<Operator>>,
        Json(raw): Json<Value>,
    ) -> Result<(StatusCode, HeaderMap, Json<Value>), StatusCode> {
        let Some(Extension(Operator(me))) = op else {
            return Err(StatusCode::UNAUTHORIZED);
        };
        let body: Card = parse(raw)?;
        if let Some(ttl) = body.ttl
            && !(1..=MAX).contains(&ttl)
        {
            return Err(StatusCode::BAD_REQUEST);
        }
        let note = body.note.unwrap_or_default();
        if note.chars().count() > 120 || note.chars().any(char::is_control) {
            return Err(StatusCode::BAD_REQUEST);
        }
        let code = wild();
        let hash = digest(&code);
        let until = body.ttl.map(|ttl| tick() + ttl);
        let row = self
            .core
            .of(me)
            .batch(async |tx| {
                let id = tx
                    .put("Invite", &[("hash", &hash), ("note", &note)])
                    .await?;
                if let Some(at) = until {
                    tx.lease("Invite", id, at).await?;
                }
                tx.one(&form("Invite").when("id", Op::Eq, &id.to_string()))
                    .await
            })
            .await
            .map_err(|err| fault(&err))?
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok((
            StatusCode::CREATED,
            private(),
            Json(json!({
                "id": row.key(),
                "code": code,
                "note": note,
                "created": row.created(),
                "expires": row.expires(),
            })),
        ))
    }

    pub(crate) async fn cancel(
        &self,
        op: Option<Extension<Operator>>,
        Route(id): Route<i64>,
    ) -> Result<StatusCode, StatusCode> {
        let Some(Extension(Operator(me))) = op else {
            return Err(StatusCode::UNAUTHORIZED);
        };
        self.core
            .of(me)
            .end("Invite", id)
            .await
            .map_err(|err| fault(&err))?;
        Ok(StatusCode::NO_CONTENT)
    }
}
