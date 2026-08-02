use plumb::config::Cascade;
use std::path::PathBuf;

#[derive(Debug, PartialEq, Cascade)]
pub struct Profile {
    pub url: String,
    pub home: String,
}

impl Default for Profile {
    fn default() -> Self {
        let seat = plumb::config::data("ensign")
            .map(|path| path.display().to_string())
            .unwrap_or_default();
        Profile {
            url: String::new(),
            home: seat,
        }
    }
}

pub fn profile() -> Result<Profile, String> {
    Profile::resolve(None).map_err(|err| err.to_string())
}

pub fn base() -> Result<String, String> {
    let held = profile()?.url;
    if held.is_empty() {
        return Err("ENSIGN_URL is required".to_string());
    }
    Ok(held)
}

pub fn home() -> Result<PathBuf, String> {
    let held = profile()?.home;
    if held.is_empty() {
        return Err("no home to store the credential".to_string());
    }
    Ok(PathBuf::from(held))
}
