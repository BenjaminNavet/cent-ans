//! Battle setup: everything the campaign hands to the battle (spec § 1, § 2).
//!
//! The setup is plain serde data so that it crosses the GDExtension boundary
//! as a `Dictionary` (via JSON) and can be stored in tests as fixtures.

use data_model::{Ability, BattleOrder, Missile, Terrain, UnitCategory, UnitStats, UnitType};
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
    /// Missile loosed by a shooting unit (lot UR2: data-driven; `None` falls
    /// back to the id/ability heuristic in `sim.rs::missile_kind`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub missile: Option<Missile>,
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
            missile: unit_type.missile,
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
    /// EP5: the general is his faction's ruler in person (royal banner,
    /// the oriflamme of Saint-Denis for France).
    #[serde(default)]
    pub sovereign: bool,
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
    /// CV3: the army was in forced march (campaign stance) when the battle
    /// began: no free deployment phase (automatic placement).
    #[serde(default, skip_serializing_if = "is_false")]
    pub forced_march: bool,
    /// CV3: the army was entrenched (campaign stance): stakes and a low
    /// palisade are ready at the start of the battle.
    #[serde(default, skip_serializing_if = "is_false")]
    pub entrenched: bool,
    /// CV3: fatigue (0-100 gauge of `Unit::fatigue`) every regiment starts
    /// the battle with (`postures.json` `forced_march.start_fatigue` for a
    /// side in forced march, 0 otherwise).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub start_fatigue: f64,
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn is_zero(value: &f64) -> bool {
    *value == 0.0
}

/// CV3: how the battle opens (spec campagne vivante § 1.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum BattleOpening {
    /// Both sides deploy as usual.
    #[default]
    Standard,
    /// The `victim` side is caught in marching column along the road, with
    /// no deployment phase; the other side deploys on the flanks.
    Ambush { victim: SideId },
}

impl BattleOpening {
    pub fn is_standard(&self) -> bool {
        matches!(self, BattleOpening::Standard)
    }

    /// The ambushed side, if any.
    pub fn ambush_victim(&self) -> Option<SideId> {
        match self {
            BattleOpening::Standard => None,
            BattleOpening::Ambush { victim } => Some(*victim),
        }
    }
}

/// Siege battle parameters (M8 § 2): the defender holds a walled town.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SiegeSetup {
    /// Campaign fortification level (0-3): wall thickness, height and HP.
    pub fortification: u32,
    /// Campaign wall damage (0-100): from 50 a breach is already open.
    #[serde(default)]
    pub breach: u8,
    /// NT1 (ADR 0126): kind of place (ring city, fortified borough,
    /// castle); a landmark plan (`siege_layout`) takes precedence.
    #[serde(
        default,
        skip_serializing_if = "crate::siege_layouts::PlaceKind::is_city"
    )]
    pub place: crate::siege_layouts::PlaceKind,
    /// NT5 (N7): the engines the besiegers built during the campaign siege.
    /// `None` (older replays, hand-made setups): the ram every besieging
    /// army brings and ladders for every foot regiment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engines: Option<SiegeEngineSetup>,
}

impl SiegeSetup {
    /// The attacker brings a battering ram.
    pub fn has_ram(&self) -> bool {
        self.engines.as_ref().is_none_or(|e| e.ram)
    }

    /// The attacker's foot may scale the walls with ladders.
    pub fn has_ladders(&self) -> bool {
        self.engines.as_ref().is_none_or(|e| e.ladders)
    }

    /// Siege towers built on the spot (battle-only regiments).
    pub fn built_towers(&self) -> &[UnitSetup] {
        self.engines.as_ref().map_or(&[], |e| e.towers.as_slice())
    }
}

/// NT5 (N7): engines built on the spot during a campaign siege
/// (`data/rules/siege_engines.json`), brought to the siege battle. They are
/// battle-only: no losses are reported to the campaign for them.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SiegeEngineSetup {
    /// A battering ram against the gate.
    #[serde(default)]
    pub ram: bool,
    /// Ladders: foot regiments may scale intact walls.
    #[serde(default)]
    pub ladders: bool,
    /// Siege towers, one regiment each (the tower unit type's stats).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub towers: Vec<UnitSetup>,
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
    /// Siege in a landmark city (L3, ADR 0026): the besieged town is drawn
    /// from the plan instead of the generic octagon. Ignored without `siege`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub siege_layout: Option<crate::siege_layout::SiegeLayout>,
    /// Catalogue of the leader's orders (`data/battle_orders/`, F10b); no
    /// order can be given when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub orders: Vec<BattleOrder>,
    /// CB4: catalogue of the regiments' active abilities
    /// (`data/battle_abilities/`); no ability can be used when empty (older
    /// replays and hand-made setups).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub abilities: Vec<data_model::BattleAbility>,
    /// Rules of the regimental standards (`data/rules/battle_standards.json`,
    /// EP5); [`data_model::BattleStandardRules::default`] when `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standards: Option<data_model::BattleStandardRules>,
    /// EP6: hand-made decor (EP7 historical maps), applied over the
    /// procedural one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decor_plan: Option<crate::decor::DecorPlan>,
    /// CV3: ambush or standard opening (`Standard` for old battles and
    /// replays).
    #[serde(default, skip_serializing_if = "BattleOpening::is_standard")]
    pub opening: BattleOpening,
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
