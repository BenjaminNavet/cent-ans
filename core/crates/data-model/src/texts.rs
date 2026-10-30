//! French sentences of the simulation core (order refusals, invariant reports, captures),
//! read from `data/ui/core_messages.fr.json` (ADR 0829). The core code names a key and its
//! parameters; the wording lives in the data file, not in Rust.

use std::collections::BTreeMap;
use std::fmt::Display;
use std::sync::OnceLock;

/// Raw embedded catalogue, for tests that scan keys.
pub const CORE_MESSAGES_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../data/ui/core_messages.fr.json"
));

/// Shape of `core_messages.fr.json`.
#[derive(serde::Deserialize)]
struct CoreMessages {
    messages: BTreeMap<String, String>,
}

/// The parsed catalogue (key to template).
pub fn core_messages() -> &'static BTreeMap<String, String> {
    static CELL: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    CELL.get_or_init(|| {
        serde_json::from_str::<CoreMessages>(CORE_MESSAGES_JSON)
            .unwrap_or_else(|e| panic!("bundled data/ui/core_messages.fr.json is invalid: {e}"))
            .messages
    })
}

/// Renders message `key`, replacing each `{name}` of the template with the matching argument.
/// An unknown key renders as the key itself (a test checks every key used in the sources).
pub fn core_text(key: &str, args: &[(&str, &dyn Display)]) -> String {
    let Some(template) = core_messages().get(key) else {
        return key.to_owned();
    };
    let mut rendered = template.clone();
    for (name, value) in args {
        rendered = rendered.replace(&format!("{{{name}}}"), &value.to_string());
    }
    rendered
}
