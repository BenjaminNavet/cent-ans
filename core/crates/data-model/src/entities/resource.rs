//! Resource: a natural or manufactured good (`resource.schema.json`).

use crate::key_enum;
use serde::{Deserialize, Serialize};

use crate::common::{LocalizedName, SocialClass, Sources};
use crate::ids::ResourceId;

key_enum! {
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceCategory {
    Food => "food",
    RawMaterial => "raw_material",
    Manufactured => "manufactured",
    Luxury => "luxury",
}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Resource {
    pub id: ResourceId,
    pub name: LocalizedName,
    pub category: ResourceCategory,
    /// Base price of one trade unit in livres tournois.
    pub base_price: u32,
    /// Classes whose goods satisfaction depends on this resource; empty:
    /// none (a material such as stone or iron, not a consumer good).
    #[serde(default)]
    pub satisfies_classes: Vec<SocialClass>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Sources,
}

#[cfg(test)]
mod key_enum_tests {
    use super::*;
    use crate::key_enum::assert_keys_match_serde;

    #[test]
    fn keys_match_serde_names() {
        assert_keys_match_serde::<ResourceCategory>();
    }
}
