use super::wire::bare;
use super::{admin, open};
use serde_json::json;

#[test]
fn teams() {
    let (stage, wire) = open();
    let crown = stage.crown();
    let boss = admin(&wire, &crown, "boss");
    let ada = wire.enrol(&crown, "ada");
    assert_eq!(wire.get("/admin/teams", &bare()).status, 401);
    assert_eq!(wire.get("/admin/teams", &crown).status, 401);
    assert_eq!(
        wire.post("/admin/teams", json!({ "name": "ops" }), &bare())
            .status,
        401
    );
    assert_eq!(
        wire.post("/admin/teams", json!({ "name": "ops" }), &crown)
            .status,
        401
    );
    assert_eq!(
        wire.post("/admin/teams", json!({ "name": "ops" }), &ada.head)
            .status,
        403
    );
    for bad in [
        json!({ "name": "" }),
        json!({ "name": "   " }),
        json!({ "name": "a".repeat(121) }),
        json!({ "name": "bad\nname" }),
        json!({ "name": "ops", "extra": true }),
    ] {
        assert_eq!(wire.post("/admin/teams", bad, &boss.head).status, 400);
    }
    let ops = wire.post("/admin/teams", json!({ "name": "ops" }), &boss.head);
    assert_eq!(ops.status, 201, "create {}", ops.body);
    assert_eq!(ops.text("name"), "ops");
    let oid = ops.id();
    let dev = wire.post("/admin/teams", json!({ "name": "dev" }), &boss.head);
    assert_eq!(dev.status, 201);
    let did = dev.id();
    assert_eq!(
        wire.post("/admin/teams", json!({ "name": "ops" }), &boss.head)
            .status,
        409
    );
    assert_eq!(
        wire.post(
            &format!("/admin/teams/{oid}/members"),
            json!({ "actor": ada.id }),
            &bare(),
        )
        .status,
        401
    );
    assert_eq!(
        wire.post(
            &format!("/admin/teams/{oid}/members"),
            json!({ "actor": ada.id }),
            &crown,
        )
        .status,
        401
    );
    assert_eq!(
        wire.post(
            &format!("/admin/teams/{oid}/members"),
            json!({ "actor": ada.id }),
            &ada.head,
        )
        .status,
        403
    );
    assert_eq!(
        wire.post(
            "/admin/teams/999999/members",
            json!({ "actor": ada.id }),
            &boss.head,
        )
        .status,
        404
    );
    assert_eq!(
        wire.post(
            &format!("/admin/teams/{oid}/members"),
            json!({ "actor": 999999 }),
            &boss.head,
        )
        .status,
        404
    );
    let joined = wire.post(
        &format!("/admin/teams/{oid}/members"),
        json!({ "actor": ada.id }),
        &boss.head,
    );
    assert_eq!(joined.status, 201, "enrol {}", joined.body);
    let tie = joined.body["tie"].as_i64().expect("tie");
    assert_eq!(joined.body["team"], json!(oid));
    assert_eq!(joined.body["actor"], json!(ada.id));
    assert_eq!(
        wire.post(
            &format!("/admin/teams/{oid}/members"),
            json!({ "actor": ada.id }),
            &boss.head,
        )
        .status,
        409
    );
    let peer = wire.get("/admin/teams", &ada.head);
    assert_eq!(peer.status, 200);
    assert_eq!(peer.header("cache-control"), Some("no-store"));
    let listed = wire.get("/admin/teams", &boss.head);
    assert_eq!(listed.status, 200);
    assert_eq!(listed.header("cache-control"), Some("no-store"));
    let rows = listed.body.as_array().expect("teams");
    let found = rows
        .iter()
        .find(|row| row["id"] == json!(oid))
        .expect("ops");
    let members = found["members"].as_array().expect("members");
    assert_eq!(members.len(), 1);
    assert_eq!(members[0]["tie"], json!(tie));
    assert_eq!(members[0]["id"], json!(ada.id));
    assert_eq!(members[0]["login"], json!("ada"));
    assert_eq!(members[0]["barred"], json!(false));
    assert_eq!(
        wire.cut(&format!("/admin/teams/{did}/members/{tie}"), &boss.head)
            .status,
        404,
        "wrong team tie"
    );
    assert_eq!(
        wire.cut(&format!("/admin/teams/{oid}/members/{tie}"), &bare())
            .status,
        401
    );
    assert_eq!(
        wire.cut(&format!("/admin/teams/{oid}/members/{tie}"), &crown)
            .status,
        401
    );
    assert_eq!(
        wire.cut(&format!("/admin/teams/{oid}/members/{tie}"), &ada.head)
            .status,
        403
    );
    assert_eq!(
        wire.cut(&format!("/admin/teams/{oid}/members/{tie}"), &boss.head)
            .status,
        204
    );
    let after = wire.get("/admin/teams", &boss.head);
    let empty = after
        .body
        .as_array()
        .expect("teams")
        .iter()
        .find(|row| row["id"] == json!(oid))
        .expect("ops")["members"]
        .as_array()
        .expect("members");
    assert!(empty.is_empty());
}
