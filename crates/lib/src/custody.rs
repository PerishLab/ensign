use radicula_oidc::Secret;
use serde_json::Value;
use std::io::Write;
use std::path::{Path, PathBuf};

const SCHEMA: u64 = 1;

pub(crate) struct Store {
    home: PathBuf,
}

impl Store {
    pub(crate) fn new(home: &Path) -> Self {
        Self { home: home.into() }
    }
    fn kept(&self) -> std::path::PathBuf {
        self.home.join("state").join("pat.json")
    }

    pub(crate) fn load(&self) -> Result<(String, Secret), String> {
        let path = self.kept();
        check(&path)?;
        let text = std::fs::read_to_string(path)
            .map_err(|_| "not signed in — run `ensign login <login>`")?;
        let held: Value = serde_json::from_str(&text).map_err(|_| "invalid retained credential")?;
        if held.get("schema").and_then(Value::as_u64) != Some(SCHEMA) {
            return Err("unsupported retained credential".into());
        }
        let base = held
            .get("base")
            .and_then(Value::as_str)
            .filter(|v| !v.is_empty())
            .ok_or("credential has no endpoint")?
            .to_string();
        let token = held
            .get("token")
            .and_then(Value::as_str)
            .filter(|v| !v.is_empty())
            .ok_or("credential has no token")?
            .to_string();
        Ok((base, Secret::new(token)))
    }

    pub(crate) fn save(&self, base: &str, token: &str) -> Result<(), String> {
        let path = self.kept();
        let dir = path
            .parent()
            .ok_or_else(|| "credential has no directory".to_string())?;
        std::fs::create_dir_all(dir).map_err(|err| format!("credential dir: {err}"))?;
        let held = serde_json::json!({ "schema": SCHEMA, "base": base, "token": token });
        let beside = path.with_extension("json.new");
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&beside)
            .map_err(|err| format!("credential: {err}"))?;
        file.write_all(held.to_string().as_bytes())
            .and_then(|_| file.sync_all())
            .map_err(|err| format!("credential: {err}"))?;
        std::fs::rename(&beside, &path).map_err(|err| format!("credential: {err}"))?;
        self.shed();
        Ok(())
    }

    pub(crate) fn wipe(&self) -> Result<(), String> {
        match std::fs::remove_file(self.kept()) {
            Ok(()) => (),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(error) => return Err(format!("credential removal: {error}")),
        }
        self.shed();
        Ok(())
    }

    fn shed(&self) {
        let _ = std::fs::remove_file(self.home.join("credential"));
    }
}

fn check(path: &std::path::Path) -> Result<(), String> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|_| "not signed in — run `ensign login <login>`")?;
    if !metadata.file_type().is_file() {
        return Err("retained credential is not a regular file".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err("retained credential must be private (mode 0600)".into());
        }
    }
    Ok(())
}
