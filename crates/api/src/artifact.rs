use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub(crate) struct Artifact {
    path: PathBuf,
}

#[derive(Clone, Debug)]
pub(crate) struct Bootstrap {
    pub(crate) sudo: Artifact,
    pub(crate) signing: Artifact,
}

impl Artifact {
    pub(crate) fn load(&self) -> Result<Option<Vec<u8>>, String> {
        match std::fs::read(&self.path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(None),
            Err(err) => Err(format!(
                "cannot read artifact {}: {err}",
                self.path.display()
            )),
        }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn keep(&self, bytes: &[u8]) -> Result<bool, String> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| {
                format!(
                    "cannot prepare artifact destination {}: {err}",
                    self.path.display()
                )
            })?;
        }
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = match options.open(&self.path) {
            Ok(file) => file,
            Err(err) if err.kind() == ErrorKind::AlreadyExists => return Ok(false),
            Err(err) => {
                return Err(format!(
                    "cannot create artifact {}: {err}",
                    self.path.display()
                ));
            }
        };
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|err| format!("cannot keep artifact {}: {err}", self.path.display()))?;
        Ok(true)
    }
}

pub(crate) fn bootstrap(specs: &[String]) -> Result<Bootstrap, String> {
    let mut found = parse(specs)?;
    let sudo = found
        .remove("sudo")
        .ok_or_else(|| "missing artifact: sudo".to_string())?;
    let signing = found
        .remove("signing")
        .ok_or_else(|| "missing artifact: signing".to_string())?;
    if let Some(name) = found.keys().next() {
        return Err(format!("unknown artifact: {name}"));
    }
    Ok(Bootstrap { sudo, signing })
}

pub(crate) fn signing(root: &Path, specs: &[String]) -> Result<Artifact, String> {
    let mut found = parse(specs)?;
    if found.contains_key("sudo") {
        return Err("sudo artifact is forbidden at runtime".to_string());
    }
    let signing = match found.remove("signing") {
        Some(signing) => signing,
        None if found.is_empty() => Artifact {
            path: root.join(".local").join("sign.pem"),
        },
        None => return Err("missing artifact: signing".to_string()),
    };
    if let Some(name) = found.keys().next() {
        return Err(format!("unknown artifact: {name}"));
    }
    Ok(signing)
}

fn parse(specs: &[String]) -> Result<BTreeMap<String, Artifact>, String> {
    let mut found = BTreeMap::new();
    for spec in specs {
        let (name, destination) = spec
            .split_once('=')
            .ok_or_else(|| "artifact must be NAME=DEST".to_string())?;
        if found.contains_key(name) {
            return Err(format!("duplicate artifact: {name}"));
        }
        let path = destination
            .strip_prefix("file:")
            .ok_or_else(|| format!("unsupported destination for artifact: {name}"))?;
        if path.is_empty() {
            return Err(format!("empty destination for artifact: {name}"));
        }
        found.insert(
            name.to_string(),
            Artifact {
                path: PathBuf::from(path),
            },
        );
    }
    Ok(found)
}
