use std::fs::OpenOptions;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

pub(super) fn keep(path: &Path, bytes: &[u8]) -> Result<bool, String> {
    let parent = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    std::fs::create_dir_all(parent).map_err(|err| {
        format!(
            "cannot prepare artifact destination {}: {err}",
            path.display()
        )
    })?;
    let draft = draft(path)?;
    let placed = stage(&draft, bytes).and_then(|_| place(&draft, path));
    let _ = std::fs::remove_file(&draft);
    let placed = placed.map_err(|err| format!("cannot keep artifact {}: {err}", path.display()))?;
    if placed {
        settle(parent)
            .map_err(|err| format!("cannot settle artifact {}: {err}", path.display()))?;
    }
    Ok(placed)
}

fn draft(path: &Path) -> Result<PathBuf, String> {
    let name = path
        .file_name()
        .ok_or_else(|| format!("artifact destination has no name: {}", path.display()))?;
    let mut seed = [0u8; 8];
    getrandom::fill(&mut seed)
        .map_err(|err| format!("cannot name artifact draft {}: {err}", path.display()))?;
    let tag: String = seed.iter().map(|byte| format!("{byte:02x}")).collect();
    Ok(path.with_file_name(format!(".{}.{tag}.part", name.to_string_lossy())))
}

fn stage(draft: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(draft)?;
    file.write_all(bytes)?;
    file.sync_all()
}

fn place(draft: &Path, path: &Path) -> std::io::Result<bool> {
    match std::fs::hard_link(draft, path) {
        Ok(()) => Ok(true),
        Err(err) if err.kind() == ErrorKind::AlreadyExists => Ok(false),
        Err(err) => Err(err),
    }
}

#[cfg(unix)]
fn settle(parent: &Path) -> std::io::Result<()> {
    std::fs::File::open(parent)?.sync_all()
}

#[cfg(not(unix))]
fn settle(_: &Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{keep, place, stage};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    struct Nest(PathBuf);

    impl Nest {
        fn new() -> Self {
            let serial = NEXT.fetch_add(1, Ordering::Relaxed);
            let path = Path::new("target")
                .join("disk")
                .join(format!("{}-{serial}", std::process::id()));
            std::fs::create_dir_all(&path).expect("nest");
            Nest(path)
        }
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .expect("list")
            .map(|entry| entry.expect("entry").file_name().to_string_lossy().into())
            .collect();
        names.sort();
        names
    }

    impl Drop for Nest {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn publish() {
        let nest = Nest::new();
        let draft = nest.0.join(".sudo.draft.part");
        let path = nest.0.join("sudo");
        stage(&draft, b"complete").expect("stage");
        assert!(place(&draft, &path).expect("place"));
        assert_eq!(std::fs::read(&path).expect("read"), b"complete");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).expect("meta").permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }

    #[test]
    fn occupied() {
        let nest = Nest::new();
        let draft = nest.0.join(".sudo.draft.part");
        let path = nest.0.join("sudo");
        std::fs::write(&path, b"held").expect("held");
        stage(&draft, b"rival").expect("stage");
        assert!(!place(&draft, &path).expect("place"));
        assert_eq!(std::fs::read(&path).expect("read"), b"held");
    }

    #[test]
    fn fresh() {
        let nest = Nest::new();
        let path = nest.0.join("deep").join("sudo");
        assert!(keep(&path, b"complete").expect("keep"));
        assert_eq!(std::fs::read(&path).expect("read"), b"complete");
        assert!(!keep(&path, b"rival").expect("again"));
        assert_eq!(std::fs::read(&path).expect("read"), b"complete");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).expect("meta").permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        assert_eq!(names(&nest.0.join("deep")), vec!["sudo".to_string()]);
    }

    #[test]
    fn leftover() {
        let nest = Nest::new();
        let stale = nest.0.join(".sudo.0000000000000000.part");
        std::fs::write(&stale, b"").expect("stale");
        let path = nest.0.join("sudo");
        assert!(keep(&path, b"complete").expect("keep"));
        assert_eq!(std::fs::read(&path).expect("read"), b"complete");
        assert_eq!(
            names(&nest.0),
            vec![
                ".sudo.0000000000000000.part".to_string(),
                "sudo".to_string()
            ]
        );
    }
}
