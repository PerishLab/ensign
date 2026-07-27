use crate::booth::Booth;
use crate::util::{Sound, lock, pct, text};
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::{Extension, Json};
use keel::{Op, Operator, Wire, form};
use keel_gate::{bearer, crumb, digest, wild};
use serde_json::{Map, Value, json};

pub(crate) async fn invite<W: Wire + 'static>(
    State(booth): State<Booth<W>>,
    headers: HeaderMap,
    op: Option<Extension<Operator>>,
    Json(body): Json<Map<String, Value>>,
) -> Result<(StatusCode, Json<Value>), StatusCode> {
    let who = op.map(|Extension(Operator(id))| id);
    let face = booth
        .face(&headers, who)
        .await
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let note = body.get("note").and_then(Value::as_str).unwrap_or("");
    let code = wild();
    face.put("Invite", &[("hash", &digest(&code)), ("note", note)])
        .await
        .map_err(|_| StatusCode::FORBIDDEN)?;
    Ok((StatusCode::CREATED, Json(json!({ "code": code }))))
}

pub(crate) async fn join<W: Wire + 'static>(
    State(booth): State<Booth<W>>,
    Json(body): Json<Map<String, Value>>,
) -> Result<(StatusCode, Json<Value>), StatusCode> {
    let code = text(&body, "code")?;
    let login = text(&body, "login")?;
    let name = text(&body, "name")?;
    let pass = text(&body, "pass")?;
    if pass.chars().count() < 8 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let card = booth
        .card(&code)
        .await
        .sound()?
        .ok_or(StatusCode::NOT_FOUND)?;
    let taken = booth.actor(&login).await.sound()?;
    if taken.is_some() {
        return Err(StatusCode::CONFLICT);
    }
    let hash = tokio::task::spawn_blocking(move || lock(&pass))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let made = booth
        .core
        .sudo()
        .batch(async |tx| {
            tx.end("Invite", card).await?;
            let key = tx
                .put(
                    "Actor",
                    &[
                        ("login", &login),
                        ("name", &name),
                        ("kind", "user"),
                        ("barred", "false"),
                    ],
                )
                .await?;
            tx.put(
                "@grant",
                &[
                    ("who", &key.to_string()),
                    ("verb", "*"),
                    ("unit", "Actor"),
                    ("scope", &format!("row {key}")),
                ],
            )
            .await?;
            tx.put("Pass", &[("hash", &hash), ("actor", &key.to_string())])
                .await?;
            Ok(key)
        })
        .await;
    match made {
        Ok(key) => Ok((StatusCode::CREATED, Json(json!({ "id": key })))),
        Err(err) => Err(gauge(&err.to_string())),
    }
}

fn gauge(note: &str) -> StatusCode {
    if note.contains("missing row") {
        return StatusCode::NOT_FOUND;
    }
    if note.contains("taken") {
        return StatusCode::CONFLICT;
    }
    StatusCode::INTERNAL_SERVER_ERROR
}

pub(crate) async fn login<W: Wire + 'static>(
    State(booth): State<Booth<W>>,
    Json(body): Json<Map<String, Value>>,
) -> Result<(StatusCode, HeaderMap, Json<Value>), StatusCode> {
    let login = text(&body, "login")?;
    let pass = text(&body, "pass")?;
    let key = booth.verify(&login, &pass).await?;
    booth.session(key).await
}

pub(crate) async fn token<W: Wire + 'static>(
    State(booth): State<Booth<W>>,
    Json(body): Json<Map<String, Value>>,
) -> Result<(StatusCode, Json<Value>), StatusCode> {
    let login = text(&body, "login")?;
    let pass = text(&body, "pass")?;
    let name = body.get("name").and_then(Value::as_str).unwrap_or("cli");
    let key = booth.verify(&login, &pass).await?;
    let pat = booth.gate.token(key, name).await.sound()?;
    Ok((StatusCode::CREATED, Json(json!({ "token": pat }))))
}

pub(crate) async fn untoken<W: Wire + 'static>(
    State(booth): State<Booth<W>>,
    headers: HeaderMap,
) -> Result<StatusCode, StatusCode> {
    let pat = bearer(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    match booth.gate.revoke(&pat).await {
        Ok(()) => Ok(StatusCode::NO_CONTENT),
        Err(deny) => Err(deny.status()),
    }
}

pub(crate) async fn logout<W: Wire + 'static>(
    State(booth): State<Booth<W>>,
    headers: HeaderMap,
) -> Result<StatusCode, StatusCode> {
    let sid = crumb(&headers).ok_or(StatusCode::BAD_REQUEST)?;
    match booth.gate.logout(&sid).await {
        Ok(()) => Ok(StatusCode::NO_CONTENT),
        Err(deny) => Err(deny.status()),
    }
}

pub(crate) async fn who<W: Wire + 'static>(
    State(booth): State<Booth<W>>,
    op: Option<Extension<Operator>>,
) -> Result<Json<Value>, StatusCode> {
    let Some(Extension(Operator(me))) = op else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let (login, name) = booth.tag(me).await.sound()?.ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(json!({ "id": me, "login": login, "name": name })))
}

pub(crate) async fn auth<W: Wire + 'static>(
    State(booth): State<Booth<W>>,
    op: Option<Extension<Operator>>,
) -> Result<(StatusCode, HeaderMap), StatusCode> {
    let Some(Extension(Operator(me))) = op else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let (login, _) = booth
        .tag(me)
        .await
        .sound()?
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let teams = booth
        .crews(me)
        .await
        .sound()?
        .iter()
        .map(|team| pct(team))
        .collect::<Vec<_>>()
        .join(",");
    let mut headers = HeaderMap::new();
    let stamp = |value: &str| value.parse().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
    headers.insert("x-ensign-user", stamp(&me.to_string())?);
    headers.insert("x-ensign-login", stamp(&login)?);
    headers.insert("x-ensign-teams", stamp(&teams)?);
    Ok((StatusCode::OK, headers))
}

pub(crate) async fn mint<W: Wire + 'static>(
    State(booth): State<Booth<W>>,
    op: Option<Extension<Operator>>,
) -> Result<(StatusCode, Json<Value>), StatusCode> {
    let Some(Extension(Operator(me))) = op else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let face = booth.core.of(me);
    let ask = form("Rescue").when("actor", Op::Eq, &me.to_string());
    let pack = face.ask(&ask).await.sound()?;
    for row in pack.rows() {
        face.end("Rescue", row.key()).await.sound()?;
    }
    let mut codes = Vec::new();
    for _ in 0..3 {
        let code = wild();
        face.put(
            "Rescue",
            &[("hash", &digest(&code)), ("actor", &me.to_string())],
        )
        .await
        .sound()?;
        codes.push(code);
    }
    Ok((StatusCode::CREATED, Json(json!({ "codes": codes }))))
}

pub(crate) async fn repass<W: Wire + 'static>(
    State(booth): State<Booth<W>>,
    op: Option<Extension<Operator>>,
    Json(body): Json<Map<String, Value>>,
) -> Result<StatusCode, StatusCode> {
    let Some(Extension(Operator(me))) = op else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let old = text(&body, "old")?;
    let pass = text(&body, "pass")?;
    if pass.chars().count() < 8 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let held = match booth.guard(me).await {
        Ok(Some(held)) => held,
        Ok(None) => return Err(StatusCode::FORBIDDEN),
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };
    let fit = tokio::task::spawn_blocking(move || crate::util::fits(&old, &held))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !fit {
        return Err(StatusCode::FORBIDDEN);
    }
    let hash = tokio::task::spawn_blocking(move || lock(&pass))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    booth.shield(me, &hash).await.sound()?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn revive<W: Wire + 'static>(
    State(booth): State<Booth<W>>,
    Json(body): Json<Map<String, Value>>,
) -> Result<StatusCode, StatusCode> {
    let login = text(&body, "login")?;
    let code = text(&body, "code")?;
    let pass = text(&body, "pass")?;
    if pass.chars().count() < 8 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let key = booth
        .actor(&login)
        .await
        .sound()?
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let spare = booth
        .spare(key, &code)
        .await
        .sound()?
        .ok_or(StatusCode::UNAUTHORIZED)?;
    match booth.barred(key).await {
        Ok(Some(false)) => {}
        Ok(Some(true)) => return Err(StatusCode::FORBIDDEN),
        Ok(None) => return Err(StatusCode::UNAUTHORIZED),
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
    let _ = spare;
    let hash = tokio::task::spawn_blocking(move || lock(&pass))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if booth.recover(key, &hash).await.is_err() {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    Ok(StatusCode::NO_CONTENT)
}
