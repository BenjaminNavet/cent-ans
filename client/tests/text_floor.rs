//! Source test (audit XP-LISIBLE): no UI text is built below the typographic floor (13 px).
//! `UiTheme::text_font` clamps at runtime; this test keeps the literals honest at the call sites.

use std::fs;
use std::path::{Path, PathBuf};

const FLOOR: f32 = 13.0;

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// The bare numeric literal that ends an argument list (`"false, false, 10.0"` -> 10.0). An
/// expression (`size * 0.62`, `Caption.size() - 1.0`) is not a bare argument and gives `None`.
fn trailing_literal(text: &str) -> Option<f32> {
    let text = text.trim_end();
    let start = text
        .rfind(|c: char| !(c.is_ascii_digit() || c == '.'))
        .map_or(0, |i| i + 1);
    let literal = &text[start..];
    let before = text[..start].trim_end();
    if literal.is_empty() || !before.ends_with(',') {
        return None;
    }
    literal.parse().ok()
}

#[test]
fn no_text_literal_below_the_floor() {
    let mut files = Vec::new();
    rust_files(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    let mut offenders = Vec::new();
    for file in files {
        let source = fs::read_to_string(&file).unwrap();
        for (number, line) in source.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            let mut found: Vec<f32> = Vec::new();
            if let Some(at) = code.find("text_font(") {
                let tail = &code[at + "text_font(".len()..];
                let end = tail.find(')').unwrap_or(tail.len());
                found.extend(trailing_literal(&tail[..end]));
            }
            if let Some(at) = code.find("FontSize::Px(") {
                let tail = &code[at + "FontSize::Px(".len()..];
                let end = tail.find(')').unwrap_or(tail.len());
                found.extend(tail[..end].trim().parse::<f32>().ok());
            }
            if found.iter().any(|v| *v < FLOOR) {
                offenders.push(format!(
                    "{}:{}: {}",
                    file.display(),
                    number + 1,
                    line.trim()
                ));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "text below {FLOOR} px:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn trailing_literal_reads_bare_arguments_only() {
    assert_eq!(trailing_literal("false, false, 10.0"), Some(10.0));
    assert_eq!(trailing_literal("false, false, size * 0.62"), None);
    assert_eq!(
        trailing_literal("true, false, UiType::Caption.size() - 1.0"),
        None
    );
}
