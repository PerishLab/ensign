use std::path::PathBuf;

pub fn base() -> Result<String, String> {
    std::env::var("ENSIGN_URL").map_err(|_| "ENSIGN_URL is required".into())
}

pub fn home() -> Result<PathBuf, String> {
    if let Ok(dir) = std::env::var("ENSIGN_HOME") {
        return Ok(PathBuf::from(dir));
    }
    if let Ok(base) = std::env::var("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(base).join("ensign"));
    }
    let base = std::env::var("HOME").map_err(|_| "no home to store the credential")?;
    Ok(PathBuf::from(base).join(".config/ensign"))
}
