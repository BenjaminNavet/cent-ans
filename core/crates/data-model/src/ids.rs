//! Typed identifiers.
//!
//! Every entity id in `data/` carries a prefix (`fac_`, `prov_`, ...) declared in
//! `common.schema.json`. Each newtype below validates that prefix and the
//! `[a-z0-9_]` alphabet when deserializing, so a typo such as `"fac-france"` or
//! a province id stored in a faction field is rejected at load time.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize};

/// Checks `id` against the pattern `^<prefix>[a-z0-9_]+$`.
fn is_valid_id(id: &str, prefix: &str) -> bool {
    match id.strip_prefix(prefix) {
        Some(rest) if !rest.is_empty() => rest
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'),
        _ => false,
    }
}

macro_rules! define_id {
    ($(#[$doc:meta])* $name:ident, $prefix:literal) => {
        $(#[$doc])*
        #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Prefix every id of this kind must start with.
            pub const PREFIX: &'static str = $prefix;

            /// Validates `id` and wraps it, or returns the offending string.
            pub fn new(id: impl Into<String>) -> Result<Self, String> {
                let id = id.into();
                if is_valid_id(&id, Self::PREFIX) {
                    Ok(Self(id))
                } else {
                    Err(id)
                }
            }

            /// The id as a string slice.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({:?})", stringify!($name), self.0)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl std::borrow::Borrow<str> for $name {
            fn borrow(&self) -> &str {
                &self.0
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let raw = String::deserialize(deserializer)?;
                Self::new(raw).map_err(|bad| {
                    serde::de::Error::custom(format!(
                        "invalid {} {bad:?}: expected `{}[a-z0-9_]+`",
                        stringify!($name),
                        $prefix
                    ))
                })
            }
        }
    };
}

define_id!(
    /// Identifier of a faction (`fac_france`).
    FactionId,
    "fac_"
);
define_id!(
    /// Identifier of a province (`prov_normandie`).
    ProvinceId,
    "prov_"
);
define_id!(
    /// Identifier of a unit type (`unit_longbowmen`).
    UnitTypeId,
    "unit_"
);
define_id!(
    /// Identifier of a building (`bld_castle`).
    BuildingId,
    "bld_"
);
define_id!(
    /// Identifier of a technology (`tech_bombards`).
    TechnologyId,
    "tech_"
);
define_id!(
    /// Identifier of a character (`chr_philippe_vi`).
    CharacterId,
    "chr_"
);
define_id!(
    /// Identifier of a resource (`res_wheat`).
    ResourceId,
    "res_"
);
define_id!(
    /// Identifier of a religion (`rel_catholic`).
    ReligionId,
    "rel_"
);
define_id!(
    /// Culture tag (`cul_french`); free vocabulary, no entity behind it.
    CultureId,
    "cul_"
);
define_id!(
    /// Identifier of a character trait (`trait_pious`).
    TraitId,
    "trait_"
);
define_id!(
    /// Identifier of a skill tree node (`skill_hardiesse`).
    SkillId,
    "skill_"
);
define_id!(
    /// Identifier of a per-culture name list (`names_fr`).
    NamesId,
    "names_"
);
define_id!(
    /// Identifier of a chronicle event (`evt_crecy`), M10.
    EventId,
    "evt_"
);
define_id!(
    /// Identifier of a province diet (`diet_bread_pottage`), H3 « La Table ».
    DietId,
    "diet_"
);
define_id!(
    /// Abstract sea zone (`sea_channel`); free vocabulary.
    SeaZoneId,
    "sea_"
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_well_formed_ids() {
        assert!(FactionId::new("fac_france").is_ok());
        assert!(ProvinceId::new("prov_ile_de_france").is_ok());
        assert!(TraitId::new("trait_2nd").is_ok());
    }

    #[test]
    fn rejects_wrong_prefix_or_alphabet() {
        assert!(FactionId::new("prov_france").is_err());
        assert!(FactionId::new("fac_").is_err());
        assert!(FactionId::new("fac_France").is_err());
        assert!(FactionId::new("fac-france").is_err());
    }

    #[test]
    fn deserialize_reports_invalid_id() {
        let err = serde_json::from_str::<FactionId>("\"prov_x\"").unwrap_err();
        assert!(err.to_string().contains("invalid FactionId"));
    }
}
