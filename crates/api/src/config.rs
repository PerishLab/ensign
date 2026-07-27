pub struct Runtime {
    pub fresh: bool,
    pub issuer: Option<String>,
    pub pg: Option<String>,
    pub port: Option<u16>,
}

pub fn load() -> Result<Runtime, String> {
    let port = match read("SIDECAR_PORT") {
        Some(raw) => Some(
            raw.parse()
                .map_err(|err: std::num::ParseIntError| format!("SIDECAR_PORT: {err}"))?,
        ),
        None => None,
    };
    Ok(Runtime {
        fresh: std::env::var_os("KEEL_FRESH").is_some(),
        issuer: read("ENSIGN_ISS"),
        pg: read("KEEL_PG"),
        port,
    })
}

fn read(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
}
