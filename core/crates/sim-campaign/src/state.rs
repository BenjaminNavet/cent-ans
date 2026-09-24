//! Campaign state types and read-only queries.
//!
//! Everything here is plain data: the rules that mutate the state live in the
//! sibling modules (`movement`, `siege`, `economy`, ...). All collections are
//! `BTreeMap`/`BTreeSet` so that iteration order, and therefore the simulation,
//! is deterministic.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use data_model::{
    BuildingId, CharacterId, FactionId, GameData, PopulationClasses, ProvinceId, ReligionId,
    ResourceId, SettlementId, SettlementKind, Sex, SkillId, Skills, TechnologyId, TraitId,
    UnitType, UnitTypeId,
};
use serde::{Deserialize, Serialize};

use crate::economy::TaxRate;
use crate::events::GameEvent;
use crate::rng::CampaignRng;

/// Year the campaign starts (spring 1337, Edward III's claim to the French throne).
pub const START_YEAR: i32 = 1337;
/// Number of turns per year (one turn per season).
pub const TURNS_PER_YEAR: u32 = 4;
/// Province steps (v1 unit) an army covers in a spring/summer/autumn turn;
/// movement points are these steps times `MovementRules::points_per_step`.
pub const MAX_MOVEMENT_POINTS: u32 = 3;
/// Province steps in winter (roads impassable, short days).
pub const WINTER_MOVEMENT_POINTS: u32 = 2;
/// Version of the serialised state; bump when the JSON layout changes.
///
/// `2`: M3 cities & economy (buildings, construction, goods, tax rate, the
/// four population gauges are now dynamic). `3`: M4 characters & dynasties
/// (experience, skills, traits, marriage, children, governors, prestige...).
/// `4`: M5 diplomacy & religion (claims, embargoes, vassals, opinion
/// modifiers, war scores, offers, papal favour, schism, heresy), M6
/// technologies (research in progress, progress, banked progress), M7
/// battles (`interactive_battles`, pending battles kept across `end_turn`,
/// `BattleRequest::attacker_origin`), M8 siege supplies and breach, M10
/// outcome. `5`: lot C4 settlements (garrison, siege, buildings,
/// construction and recruitment move from provinces to settlements; armies
/// stand on settlements). `6`: lot M2 free movement (`Army::position`,
/// `movement_left`, `planned_path` replace `location`, `movement_points`,
/// `path`; field battles carry a point).
/// [`CampaignState::load_json`] refuses any other version.
pub const STATE_VERSION: u32 = 6;

/// One of the four seasons; one campaign turn spans one season.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Season {
    Spring,
    Summer,
    Autumn,
    Winter,
}

impl Season {
    /// Seasons in turn order, starting with spring.
    pub const ALL: [Season; 4] = [
        Season::Spring,
        Season::Summer,
        Season::Autumn,
        Season::Winter,
    ];

    /// The season following this one (winter wraps to spring).
    pub fn next(self) -> Season {
        match self {
            Season::Spring => Season::Summer,
            Season::Summer => Season::Autumn,
            Season::Autumn => Season::Winter,
            Season::Winter => Season::Spring,
        }
    }

    /// French display name used by the UI.
    pub fn label_fr(self) -> &'static str {
        match self {
            Season::Spring => "Printemps",
            Season::Summer => "Été",
            Season::Autumn => "Automne",
            Season::Winter => "Hiver",
        }
    }

    /// Province steps granted to every army at the start of a turn (times
    /// `MovementRules::points_per_step` for movement points).
    pub fn movement_steps(self) -> u32 {
        match self {
            Season::Winter => WINTER_MOVEMENT_POINTS,
            _ => MAX_MOVEMENT_POINTS,
        }
    }
}

/// Identifier of an army, `"army_0001"`, `"army_0002"`, ... allocated by the state.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ArmyId(String);

impl ArmyId {
    pub const PREFIX: &'static str = "army_";

    /// Builds the id for the `n`-th army created (`army_0001` for 1).
    pub fn from_index(index: u32) -> Self {
        ArmyId(format!("{}{index:04}", Self::PREFIX))
    }

    /// Parses an existing id such as `"army_0007"`.
    pub fn parse(raw: &str) -> Option<Self> {
        let digits = raw.strip_prefix(Self::PREFIX)?;
        if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
            Some(ArmyId(raw.to_owned()))
        } else {
            None
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ArmyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ArmyId({:?})", self.0)
    }
}

impl fmt::Display for ArmyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for ArmyId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// How an army behaves where it stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stance {
    /// March and fight; do not besiege.
    #[default]
    Normal,
    /// Chevauchée: devastate enemy provinces for loot.
    Raid,
    /// Besiege the enemy settlement the army stands on.
    Siege,
}

/// A regiment of a given type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unit {
    pub unit_type: UnitTypeId,
    /// Current head count.
    pub strength: u32,
    pub max_strength: u32,
    /// 0-10.
    pub experience: u8,
    /// 0-100.
    pub morale: u8,
    /// G1: flat armour bonus of the buildings of the levying province.
    #[serde(default, skip_serializing_if = "is_zero_u8")]
    pub levy_armor: u8,
    /// G1: flat ranged bonus of the buildings of the levying province.
    #[serde(default, skip_serializing_if = "is_zero_u8")]
    pub levy_ranged: u8,
}

fn is_zero_u8(value: &u8) -> bool {
    *value == 0
}

impl Unit {
    /// A freshly recruited, full-strength unit.
    pub fn fresh(unit_type: &UnitType) -> Self {
        Unit {
            unit_type: unit_type.id.clone(),
            strength: unit_type.soldiers,
            max_strength: unit_type.soldiers,
            experience: 0,
            morale: unit_type.stats.morale,
            levy_armor: 0,
            levy_ranged: 0,
        }
    }
}

/// Where an army stands (lot M2, spec § 3.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArmyPosition {
    /// In the field, at a free point of the map (pixels of the 4096² map).
    Field { x: f32, y: f32 },
    /// Stationed in a settlement: garrison, siege or friendly stop.
    Settlement(SettlementId),
}

impl ArmyPosition {
    /// A field position at map pixel `point`.
    pub fn field(point: [f32; 2]) -> Self {
        ArmyPosition::Field {
            x: point[0],
            y: point[1],
        }
    }
}

/// Destination of a move order (lot M2): a point of the map, or a
/// settlement to enter (siege, capture or stop).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MoveTarget {
    Settlement(SettlementId),
    Point { x: f32, y: f32 },
}

/// A field army.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Army {
    pub faction: FactionId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub general: Option<CharacterId>,
    /// Where the army stands (lot M2).
    pub position: ArmyPosition,
    pub units: Vec<Unit>,
    /// Movement points left this turn, in grid costs (10 = one plain cell
    /// of the navigation grid, about 1.44 km).
    pub movement_left: u32,
    /// 0-100.
    pub supply: u8,
    pub stance: Stance,
    /// Corners (grid cells) of the rest of a march spanning several turns.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub planned_path: Vec<crate::navigation::Cell>,
    /// Destination of `planned_path` (a settlement is entered on arrival).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destination: Option<MoveTarget>,
}

impl Army {
    /// A fresh army standing at `position` with no movement left.
    pub fn new(faction: FactionId, position: ArmyPosition, units: Vec<Unit>) -> Self {
        Army {
            faction,
            general: None,
            position,
            units,
            movement_left: 0,
            supply: 100,
            stance: Stance::Normal,
            planned_path: Vec::new(),
            destination: None,
        }
    }

    pub fn total_strength(&self) -> u32 {
        self.units.iter().map(|u| u.strength).sum()
    }

    /// The settlement the army is stationed in, if any.
    pub fn settlement(&self) -> Option<&SettlementId> {
        match &self.position {
            ArmyPosition::Settlement(id) => Some(id),
            ArmyPosition::Field { .. } => None,
        }
    }

    /// `true` when the army is stationed in `settlement`.
    pub fn is_at(&self, settlement: &SettlementId) -> bool {
        self.settlement() == Some(settlement)
    }

    /// Forgets the rest of a multi-turn march.
    pub fn clear_plan(&mut self) {
        self.planned_path.clear();
        self.destination = None;
    }
}

/// An ongoing siege of a settlement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiegeState {
    pub attacker: FactionId,
    /// Estimated turns before the garrison runs out of food and capitulates.
    pub turns_left: u32,
    // ----- M8: siege warfare --------------------------------------------------
    #[serde(default)]
    pub turns_elapsed: u32,
    /// Food left in the besieged town (0-100); capitulation at 0.
    #[serde(default = "full_supplies")]
    pub supplies: u8,
    /// Damage to the walls (0-100) from siege engines; from 50 an assault
    /// no longer suffers the wall penalty.
    #[serde(default)]
    pub breach: u8,
    /// Lot M2: turn the siege began (an army entering the place starts it
    /// at once; it progresses from the next end of turn).
    #[serde(default)]
    pub started_turn: u32,
}

fn full_supplies() -> u8 {
    100
}

/// A building under construction in a settlement (spec § 1.2); one at a time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Construction {
    pub building: BuildingId,
    pub turns_left: u32,
}

/// Dynamic state of a province (static data stays in [`GameData`]).
///
/// Lot C4: the land and the people stay here; owner, controller, garrison,
/// siege, buildings, construction and recruitment belong to the settlements
/// ([`SettlementState`]). The province's owner and controller are those of
/// its city ([`CampaignState::province_owner`],
/// [`CampaignState::province_controller`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvinceState {
    /// The province's city (capital settlement).
    pub city: SettlementId,
    /// Every settlement of the province, the city first.
    pub settlements: Vec<SettlementId>,
    /// 0-100.
    pub unrest: u8,
    /// 0-100, raised by chevauchées.
    pub devastation: u8,
    pub population: PopulationClasses,
    /// Consecutive seasons the weighted-average unrest of the province stayed
    /// above the revolt threshold (spec § 1.1); resets to 0 below it.
    #[serde(default)]
    pub revolt_seasons: u32,
    /// Share (0-100) of the population following `heresy_religion` (M5).
    #[serde(default)]
    pub heresy: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heresy_religion: Option<ReligionId>,
    /// H3 « La Table »: diet chosen by the controller (`None`: the default
    /// `diet_bread_pottage`); see [`CampaignState::province_diet`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diet: Option<crate::table::DietChoice>,
    /// Lot C4: regional edict chosen by the controller (`None`: the default
    /// `edict_none`); see [`CampaignState::province_edict`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edict: Option<crate::edicts::EdictChoice>,
}

/// Dynamic state of a settlement (spec § 4.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettlementState {
    /// Province the settlement belongs to (copied from the data so that the
    /// state answers province queries on its own).
    pub province: ProvinceId,
    pub kind: SettlementKind,
    /// De jure holder.
    pub owner: FactionId,
    /// Faction occupying the settlement.
    pub controller: FactionId,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub garrison: Vec<Unit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub siege: Option<SiegeState>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub buildings: Vec<BuildingId>,
    /// One building under construction at a time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub construction: Option<Construction>,
    /// Units paid for this turn that join the garrison at the end of the turn.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recruit_queue: Vec<UnitTypeId>,
    /// Base fortification level from the data (0 village, 1-4 otherwise).
    #[serde(default)]
    pub fortification_level: u8,
}

impl SettlementState {
    pub fn garrison_strength(&self) -> u32 {
        self.garrison.iter().map(|u| u.strength).sum()
    }
}

/// Dynamic state of a faction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactionState {
    /// Livres tournois; may go negative.
    pub treasury: i64,
    pub income_last_turn: i64,
    pub upkeep_last_turn: i64,
    pub at_war_with: BTreeSet<FactionId>,
    pub allies: BTreeSet<FactionId>,
    /// Truce partner -> turn at which the truce ends.
    pub truces: BTreeMap<FactionId, u32>,
    pub alive: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ruler: Option<CharacterId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heir: Option<CharacterId>,
    pub capital: ProvinceId,
    pub technologies: BTreeSet<TechnologyId>,
    /// Tax bracket in effect (spec § 1.4); `set_tax_rate` changes it.
    #[serde(default)]
    pub tax_rate: TaxRate,
    /// Number of controlled/allied provinces producing each resource (spec § 1.3).
    #[serde(default)]
    pub goods: BTreeMap<ResourceId, u32>,
    /// Cached from the last `resolve_economy`, so [`CampaignState::faction_summary`]
    /// (which does not take [`GameData`]) can still report them.
    #[serde(default)]
    pub army_upkeep_last_turn: i64,
    #[serde(default)]
    pub building_upkeep_last_turn: i64,
    #[serde(default)]
    pub projected_income: i64,
    /// A regency governs for a minor ruler (M4 spec § 2); tracked so the
    /// journal reports its start and end once instead of every turn.
    #[serde(default)]
    pub regency: bool,

    // ----- M5: diplomacy & religion ----------------------------------------
    /// Factions this faction imposes an embargo on.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub embargoes: BTreeSet<FactionId>,
    /// Overlord of a vassal faction (also listed in `allies`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suzerain: Option<FactionId>,
    /// Loyalty (0-100) of a vassal towards its suzerain.
    #[serde(default = "default_faction_loyalty")]
    pub loyalty: u8,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub claims: Vec<crate::diplomacy::Claim>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modifiers: Vec<crate::diplomacy::OpinionModifier>,
    /// Battle part of the war score against each enemy (-100..100).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub war_scores: BTreeMap<FactionId, i32>,
    /// Turn at which each ongoing war started (war weariness).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub war_started: BTreeMap<FactionId, u32>,
    /// State religion or obedience (changes at the Great Schism).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub religion: Option<ReligionId>,
    /// 0-100.
    #[serde(default = "default_papal_favor")]
    pub papal_favor: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub excommunicated_until: Option<u32>,
    /// Proposals received by the player, answered with `answer_offer`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub offers: Vec<crate::diplomacy::Offer>,
    /// Last turn an AI faction sent an offer to the player (throttling).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub last_offer_turn: BTreeMap<FactionId, u32>,
    /// Last turn this faction declared a war (AI throttling).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_war_declared: Option<u32>,
    // ----- M6: research (spec § 2, `research.rs`) --------------------------
    /// Technology being researched, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub research: Option<TechnologyId>,
    /// Points accumulated towards `research`; while no research runs, the
    /// surplus of the last completed technology, carried over to the next (F1).
    #[serde(default)]
    pub research_progress: u32,
    /// Points produced during the last resolved turn.
    #[serde(default)]
    pub research_points_last_turn: u32,
    /// Progress kept for abandoned research (switching back resumes it).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub research_banked: BTreeMap<TechnologyId, u32>,
    // ----- H3: La Table -------------------------------------------------------
    /// Diets paid during the last resolved turn (budget line « Table »).
    #[serde(default)]
    pub table_upkeep_last_turn: i64,
    // ----- H5: coinage (`coinage.rs`) ---------------------------------------
    /// Silver content of the faction's coins.
    #[serde(default)]
    pub coinage: crate::coinage::CoinageLevel,
    /// Price level in per cent of the 1337 prices (100 = base); scales
    /// recruitment, upkeep and construction.
    #[serde(default = "crate::coinage::default_price_level")]
    pub price_level: u32,
    /// Year of the last `set_coinage` (one change per year).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coinage_changed_year: Option<i32>,
    /// Seigniorage collected during the last resolved turn.
    #[serde(default)]
    pub seigniorage_last_turn: i64,
    /// Recoinage (strong money) paid during the last resolved turn.
    #[serde(default)]
    pub recoinage_last_turn: i64,
    // ----- H6: ransoms and chivalric orders -----------------------------------
    /// Ransoms being paid by installments (this faction owes them).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ransom_debts: Vec<crate::ransom::RansomDebt>,
    /// The chivalric order founded by the faction (at most one).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chivalric_order: Option<crate::chivalry::OrderState>,
    // ----- C5: trade (`trade.rs`) --------------------------------------------
    /// Factions this faction has a formal trade agreement with (mirrored on
    /// both sides); see [`CampaignState::has_trade_agreement`].
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub trade_agreements: BTreeSet<FactionId>,
    /// Trade income collected during the last resolved turn.
    #[serde(default)]
    pub trade_income_last_turn: i64,
}

fn default_faction_loyalty() -> u8 {
    100
}

fn default_papal_favor() -> u8 {
    50
}

/// Dynamic state of a character.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterState {
    pub faction: FactionId,
    pub alive: bool,
    pub birth_year: i32,
    pub sex: Sex,
    pub house: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<ProvinceId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub army: Option<ArmyId>,
    pub skills: Skills,
    pub captive: bool,
    /// F1: faction holding the character captive (battle or chronicle
    /// capture); it receives the ransom.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub captor: Option<FactionId>,

    // ----- M4: characters & dynasties (spec § 2) ---------------------------
    /// Display name of a generated character (historical ones take theirs
    /// from `data.characters`); see [`CampaignState::character_name`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Accumulated experience not yet converted into a skill point.
    #[serde(default)]
    pub experience: u32,
    /// Unspent skill points.
    #[serde(default)]
    pub skill_points: u32,
    /// `data.skills` ids learned (`learn_skill`).
    #[serde(default)]
    pub skills_learned: BTreeSet<SkillId>,
    #[serde(default)]
    pub traits: BTreeSet<TraitId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spouse: Option<CharacterId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<CharacterId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub father: Option<CharacterId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mother: Option<CharacterId>,
    /// 0-100.
    #[serde(default)]
    pub piety: u8,
    #[serde(default)]
    pub prestige: i32,
    /// 0-100 (vassal loyalty, mostly relevant from M5).
    #[serde(default = "default_loyalty")]
    pub loyalty: u8,
    /// Principal title displayed by the UI (the first title with no `to`
    /// date in the static `Character::titles`, spec § 2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub governor_of: Option<ProvinceId>,

    /// Battles fought (general), for the `trait_veteran` trigger (spec § 2:
    /// after 5 battles).
    #[serde(default)]
    pub battles_fought: u32,
    /// Sieges won as the besieging general, for `trait_siege_master` (after 3).
    #[serde(default)]
    pub sieges_won: u32,
    /// Chevauchées led, for `trait_cruel` (after 3).
    #[serde(default)]
    pub raids_led: u32,
    /// H6: terms set by the captor while the character is captive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ransom_terms: Option<crate::ransom::RansomTerms>,
    /// C7: year of death, set by `characters::kill` (and at setup for the
    /// characters already dead in 1337). `None` while alive, and for saves
    /// written before C7.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub death_year: Option<i32>,
    /// C7: the character's retinue (`data/retinue.json` companion ids), in
    /// order of arrival; see `crate::retinue`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub retinue: Vec<data_model::CompanionId>,
}

fn default_loyalty() -> u8 {
    100
}

fn default_interactive_battles() -> bool {
    true
}

impl CampaignState {
    /// Display name of a character: the static historical name, else the
    /// generated one, else the raw id.
    pub fn character_name(&self, data: &GameData, id: &CharacterId) -> String {
        data.characters
            .get(id)
            .map(|c| c.name.display.clone())
            .or_else(|| self.characters.get(id).and_then(|c| c.name.clone()))
            .unwrap_or_else(|| id.to_string())
    }
}

impl CharacterState {
    pub fn age(&self, year: i32) -> i32 {
        year - self.birth_year
    }

    /// `true` once the character has reached majority (spec § 2: 15 years).
    pub fn is_major(&self, year: i32) -> bool {
        self.age(year) >= crate::dynasty::MAJORITY_AGE
    }
}

/// A player battle awaiting resolution (M7): fought in 3D or auto-resolved
/// before the next `end_turn`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleRequest {
    pub attacker: ArmyId,
    pub defender: ArmyId,
    /// Settlement where the battle takes place (the nearest one to a field
    /// battle, lot M2).
    pub location: SettlementId,
    /// Its province (terrain, names).
    pub province: ProvinceId,
    /// M8: an assault on the settlement `location`; the defender is its
    /// garrison (`defender` then repeats the attacker's id).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub siege: bool,
}

/// Aggregated view of a faction for the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactionSummary {
    pub treasury: i64,
    pub income: i64,
    pub upkeep: i64,
    pub at_war_with: Vec<FactionId>,
    pub allies: Vec<FactionId>,
    pub provinces_count: usize,
    pub armies_count: usize,
    pub alive: bool,
    pub ruler: Option<CharacterId>,
    /// Income the faction would collect next turn if nothing changes (spec § 1.4).
    pub projected_income: i64,
    pub army_upkeep: i64,
    pub building_upkeep: i64,
    pub tax_rate: TaxRate,
}

/// Full state of a campaign at a given turn.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CampaignState {
    /// Layout version of the serialised state (see [`STATE_VERSION`]).
    pub state_version: u32,
    /// Zero-based turn counter; turn 0 is spring 1337.
    pub turn: u32,
    pub season: Season,
    pub year: i32,
    pub seed: u64,
    pub rng: CampaignRng,
    pub player_faction: FactionId,
    pub provinces: BTreeMap<ProvinceId, ProvinceState>,
    /// Settlements inside provinces (lot C4: they hold garrisons, sieges,
    /// buildings, construction and recruitment).
    pub settlements: BTreeMap<SettlementId, SettlementState>,
    pub factions: BTreeMap<FactionId, FactionState>,
    pub armies: BTreeMap<ArmyId, Army>,
    pub characters: BTreeMap<CharacterId, CharacterState>,
    /// Journal of the last resolved turn.
    pub events: Vec<GameEvent>,
    pub pending_battles: Vec<BattleRequest>,
    /// Player setting (M7): battles involving the player wait in
    /// `pending_battles` for the 3D battle instead of being auto-resolved.
    #[serde(default = "default_interactive_battles")]
    pub interactive_battles: bool,
    pub(crate) next_army_index: u32,
    /// Great Western Schism in progress (M5, 1378-1417).
    #[serde(default)]
    pub schism: bool,
    #[serde(default)]
    pub(crate) next_offer_id: u32,
    /// Events produced by orders (they apply immediately); they open the
    /// next turn's journal.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending_events: Vec<GameEvent>,
    /// Chronicle events fired, player decisions, Black Death wave (M10).
    #[serde(default)]
    pub chronicle: crate::chronicle::ChronicleState,
    /// The player's campaign outcome, once reached (M10).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<crate::victory::Outcome>,
    /// Consecutive seasons the player has met all objectives (F9).
    #[serde(default)]
    pub victory_streak: u32,
    /// Lot C6: spies, heralds and preachers (absent from older saves; no
    /// change of [`STATE_VERSION`]).
    #[serde(default)]
    pub agents: crate::agents::AgentsState,
}

impl CampaignState {
    /// Creates an empty campaign at spring 1337 (no provinces, no factions).
    ///
    /// Legacy constructor kept for the M1 bridge; new code uses
    /// [`CampaignState::new_1337`].
    pub fn new(seed: u64) -> Self {
        CampaignState::empty(FactionId::new("fac_france").expect("well-formed id"), seed)
    }

    pub(crate) fn empty(player: FactionId, seed: u64) -> Self {
        CampaignState {
            state_version: STATE_VERSION,
            turn: 0,
            season: Season::Spring,
            year: START_YEAR,
            seed,
            rng: CampaignRng::from_seed(seed),
            player_faction: player,
            provinces: BTreeMap::new(),
            settlements: BTreeMap::new(),
            factions: BTreeMap::new(),
            armies: BTreeMap::new(),
            characters: BTreeMap::new(),
            events: Vec::new(),
            pending_battles: Vec::new(),
            interactive_battles: true,
            next_army_index: 1,
            schism: false,
            next_offer_id: 1,
            pending_events: Vec::new(),
            chronicle: crate::chronicle::ChronicleState::default(),
            outcome: None,
            victory_streak: 0,
            agents: crate::agents::AgentsState::default(),
        }
    }

    // ----- date -----------------------------------------------------------

    /// Human-readable date in French, e.g. `"Printemps 1337"`.
    pub fn date_label(&self) -> String {
        format!("{} {}", self.season.label_fr(), self.year)
    }

    pub fn turn(&self) -> u32 {
        self.turn
    }

    pub fn season(&self) -> Season {
        self.season
    }

    pub fn year(&self) -> i32 {
        self.year
    }

    pub fn player_faction(&self) -> &FactionId {
        &self.player_faction
    }

    pub(crate) fn advance_date(&mut self) {
        self.turn += 1;
        if self.season == Season::Winter {
            self.year += 1;
        }
        self.season = self.season.next();
    }

    // ----- queries --------------------------------------------------------

    pub fn events(&self) -> &[GameEvent] {
        &self.events
    }

    pub fn army(&self, id: &ArmyId) -> Option<&Army> {
        self.armies.get(id)
    }

    pub fn armies(&self) -> &BTreeMap<ArmyId, Army> {
        &self.armies
    }

    pub fn province_state(&self, id: &ProvinceId) -> Option<&ProvinceState> {
        self.provinces.get(id)
    }

    pub fn settlement_state(&self, id: &SettlementId) -> Option<&SettlementState> {
        self.settlements.get(id)
    }

    /// Settlements of `province` with their state, the city first (order of
    /// `GameData::settlements_by_province`).
    pub fn province_settlements<'a>(
        &'a self,
        data: &'a GameData,
        province: &ProvinceId,
    ) -> Vec<(&'a SettlementId, &'a SettlementState)> {
        data.settlements_by_province
            .get(province)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.settlements.get_key_value(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn faction_state(&self, id: &FactionId) -> Option<&FactionState> {
        self.factions.get(id)
    }

    pub fn character(&self, id: &CharacterId) -> Option<&CharacterState> {
        self.characters.get(id)
    }

    pub fn faction_summary(&self, id: &FactionId) -> Option<FactionSummary> {
        let faction = self.factions.get(id)?;
        Some(FactionSummary {
            treasury: faction.treasury,
            income: faction.income_last_turn,
            upkeep: faction.upkeep_last_turn,
            at_war_with: faction.at_war_with.iter().cloned().collect(),
            allies: faction.allies.iter().cloned().collect(),
            provinces_count: self.controlled_provinces(id).len(),
            armies_count: self.armies.values().filter(|a| &a.faction == id).count(),
            alive: faction.alive,
            ruler: faction.ruler.clone(),
            projected_income: faction.projected_income,
            army_upkeep: faction.army_upkeep_last_turn,
            building_upkeep: faction.building_upkeep_last_turn,
            tax_rate: faction.tax_rate,
        })
    }

    /// Ids of the armies standing in `province` (in one of its settlements
    /// or in the field inside it), in id order.
    pub fn armies_in(&self, data: &GameData, province: &ProvinceId) -> Vec<ArmyId> {
        self.armies
            .iter()
            .filter(|(_, army)| self.army_province(data, army).as_ref() == Some(province))
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Ids of the armies stationed in `settlement`, in id order.
    pub fn armies_at(&self, settlement: &SettlementId) -> Vec<ArmyId> {
        self.armies
            .iter()
            .filter(|(_, army)| army.is_at(settlement))
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// The id the next created army will receive (useful for planners that
    /// chain a `create_army` with a `merge_armies` in the same turn).
    pub fn peek_next_army_id(&self) -> ArmyId {
        ArmyId::from_index(self.next_army_index)
    }

    pub(crate) fn allocate_army_id(&mut self) -> ArmyId {
        let id = ArmyId::from_index(self.next_army_index);
        self.next_army_index += 1;
        id
    }

    // ----- diplomacy helpers ----------------------------------------------

    pub fn is_at_war(&self, a: &FactionId, b: &FactionId) -> bool {
        a != b
            && self
                .factions
                .get(a)
                .is_some_and(|f| f.at_war_with.contains(b))
    }

    pub fn is_allied(&self, a: &FactionId, b: &FactionId) -> bool {
        a == b || self.factions.get(a).is_some_and(|f| f.allies.contains(b))
    }

    /// Lot C5: a formal trade agreement is active between `a` and `b` (also
    /// `true` for a faction and itself, its own trade always flows). It is
    /// never stored as broken: war or an embargo between the two just makes
    /// [`crate::trade::trade_routes`] ignore it for the season, so it comes
    /// back on its own the moment peace and embargoes are lifted.
    pub fn has_trade_agreement(&self, a: &FactionId, b: &FactionId) -> bool {
        a == b
            || self
                .factions
                .get(a)
                .is_some_and(|f| f.trade_agreements.contains(b))
    }

    /// `true` when `province` is controlled by `faction` or one of its allies.
    pub fn is_friendly_territory(&self, faction: &FactionId, province: &ProvinceId) -> bool {
        self.province_controller(province)
            .is_some_and(|c| self.is_allied(faction, c))
    }

    /// `true` when `province` is controlled by a faction `faction` is at war with.
    pub fn is_hostile_territory(&self, faction: &FactionId, province: &ProvinceId) -> bool {
        self.province_controller(province)
            .is_some_and(|c| self.is_at_war(faction, c))
    }

    /// `true` when `settlement` is held by `faction` or one of its allies.
    pub fn is_friendly_settlement(&self, faction: &FactionId, settlement: &SettlementId) -> bool {
        self.settlements
            .get(settlement)
            .is_some_and(|s| self.is_allied(faction, &s.controller))
    }

    /// `true` when `settlement` is held by a faction `faction` is at war with.
    pub fn is_hostile_settlement(&self, faction: &FactionId, settlement: &SettlementId) -> bool {
        self.settlements
            .get(settlement)
            .is_some_and(|s| self.is_at_war(faction, &s.controller))
    }

    /// Ids of the armies stationed in `settlement` whose faction is at war
    /// with `faction`.
    pub fn hostile_armies_at(&self, faction: &FactionId, settlement: &SettlementId) -> Vec<ArmyId> {
        self.armies
            .iter()
            .filter(|(_, army)| army.is_at(settlement) && self.is_at_war(faction, &army.faction))
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Ids of the armies on `settlement` allied with (or belonging to) `faction`.
    pub fn friendly_armies_at(
        &self,
        faction: &FactionId,
        settlement: &SettlementId,
    ) -> Vec<ArmyId> {
        self.armies
            .iter()
            .filter(|(_, army)| army.is_at(settlement) && self.is_allied(faction, &army.faction))
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Ids of the armies in `province` whose faction is at war with `faction`.
    pub fn hostile_armies_in(
        &self,
        data: &GameData,
        faction: &FactionId,
        province: &ProvinceId,
    ) -> Vec<ArmyId> {
        self.armies
            .iter()
            .filter(|(_, army)| {
                self.is_at_war(faction, &army.faction)
                    && self.army_province(data, army).as_ref() == Some(province)
            })
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Ids of the armies in `province` allied with (or belonging to) `faction`.
    pub fn friendly_armies_in(
        &self,
        data: &GameData,
        faction: &FactionId,
        province: &ProvinceId,
    ) -> Vec<ArmyId> {
        self.armies
            .iter()
            .filter(|(_, army)| {
                self.is_allied(faction, &army.faction)
                    && self.army_province(data, army).as_ref() == Some(province)
            })
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Sum of `strength × melee` over the garrisons of the settlements held
    /// by the province's controller and its friendly armies there: a cheap
    /// defensive-power estimate used by planners.
    pub fn defensive_power(&self, data: &GameData, province: &ProvinceId) -> f64 {
        let Some(controller) = self.province_controller(province) else {
            return 0.0;
        };
        let garrisons: f64 = self
            .settlements_of(province)
            .filter(|(_, s)| &s.controller == controller)
            .map(|(_, s)| unit_power(data, &s.garrison))
            .sum();
        let field: f64 = self
            .armies
            .values()
            .filter(|a| {
                self.is_allied(controller, &a.faction)
                    && self.army_province(data, a).as_ref() == Some(province)
            })
            .map(|a| unit_power(data, &a.units))
            .sum();
        garrisons + field
    }

    /// Defensive power of one settlement: its garrison plus the armies
    /// allied with its controller standing on it.
    pub fn settlement_defensive_power(&self, data: &GameData, settlement: &SettlementId) -> f64 {
        let Some(s) = self.settlements.get(settlement) else {
            return 0.0;
        };
        let field: f64 = self
            .armies
            .values()
            .filter(|a| a.is_at(settlement) && self.is_allied(&s.controller, &a.faction))
            .map(|a| unit_power(data, &a.units))
            .sum();
        unit_power(data, &s.garrison) + field
    }

    /// Rough offensive-power estimate of an army (`strength × melee`).
    pub fn army_power(&self, data: &GameData, army: &ArmyId) -> f64 {
        self.armies
            .get(army)
            .map_or(0.0, |a| unit_power(data, &a.units))
    }
}

/// `Σ strength × max(melee, ranged) / 100`.
pub fn unit_power(data: &GameData, units: &[Unit]) -> f64 {
    units
        .iter()
        .map(|unit| {
            let attack = data
                .unit_types
                .get(&unit.unit_type)
                .map_or(30.0, |t| f64::from(t.stats.melee.max(t.stats.ranged)));
            f64::from(unit.strength) * attack / 100.0
        })
        .sum()
}
