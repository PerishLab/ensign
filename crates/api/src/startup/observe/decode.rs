use axum::body::{Body, to_bytes};
use axum::http::{HeaderMap, StatusCode, header};
use flate2::read::MultiGzDecoder;
use std::io::{Cursor, Read};

const RAW: usize = 256 * 1024;
const GZIP: usize = 128 * 1024;

enum Coding {
    Identity,
    Gzip,
}

pub(crate) async fn read(headers: &HeaderMap, body: Body) -> Result<Vec<u8>, StatusCode> {
    match coding(headers)? {
        Coding::Identity => {
            let bytes = to_bytes(body, RAW)
                .await
                .map_err(|_| StatusCode::PAYLOAD_TOO_LARGE)?;
            Ok(bytes.to_vec())
        }
        Coding::Gzip => {
            let compressed = to_bytes(body, GZIP)
                .await
                .map_err(|_| StatusCode::PAYLOAD_TOO_LARGE)?;
            gunzip(&compressed)
        }
    }
}

fn coding(headers: &HeaderMap) -> Result<Coding, StatusCode> {
    let mut values = headers.get_all(header::CONTENT_ENCODING).iter();
    let Some(first) = values.next() else {
        return Ok(Coding::Identity);
    };
    if values.next().is_some() {
        return Err(StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }
    let raw = first
        .to_str()
        .map_err(|_| StatusCode::UNSUPPORTED_MEDIA_TYPE)?;
    match raw.trim().to_ascii_lowercase().as_str() {
        "" | "identity" => Ok(Coding::Identity),
        "gzip" => Ok(Coding::Gzip),
        _ => Err(StatusCode::UNSUPPORTED_MEDIA_TYPE),
    }
}

fn gunzip(compressed: &[u8]) -> Result<Vec<u8>, StatusCode> {
    if compressed.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut decoder = MultiGzDecoder::new(Cursor::new(compressed));
    let mut out = Vec::new();
    let cap = (RAW + 1) as u64;
    (&mut decoder)
        .take(cap)
        .read_to_end(&mut out)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    if out.len() > RAW {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }
    if decoder.into_inner().position() as usize != compressed.len() {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(out)
}
