use super::wire::bare;
use super::{admin, open};
use serde_json::json;

#[test]
fn actors() {
    let (stage, wire) = open();
    let crown = stage.crown();
    let boss = admin(&wire, &crown, "boss");
    let ada = wire.enrol(&crown, "ada");
    let team = wire.post("/admin/teams", json!({ "name": "crew" }), &boss.head);
    assert_eq!(team.status, 201);
    assert_eq!(
        wire.post(
            &format!("/admin/teams/{}/members", team.id()),
            json!({ "actor": ada.id }),
            &boss.head,
        )
        .status,
        201
    );
    assert_eq!(wire.get("/admin/actors", &bare()).status, 401);
    assert_eq!(wire.get("/admin/actors", &crown).status, 401);
    assert_eq!(
        wire.get(&format!("/admin/actors/{}", ada.id), &bare())
            .status,
        401
    );
    assert_eq!(
        wire.get(&format!("/admin/actors/{}", ada.id), &crown)
            .status,
        401
    );
    let peer = wire.get("/admin/actors", &ada.head);
    assert_eq!(peer.status, 200);
    assert_eq!(peer.header("cache-control"), Some("no-store"));
    let list = wire.get("/admin/actors", &boss.head);
    assert_eq!(list.status, 200);
    assert_eq!(list.header("cache-control"), Some("no-store"));
    let rows = list.body.as_array().expect("actors");
    let svc = rows
        .iter()
        .find(|row| row["sub"] == json!("ensign"))
        .expect("service actor");
    assert_eq!(svc["kind"], json!("svc"));
    assert_eq!(svc["barred"], json!(false));
    let ensign = svc["id"].as_i64().expect("svc id");
    let one = wire.get(&format!("/admin/actors/{}", ada.id), &boss.head);
    assert_eq!(one.status, 200);
    assert_eq!(one.header("cache-control"), Some("no-store"));
    assert_eq!(one.text("login"), "ada");
    assert_eq!(one.text("name"), "ada");
    assert_eq!(one.text("kind"), "user");
    assert_eq!(one.body["barred"], json!(false));
    assert_eq!(one.body["teams"], json!(["crew"]));
    assert!(one.body["created"].as_i64().is_some());
    assert_eq!(wire.get("/admin/actors/999999", &boss.head).status, 404);
    assert_eq!(
        wire.patch(
            &format!("/admin/actors/{}", ada.id),
            json!({ "barred": true }),
            &bare(),
        )
        .status,
        401
    );
    assert_eq!(
        wire.patch(
            &format!("/admin/actors/{}", ada.id),
            json!({ "barred": true }),
            &crown,
        )
        .status,
        401
    );
    assert_eq!(
        wire.patch(
            &format!("/admin/actors/{}", boss.id),
            json!({ "barred": true }),
            &ada.head,
        )
        .status,
        403
    );
    assert_eq!(
        wire.patch(
            &format!("/admin/actors/{}", ada.id),
            json!({ "barred": false }),
            &ada.head,
        )
        .status,
        400,
        "self patch refused unconditionally"
    );
    assert_eq!(
        wire.patch(
            &format!("/admin/actors/{}", boss.id),
            json!({ "barred": true }),
            &boss.head,
        )
        .status,
        400,
        "admin self bar"
    );
    assert_eq!(
        wire.patch(
            &format!("/admin/actors/{ensign}"),
            json!({ "barred": true }),
            &ada.head,
        )
        .status,
        403,
        "non-admin barring service actor gets 403"
    );
    assert_eq!(
        wire.patch(
            &format!("/admin/actors/{ensign}"),
            json!({ "barred": true }),
            &boss.head,
        )
        .status,
        409,
        "admin barring service actor gets 409"
    );
    let intact = wire.get(&format!("/admin/actors/{ensign}"), &boss.head);
    assert_eq!(intact.body["barred"], json!(false));
    for bad in [
        json!({ "barred": true, "sub": "forged" }),
        json!({ "barred": true, "kind": "svc" }),
        json!({ "sub": "forged" }),
        json!({}),
    ] {
        assert_eq!(
            wire.patch(&format!("/admin/actors/{}", ada.id), bad, &boss.head)
                .status,
            400
        );
    }
    assert_eq!(
        wire.patch(
            "/admin/actors/999999",
            json!({ "barred": true }),
            &boss.head
        )
        .status,
        404
    );
    let shut = wire.patch(
        &format!("/admin/actors/{}", ada.id),
        json!({ "barred": true }),
        &boss.head,
    );
    assert_eq!(shut.status, 200);
    assert_eq!(shut.body["barred"], json!(true));
    assert_eq!(wire.get("/whoami", &ada.head).status, 401);
    let open = wire.patch(
        &format!("/admin/actors/{}", ada.id),
        json!({ "barred": false }),
        &boss.head,
    );
    assert_eq!(open.status, 200);
    assert_eq!(open.body["barred"], json!(false));
    assert_eq!(wire.get("/whoami", &ada.head).status, 200);
}
