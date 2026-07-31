use plumb::config::{Cascade, Env};
use std::path::Path;

pub const NAME: &str = "ensign.toml";

#[derive(Debug, Default, PartialEq, Cascade)]
pub struct Runtime {
    pub fresh: bool,
    pub iss: String,
    #[cascade(section)]
    pub listen: Listen,
    #[cascade(section)]
    pub store: Store,
    #[cascade(section)]
    pub cache: Cache,
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
            port: 3500,
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

pub fn load(start: &Path) -> Result<Runtime, plumb::config::Error> {
    match plumb::config::discover(start, NAME) {
        Ok(found) => Runtime::resolve(Some(&found)),
        Err(_) => Runtime::resolve(None),
    }
}
