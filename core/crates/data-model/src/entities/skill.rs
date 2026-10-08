//! Skill tree node: command, governance or court branch (`skill.schema.json`).

use crate::key_enum;
use serde::{Deserialize, Serialize};

use crate::common::{Effect, LocalizedName, Sources};
use crate::ids::SkillId;

key_enum! {
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillBranch {
    Command => "command",
    Governance => "governance",
    Court => "court",
}
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Skill {
    pub id: SkillId,
    pub name: LocalizedName,
    pub branch: SkillBranch,
    /// 1 to 3.
    pub tier: u8,
    /// Prerequisite skills, always in the same branch.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prerequisites: Vec<SkillId>,
    /// Skill points cost (equal to `tier`).
    pub cost: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<Effect>,
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Sources,
}

#[cfg(test)]
mod key_enum_tests {
    use super::*;
    use crate::key_enum::assert_keys_match_serde;

    #[test]
    fn keys_match_serde_names() {
        assert_keys_match_serde::<SkillBranch>();
    }
}
