use crate::booth::Booth;
use crate::util::{Sound, fault, parse, private};
use axum::extract::Path as Route;
use axum::http::{HeaderMap, StatusCode};
use axum::{Extension, Json};
use keel::life::Ends;
use keel::{Op, Operator, Row, Wire, form};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Named {
    name: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Enrol {
    actor: i64,
}

impl<W: Wire + 'static> Booth<W> {
    pub(crate) async fn teams(
        &self,
        op: Option<Extension<Operator>>,
    ) -> Result<(StatusCode, HeaderMap, Json<Value>), StatusCode> {
        let Some(Extension(Operator(me))) = op else {
            return Err(StatusCode::UNAUTHORIZED);
        };
        let face = self.core.of(me);
        let teams = face.live("Team").await.sound()?;
        let actors: HashMap<i64, Row> = face
            .live("Actor")
            .await
            .sound()?
            .into_iter()
            .map(|row| (row.key(), row))
            .collect();
        let profiles = self.profiles().await.sound()?;
        let mut out = Vec::with_capacity(teams.len());
        for team in teams {
            let ties = face.ties("Team", "members", team.key()).await.sound()?;
            let mut members = Vec::new();
            for tie in ties {
                let Some(actor) = actors.get(&tie.right()) else {
                    continue;
                };
                let profile = profiles.get(&actor.key());
                members.push(member(tie.key(), actor, profile));
            }
            out.push(json!({
                "id": team.key(),
                "name": team.text("name").unwrap_or_default(),
                "created": team.created(),
                "members": members,
            }));
        }
        Ok((StatusCode::OK, private(), Json(json!(out))))
    }

    pub(crate) async fn forge(
        &self,
        op: Option<Extension<Operator>>,
        Json(raw): Json<Value>,
    ) -> Result<(StatusCode, Json<Value>), StatusCode> {
        let Some(Extension(Operator(me))) = op else {
            return Err(StatusCode::UNAUTHORIZED);
        };
        let body: Named = parse(raw)?;
        let name = body.name.trim();
        let len = name.chars().count();
        if len == 0 || len > 120 || body.name.chars().any(char::is_control) {
            return Err(StatusCode::BAD_REQUEST);
        }
        let face = self.core.of(me);
        let id = face
            .put("Team", &[("name", name)])
            .await
            .map_err(|err| fault(&err))?;
        let row = face
            .one(&form("Team").when("id", Op::Eq, &id.to_string()))
            .await
            .sound()?
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok((
            StatusCode::CREATED,
            Json(json!({
                "id": row.key(),
                "name": row.text("name").unwrap_or(name),
                "created": row.created(),
                "members": [],
            })),
        ))
    }

    pub(crate) async fn enrol(
        &self,
        op: Option<Extension<Operator>>,
        Route(id): Route<i64>,
        Json(raw): Json<Value>,
    ) -> Result<(StatusCode, Json<Value>), StatusCode> {
        let Some(Extension(Operator(me))) = op else {
            return Err(StatusCode::UNAUTHORIZED);
        };
        let body: Enrol = parse(raw)?;
        let ends = Ends {
            left: id,
            right: body.actor,
        };
        let tie = self
            .core
            .of(me)
            .tie("Team", "members", ends, &[])
            .await
            .map_err(|err| fault(&err))?;
        Ok((
            StatusCode::CREATED,
            Json(json!({
                "tie": tie,
                "team": id,
                "actor": body.actor,
            })),
        ))
    }

    pub(crate) async fn expel(
        &self,
        op: Option<Extension<Operator>>,
        Route((id, tie)): Route<(i64, i64)>,
    ) -> Result<StatusCode, StatusCode> {
        let Some(Extension(Operator(me))) = op else {
            return Err(StatusCode::UNAUTHORIZED);
        };
        let face = self.core.of(me);
        let ties = face
            .ties("Team", "members", id)
            .await
            .map_err(|err| fault(&err))?;
        if !ties.iter().any(|row| row.key() == tie) {
            return Err(StatusCode::NOT_FOUND);
        }
        face.cut("Team", "members", tie)
            .await
            .map_err(|err| fault(&err))?;
        Ok(StatusCode::NO_CONTENT)
    }
}

fn member(tie: i64, actor: &Row, profile: Option<&(String, String)>) -> Value {
    let login = profile.map(|(handle, _)| handle.as_str());
    let name = profile.map(|(_, name)| name.as_str());
    json!({
        "tie": tie,
        "id": actor.key(),
        "sub": actor.text("sub").unwrap_or_default(),
        "login": login,
        "name": name,
        "kind": actor.text("kind").unwrap_or_default(),
        "barred": actor.flag("barred") == Some(true),
    })
}
