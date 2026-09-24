//! Battle setup: everything the campaign hands to the battle (spec § 1, § 2).
//!
//! The setup is plain serde data so that it crosses the GDExtension boundary
//! as a `Dictionary` (via JSON) and can be stored in tests as fixtures.

use data_model::{Ability, BattleOrder, Terrain, UnitCategory, UnitStats, UnitType};
use serde::{Deserialize, Serialize};

/// One of the two sides of a battle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SideId {
    Attacker,
    Defender,
}

impl SideId {
    pub const BOTH: [SideId; 2] = [SideId::Attacker, SideId::Defender];

    pub fn other(self) -> SideId {
        match self {
            SideId::Attacker => SideId::Defender,
            SideId::Defender => SideId::Attacker,
        }
    }

    pub fn index(self) -> usize {
        match self {
            SideId::Attacker => 0,
            SideId::Defender => 1,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            SideId::Attacker => "attacker",
            SideId::Defender => "defender",
        }
    }

    /// Parses `"attacker"` / `"defender"`.
    pub fn parse(raw: &str) -> Option<SideId> {
        match raw {
            "attacker" => Some(SideId::Attacker),
            "defender" => Some(SideId::Defender),
            _ => None,
        }
    }
}

/// Season of the campaign turn (drives the weather draw).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BattleSeason {
    #[default]
    Spring,
    Summer,
    Autumn,
    Winter,
}

impl BattleSeason {
    pub fn label_fr(self) -> &'static str {
        match self {
            BattleSeason::Spring => "printemps",
            BattleSeason::Summer => "été",
            BattleSeason::Autumn => "automne",
            BattleSeason::Winter => "hiver",
        }
    }
}

/// A regiment as recruited in the campaign.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnitSetup {
    pub unit_type: String,
    /// French display name of the unit type.
    pub name: String,
    pub category: UnitCategory,
    #[serde(default)]
    pub mounted: bool,
    /// Living soldiers (the campaign strength is a head count).
    pub soldiers: u32,
    /// Full-strength head count.
    pub max_soldiers: u32,
    /// 0-100, campaign morale.
    pub morale: u8,
    /// 0-10.
    #[serde(default)]
    pub experience: u8,
    pub stats: UnitStats,
    #[serde(default)]
    pub abilities: Vec<Ability>,
}

impl UnitSetup {
    /// A unit of `unit_type` with `soldiers` living soldiers.
    pub fn from_unit_type(unit_type: &UnitType, soldiers: u32, morale: u8, experience: u8) -> Self {
        UnitSetup {
            unit_type: unit_type.id.to_string(),
            name: unit_type.name.display.clone(),
            category: unit_type.category,
            mounted: unit_type.mounted,
            soldiers,
            max_soldiers: unit_type.soldiers.max(soldiers),
            morale,
            experience,
            stats: unit_type.stats.clone(),
            abilities: unit_type.abilities.clone(),
        }
    }
}

/// The campaign general of a side and the battle effects of his traits and
/// skills (M4 `character_effects`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneralSetup {
    pub character: String,
    pub name: String,
    /// Command skill, 0-10.
    pub command: u8,
    /// Index in [`SideSetup::units`] of the regiment carrying the general.
    pub unit_index: usize,
    /// Flat morale bonus (`ArmyMorale`).
    #[serde(default)]
    pub morale_bonus: f64,
    /// Percent bonus to the charge (`BattleCharge`).
    #[serde(default)]
    pub charge_percent: f64,
    /// Percent bonus to shooting (`BattleRanged`).
    #[serde(default)]
    pub ranged_percent: f64,
    /// Armour points added to every unit (`BattleDefense`).
    #[serde(default)]
    pub defense_percent: f64,
}

/// One army.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SideSetup {
    pub faction: String,
    /// Short French name of the faction ("France", "Angleterre").
    pub faction_name: String,
    #[serde(default)]
    pub army: String,
    pub units: Vec<UnitSetup>,
    #[serde(default)]
    pub general: Option<GeneralSetup>,
}

/// Siege battle parameters (M8 § 2): the defender holds a walled town.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SiegeSetup {
    /// Campaign fortification level (0-3): wall thickness, height and HP.
    pub fortification: u32,
    /// Campaign wall damage (0-100): from 50 a breach is already open.
    #[serde(default)]
    pub breach: u8,
}

/// Full description of a battle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BattleSetup {
    #[serde(default)]
    pub province: String,
    #[serde(default)]
    pub province_name: String,
    pub terrain: Terrain,
    /// The province has a river: the field gets a river with fords.
    #[serde(default)]
    pub river: bool,
    #[serde(default)]
    pub season: BattleSeason,
    /// The province touches the sea: the field may have a coast on a flank
    /// (B5).
    #[serde(default)]
    pub coastal: bool,
    /// Village or farm on the field (B5): `Some(true)` forces one,
    /// `Some(false)` forbids it, `None` draws it from the terrain.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub village: Option<bool>,
    pub attacker: SideSetup,
    pub defender: SideSetup,
    /// Side commanded by the player; the other one (both when `None`) is
    /// driven by the battle AI.
    #[serde(default)]
    pub player_side: Option<SideId>,
    /// Siege battle: the defender holds the town walls (M8 § 2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub siege: Option<SiegeSetup>,
    /// Catalogue of the leader's orders (`data/battle_orders/`, F10b); no
    /// order can be given when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub orders: Vec<BattleOrder>,
}

impl BattleSetup {
    /// The campaign site of the battle (B5). Sieges have no coast and no
    /// village on the field (the town is the settlement).
    pub fn field_site(&self) -> crate::site::FieldSite {
        let siege = self.siege.is_some();
        crate::site::FieldSite {
            terrain: self.terrain,
            river: self.river && !siege,
            coastal: self.coastal && !siege,
            season: self.season,
            village: if siege { Some(false) } else { self.village },
        }
    }

    pub fn side(&self, side: SideId) -> &SideSetup {
        match side {
            SideId::Attacker => &self.attacker,
            SideId::Defender => &self.defender,
        }
    }
}
