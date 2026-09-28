use plumb::cookbook::{Cookbook, Entry};
use std::fmt::Write;

const SOURCES: &[&str] = &[
    include_str!("../cookbook/bootstrap.sudo-custody.txt"),
    include_str!("../cookbook/signing.artifact-invalid.txt"),
];

pub(crate) fn render(code: Option<&str>, json: bool) -> Result<String, String> {
    let book = book()?;
    let Some(code) = code else {
        return if json {
            serde_json::to_string_pretty(&book).map_err(|error| error.to_string())
        } else {
            Ok(ledger(&book))
        };
    };
    let entry = book.get(code).ok_or_else(|| {
        format!(
            "unknown Cookbook code `{code}`; available: {}",
            names(&book)
        )
    })?;
    if json {
        serde_json::to_string_pretty(entry).map_err(|error| error.to_string())
    } else {
        Ok(page(entry))
    }
}

fn book() -> Result<Cookbook, String> {
    let entries = SOURCES
        .iter()
        .map(|source| Entry::parse(source))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Cookbook::new(entries).map_err(|error| error.to_string())
}

fn ledger(book: &Cookbook) -> String {
    let mut out = String::new();
    for entry in book.entries() {
        let _ = writeln!(out, "{}", entry.code());
        if let Some(exit) = entry.exit() {
            let _ = writeln!(out, "  EXIT: {exit}");
        }
    }
    out.trim_end().to_string()
}

fn page(entry: &Entry) -> String {
    let mut out = format!("# {}", entry.code());
    section(&mut out, "Trigger", entry.trigger());
    section(&mut out, "Solution", entry.solution());
    if let Some(evidence) = entry.evidence() {
        section(&mut out, "Evidence", evidence);
    }
    if let Some(exit) = entry.exit() {
        section(&mut out, "EXIT", exit);
    }
    out
}

fn section(page: &mut String, name: &str, body: &str) {
    let _ = write!(page, "\n\n## {name}\n\n{body}");
}

fn names(book: &Cookbook) -> String {
    book.entries()
        .iter()
        .map(|entry| entry.code().text())
        .collect::<Vec<_>>()
        .join(", ")
}
