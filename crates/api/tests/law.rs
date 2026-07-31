use std::fs;
use std::path::{Path, PathBuf};

fn sources() -> Vec<PathBuf> {
    let here = Path::new(env!("CARGO_MANIFEST_DIR"));
    let crates = here.parent().expect("crates");
    let mut found = Vec::new();
    for seat in ["api", "cli"] {
        walk(&crates.join(seat).join("src"), &mut found);
    }
    assert!(found.len() > 8, "product sources not found: {found:?}");
    found
}

fn walk(dir: &Path, found: &mut Vec<PathBuf>) {
    let held = fs::read_dir(dir).unwrap_or_else(|err| panic!("read {}: {err}", dir.display()));
    for entry in held {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            walk(&path, found);
        } else if path.extension().is_some_and(|kind| kind == "rs") {
            found.push(path);
        }
    }
}

fn bearing(mark: &str) -> Vec<String> {
    let mut hits = Vec::new();
    for path in sources() {
        let text = fs::read_to_string(&path).expect("read");
        if text.contains(mark) {
            hits.push(path.display().to_string());
        }
    }
    hits
}

#[test]
fn grant() {
    let hits = bearing("\"@grant\"");
    assert!(
        hits.is_empty(),
        "grant construction is keel vocabulary and must not be retold in ensign: {hits:?}"
    );
}

#[test]
fn ambient() {
    let hits = bearing(".sudo()");
    let allowed: Vec<&String> = hits
        .iter()
        .filter(|path| path.ends_with("api/src/door/mod.rs"))
        .collect();
    assert_eq!(
        hits.len(),
        allowed.len(),
        "ambient sudo is admitted only for identity birth in door/mod.rs: {hits:?}"
    );
}
