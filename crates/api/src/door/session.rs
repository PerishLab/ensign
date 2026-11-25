use crate::booth::Booth;
use crate::util::{Sound, clear, fault, private};
use axum::extract::Path as Route;
use axum::http::{HeaderMap, StatusCode};
use axum::{Extension, Json};
use keel::{Op, Operator, Wire, form};
use keel_gate::{bearer, crumb, digest};
use serde_json::{Value, json};

impl<W: Wire + 'static> Booth<W> {
    pub(crate) async fn sessions(
        &self,
        op: Option<Extension<Operator>>,
        headers: HeaderMap,
    ) -> Result<(StatusCode, HeaderMap, Json<Value>), StatusCode> {
        let Some(Extension(Operator(me))) = op else {
            return Err(StatusCode::UNAUTHORIZED);
        };
        let (session, token) = active(&headers);
        let seat = me.to_string();
        let face = self.core.of(me);
        let held = face
            .ask(&form("Session").when("actor", Op::Eq, &seat))
            .await
            .sound()?;
        let sessions: Vec<Value> = held
            .rows()
            .iter()
            .map(|row| {
                let current = session
                    .as_deref()
                    .is_some_and(|hash| row.text("hash") == Some(hash));
                json!({
                    "id": row.key(),
                    "created": row.created(),
                    "expires": row.expires(),
                    "current": current,
                })
            })
            .collect();
        let keys = face
            .ask(&form("Token").when("actor", Op::Eq, &seat))
            .await
            .sound()?;
        let tokens: Vec<Value> = keys
            .rows()
            .iter()
            .map(|row| {
                let current = token
                    .as_deref()
                    .is_some_and(|hash| row.text("hash") == Some(hash));
                json!({
                    "id": row.key(),
                    "name": row.text("name").unwrap_or_default(),
                    "created": row.created(),
                    "expires": row.expires(),
                    "current": current,
                })
            })
            .collect();
        Ok((
            StatusCode::OK,
            private(),
            Json(json!({ "sessions": sessions, "tokens": tokens })),
        ))
    }

    pub(crate) async fn evict(
        &self,
        op: Option<Extension<Operator>>,
        headers: HeaderMap,
        Route(id): Route<i64>,
    ) -> Result<(StatusCode, HeaderMap), StatusCode> {
        let Some(Extension(Operator(me))) = op else {
            return Err(StatusCode::UNAUTHORIZED);
        };
        let face = self.core.of(me);
        let row = face
            .one(&form("Session").when("id", Op::Eq, &id.to_string()))
            .await
            .sound()?
            .ok_or(StatusCode::NOT_FOUND)?;
        if row.int("actor") != Some(me) {
            return Err(StatusCode::NOT_FOUND);
        }
        let (session, _) = active(&headers);
        let current = session
            .as_deref()
            .is_some_and(|hash| row.text("hash") == Some(hash));
        face.end("Session", id).await.map_err(|err| fault(&err))?;
        let out = if current {
            clear(self.secure)?
        } else {
            HeaderMap::new()
        };
        Ok((StatusCode::NO_CONTENT, out))
    }

    pub(crate) async fn revoke(
        &self,
        op: Option<Extension<Operator>>,
        Route(id): Route<i64>,
    ) -> Result<StatusCode, StatusCode> {
        let Some(Extension(Operator(me))) = op else {
            return Err(StatusCode::UNAUTHORIZED);
        };
        let face = self.core.of(me);
        let row = face
            .one(&form("Token").when("id", Op::Eq, &id.to_string()))
            .await
            .sound()?
            .ok_or(StatusCode::NOT_FOUND)?;
        if row.int("actor") != Some(me) {
            return Err(StatusCode::NOT_FOUND);
        }
        face.end("Token", id).await.map_err(|err| fault(&err))?;
        Ok(StatusCode::NO_CONTENT)
    }
}

fn active(headers: &HeaderMap) -> (Option<String>, Option<String>) {
    if let Some(pat) = bearer(headers) {
        return (None, Some(digest(&pat)));
    }
    (crumb(headers).map(|sid| digest(&sid)), None)
}
