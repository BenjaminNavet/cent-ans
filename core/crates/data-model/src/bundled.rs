//! Rules files compiled into the crate: the single source of truth for the
//! `Default` of a rules struct and for the `bundled()` accessors of the
//! simulation crates. The JSON in `data/` is the only place a value lives.

use serde::de::DeserializeOwned;

/// Parses `json` (the contents of `data/<name>`) as a `T`; `pointer` (an
/// RFC 6901 JSON pointer, `""` for the whole file) selects a sub-object.
/// Panics when the embedded file does not match the type: a build-time bug,
/// caught by any test that touches the rules.
pub fn parse<T: DeserializeOwned>(name: &str, json: &str, pointer: &str) -> T {
    let parsed = if pointer.is_empty() {
        serde_json::from_str(json)
    } else {
        serde_json::from_str::<serde_json::Value>(json)
            .map_err(<serde_json::Error as serde::de::Error>::custom)
            .and_then(|root| {
                root.pointer(pointer)
                    .cloned()
                    .ok_or_else(|| serde::de::Error::custom("pointer not found"))
            })
            .and_then(|sub| serde_json::from_value(sub).map_err(serde::de::Error::custom))
    };
    parsed.unwrap_or_else(|e| panic!("bundled data/{name}{pointer} is invalid: {e}"))
}

/// Declares `Type::bundled()`: the process-wide parsed copy of the embedded
/// `data/<file>` (optionally of the sub-object at `at "<json pointer>"`).
/// With a trailing `default`, also implements `Default` as a clone of it.
///
/// ```ignore
/// data_model::bundled_rules!(DuelRules, "rules/battle_duel.json");
/// data_model::bundled_rules!(TaxPerHead, "rules/economy.json", at "/tax_per_head", default);
/// ```
#[macro_export]
macro_rules! bundled_rules {
    ($ty:ty, $file:literal) => {
        $crate::bundled_rules!(@bundled $ty, $file, "");
    };
    ($ty:ty, $file:literal, default) => {
        $crate::bundled_rules!(@bundled $ty, $file, "");
        $crate::bundled_rules!(@default $ty);
    };
    ($ty:ty, $file:literal, at $pointer:literal) => {
        $crate::bundled_rules!(@bundled $ty, $file, $pointer);
    };
    ($ty:ty, $file:literal, at $pointer:literal, default) => {
        $crate::bundled_rules!(@bundled $ty, $file, $pointer);
        $crate::bundled_rules!(@default $ty);
    };
    (@default $ty:ty) => {
        impl ::std::default::Default for $ty {
            fn default() -> Self {
                <$ty>::bundled().clone()
            }
        }
    };
    (@bundled $ty:ty, $file:literal, $pointer:literal) => {
        impl $ty {
            /// The embedded rules file, parsed once.
            pub fn bundled() -> &'static $ty {
                static CELL: ::std::sync::OnceLock<$ty> = ::std::sync::OnceLock::new();
                CELL.get_or_init(|| {
                    $crate::bundled::parse::<$ty>(
                        $file,
                        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../data/", $file)),
                        $pointer,
                    )
                })
            }
        }
    };
}
