//! `key_enum!`: an enum of unit variants, each tied to a stable string key.
//!
//! Generates the enum, `key()`, `from_key()`, `ALL` and the [`KeyEnum`]
//! implementation, so enum <-> string conversions are never written by hand.
//! Serde attributes stay on the enum; `assert_keys_match_serde` checks in tests
//! that every key equals the serde name.

use serde::Serialize;

/// An enum whose variants map one to one onto string keys.
pub trait KeyEnum: Copy + 'static {
    /// Every variant, in declaration order.
    fn all() -> &'static [Self];
    /// The variant's key.
    fn key_str(self) -> &'static str;
    /// Inverse of [`KeyEnum::key_str`].
    fn from_key_str(key: &str) -> Option<Self>;
}

/// Declares a unit-variant enum with a string key per variant.
///
/// ```ignore
/// key_enum! {
///     #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
///     #[serde(rename_all = "snake_case")]
///     pub enum Mood {
///         /// Doc comments and `#[serde(rename = ..)]` stay on variants.
///         Calm => "calm",
///     }
/// }
/// ```
#[macro_export]
macro_rules! key_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $( $(#[$vmeta:meta])* $variant:ident => $key:literal ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        $vis enum $name {
            $( $(#[$vmeta])* $variant ),+
        }

        #[allow(dead_code)]
        impl $name {
            /// Every variant, in declaration order.
            pub const ALL: [$name; [$($key),+].len()] = [$($name::$variant),+];

            /// Stable string key (the serde name).
            pub fn key(self) -> &'static str {
                match self {
                    $( $name::$variant => $key ),+
                }
            }

            /// Variant whose key is `key`.
            pub fn from_key(key: &str) -> Option<$name> {
                match key {
                    $( $key => Some($name::$variant), )+
                    _ => None,
                }
            }
        }

        impl $crate::key_enum::KeyEnum for $name {
            fn all() -> &'static [Self] {
                &$name::ALL
            }
            fn key_str(self) -> &'static str {
                self.key()
            }
            fn from_key_str(key: &str) -> Option<Self> {
                $name::from_key(key)
            }
        }
    };
}

/// Test helper: every variant's key equals its serde name, and `from_key` inverts `key`.
pub fn assert_keys_match_serde<E: KeyEnum + Serialize + PartialEq + std::fmt::Debug>() {
    for &variant in E::all() {
        let serde_name = serde_json::to_value(variant).expect("serializable variant");
        assert_eq!(
            serde_name.as_str(),
            Some(variant.key_str()),
            "{variant:?}: key differs from serde name"
        );
        assert_eq!(E::from_key_str(variant.key_str()), Some(variant));
    }
}
