//! Automatic battle resolution (spec § 1.4, refounded by lot N1).
//!
//! A battle is fought in phases between unit families, with the
//! coefficients of `data/rules/auto_resolve.json`
//! ([`AutoResolveRules`]):
//!
//! 1. **Volleys**: shooters fire before the lines meet; armour and pavises
//!    blunt the shots, rain slackens bowstrings, fog shortens the range.
//! 2. **Charge**: cavalry charges; it crushes light foot and shooters caught
//!    in the open (flanks), pikes and stakes break it (pikes strike back),
//!    mud and broken ground slow it.
//! 3. **Melee rounds**: foot and horse fight, armour stopping most blows;
//!    shooters screened by their own front line are hardly reached, and
//!    keep shooting.
//!
//! After every phase each side loses morale in proportion to its losses; a
//! side whose morale falls below the break point flees and is pursued (more
//! so by horsemen). Losses fall where the blows fell: a knight in plate
//! loses fewer men than a militiaman. See ADR 0013.
//!
//! [`resolve_auto`] is the pure entry point without unit profiles (every
//! melee unit is counted as foot); [`resolve_with`] takes the profiles, the
//! field conditions and the rules; [`resolve_field`] builds them from the
//! campaign state.

use data_model::{
    Ability, AutoResolveRules, GameData, Province, RiverCrossingRules, Terrain, UnitCategory,
    UnitType, WeatherChances,
};
use serde::{Deserialize, Serialize};
use sim_battle::Weather;

use crate::rng::CampaignRng;
use crate::state::{ArmyId, CampaignState, Season};

/// One unit as seen by the battle resolver (no id: purely numbers).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleUnit {
    pub strength: u32,
    pub max_strength: u32,
    /// 0-10.
    pub experience: u8,
    /// 0-100.
    pub morale: u8,
    pub melee: u8,
    pub ranged: u8,
    pub armor: u8,
    /// Uses `ranged` instead of `melee` as its attack value.
    pub is_ranged: bool,
}

/// Family of a unit in the auto-resolve (lot N1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitFamily {
    /// Foot fighting hand to hand (militia, men-at-arms).
    #[default]
    Infantry,
    /// Foot with `pike_square`: breaks charges.
    Pikes,
    /// Foot shooters (bows, crossbows).
    Shooters,
    /// Mounted shooters (`skirmish`): hard to catch.
    HorseArchers,
    /// Horsemen with a charge.
    Cavalry,
    /// Engines: shoot, never fight.
    Siege,
}

/// What the auto-resolve needs to know of a unit type beyond its numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct UnitProfile {
    pub family: UnitFamily,
    /// `stats.charge` (0 when none).
    pub charge: u8,
    /// Plants stakes when defending (`stakes`).
    pub stakes: bool,
    /// Shelters behind pavises (`pavise`).
    pub pavise: bool,
    /// Shoots worse in the rain (`rain_penalty`).
    pub rain_penalty: bool,
}

impl UnitProfile {
    /// Profile of a unit type (family from category, mount and abilities).
    pub fn of(unit_type: &UnitType) -> Self {
        let has = |ability: Ability| unit_type.abilities.contains(&ability);
        let family = match unit_type.category {
            UnitCategory::Siege => UnitFamily::Siege,
            UnitCategory::Ranged if unit_type.mounted => UnitFamily::HorseArchers,
            UnitCategory::Ranged => UnitFamily::Shooters,
            UnitCategory::Cavalry => UnitFamily::Cavalry,
            _ if has(Ability::PikeSquare) => UnitFamily::Pikes,
            _ => UnitFamily::Infantry,
        };
        UnitProfile {
            family,
            charge: unit_type.stats.charge.unwrap_or(0),
            stakes: has(Ability::Stakes),
            pavise: has(Ability::Pavise),
            rain_penalty: has(Ability::RainPenalty),
        }
    }

    /// Profile guessed from the numbers alone: shooters or foot.
    pub fn infer(unit: &BattleUnit) -> Self {
        UnitProfile {
            family: if unit.is_ranged {
                UnitFamily::Shooters
            } else {
                UnitFamily::Infantry
            },
            ..UnitProfile::default()
        }
    }

    fn is_shooter(self) -> bool {
        matches!(
            self.family,
            UnitFamily::Shooters | UnitFamily::HorseArchers | UnitFamily::Siege
        )
    }

    fn is_front(self) -> bool {
        matches!(
            self.family,
            UnitFamily::Infantry | UnitFamily::Pikes | UnitFamily::Cavalry
        )
    }
}

/// One army in a battle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Side {
    pub units: Vec<BattleUnit>,
    /// Command skill of the general (0 when none).
    pub general_command: u8,
    /// 0-100.
    pub supply: u8,
    /// Flat morale bonus from the general's traits/skills (spec § 2
    /// `ArmyMorale`).
    pub general_morale_bonus: f64,
    /// Percent bonus to melee power from the general's traits/skills (spec §
    /// 2 `BattleCharge`).
    pub general_charge_percent: f64,
    /// Percent bonus to ranged power (spec § 2 `BattleRanged`).
    pub general_ranged_percent: f64,
    /// Percent reduction to incoming damage, folded into effective armour
    /// (spec § 2 `BattleDefense`).
    pub general_defense_percent: f64,
    /// F1: the general's `Intrigue` (spies, ruses): raises the chance of
    /// capturing a beaten enemy general, lowers the chance of being taken.
    #[serde(default)]
    pub general_intrigue: f64,
}

impl Default for Side {
    fn default() -> Self {
        Side {
            units: Vec::new(),
            general_command: 0,
            supply: 100,
            general_morale_bonus: 0.0,
            general_charge_percent: 0.0,
            general_ranged_percent: 0.0,
            general_defense_percent: 0.0,
            general_intrigue: 0.0,
        }
    }
}

/// A6-L2: what the walls of an assaulted place are worth (the level of the
/// walls, the breach and the share of ready engines of the siege).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct WallStand {
    pub level: u32,
    pub breach_percent: u32,
    pub engines_ready_percent: u32,
}

/// Situation modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct BattleContext {
    /// Defender holds hills, forest or mountains (used when the terrain is
    /// unknown; otherwise the terrain's own effects apply).
    pub defender_terrain_bonus: bool,
    /// Attacker crosses a river.
    pub river_crossing: bool,
    /// Attacker assaults fortifications.
    pub walls: bool,
    /// NT9: behind standing walls, the attacker's damage is raised by this
    /// percentage (built engines such as the ram, `siege_engines.json`).
    #[serde(default)]
    pub assault_bonus_percent: u32,
    /// A6-L2: level, breach and engines behind the walls (`walls` only).
    #[serde(default)]
    pub wall: WallStand,
    /// RC (ADR 0141): the attacker forces this river crossing; its
    /// coefficients (`data/rules/river_crossings.json`) replace
    /// `river_crossing`.
    #[serde(default)]
    pub crossing: Option<crate::river_crossing::CrossingEffect>,
}

/// Field, season and weather of a battle (lot N1). `weather: None` draws
/// it from the season; no season means clear weather.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FieldConditions {
    pub terrain: Option<Terrain>,
    pub season: Option<Season>,
    pub weather: Option<Weather>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Winner {
    Attacker,
    Defender,
}

/// What happened to one side.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SideOutcome {
    /// Pre-battle estimate of the damage the side deals (for journals and
    /// odds), after the situation modifiers.
    pub power: f64,
    /// Casualties per unit, same order as `Side::units`.
    pub losses: Vec<u32>,
    pub total_losses: u32,
    pub morale_delta: i32,
    /// Average morale fell below the rout threshold.
    pub routed: bool,
    pub general_captured: bool,
    /// CV3: the commanding general fell (3D battle); counts as a lost
    /// general for the outcome class.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub general_killed: bool,
}

/// Result of [`resolve_auto`]; the caller decides where the loser retreats.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BattleResult {
    pub winner: Winner,
    pub attacker: SideOutcome,
    pub defender: SideOutcome,
}

/// Morale below which a side routs after defeat.
pub const ROUT_MORALE: f64 = 25.0;
/// Probability (per cent) that the losing general is captured.
pub const CAPTURE_CHANCE_PERCENT: u32 = 10;
/// Capture chance (percentage points) per point of `Intrigue` difference
/// between the winning and the losing general (F1).
pub const CAPTURE_PERCENT_PER_INTRIGUE: f64 = 2.0;
/// Ceiling of the capture chance (per cent).
pub const CAPTURE_CHANCE_MAX_PERCENT: f64 = 50.0;

/// Capture chance (per cent) of a beaten general of intrigue `loser` facing
/// a victor of intrigue `winner` (F1).
pub fn capture_chance_percent(winner: f64, loser: f64) -> u32 {
    (f64::from(CAPTURE_CHANCE_PERCENT) + CAPTURE_PERCENT_PER_INTRIGUE * (winner - loser))
        .round()
        .clamp(0.0, CAPTURE_CHANCE_MAX_PERCENT) as u32
}

fn average(values: impl Iterator<Item = f64>) -> f64 {
    let (sum, count) = values.fold((0.0, 0usize), |(s, n), v| (s + v, n + 1));
    if count == 0 {
        0.0
    } else {
        sum / count as f64
    }
}

/// Head-count weighted average armour of a side (0-100).
pub fn average_armor(side: &Side) -> f64 {
    let total: u32 = side.units.iter().map(|u| u.strength).sum();
    if total == 0 {
        return 0.0;
    }
    side.units
        .iter()
        .map(|u| f64::from(u.strength) * f64::from(u.armor))
        .sum::<f64>()
        / f64::from(total)
}

/// Quick power estimate of `side` facing an enemy of average armour
/// `enemy_armor` (the pre-N1 formula, kept for odds shown in the UI).
pub fn side_power(side: &Side, enemy_armor: f64, modifier: f64) -> f64 {
    let base: f64 = side
        .units
        .iter()
        .map(|unit| {
            let attack = if unit.is_ranged {
                f64::from(unit.ranged)
                    * (1.0 - enemy_armor / 200.0)
                    * (1.0 + side.general_ranged_percent / 100.0)
            } else {
                f64::from(unit.melee) * (1.0 + side.general_charge_percent / 100.0)
            };
            f64::from(unit.strength) / 100.0 * attack * (1.0 + f64::from(unit.experience) / 10.0)
        })
        .sum();
    base * (0.5 + side_morale(side) / 200.0)
        * (0.7 + 0.3 * f64::from(side.supply) / 100.0)
        * (1.0 + f64::from(side.general_command) * 0.03)
        * modifier
}

/// `average_armor` plus the general's `BattleDefense` bonus, folded in the
/// same units (percentage points of the 0-100 armour scale, spec § 2).
pub fn effective_armor(side: &Side) -> f64 {
    (average_armor(side) + side.general_defense_percent).clamp(0.0, 100.0)
}

fn side_morale(side: &Side) -> f64 {
    (average(side.units.iter().map(|u| f64::from(u.morale))) + side.general_morale_bonus)
        .clamp(0.0, 100.0)
}

/// Resolves a battle without unit profiles, field conditions or data (every
/// melee unit fights as foot, clear weather, [`AutoResolveRules::default`]).
/// Deterministic for a given RNG state.
pub fn resolve_auto(
    attacker: &Side,
    defender: &Side,
    context: &BattleContext,
    rng: &mut CampaignRng,
) -> BattleResult {
    let attacker_profiles: Vec<UnitProfile> =
        attacker.units.iter().map(UnitProfile::infer).collect();
    let defender_profiles: Vec<UnitProfile> =
        defender.units.iter().map(UnitProfile::infer).collect();
    resolve_with(
        attacker,
        &attacker_profiles,
        defender,
        &defender_profiles,
        context,
        &FieldConditions::default(),
        &AutoResolveRules::default(),
        rng,
    )
}

/// Profiles of the regiments of `ids`, in the order of
/// `movement::coalition_side` (armies in `ids` order, then their units).
pub fn coalition_profiles(
    state: &CampaignState,
    data: &GameData,
    ids: &[ArmyId],
) -> Vec<UnitProfile> {
    ids.iter()
        .filter_map(|id| state.armies.get(id))
        .flat_map(|army| army.units.iter())
        .map(|unit| {
            data.unit_types
                .get(&unit.unit_type)
                .map(UnitProfile::of)
                .unwrap_or_default()
        })
        .collect()
}

/// Auto-resolves a field battle of the campaign (lot N1): the coalitions
/// `attackers` and `defenders` (whose sides are given) fight on
/// `province`'s terrain, in the current season and a weather drawn from
/// it, with the rules of `data`.
#[allow(clippy::too_many_arguments)]
pub fn resolve_field(
    state: &mut CampaignState,
    data: &GameData,
    attackers: &[ArmyId],
    defenders: &[ArmyId],
    attacker_side: &Side,
    defender_side: &Side,
    context: &BattleContext,
    province: Option<&Province>,
) -> BattleResult {
    let profiles = |state: &CampaignState, ids: &[ArmyId], side: &Side| {
        let profiles = coalition_profiles(state, data, ids);
        if profiles.len() == side.units.len() {
            profiles
        } else {
            side.units.iter().map(UnitProfile::infer).collect()
        }
    };
    let attacker_profiles = profiles(state, attackers, attacker_side);
    let defender_profiles = profiles(state, defenders, defender_side);
    resolve_profiled(
        state,
        data,
        (attacker_side, &attacker_profiles),
        (defender_side, &defender_profiles),
        context,
        province,
    )
}

/// Profiles of the regiments of one army (same order as its units), for
/// armies that are not in `state.armies` (a settlement's garrison).
pub fn army_profiles(data: &GameData, army: &crate::state::Army) -> Vec<UnitProfile> {
    army.units
        .iter()
        .map(|unit| {
            data.unit_types
                .get(&unit.unit_type)
                .map(UnitProfile::of)
                .unwrap_or_default()
        })
        .collect()
}

/// [`resolve_field`] with the profiles given (sieges: the garrison is not an
/// army of the state). A profile list of the wrong length is replaced by
/// guessed profiles.
pub fn resolve_profiled(
    state: &mut CampaignState,
    data: &GameData,
    attacker: (&Side, &[UnitProfile]),
    defender: (&Side, &[UnitProfile]),
    context: &BattleContext,
    province: Option<&Province>,
) -> BattleResult {
    let checked = |side: &Side, profiles: &[UnitProfile]| -> Vec<UnitProfile> {
        if profiles.len() == side.units.len() {
            profiles.to_vec()
        } else {
            side.units.iter().map(UnitProfile::infer).collect()
        }
    };
    let (attacker_side, defender_side) = (attacker.0, defender.0);
    let attacker_profiles = checked(attacker_side, attacker.1);
    let defender_profiles = checked(defender_side, defender.1);
    let conditions = FieldConditions {
        terrain: province.map(|p| p.terrain),
        season: Some(state.season),
        weather: None,
    };
    resolve_with_crossings(
        attacker_side,
        &attacker_profiles,
        defender_side,
        &defender_profiles,
        context,
        &conditions,
        &data.auto_resolve,
        &data.river_crossing_rules,
        &mut state.rng,
    )
}

fn season_chances(rules: &AutoResolveRules, season: Season) -> WeatherChances {
    match season {
        Season::Spring => rules.weather.spring,
        Season::Summer => rules.weather.summer,
        Season::Autumn => rules.weather.autumn,
        Season::Winter => rules.weather.winter,
    }
}

/// Weather of a battle in `season`, drawn from the rules' chances.
pub fn draw_weather(rules: &AutoResolveRules, season: Season, rng: &mut CampaignRng) -> Weather {
    let chances = season_chances(rules, season);
    let total = (chances.clear + chances.rain + chances.fog + chances.snow).max(1);
    let mut roll = rng.below(total);
    for (weather, chance) in [
        (Weather::Clear, chances.clear),
        (Weather::Rain, chances.rain),
        (Weather::Fog, chances.fog),
        (Weather::Snow, chances.snow),
    ] {
        if roll < chance {
            return weather;
        }
        roll -= chance;
    }
    Weather::Clear
}

/// One regiment during the fight.
#[derive(Debug, Clone)]
struct Fighter {
    men: f64,
    start: f64,
    profile: UnitProfile,
    melee: f64,
    ranged: f64,
    /// 0-100, the general's `BattleDefense` included.
    armor: f64,
    /// Experience and supply.
    quality: f64,
}

/// One side during the fight.
#[derive(Debug, Clone)]
struct Host {
    fighters: Vec<Fighter>,
    morale: f64,
    start_men: f64,
    /// Multiplier on everything the side deals (command, terrain, walls...).
    damage: f64,
    /// Extra multiplier on the side's shooting (general, walls, weather
    /// handled per unit).
    ranged: f64,
    /// Extra multiplier on melee and charge (general's `BattleCharge`).
    shock: f64,
    defending: bool,
}

impl Host {
    fn new(side: &Side, profiles: &[UnitProfile], defending: bool) -> Host {
        let supply = 0.7 + 0.3 * f64::from(side.supply) / 100.0;
        let fighters: Vec<Fighter> = side
            .units
            .iter()
            .enumerate()
            .map(|(index, unit)| {
                let men = f64::from(unit.strength);
                Fighter {
                    men,
                    start: men,
                    profile: profiles
                        .get(index)
                        .copied()
                        .unwrap_or_else(|| UnitProfile::infer(unit)),
                    melee: f64::from(unit.melee),
                    ranged: f64::from(unit.ranged),
                    armor: (f64::from(unit.armor) + side.general_defense_percent).clamp(0.0, 100.0),
                    quality: (1.0 + f64::from(unit.experience) / 10.0) * supply,
                }
            })
            .collect();
        let start_men = fighters.iter().map(|f| f.men).sum();
        Host {
            fighters,
            morale: side_morale(side),
            start_men,
            damage: 1.0 + f64::from(side.general_command) * 0.03,
            ranged: 1.0 + side.general_ranged_percent / 100.0,
            shock: 1.0 + side.general_charge_percent / 100.0,
            defending,
        }
    }

    fn men(&self) -> f64 {
        self.fighters.iter().map(|f| f.men).sum()
    }

    fn morale_factor(&self) -> f64 {
        0.5 + self.morale.clamp(0.0, 100.0) / 200.0
    }

    /// The front line (foot and horse) still screens the shooters.
    fn screened(&self, rules: &AutoResolveRules) -> bool {
        let front: f64 = self
            .fighters
            .iter()
            .filter(|f| f.profile.is_front())
            .map(|f| f.men)
            .sum();
        let men = self.men();
        men > 0.0 && front >= rules.screen_share * men
    }

    fn cavalry_men(&self) -> f64 {
        self.fighters
            .iter()
            .filter(|f| f.profile.family == UnitFamily::Cavalry)
            .map(|f| f.men)
            .sum()
    }
}

/// Everything fixed for the whole battle.
struct Field<'a> {
    rules: &'a AutoResolveRules,
    weather: Weather,
    terrain_charge: f64,
    terrain_ranged: f64,
}

impl Field<'_> {
    fn bow_factor(&self, profile: UnitProfile) -> f64 {
        let wet = if profile.rain_penalty {
            match self.weather {
                Weather::Rain => self.rules.weather.rain_bow_factor,
                Weather::Snow => self.rules.weather.snow_bow_factor,
                _ => 1.0,
            }
        } else {
            1.0
        };
        let fog = if self.weather == Weather::Fog {
            self.rules.weather.fog_ranged_factor
        } else {
            1.0
        };
        wet * fog * self.terrain_ranged
    }

    fn charge_factor(&self) -> f64 {
        let wet = if matches!(self.weather, Weather::Rain | Weather::Snow) {
            self.rules.weather.wet_charge_factor
        } else {
            1.0
        };
        wet * self.terrain_charge
    }
}

/// Spreads `raw` over the targets by weight, each share reduced by
/// `reduction(target)`; adds the kills to `kills`.
fn spread(
    raw: f64,
    targets: &Host,
    weight: impl Fn(&Fighter) -> f64,
    reduction: impl Fn(&Fighter) -> f64,
    kills: &mut [f64],
) {
    if raw <= 0.0 {
        return;
    }
    let weights: Vec<f64> = targets
        .fighters
        .iter()
        .map(|f| if f.men > 0.0 { f.men * weight(f) } else { 0.0 })
        .collect();
    let total: f64 = weights.iter().sum();
    if total <= 0.0 {
        return;
    }
    for ((fighter, w), kill) in targets.fighters.iter().zip(&weights).zip(kills.iter_mut()) {
        *kill += raw * w / total * reduction(fighter).max(0.0);
    }
}

fn armor_reduction(armor: f64, coefficient: f64) -> f64 {
    (1.0 - armor / 100.0 * coefficient).max(0.0)
}

/// Kills dealt by `from`'s shooters on `to` (`share` of a full volley).
fn fire(field: &Field, from: &Host, to: &Host, share: f64, kills: &mut [f64]) {
    let rules = field.rules;
    let raw: f64 = from
        .fighters
        .iter()
        .filter(|f| f.profile.is_shooter() && f.ranged > 0.0)
        .map(|f| f.men / 100.0 * f.ranged * f.quality * field.bow_factor(f.profile))
        .sum::<f64>()
        * rules.ranged_lethality
        * share
        * from.ranged
        * from.damage
        * from.morale_factor();
    spread(
        raw,
        to,
        |f| {
            if f.profile.family == UnitFamily::HorseArchers {
                rules.skirmish_exposure
            } else {
                1.0
            }
        },
        |f| {
            let mounted = matches!(
                f.profile.family,
                UnitFamily::Cavalry | UnitFamily::HorseArchers
            );
            armor_reduction(f.armor, rules.armor_vs_ranged)
                * if mounted {
                    rules.mounted_target_ranged_factor
                } else {
                    1.0
                }
                * if f.profile.pavise {
                    rules.pavise_factor
                } else {
                    1.0
                }
        },
        kills,
    );
}

/// Kills dealt by `from`'s cavalry charge on `to`, and the kills pikes
/// strike back on the riders (`reflected`, indexed like `from`).
fn charge(field: &Field, from: &Host, to: &Host, kills: &mut [f64], reflected: &mut [f64]) {
    let rules = field.rules;
    let raw: f64 = from
        .fighters
        .iter()
        .filter(|f| f.profile.family == UnitFamily::Cavalry)
        .map(|f| f.men / 100.0 * f64::from(f.profile.charge) * f.quality)
        .sum::<f64>()
        * rules.charge_lethality
        * field.charge_factor()
        * from.shock
        * from.damage
        * from.morale_factor();
    if raw <= 0.0 {
        return;
    }
    let weight = |f: &Fighter| match f.profile.family {
        UnitFamily::Shooters | UnitFamily::Siege => rules.cavalry_flank_exposure,
        UnitFamily::HorseArchers => rules.skirmish_exposure,
        _ => 1.0,
    };
    let received = |f: &Fighter| {
        let mut factor = 1.0;
        if f.profile.family == UnitFamily::Pikes {
            factor *= rules.pike_charge_factor;
        }
        if f.profile.stakes && to.defending {
            factor *= rules.stakes_charge_factor;
        }
        factor
    };
    spread(
        raw,
        to,
        weight,
        |f| armor_reduction(f.armor, rules.armor_vs_charge) * received(f),
        kills,
    );
    // Pikes strike back: kills on the riders in proportion to the share of
    // the charge they received.
    let total_weight: f64 = to
        .fighters
        .iter()
        .filter(|f| f.men > 0.0)
        .map(|f| f.men * weight(f))
        .sum();
    if total_weight <= 0.0 {
        return;
    }
    let pike_share: f64 = to
        .fighters
        .iter()
        .filter(|f| f.men > 0.0 && f.profile.family == UnitFamily::Pikes)
        .map(|f| f.men * weight(f))
        .sum::<f64>()
        / total_weight;
    let back = raw * pike_share * rules.pike_reflect * to.morale_factor();
    let riders: f64 = from
        .fighters
        .iter()
        .filter(|f| f.profile.family == UnitFamily::Cavalry)
        .map(|f| f.men)
        .sum();
    if riders <= 0.0 {
        return;
    }
    for (fighter, kill) in from.fighters.iter().zip(reflected.iter_mut()) {
        if fighter.profile.family == UnitFamily::Cavalry {
            *kill +=
                back * fighter.men / riders * armor_reduction(fighter.armor, rules.armor_vs_melee);
        }
    }
}

/// Kills dealt by `from` on `to` in one melee round (shooters keep firing).
fn melee(field: &Field, from: &Host, to: &Host, kills: &mut [f64]) {
    let rules = field.rules;
    let base = rules.melee_lethality * from.shock * from.damage * from.morale_factor();
    let blows = |family: UnitFamily| -> f64 {
        from.fighters
            .iter()
            .filter(|f| f.profile.family == family)
            .map(|f| f.men / 100.0 * f.melee * f.quality)
            .sum::<f64>()
            * base
    };
    let infantry = blows(UnitFamily::Infantry);
    let pikes = blows(UnitFamily::Pikes);
    let horse = blows(UnitFamily::Cavalry) * rules.cavalry_melee_factor;
    // Shooters fight with half their hearts: they keep shooting.
    let shooters = (blows(UnitFamily::Shooters) + blows(UnitFamily::HorseArchers)) * 0.5;
    let screened = to.screened(rules);
    let reduction = |f: &Fighter| armor_reduction(f.armor, rules.armor_vs_melee);
    let foot_weight = |f: &Fighter| match f.profile.family {
        UnitFamily::Shooters | UnitFamily::Siege if screened => rules.screened_exposure,
        UnitFamily::HorseArchers => rules.skirmish_exposure,
        _ => 1.0,
    };
    spread(infantry + shooters, to, foot_weight, reduction, kills);
    // Pikes strike riders harder, riders strike pikes softer.
    spread(
        pikes,
        to,
        foot_weight,
        |f| {
            reduction(f)
                * if f.profile.family == UnitFamily::Cavalry {
                    rules.pike_vs_cavalry
                } else {
                    1.0
                }
        },
        kills,
    );
    spread(
        horse,
        to,
        |f| match f.profile.family {
            UnitFamily::Shooters | UnitFamily::Siege => rules.cavalry_flank_exposure,
            UnitFamily::HorseArchers => rules.skirmish_exposure,
            _ => 1.0,
        },
        |f| {
            reduction(f)
                / if f.profile.family == UnitFamily::Pikes {
                    rules.pike_vs_cavalry.max(0.01)
                } else {
                    1.0
                }
        },
        kills,
    );
    fire(field, from, to, rules.ranged_in_melee, kills);
}

/// Applies kills (scaled by the phase's fortune) and the morale they cost.
fn suffer(host: &mut Host, kills: &[f64], rules: &AutoResolveRules) {
    let mut lost = 0.0;
    for (fighter, kill) in host.fighters.iter_mut().zip(kills) {
        let k = kill.min(fighter.men).max(0.0);
        fighter.men -= k;
        lost += k;
    }
    if host.start_men > 0.0 {
        host.morale -= 100.0 * lost / host.start_men * rules.morale_per_loss_percent;
    }
}

/// Resolves a battle with unit profiles (same order as the sides' units),
/// field conditions and rules. Deterministic for a given RNG state.
#[allow(clippy::too_many_arguments)]
pub fn resolve_with(
    attacker: &Side,
    attacker_profiles: &[UnitProfile],
    defender: &Side,
    defender_profiles: &[UnitProfile],
    context: &BattleContext,
    conditions: &FieldConditions,
    rules: &AutoResolveRules,
    rng: &mut CampaignRng,
) -> BattleResult {
    static BUNDLED_CROSSINGS: std::sync::OnceLock<RiverCrossingRules> = std::sync::OnceLock::new();
    resolve_with_crossings(
        attacker,
        attacker_profiles,
        defender,
        defender_profiles,
        context,
        conditions,
        rules,
        BUNDLED_CROSSINGS.get_or_init(RiverCrossingRules::default),
        rng,
    )
}

/// [`resolve_with`] with the river crossing rules given (RC: the campaign
/// passes `data.river_crossing_rules`; [`resolve_with`] uses the bundled file).
#[allow(clippy::too_many_arguments)]
pub fn resolve_with_crossings(
    attacker: &Side,
    attacker_profiles: &[UnitProfile],
    defender: &Side,
    defender_profiles: &[UnitProfile],
    context: &BattleContext,
    conditions: &FieldConditions,
    rules: &AutoResolveRules,
    crossing_rules: &RiverCrossingRules,
    rng: &mut CampaignRng,
) -> BattleResult {
    let weather = conditions.weather.unwrap_or_else(|| {
        conditions
            .season
            .map_or(Weather::Clear, |season| draw_weather(rules, season, rng))
    });
    let terrain = conditions.terrain.map(|t| rules.terrain_effects(t));
    let field = Field {
        rules,
        weather,
        terrain_charge: terrain.map_or(1.0, |t| t.charge),
        terrain_ranged: terrain.map_or(1.0, |t| t.ranged),
    };
    let mut a = Host::new(attacker, attacker_profiles, false);
    let mut d = Host::new(defender, defender_profiles, true);
    if let Some(crossing) = context.crossing {
        a.damage *= crossing.attacker_factor(crossing_rules);
        d.ranged *= crossing.defender_ranged_factor(crossing_rules);
    } else if context.river_crossing {
        a.damage *= rules.river_attacker;
    }
    if context.walls {
        a.damage *= rules.walls_attacker
            * (1.0 + f64::from(context.assault_bonus_percent) / 100.0)
            * rules.wall_attacker_factor(
                context.wall.level,
                context.wall.breach_percent,
                context.wall.engines_ready_percent,
            );
        d.ranged *= rules.walls_defender_ranged;
    }
    d.damage *= match terrain {
        Some(t) => t.defender,
        None if context.defender_terrain_bonus => rules.legacy_defender_bonus,
        None => 1.0,
    };
    let attacker_power = estimate(&a, &d, &field);
    let defender_power = estimate(&d, &a, &field);

    // Phases: volleys, one charge, melee rounds.
    #[derive(Clone, Copy)]
    enum Phase {
        Volley,
        Charge,
        Melee,
    }
    let phases = std::iter::repeat_n(Phase::Volley, rules.volleys as usize)
        .chain(std::iter::once(Phase::Charge))
        .chain(std::iter::repeat_n(
            Phase::Melee,
            rules.melee_rounds as usize,
        ));
    let mut broken = (false, false);
    for phase in phases {
        let mut on_d = vec![0.0; d.fighters.len()];
        let mut on_a = vec![0.0; a.fighters.len()];
        match phase {
            Phase::Volley => {
                fire(&field, &a, &d, 1.0, &mut on_d);
                fire(&field, &d, &a, 1.0, &mut on_a);
            }
            Phase::Charge => {
                let mut back_on_a = vec![0.0; a.fighters.len()];
                let mut back_on_d = vec![0.0; d.fighters.len()];
                charge(&field, &a, &d, &mut on_d, &mut back_on_a);
                charge(&field, &d, &a, &mut on_a, &mut back_on_d);
                on_a.iter_mut().zip(back_on_a).for_each(|(k, b)| *k += b);
                on_d.iter_mut().zip(back_on_d).for_each(|(k, b)| *k += b);
            }
            Phase::Melee => {
                melee(&field, &a, &d, &mut on_d);
                melee(&field, &d, &a, &mut on_a);
            }
        }
        let fortune_a = 1.0 + rules.jitter * (2.0 * rng.unit_f64() - 1.0);
        let fortune_d = 1.0 + rules.jitter * (2.0 * rng.unit_f64() - 1.0);
        on_d.iter_mut().for_each(|k| *k *= fortune_a);
        on_a.iter_mut().for_each(|k| *k *= fortune_d);
        suffer(&mut a, &on_a, rules);
        suffer(&mut d, &on_d, rules);
        broken = (
            a.morale < rules.break_morale || a.men() <= 0.0,
            d.morale < rules.break_morale || d.men() <= 0.0,
        );
        if broken.0 || broken.1 {
            break;
        }
    }
    let winner = match broken {
        (true, false) => Winner::Defender,
        (false, true) => Winner::Attacker,
        // Both break, or neither: the steadier side holds the field; the
        // attacker must do better than the defender to win.
        _ => {
            if a.morale > d.morale {
                Winner::Attacker
            } else {
                Winner::Defender
            }
        }
    };

    // Rout and pursuit.
    let (winning, losing) = match winner {
        Winner::Attacker => (&a, &mut d),
        Winner::Defender => (&d, &mut a),
    };
    let remaining = losing.men();
    if remaining > 0.0 {
        let chasers = winning.cavalry_men() / remaining;
        let fraction = (rules.pursuit_base + rules.pursuit_per_cavalry * chasers).min(1.0);
        let mut kills = vec![0.0; losing.fighters.len()];
        spread(
            remaining * fraction,
            losing,
            |f| match f.profile.family {
                UnitFamily::Cavalry | UnitFamily::HorseArchers => 0.3,
                _ => 1.0,
            },
            |_| 1.0,
            &mut kills,
        );
        for (fighter, kill) in losing.fighters.iter_mut().zip(kills) {
            fighter.men -= kill.min(fighter.men);
        }
    }

    let won_a = winner == Winner::Attacker;
    let a_losses = capped_losses(&a, won_a, rules);
    let d_losses = capped_losses(&d, !won_a, rules);
    let mut outcome = |side: &Side, enemy: &Side, power: f64, losses: Vec<u32>, won: bool| {
        let total_losses = losses.iter().sum();
        let morale_delta = if won { 5 } else { -20 };
        let morale_after = average(
            side.units
                .iter()
                .map(|u| (f64::from(u.morale) + f64::from(morale_delta)).max(0.0)),
        );
        let routed = !won && morale_after < ROUT_MORALE;
        let general_captured = !won
            && side.general_command > 0
            && rng.below(100)
                < capture_chance_percent(enemy.general_intrigue, side.general_intrigue);
        SideOutcome {
            power,
            losses,
            total_losses,
            morale_delta,
            routed,
            general_captured,
            general_killed: false,
        }
    };
    let attacker_outcome = outcome(attacker, defender, attacker_power, a_losses, won_a);
    let defender_outcome = outcome(defender, attacker, defender_power, d_losses, !won_a);
    BattleResult {
        winner,
        attacker: attacker_outcome,
        defender: defender_outcome,
    }
}

/// Per-unit losses of `host`, with the side's total brought within the
/// rules' bounds (winner at most `winner_max_losses`, loser between
/// `loser_min_losses` and `loser_max_losses`).
fn capped_losses(host: &Host, won: bool, rules: &AutoResolveRules) -> Vec<u32> {
    let raw: Vec<f64> = host.fighters.iter().map(|f| f.start - f.men).collect();
    let total: f64 = raw.iter().sum();
    let start = host.start_men.max(1.0);
    let (low, high) = if won {
        (0.0, rules.winner_max_losses)
    } else {
        (rules.loser_min_losses, rules.loser_max_losses)
    };
    let target = (total / start).clamp(low, high) * start;
    let scaled: Vec<f64> = if total > 0.0 {
        raw.iter().map(|r| r * target / total).collect()
    } else {
        // No blow landed: the loser still scatters evenly.
        host.fighters
            .iter()
            .map(|f| f.start / start * target)
            .collect()
    };
    host.fighters
        .iter()
        .zip(scaled)
        .map(|(f, loss)| (loss.round().max(0.0) as u32).min(f.start as u32))
        .collect()
}

/// Pre-battle estimate of the kills `host` deals to `enemy` over a whole
/// battle (volleys, charge, melee rounds), without fortune or morale loss.
fn estimate(host: &Host, enemy: &Host, field: &Field) -> f64 {
    let rules = field.rules;
    let mut kills = vec![0.0; enemy.fighters.len()];
    for _ in 0..rules.volleys {
        fire(field, host, enemy, 1.0, &mut kills);
    }
    let mut reflected = vec![0.0; host.fighters.len()];
    charge(field, host, enemy, &mut kills, &mut reflected);
    for _ in 0..rules.melee_rounds {
        melee(field, host, enemy, &mut kills);
    }
    kills.iter().sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(strength: u32, melee: u8, ranged: u8, armor: u8, is_ranged: bool) -> BattleUnit {
        BattleUnit {
            strength,
            max_strength: strength,
            experience: 0,
            morale: 60,
            melee,
            ranged,
            armor,
            is_ranged,
        }
    }

    fn side(units: Vec<BattleUnit>) -> Side {
        Side {
            units,
            ..Default::default()
        }
    }

    fn infantry(count: usize) -> Side {
        side((0..count).map(|_| unit(100, 50, 0, 40, false)).collect())
    }

    #[test]
    fn symmetric_sides_have_equal_power() {
        let result = resolve_auto(
            &infantry(4),
            &infantry(4),
            &BattleContext::default(),
            &mut CampaignRng::from_seed(3),
        );
        assert!((result.attacker.power - result.defender.power).abs() < 1e-9);
        assert_eq!(result.attacker.losses.len(), 4);
        assert_eq!(result.defender.losses.len(), 4);
    }

    #[test]
    fn deterministic_for_same_seed() {
        let a = resolve_auto(
            &infantry(3),
            &infantry(5),
            &BattleContext::default(),
            &mut CampaignRng::from_seed(42),
        );
        let b = resolve_auto(
            &infantry(3),
            &infantry(5),
            &BattleContext::default(),
            &mut CampaignRng::from_seed(42),
        );
        assert_eq!(a, b);
    }

    #[test]
    fn numbers_win_and_loser_bleeds_more() {
        let rules = AutoResolveRules::default();
        let mut wins = 0;
        for seed in 0..50 {
            let result = resolve_auto(
                &infantry(8),
                &infantry(3),
                &BattleContext::default(),
                &mut CampaignRng::from_seed(seed),
            );
            if result.winner == Winner::Attacker {
                wins += 1;
                let attacker_rate = f64::from(result.attacker.total_losses) / 800.0;
                let defender_rate = f64::from(result.defender.total_losses) / 300.0;
                assert!(defender_rate > attacker_rate);
                assert!(attacker_rate <= rules.winner_max_losses + 0.01);
                assert!(
                    (rules.loser_min_losses - 0.01..=rules.loser_max_losses + 0.01)
                        .contains(&defender_rate)
                );
            }
        }
        assert_eq!(wins, 50, "8 units against 3 must always win");
    }

    #[test]
    fn terrain_favours_defender() {
        let neutral = resolve_auto(
            &infantry(4),
            &infantry(4),
            &BattleContext::default(),
            &mut CampaignRng::from_seed(1),
        );
        let hills = resolve_auto(
            &infantry(4),
            &infantry(4),
            &BattleContext {
                defender_terrain_bonus: true,
                river_crossing: true,
                walls: false,
                assault_bonus_percent: 0,
                wall: Default::default(),
                crossing: None,
            },
            &mut CampaignRng::from_seed(1),
        );
        assert!(hills.defender.power > neutral.defender.power);
        assert!(hills.attacker.power < neutral.attacker.power);
    }

    /// NT9: the engines' assault bonus softens the walls for the attacker.
    #[test]
    fn a_broken_gate_softens_the_walls() {
        let walls = |assault_bonus_percent| {
            resolve_auto(
                &infantry(4),
                &infantry(4),
                &BattleContext {
                    walls: true,
                    assault_bonus_percent,
                    ..BattleContext::default()
                },
                &mut CampaignRng::from_seed(1),
            )
        };
        assert!(walls(20).attacker.power > walls(0).attacker.power);
    }

    #[test]
    fn a_crossing_replaces_the_river_flag() {
        let fight = |context: BattleContext| {
            resolve_auto(
                &infantry(4),
                &infantry(4),
                &context,
                &mut CampaignRng::from_seed(1),
            )
        };
        let river = fight(BattleContext {
            river_crossing: true,
            ..BattleContext::default()
        });
        let bridge = fight(BattleContext {
            river_crossing: false,
            crossing: Some(crate::river_crossing::CrossingEffect {
                structure: sim_battle::CrossingStructure::StoneBridge,
                strength_permille: 1000,
            }),
            ..BattleContext::default()
        });
        let both = fight(BattleContext {
            river_crossing: true,
            crossing: Some(crate::river_crossing::CrossingEffect {
                structure: sim_battle::CrossingStructure::StoneBridge,
                strength_permille: 1000,
            }),
            ..BattleContext::default()
        });
        assert!(bridge.attacker.power < river.attacker.power);
        assert_eq!(bridge.attacker.power, both.attacker.power);
    }

    #[test]
    fn armour_blunts_archers() {
        let archers = side(vec![unit(100, 20, 70, 20, true); 4]);
        let light = side(vec![unit(100, 40, 0, 10, false); 4]);
        let heavy = side(vec![unit(100, 40, 0, 80, false); 4]);
        let ctx = BattleContext::default();
        let vs_light = resolve_auto(&archers, &light, &ctx, &mut CampaignRng::from_seed(1));
        let vs_heavy = resolve_auto(&archers, &heavy, &ctx, &mut CampaignRng::from_seed(1));
        assert!(vs_light.attacker.power > vs_heavy.attacker.power);
    }

    fn profile(family: UnitFamily, charge: u8) -> UnitProfile {
        UnitProfile {
            family,
            charge,
            ..UnitProfile::default()
        }
    }

    fn duel(
        attacker: (&Side, UnitProfile),
        defender: (&Side, UnitProfile),
        conditions: FieldConditions,
    ) -> usize {
        let rules = AutoResolveRules::default();
        let pa = vec![attacker.1; attacker.0.units.len()];
        let pd = vec![defender.1; defender.0.units.len()];
        (0..100)
            .filter(|seed| {
                resolve_with(
                    attacker.0,
                    &pa,
                    defender.0,
                    &pd,
                    &BattleContext::default(),
                    &conditions,
                    &rules,
                    &mut CampaignRng::from_seed(*seed),
                )
                .winner
                    == Winner::Attacker
            })
            .count()
    }

    #[test]
    fn armoured_cavalry_rides_down_cheap_foot_but_not_pikes() {
        let knights = side(vec![
            BattleUnit {
                morale: 80,
                ..unit(60, 75, 0, 80, false)
            };
            4
        ]);
        let militia = side(vec![
            BattleUnit {
                morale: 40,
                ..unit(120, 40, 0, 30, false)
            };
            20
        ]);
        let pikes = side(vec![unit(120, 55, 0, 40, false); 4]);
        let horse = profile(UnitFamily::Cavalry, 90);
        let clear = FieldConditions::default();
        assert!(
            duel(
                (&knights, horse),
                (&militia, profile(UnitFamily::Infantry, 0)),
                clear
            ) > 50
        );
        assert!(
            duel(
                (&knights, horse),
                (&pikes, profile(UnitFamily::Pikes, 0)),
                clear
            ) < 50
        );
    }

    #[test]
    fn rain_slackens_bows() {
        let archers = side(vec![unit(120, 30, 70, 20, true); 8]);
        let foot = side(vec![unit(100, 50, 0, 40, false); 6]);
        let bows = UnitProfile {
            rain_penalty: true,
            ..profile(UnitFamily::Shooters, 0)
        };
        let dry = FieldConditions {
            weather: Some(Weather::Clear),
            ..FieldConditions::default()
        };
        let wet = FieldConditions {
            weather: Some(Weather::Rain),
            ..FieldConditions::default()
        };
        let foot_profile = profile(UnitFamily::Infantry, 0);
        assert!(
            duel((&foot, foot_profile), (&archers, bows), wet)
                >= duel((&foot, foot_profile), (&archers, bows), dry)
        );
    }
}
