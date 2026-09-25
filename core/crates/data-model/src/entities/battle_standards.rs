//! Regimental standards in battle (lot EP5, ADR 0034), mirroring
//! `data/schemas/battle_standard_rules.schema.json`
//! (`data/rules/battle_standards.json`).

use serde::{Deserialize, Serialize};

/// Contents of `data/rules/battle_standards.json`: when a regiment's
/// standard falls, is raised again or is taken, and what it does to morale
/// and cohesion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleStandardRules {
    /// Regiments of at least this many soldiers (full strength) carry two
    /// standards (rendering: a second bearer; the rules follow the first).
    pub two_bearers_from_soldiers: u32,
    /// The standard may fall only once the regiment is below this share of
    /// its full strength (0-1).
    pub heavy_losses_below: f64,
    /// Chance that the standard falls, per percent of the regiment's full
    /// strength lost in one step once below `heavy_losses_below`.
    pub fall_chance_per_loss_percent: f64,
    /// Chance that the standard is dropped when the regiment routs while
    /// engaged.
    pub rout_drop_chance: f64,
    /// Seconds before a fallen standard is raised again (if the regiment
    /// still holds) or taken (if the enemy holds the spot).
    pub raise_seconds: f64,
    /// An able enemy regiment within this distance (m) of the fallen
    /// standard holds the spot and takes it.
    pub capture_radius_m: f64,
    /// Morale lost at once by the regiment when its standard falls.
    pub fall_morale_shock: f64,
    /// Morale lost per second while the standard is down.
    pub fallen_morale_per_second: f64,
    /// Melee damage dealt while the standard is down (cohesion: the men lose
    /// their rallying point), as a factor (0-1).
    pub fallen_melee_factor: f64,
    /// Morale gained at once by the enemy regiments near a falling standard.
    pub enemy_morale_bonus: f64,
    /// Radius (m) of the enemy bonus and of the dismay of a capture.
    pub enemy_bonus_radius_m: f64,
    /// Morale lost at once by the regiments of the owner near a captured
    /// standard (the regiment itself loses twice as much).
    pub capture_morale_shock: f64,
    /// Morale gained at once by the regiment that takes the standard.
    pub captor_morale_bonus: f64,
    /// Morale effects multiplied by this for the general's banner.
    pub general_factor: f64,
    /// Once taken, the regiment's morale ceiling is lowered by this much for
    /// the rest of the battle.
    pub lost_morale_cap: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Default for BattleStandardRules {
    /// Fallback when `data/rules/battle_standards.json` is absent; kept equal
    /// to that file (checked by `tests/real_data.rs`).
    fn default() -> Self {
        BattleStandardRules {
            two_bearers_from_soldiers: 120,
            heavy_losses_below: 0.6,
            fall_chance_per_loss_percent: 0.03,
            rout_drop_chance: 0.35,
            raise_seconds: 5.0,
            capture_radius_m: 15.0,
            fall_morale_shock: 6.0,
            fallen_morale_per_second: 0.5,
            fallen_melee_factor: 0.85,
            enemy_morale_bonus: 4.0,
            enemy_bonus_radius_m: 60.0,
            capture_morale_shock: 5.0,
            captor_morale_bonus: 8.0,
            general_factor: 2.0,
            lost_morale_cap: 10.0,
            description: None,
        }
    }
}
