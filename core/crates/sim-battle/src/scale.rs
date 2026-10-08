//! Scale of a battle (lot EP1, ADR 0076): size of the field, gap between the
//! two battle lines, depth of the deployment zones and number of regiments a
//! side may field at once, by tier of total head count
//! (`data/rules/battle_scale.json`, schema
//! `data/schemas/battle_scale_rules.schema.json`).
//!
//! The first tier keeps the historical 1200 × 800 m field: every derived
//! value ([`FieldSize::attacker_line_z`], [`FieldSize::line_half`]…) is then
//! bit-for-bit the old constant, so seeds and tests are unchanged.

use serde::{Deserialize, Serialize};

use crate::setup::BattleSetup;

/// Spacing of the height grid, in metres (every tier).
pub const GRID_RESOLUTION: f64 = 10.0;

/// Width and depth of the field and the layout of the battle lines.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FieldSize {
    /// Along x, in metres.
    pub width: f64,
    /// Along z, in metres.
    pub depth: f64,
    /// Gap between the attacker's and the defender's battle lines.
    pub line_gap: f64,
    /// Depth of a deployment zone (the battle line 50 m from its front).
    pub zone_depth: f64,
}

impl Default for FieldSize {
    fn default() -> Self {
        Self::STANDARD
    }
}

impl FieldSize {
    /// The historical field (skirmishes, sieges).
    pub const STANDARD: FieldSize = FieldSize {
        width: 1200.0,
        depth: 800.0,
        line_gap: 300.0,
        zone_depth: 300.0,
    };

    /// Width relative to the standard field (1.0 exactly for it).
    pub fn sx(&self) -> f64 {
        self.width / Self::STANDARD.width
    }

    /// Depth relative to the standard field (1.0 exactly for it).
    pub fn sz(&self) -> f64 {
        self.depth / Self::STANDARD.depth
    }

    /// Area relative to the standard field.
    pub fn area_ratio(&self) -> f64 {
        self.sx() * self.sz()
    }

    pub fn center_x(&self) -> f64 {
        self.width * 0.5
    }

    pub fn center_z(&self) -> f64 {
        self.depth * 0.5
    }

    /// z of the attacker's battle line at deployment (250 on the standard field).
    pub fn attacker_line_z(&self) -> f64 {
        self.depth * 0.5 - self.line_gap * 0.5
    }

    /// z of the defender's battle line at deployment (550 on the standard field).
    pub fn defender_line_z(&self) -> f64 {
        self.depth * 0.5 + self.line_gap * 0.5
    }

    /// Half-width of the centre of the battle lines kept clear of woods,
    /// mud and steep ground (360 m on the standard field).
    pub fn line_half(&self) -> f64 {
        360.0 * self.sx()
    }

    /// Grid points along x.
    pub fn nx(&self) -> usize {
        (self.width / GRID_RESOLUTION) as usize + 1
    }

    /// Grid points along z.
    pub fn nz(&self) -> usize {
        (self.depth / GRID_RESOLUTION) as usize + 1
    }

    /// The size whose grid has `nx` × `nz` points and standard lines (for
    /// helpers that only see a height grid).
    pub fn from_grid(nx: usize, nz: usize) -> FieldSize {
        let width = (nx.saturating_sub(1)) as f64 * GRID_RESOLUTION;
        let depth = (nz.saturating_sub(1)) as f64 * GRID_RESOLUTION;
        if width == Self::STANDARD.width && depth == Self::STANDARD.depth {
            return Self::STANDARD;
        }
        FieldSize {
            width,
            depth,
            ..Self::STANDARD
        }
    }
}

/// One tier of `data/rules/battle_scale.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScaleTier {
    pub key: String,
    pub label: String,
    /// Total head count (both sides) this tier covers; `None` for the last.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_soldiers: Option<u32>,
    pub width_m: f64,
    pub depth_m: f64,
    pub line_gap_m: f64,
    /// Gap of the lines in a field battle (ADR 0184); sieges keep
    /// `line_gap_m`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field_line_gap_m: Option<f64>,
    pub zone_depth_m: f64,
    pub max_regiments_per_side: usize,
}

impl ScaleTier {
    pub fn field_size(&self) -> FieldSize {
        FieldSize {
            width: self.width_m,
            depth: self.depth_m,
            line_gap: self.line_gap_m,
            zone_depth: self.zone_depth_m,
        }
    }

    /// [`Self::scale`] for a field battle: with `field_line_gap_m` when set.
    pub fn field_scale(&self) -> BattleScale {
        let mut scale = self.scale();
        if let Some(gap) = self.field_line_gap_m {
            scale.field.line_gap = gap;
        }
        scale
    }

    pub fn scale(&self) -> BattleScale {
        BattleScale {
            key: self.key.clone(),
            field: self.field_size(),
            max_on_field: self.max_regiments_per_side,
        }
    }
}

/// Contents of `data/rules/battle_scale.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleScaleRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub tiers: Vec<ScaleTier>,
}

data_model::bundled_rules!(BattleScaleRules, "rules/battle_scale.json");

impl BattleScaleRules {
    /// The first tier covering `soldiers` (the last one beyond).
    pub fn tier_for(&self, soldiers: u32) -> &ScaleTier {
        self.tiers
            .iter()
            .find(|t| t.max_soldiers.is_none_or(|max| soldiers <= max))
            .or(self.tiers.last())
            .expect("at least one battle scale tier")
    }

    pub fn tier(&self, key: &str) -> Option<&ScaleTier> {
        self.tiers.iter().find(|t| t.key == key)
    }
}

/// The scale a battle is fought at.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BattleScale {
    /// Key of the tier (`skirmish`, `large`, `epic`…).
    pub key: String,
    pub field: FieldSize,
    /// Regiments a side may field at once (the general's always among them).
    pub max_on_field: usize,
}

impl Default for BattleScale {
    fn default() -> Self {
        BattleScaleRules::bundled().tiers[0].scale()
    }
}

impl BattleScale {
    /// The scale of `setup`: by total head count; sieges always on the first
    /// tier (the town plan is laid out on the standard field).
    pub fn for_setup(setup: &BattleSetup) -> BattleScale {
        let rules = BattleScaleRules::bundled();
        if setup.siege.is_some() {
            return rules.tiers[0].scale();
        }
        rules.tier_for(total_soldiers(setup)).field_scale()
    }

    /// The tier named `key`, if any.
    pub fn named(key: &str) -> Option<BattleScale> {
        BattleScaleRules::bundled()
            .tier(key)
            .map(ScaleTier::field_scale)
    }
}

/// Soldiers of both sides (reserves included).
pub fn total_soldiers(setup: &BattleSetup) -> u32 {
    [&setup.attacker, &setup.defender]
        .iter()
        .flat_map(|s| s.units.iter())
        .map(|u| u.soldiers)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_tier_is_the_standard_field() {
        let rules = BattleScaleRules::bundled();
        let first = &rules.tiers[0];
        assert_eq!(first.field_size(), FieldSize::STANDARD);
        assert_eq!(first.max_regiments_per_side, 40);
        let s = FieldSize::STANDARD;
        assert_eq!(s.attacker_line_z(), 250.0);
        assert_eq!(s.defender_line_z(), 550.0);
        assert_eq!(s.line_half(), 360.0);
        assert_eq!((s.nx(), s.nz()), (121, 81));
        assert_eq!(FieldSize::from_grid(121, 81), s);
    }

    #[test]
    fn tiers_grow_with_the_head_count() {
        let rules = BattleScaleRules::bundled();
        assert_eq!(rules.tier_for(1000).key, "skirmish");
        assert_eq!(rules.tier_for(4000).key, "skirmish");
        assert_eq!(rules.tier_for(4001).key, "large");
        assert_eq!(rules.tier_for(30_000).key, "epic");
        let epic = rules.tier_for(30_000);
        assert!(epic.max_regiments_per_side >= 80);
        assert!(epic.width_m >= 2400.0);
    }
}
