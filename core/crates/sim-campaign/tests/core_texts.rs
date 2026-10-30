//! Every `core_text("key", ...)` call of the sources names a key of
//! `data/ui/core_messages.fr.json`, and every key of the file is used (ADR 0829).

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

fn collect_keys(dir: &Path, keys: &mut BTreeSet<String>) {
    for entry in fs::read_dir(dir).expect("source dir readable") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            collect_keys(&path, keys);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            let text = fs::read_to_string(&path).expect("source readable");
            for piece in text.split("core_text(\"").skip(1) {
                if let Some(end) = piece.find('"') {
                    keys.insert(piece[..end].to_owned());
                }
            }
        }
    }
}

#[test]
fn core_text_keys_match_the_catalogue() {
    let mut used = BTreeSet::new();
    // This crate and the crates that also word core messages.
    collect_keys(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut used,
    );
    let catalogue: BTreeSet<String> = data_model::texts::core_messages().keys().cloned().collect();
    let missing: Vec<_> = used.difference(&catalogue).collect();
    assert!(
        missing.is_empty(),
        "keys used but absent from core_messages.fr.json: {missing:?}"
    );
    let dead: Vec<_> = catalogue.difference(&used).collect();
    assert!(
        dead.is_empty(),
        "keys in core_messages.fr.json never used: {dead:?}"
    );
}

#[test]
fn core_text_fills_placeholders() {
    let text = data_model::texts::core_text("order.unknown_army", &[("0", &"armée-7")]);
    assert_eq!(text, "armée inconnue : armée-7");
}
