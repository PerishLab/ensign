use plumb::config::{Cascade, Env};
use std::path::Path;

#[derive(Debug, Default, PartialEq, Cascade)]
pub struct Runtime {
    pub fresh: bool,
    pub iss: String,
    #[cascade(section)]
    pub store: Store,
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
    match plumb::config::discover(start, keel::config::NAME) {
        Ok(found) => Runtime::resolve(Some(&found)),
        Err(_) => Runtime::resolve(None),
    }
}
