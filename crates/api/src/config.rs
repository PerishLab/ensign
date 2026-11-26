use radicula_config::{Cascade, Env};
use std::fmt;
use std::path::{Path, PathBuf};

pub const NAME: &str = "ensign.toml";

#[derive(Debug, Default, PartialEq, Cascade)]
#[plumb::config::prefix]
pub struct Runtime {
    pub fresh: bool,
    pub iss: String,
    #[cascade(section)]
    pub listen: Listen,
    #[cascade(section)]
    pub store: Store,
    #[cascade(section)]
    pub cache: Cache,
    #[cascade(section)]
    pub locus: Locus,
}

#[derive(Debug, serde::Deserialize, PartialEq, Cascade)]
#[cascade(section)]
#[serde(default)]
pub struct Listen {
    pub host: String,
    pub port: u16,
    pub prefix: String,
}

impl Default for Listen {
    fn default() -> Self {
        Listen {
            host: "127.0.0.1".to_string(),
            port: 8080,
            prefix: String::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Hold {
    #[default]
    Memory,
    None,
}

impl Env for Hold {
    fn read(value: &str) -> Result<Self, String> {
        match value {
            "memory" => Ok(Hold::Memory),
            "none" => Ok(Hold::None),
            _ => Err("neither memory nor none".to_string()),
        }
    }
}

#[derive(Debug, Default, serde::Deserialize, PartialEq, Cascade)]
#[cascade(section)]
#[serde(default)]
pub struct Cache {
    pub kind: Hold,
}

#[derive(Clone, Copy, Debug, Default, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    #[default]
    Memory,
    File,
    Pg,
}

impl Env for Kind {
    fn read(value: &str) -> Result<Self, String> {
        match value {
            "memory" => Ok(Kind::Memory),
            "file" => Ok(Kind::File),
            "pg" => Ok(Kind::Pg),
            _ => Err("neither memory, file, nor pg".to_string()),
        }
    }
}

#[derive(Debug, Default, serde::Deserialize, PartialEq, Cascade)]
#[cascade(section)]
#[serde(default)]
pub struct Store {
    pub kind: Kind,
    pub path: String,
    pub url: String,
}

#[derive(Clone, serde::Deserialize, PartialEq, Cascade)]
#[cascade(section)]
#[serde(default)]
pub struct Locus {
    pub endpoint: String,
    pub key: String,
    pub keyfile: String,
    pub producer: String,
    pub buffer: String,
    pub origins: String,
    pub rpm: u32,
    pub burst: u32,
    pub forwarded: bool,
}

impl Default for Locus {
    fn default() -> Self {
        Locus {
            endpoint: String::new(),
            key: String::new(),
            keyfile: String::new(),
            producer: "ensign-web".to_string(),
            buffer: String::new(),
            origins: String::new(),
            rpm: 120,
            burst: 30,
            forwarded: false,
        }
    }
}

impl fmt::Debug for Locus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Locus")
            .field("endpoint", &self.endpoint)
            .field("key", &"[redacted]")
            .field("keyfile", &self.keyfile)
            .field("producer", &self.producer)
            .field("buffer", &self.buffer)
            .field("origins", &self.origins)
            .field("rpm", &self.rpm)
            .field("burst", &self.burst)
            .field("forwarded", &self.forwarded)
            .finish()
    }
}

impl Locus {
    pub fn enabled(&self) -> bool {
        !self.endpoint.trim().is_empty()
    }

    pub fn secret(&self, root: &Path) -> Result<String, String> {
        let direct = self.key.trim();
        if !direct.is_empty() {
            return Ok(direct.to_string());
        }
        let file = self.keyfile.trim();
        if file.is_empty() {
            return Err("locus key is required when endpoint is configured".into());
        }
        let path = plumb::config::rebase(Path::new(file), root);
        let raw = std::fs::read_to_string(&path).map_err(|err| format!("locus keyfile: {err}"))?;
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err("locus keyfile is empty".into());
        }
        Ok(trimmed.to_string())
    }

    pub fn dir(&self, root: &Path) -> Result<PathBuf, String> {
        let raw = self.buffer.trim();
        if raw.is_empty() {
            return Ok(std::env::temp_dir().join("ensign-locus-relay"));
        }
        let path = plumb::config::rebase(Path::new(raw), root);
        if path.is_absolute() {
            return Ok(path);
        }
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .map_err(|err| format!("locus buffer: {err}"))
    }
}

pub fn load(start: &Path) -> Result<Runtime, radicula_config::Error> {
    let found = plumb::config::discover(start, NAME).ok();
    let mut runtime =
        Runtime::resolve(found.as_deref(), &Runtime::prefix(), &plumb::config::value)?;
    if let Some(value) = plumb::config::value("ENSIGN_LOCUS_KEY") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            runtime.locus.key = trimmed.to_string();
        }
    }
    Ok(runtime)
}
