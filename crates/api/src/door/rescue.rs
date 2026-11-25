use crate::booth::Booth;
use crate::util::{fault, private};
use axum::http::{HeaderMap, StatusCode};
use axum::{Extension, Json};
use keel::{Op, Operator, Row, Wire, form};
use keel_gate::{digest, wild};
use serde_json::{Value, json};

impl<W: Wire + 'static> Booth<W> {
    pub(crate) async fn rescue(
        &self,
        op: Option<Extension<Operator>>,
    ) -> Result<(StatusCode, HeaderMap, Json<Value>), StatusCode> {
        let Some(Extension(Operator(me))) = op else {
            return Err(StatusCode::UNAUTHORIZED);
        };
        let ask = form("Rescue").when("actor", Op::Eq, &me.to_string());
        let pack = self
            .core
            .of(me)
            .ask(&ask)
            .await
            .map_err(|err| fault(&err))?;
        let rows = pack.rows();
        let count = rows.len();
        let created = rows.iter().map(Row::created).max();
        Ok((
            StatusCode::OK,
            private(),
            Json(json!({ "count": count, "created": created })),
        ))
    }

    pub(crate) async fn mint(
        &self,
        op: Option<Extension<Operator>>,
    ) -> Result<(StatusCode, HeaderMap, Json<Value>), StatusCode> {
        let Some(Extension(Operator(me))) = op else {
            return Err(StatusCode::UNAUTHORIZED);
        };
        let codes: Vec<String> = (0..3).map(|_| wild()).collect();
        let hashes: Vec<String> = codes.iter().map(|code| digest(code)).collect();
        let seat = me.to_string();
        self.core
            .of(me)
            .batch(async |tx| {
                let ask = form("Rescue").when("actor", Op::Eq, &seat);
                let pack = tx.ask(&ask).await?;
                for row in pack.rows() {
                    tx.end("Rescue", row.key()).await?;
                }
                for hash in &hashes {
                    tx.put("Rescue", &[("hash", hash), ("actor", &seat)])
                        .await?;
                }
                Ok(())
            })
            .await
            .map_err(|err| fault(&err))?;
        Ok((
            StatusCode::CREATED,
            private(),
            Json(json!({ "codes": codes })),
        ))
    }
}
