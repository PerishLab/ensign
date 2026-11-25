use crate::booth::Booth;
use crate::startup::SERVICE;
use crate::util::{Sound, fault, parse, private};
use axum::extract::Path as Route;
use axum::http::{HeaderMap, StatusCode};
use axum::{Extension, Json};
use keel::adapt::Error;
use keel::{Op, Operator, Row, Wire, form};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;

const SEALED: &str = "sealed service actor";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Toggle {
    barred: bool,
}

impl<W: Wire + 'static> Booth<W> {
    pub(crate) async fn profiles(
        &self,
    ) -> Result<HashMap<i64, (String, String)>, keel::adapt::Error> {
        let rows = self.core.of(self.svc).live("Profile").await?;
        let mut map = HashMap::new();
        for row in rows {
            let Some(actor) = row.int("actor") else {
                continue;
            };
            let Some(login) = row.text("handle").map(str::to_string) else {
                continue;
            };
            let name = row.text("name").unwrap_or_default().to_string();
            map.insert(actor, (login, name));
        }
        Ok(map)
    }

    async fn roster(&self, me: i64) -> Result<HashMap<i64, Vec<String>>, keel::adapt::Error> {
        let face = self.core.of(me);
        let teams = face.live("Team").await?;
        let mut map: HashMap<i64, Vec<String>> = HashMap::new();
        for team in teams {
            let name = team.text("name").unwrap_or_default().to_string();
            let ties = face.ties("Team", "members", team.key()).await?;
            for tie in ties {
                map.entry(tie.right()).or_default().push(name.clone());
            }
        }
        Ok(map)
    }

    pub(crate) async fn actors(
        &self,
        op: Option<Extension<Operator>>,
    ) -> Result<(StatusCode, HeaderMap, Json<Value>), StatusCode> {
        let Some(Extension(Operator(me))) = op else {
            return Err(StatusCode::UNAUTHORIZED);
        };
        let rows = self.core.of(me).live("Actor").await.sound()?;
        let profiles = self.profiles().await.sound()?;
        let roster = self.roster(me).await.sound()?;
        let out: Vec<Value> = rows
            .iter()
            .map(|row| {
                let id = row.key();
                let profile = profiles.get(&id);
                let teams = roster.get(&id).cloned().unwrap_or_default();
                detail(row, profile, teams)
            })
            .collect();
        Ok((StatusCode::OK, private(), Json(json!(out))))
    }

    pub(crate) async fn actor(
        &self,
        op: Option<Extension<Operator>>,
        Route(id): Route<i64>,
    ) -> Result<(StatusCode, HeaderMap, Json<Value>), StatusCode> {
        let Some(Extension(Operator(me))) = op else {
            return Err(StatusCode::UNAUTHORIZED);
        };
        let ask = form("Actor").when("id", Op::Eq, &id.to_string());
        let row = self
            .core
            .of(me)
            .one(&ask)
            .await
            .sound()?
            .ok_or(StatusCode::NOT_FOUND)?;
        let profile = self.tag(id).await.sound()?;
        let teams = self.crews(id).await.sound()?;
        Ok((
            StatusCode::OK,
            private(),
            Json(detail(&row, profile.as_ref(), teams)),
        ))
    }

    pub(crate) async fn bar(
        &self,
        op: Option<Extension<Operator>>,
        Route(id): Route<i64>,
        Json(raw): Json<Value>,
    ) -> Result<Json<Value>, StatusCode> {
        let Some(Extension(Operator(me))) = op else {
            return Err(StatusCode::UNAUTHORIZED);
        };
        let body: Toggle = parse(raw)?;
        if id == me {
            return Err(StatusCode::BAD_REQUEST);
        }
        let ask = form("Actor").when("id", Op::Eq, &id.to_string());
        let face = self.core.of(me);
        let flag = if body.barred { "true" } else { "false" };
        let row = face
            .batch(async |tx| {
                let held = tx
                    .one(&ask)
                    .await?
                    .ok_or_else(|| Error::Adapt(format!("missing row {id}")))?;
                tx.set("Actor", id, &[("barred", flag)]).await?;
                if held.text("sub") == Some(SERVICE) && body.barred {
                    return Err(Error::Adapt(SEALED.into()));
                }
                tx.one(&ask)
                    .await?
                    .ok_or_else(|| Error::Adapt(format!("missing row {id}")))
            })
            .await
            .map_err(|err| match &err {
                Error::Adapt(note) if note == SEALED => StatusCode::CONFLICT,
                _ => fault(&err),
            })?;
        let profile = self.tag(id).await.sound()?;
        let teams = self.crews(id).await.sound()?;
        Ok(Json(detail(&row, profile.as_ref(), teams)))
    }
}

fn detail(row: &Row, profile: Option<&(String, String)>, teams: Vec<String>) -> Value {
    let login = profile.map(|(handle, _)| handle.as_str());
    let name = profile.map(|(_, name)| name.as_str());
    json!({
        "id": row.key(),
        "sub": row.text("sub").unwrap_or_default(),
        "login": login,
        "name": name,
        "kind": row.text("kind").unwrap_or_default(),
        "barred": row.flag("barred") == Some(true),
        "created": row.created(),
        "teams": teams,
    })
}
