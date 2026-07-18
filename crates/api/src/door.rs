use crate::booth::Booth;
use crate::util::{crumb, digest, fits, lock, owner, pct, text, wearer, wild};
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::{Extension, Json};
use keel::{Operator, Wire};
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
    let card = booth.card(&code).await.ok_or(StatusCode::NOT_FOUND)?;
    if booth.actor(&login).await.is_some() {
        return Err(StatusCode::CONFLICT);
    }
    let hash = tokio::task::spawn_blocking(move || lock(&pass))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let key = booth
        .birth(&login, &name)
        .await
        .map_err(|_| StatusCode::CONFLICT)?;
    if let Err(err) = booth.core.of(booth.svc).end("Invite", card).await {
        booth.unbirth(key).await;
        let gone = err.to_string().contains("missing row");
        return Err(if gone {
            StatusCode::NOT_FOUND
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        });
    }
    let armed = booth
        .core
        .of(key)
        .put("Pass", &[("hash", &hash), ("actor", &key.to_string())])
        .await;
    if armed.is_err() {
        booth.unbirth(key).await;
        booth.rearm(&digest(&code)).await;
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    Ok((StatusCode::CREATED, Json(json!({ "id": key }))))
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
    let pat = wild();
    booth
        .core
        .put(
            "Token",
            &[
                ("name", name),
                ("hash", &digest(&pat)),
                ("actor", &key.to_string()),
            ],
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, Json(json!({ "token": pat }))))
}

pub(crate) async fn untoken<W: Wire + 'static>(
    State(booth): State<Booth<W>>,
    headers: HeaderMap,
) -> Result<StatusCode, StatusCode> {
    let pat = wearer(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let q = format!(r#"from Token where hash = "{}""#, digest(&pat));
    let pack = booth
        .core
        .of(booth.svc)
        .query(&q)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let row = pack.rows().first().ok_or(StatusCode::NOT_FOUND)?;
    let key = owner(row).ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    booth
        .core
        .of(key)
        .end("Token", row.key())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn logout<W: Wire + 'static>(
    State(booth): State<Booth<W>>,
    headers: HeaderMap,
) -> Result<StatusCode, StatusCode> {
    let sid = crumb(&headers).ok_or(StatusCode::BAD_REQUEST)?;
    let q = format!(r#"from Session where hash = "{}""#, digest(&sid));
    let face = booth.core.of(booth.svc);
    let pack = face
        .query(&q)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let row = pack.rows().first().ok_or(StatusCode::NOT_FOUND)?;
    let key = owner(row).ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    booth
        .core
        .of(key)
        .end("Session", row.key())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn who<W: Wire + 'static>(
    State(booth): State<Booth<W>>,
    op: Option<Extension<Operator>>,
) -> Result<Json<Value>, StatusCode> {
    let Some(Extension(Operator(me))) = op else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let (login, name) = booth.tag(me).await.ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(json!({ "id": me, "login": login, "name": name })))
}

pub(crate) async fn auth<W: Wire + 'static>(
    State(booth): State<Booth<W>>,
    op: Option<Extension<Operator>>,
) -> Result<(StatusCode, HeaderMap), StatusCode> {
    let Some(Extension(Operator(me))) = op else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let (login, _) = booth.tag(me).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let teams = booth
        .crews(me)
        .await
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
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
    let q = format!(r#"from Rescue where actor = "{me}""#);
    let pack = face
        .query(&q)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    for row in pack.rows() {
        face.end("Rescue", row.key())
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    let mut codes = Vec::new();
    for _ in 0..3 {
        let code = wild();
        face.put(
            "Rescue",
            &[("hash", &digest(&code)), ("actor", &me.to_string())],
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
    let fit = tokio::task::spawn_blocking(move || fits(&old, &held))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !fit {
        return Err(StatusCode::FORBIDDEN);
    }
    let hash = tokio::task::spawn_blocking(move || lock(&pass))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    booth
        .shield(me, &hash)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
    let key = booth.actor(&login).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let spare = booth
        .spare(key, &code)
        .await
        .ok_or(StatusCode::UNAUTHORIZED)?;
    match booth.barred(key).await {
        Some(false) => {}
        Some(true) => return Err(StatusCode::FORBIDDEN),
        None => return Err(StatusCode::INTERNAL_SERVER_ERROR),
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
