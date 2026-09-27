//! Map encounters (lot CV3-3, spec `docs/design/2026-09-27-campagne-vivante.md`
//! § 2).
//!
//! Encounter sites (`data/encounters/enc_*.json`) appear at the start of each
//! season on passable land of a province, away from settlements and from one
//! another, and expire after a drawn number of seasons. An army whose march
//! ends within `trigger_radius_km` of a site meets it: the player answers
//! with the order `choose_encounter_option` (unanswered encounters take their
//! default option at the end of the turn), the AI draws an option weighted by
//! `ai_weight` at once. An option applies chronicle effects, army effects and
//! at most one special outcome: a battle against a `fac_rebels` troop raised
//! beside the army (fought through `movement::fight`, 3D or auto, whose
//! result applies `on_win` / `on_loss`), or regiments joining the army.

use std::collections::BTreeSet;

use data_model::{
    Effect, EffectKind, EffectMode, Encounter, EncounterId, EncounterOption, EncounterOutcome,
    EncounterResult, EncounterUnits, EventSeason, FactionId, GameData, ProvinceId, SpawnWar,
};
use serde::{Deserialize, Serialize};

use crate::battle_auto::Winner;
use crate::chronicle::{apply_effect, EventContext};
use crate::diplomacy::REBELS_FACTION;
use crate::events::{EventKind, GameEvent};
use crate::march::px_per_km;
use crate::navigation::Cell;
use crate::rng::CampaignRng;
use crate::state::{Army, ArmyId, ArmyPosition, CampaignState, Season, Unit};

/// Encounter part of the campaign state (`#[serde(default)]`: older saves
/// load without it).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EncounterState {
    /// Sites on the map, in creation order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sites: Vec<EncounterSite>,
    /// Player encounters awaiting a choice.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending: Vec<PendingEncounter>,
    /// Encounter battles not yet resolved (link from the site to the battle).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub battles: Vec<EncounterBattle>,
    #[serde(default)]
    pub next_site_id: u32,
}

/// An encounter waiting on the map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncounterSite {
    pub id: u32,
    pub encounter: EncounterId,
    pub cell: Cell,
    pub province: ProvinceId,
    /// The site disappears at the start of this turn.
    pub expires_turn: u32,
    /// Player army that met it and whose choice is pending.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claimed_by: Option<ArmyId>,
}

/// An encounter met by a player army, awaiting the player's choice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingEncounter {
    pub site: u32,
    pub encounter: EncounterId,
    pub army: ArmyId,
    pub faction: FactionId,
    pub province: ProvinceId,
    /// Turn it was met.
    pub turn: u32,
}

/// A battle started by an encounter option, until it is resolved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncounterBattle {
    pub site: u32,
    pub encounter: EncounterId,
    pub option: usize,
    /// The army that met the encounter.
    pub army: ArmyId,
    pub faction: FactionId,
    pub province: ProvinceId,
    /// The `fac_rebels` troop raised for the battle.
    pub rebels: ArmyId,
    /// The war between `faction` and the rebels was declared for this
    /// battle (and ends with it).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub declared_war: bool,
}

/// Why a `choose_encounter_option` order was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EncounterError {
    #[error("aucune rencontre en attente pour cette armée sur le site n°{0}")]
    NoPendingEncounter(u32),
    #[error("cette rencontre ne concerne pas la faction {0}")]
    NotYours(FactionId),
    #[error("rencontre inconnue : {0}")]
    UnknownEncounter(EncounterId),
    #[error("choix invalide : {0}")]
    InvalidOption(usize),
    #[error("choix impossible : {0}")]
    OptionUnavailable(String),
}

/// A site shown on the map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EncounterSiteView {
    pub id: u32,
    pub encounter: EncounterId,
    pub title: String,
    /// Map-pixel centre of the site's cell.
    pub point: [f32; 2],
    pub province: ProvinceId,
    pub province_name: String,
    pub expires_turn: u32,
    /// Seasons left before it disappears (1 = gone next season).
    pub expires_in: u32,
    /// A player army met it and the choice is pending.
    pub claimed: bool,
}

/// One option of a pending encounter, for the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncounterOptionView {
    pub index: usize,
    pub label: String,
    pub available: bool,
    /// French reason when `available` is `false`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// French summary of the effects, one per line.
    pub effects_text: String,
    /// `""`, `"battle"` or `"join"`.
    pub outcome: String,
    pub default: bool,
}

/// A pending encounter, for the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingEncounterView {
    pub site: u32,
    pub encounter: EncounterId,
    pub army: ArmyId,
    pub army_name: String,
    pub title: String,
    pub text: String,
    pub province: ProvinceId,
    pub province_name: String,
    pub options: Vec<EncounterOptionView>,
}

// =========================================================================
// Helpers
// =========================================================================

/// Salts of the derived random streams (spawn, AI choice): the encounters
/// never draw from the main campaign stream, so a campaign without
/// encounter data keeps its exact history.
const SPAWN_SALT: u64 = 0x454e_435f_5350_574e;
const CHOICE_SALT: u64 = 0x454e_435f_4348_4f49;

fn derived_rng(state: &CampaignState, salt: u64, extra: u64) -> CampaignRng {
    let mixed = state.seed
        ^ salt
        ^ u64::from(state.turn).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ extra.wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    CampaignRng::from_seed(mixed)
}

fn season_of(season: Season) -> EventSeason {
    match season {
        Season::Spring => EventSeason::Spring,
        Season::Summer => EventSeason::Summer,
        Season::Autumn => EventSeason::Autumn,
        Season::Winter => EventSeason::Winter,
    }
}

fn distance(a: [f32; 2], b: [f32; 2]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

fn km_px(data: &GameData, km: f64) -> f32 {
    km as f32 * px_per_km(data)
}

fn rebels() -> FactionId {
    FactionId::new(REBELS_FACTION).expect("well-formed id")
}

fn province_name(data: &GameData, id: &ProvinceId) -> String {
    data.provinces
        .get(id)
        .map_or_else(|| id.to_string(), |p| p.name.display.clone())
}

fn unit_name(data: &GameData, units: &EncounterUnits) -> String {
    data.unit_types
        .get(&units.unit_type)
        .map_or_else(|| units.unit_type.to_string(), |u| u.name.display.clone())
}

fn units_text(data: &GameData, units: &[EncounterUnits]) -> String {
    units
        .iter()
        .map(|u| format!("{} × {}", u.count, unit_name(data, u)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn at_war_with_anyone(state: &CampaignState, faction: &FactionId) -> bool {
    state.factions.get(faction).is_some_and(|f| {
        f.at_war_with
            .iter()
            .any(|enemy| enemy.as_str() != REBELS_FACTION)
    })
}

// =========================================================================
// Spawn and expiry
// =========================================================================

/// Start of a season: sites expire, then new ones appear.
pub fn start_season(state: &mut CampaignState, data: &GameData, _events: &mut Vec<GameEvent>) {
    settle_orphan_battles(state);
    let turn = state.turn;
    state
        .encounters
        .sites
        .retain(|site| site.expires_turn > turn || site.claimed_by.is_some());
    if data.encounters.is_empty() {
        return;
    }
    let rules = &data.encounter_rules;
    let mut rng = derived_rng(state, SPAWN_SALT, 0);
    for _ in 0..rules.spawn_per_season {
        if state.encounters.sites.len() >= rules.max_active as usize {
            break;
        }
        spawn_one(state, data, &mut rng);
    }
}

/// `true` when `encounter` may appear this season (date and weight only).
fn date_allows(state: &CampaignState, encounter: &Encounter) -> bool {
    let spawn = &encounter.spawn;
    spawn.from_year.is_none_or(|y| state.year >= y)
        && spawn.to_year.is_none_or(|y| state.year <= y)
        && (spawn.seasons.is_empty() || spawn.seasons.contains(&season_of(state.season)))
        && spawn.weight > 0
}

/// Provinces where `encounter` may appear now (a site already standing in
/// a province keeps others out), in id order.
pub fn eligible_provinces(
    state: &CampaignState,
    data: &GameData,
    encounter: &Encounter,
) -> Vec<ProvinceId> {
    let spawn = &encounter.spawn;
    let occupied: BTreeSet<&ProvinceId> =
        state.encounters.sites.iter().map(|s| &s.province).collect();
    data.provinces
        .values()
        .filter(|p| state.provinces.contains_key(&p.id) && !occupied.contains(&p.id))
        .filter(|p| spawn.terrains.is_empty() || spawn.terrains.contains(&p.terrain))
        .filter(|p| {
            (spawn.regions.is_empty() && spawn.provinces.is_empty())
                || spawn.regions.contains(&p.region)
                || spawn.provinces.contains(&p.id)
        })
        .filter(|p| {
            let controller = state.province_controller(&p.id).cloned();
            let war_ok = match spawn.war {
                SpawnWar::Any => true,
                SpawnWar::War => controller
                    .as_ref()
                    .is_some_and(|c| at_war_with_anyone(state, c)),
                SpawnWar::Peace => controller
                    .as_ref()
                    .is_none_or(|c| !at_war_with_anyone(state, c)),
            };
            let ctx = EventContext {
                faction: controller,
                province: Some(p.id.clone()),
            };
            war_ok
                && spawn
                    .conditions
                    .iter()
                    .all(|c| state.condition_holds(data, c, &ctx))
        })
        .map(|p| p.id.clone())
        .collect()
}

/// Provinces the player controls or borders.
fn near_player(state: &CampaignState, data: &GameData) -> BTreeSet<ProvinceId> {
    let mut near = BTreeSet::new();
    for id in state.provinces.keys() {
        if state.province_controller(id) == Some(&state.player_faction) {
            near.insert(id.clone());
            if let Some(p) = data.provinces.get(id) {
                near.extend(p.neighbors.iter().cloned());
            }
        }
    }
    near
}

/// Draws one site; `false` when none could be placed.
fn spawn_one(state: &mut CampaignState, data: &GameData, rng: &mut CampaignRng) -> bool {
    let rules = &data.encounter_rules;
    let candidates: Vec<(&Encounter, Vec<ProvinceId>)> = data
        .encounters
        .values()
        .filter(|e| date_allows(state, e))
        .map(|e| (e, eligible_provinces(state, data, e)))
        .filter(|(_, provinces)| !provinces.is_empty())
        .collect();
    let total: u32 = candidates.iter().map(|(e, _)| e.spawn.weight).sum();
    if total == 0 {
        return false;
    }
    let mut pick = rng.below(total);
    let Some((encounter, provinces)) = candidates.iter().find(|(e, _)| {
        if pick < e.spawn.weight {
            true
        } else {
            pick -= e.spawn.weight;
            false
        }
    }) else {
        return false;
    };
    let mut pool: Vec<ProvinceId> = provinces.clone();
    if rng.chance_permille(rules.near_player_permille) {
        let near = near_player(state, data);
        let close: Vec<ProvinceId> = pool.iter().filter(|p| near.contains(*p)).cloned().collect();
        if !close.is_empty() {
            pool = close;
        }
    }
    let province = pool[rng.below(pool.len() as u32) as usize].clone();
    let Some(cell) = draw_cell(state, data, &province, rng) else {
        return false;
    };
    let [low, high] = encounter.spawn.lifetime;
    let low = low.max(1);
    let high = high.max(low);
    let lifetime = low + rng.below(high - low + 1);
    let id = state.encounters.next_site_id;
    state.encounters.next_site_id += 1;
    state.encounters.sites.push(EncounterSite {
        id,
        encounter: encounter.id.clone(),
        cell,
        province,
        expires_turn: state.turn + lifetime,
        claimed_by: None,
    });
    true
}

/// A passable cell of `province`, within `spawn_radius_km` of one of its
/// settlements, away from every settlement and every other site.
pub fn draw_cell(
    state: &CampaignState,
    data: &GameData,
    province: &ProvinceId,
    rng: &mut CampaignRng,
) -> Option<Cell> {
    let rules = &data.encounter_rules;
    let grid = data.navgrid();
    let anchors: Vec<[f32; 2]> = data
        .settlements_by_province
        .get(province)
        .into_iter()
        .flatten()
        .filter_map(|id| data.settlement_point(id))
        .collect();
    if anchors.is_empty() {
        return None;
    }
    let radius = f64::from(km_px(data, rules.spawn_radius_km));
    for _ in 0..rules.spawn_attempts {
        let anchor = anchors[rng.below(anchors.len() as u32) as usize];
        let angle = rng.unit_f64() * std::f64::consts::TAU;
        let reach = radius * rng.unit_f64().sqrt();
        let point = [
            anchor[0] + (reach * angle.cos()) as f32,
            anchor[1] + (reach * angle.sin()) as f32,
        ];
        let cell = Cell::of_point(grid, point);
        if cell_fits(state, data, province, cell) {
            return Some(cell);
        }
    }
    None
}

/// `true` when a site may stand on `cell` of `province`: passable land of
/// the province, at least `min_settlement_distance_km` from any settlement
/// and `min_site_distance_km` from any other site.
pub fn cell_fits(
    state: &CampaignState,
    data: &GameData,
    province: &ProvinceId,
    cell: Cell,
) -> bool {
    let rules = &data.encounter_rules;
    let grid = data.navgrid();
    if !grid.passable(i64::from(cell.x), i64::from(cell.y)) {
        return false;
    }
    let center = cell.center(grid);
    if data.province_at_point(center[0], center[1]) != Some(province) {
        return false;
    }
    if !crate::march::settlements_near(data, center, rules.min_settlement_distance_km).is_empty() {
        return false;
    }
    let gap = km_px(data, rules.min_site_distance_km);
    state
        .encounters
        .sites
        .iter()
        .all(|s| distance(s.cell.center(grid), center) >= gap)
}

// =========================================================================
// Trigger and choice
// =========================================================================

/// End of a march of `army_id`: meets the nearest untouched site within
/// `trigger_radius_km`. A player army waits for the player's choice; an AI
/// army chooses at once.
pub(crate) fn on_march_end(
    state: &mut CampaignState,
    data: &GameData,
    army_id: &ArmyId,
    events: &mut Vec<GameEvent>,
) {
    if state.encounters.sites.is_empty() {
        return;
    }
    let Some(army) = state.armies.get(army_id) else {
        return;
    };
    if army.units.is_empty() || army.faction.as_str() == REBELS_FACTION {
        return;
    }
    let busy = state.encounters.pending.iter().any(|p| &p.army == army_id)
        || state
            .pending_battles
            .iter()
            .any(|b| &b.attacker == army_id || &b.defender == army_id);
    if busy {
        return;
    }
    let grid = data.navgrid();
    let point = state.army_point(data, army);
    let radius = km_px(data, data.encounter_rules.trigger_radius_km);
    let Some(site) = state
        .encounters
        .sites
        .iter()
        .filter(|s| s.claimed_by.is_none() && data.encounters.contains_key(&s.encounter))
        .map(|s| (distance(s.cell.center(grid), point), s))
        .filter(|(d, _)| *d <= radius)
        .min_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.id.cmp(&b.1.id)))
        .map(|(_, s)| s.clone())
    else {
        return;
    };
    let faction = army.faction.clone();
    let encounter = &data.encounters[&site.encounter];
    if faction == state.player_faction {
        if let Some(s) = state.encounters.sites.iter_mut().find(|s| s.id == site.id) {
            s.claimed_by = Some(army_id.clone());
        }
        state.encounters.pending.push(PendingEncounter {
            site: site.id,
            encounter: site.encounter.clone(),
            army: army_id.clone(),
            faction: faction.clone(),
            province: site.province.clone(),
            turn: state.turn,
        });
        events.push(
            GameEvent::new(
                EventKind::Chronicle,
                format!(
                    "Rencontre {} : {}.",
                    crate::events::de(&province_name(data, &site.province)),
                    encounter.title
                ),
            )
            .province(&site.province)
            .army(army_id)
            .faction(&faction),
        );
        return;
    }
    match ai_option(state, data, encounter, army_id, &site) {
        Some(option) => apply_option(state, data, &site, army_id, option, events),
        None => remove_site(state, site.id),
    }
}

/// The AI's option: a draw weighted by `ai_weight` among the available
/// options (the default one, else the first available, when every weight
/// is zero).
fn ai_option(
    state: &CampaignState,
    data: &GameData,
    encounter: &Encounter,
    army_id: &ArmyId,
    site: &EncounterSite,
) -> Option<usize> {
    let available: Vec<usize> = (0..encounter.options.len())
        .filter(|i| {
            option_availability(state, data, encounter, *i, army_id, &site.province).is_ok()
        })
        .collect();
    let total: u32 = available
        .iter()
        .map(|i| encounter.options[*i].ai_weight)
        .sum();
    if total == 0 {
        let default = encounter.default_option();
        return if available.contains(&default) {
            Some(default)
        } else {
            available.first().copied()
        };
    }
    let mut rng = derived_rng(state, CHOICE_SALT, u64::from(site.id));
    let mut pick = rng.below(total);
    available.into_iter().find(|i| {
        let weight = encounter.options[*i].ai_weight;
        if pick < weight {
            true
        } else {
            pick -= weight;
            false
        }
    })
}

/// Why option `index` of `encounter` is closed to `army_id` (French), or
/// `Ok` when it may be taken.
pub fn option_availability(
    state: &CampaignState,
    data: &GameData,
    encounter: &Encounter,
    index: usize,
    army_id: &ArmyId,
    province: &ProvinceId,
) -> Result<(), String> {
    let option = encounter
        .options
        .get(index)
        .ok_or_else(|| format!("choix n°{index} inconnu"))?;
    let army = state
        .armies
        .get(army_id)
        .ok_or_else(|| "l'armée n'existe plus".to_owned())?;
    let ctx = EventContext {
        faction: Some(army.faction.clone()),
        province: Some(province.clone()),
    };
    for condition in &option.conditions {
        if !state.condition_holds(data, condition, &ctx) {
            return Err(match condition {
                data_model::Condition::TreasuryAbove { amount, .. } => {
                    format!("trésor insuffisant (plus de {amount} livres nécessaires)")
                }
                _ => "conditions non remplies".to_owned(),
            });
        }
    }
    match &option.outcome {
        Some(EncounterOutcome::Join { .. }) => {
            let max = data.encounter_rules.max_army_units as usize;
            if army.units.len() >= max {
                return Err(format!("armée au complet ({max} régiments)"));
            }
        }
        Some(EncounterOutcome::Battle { .. }) if !state.factions.contains_key(&rebels()) => {
            return Err("aucune troupe adverse possible".to_owned());
        }
        _ => {}
    }
    Ok(())
}

/// Order `choose_encounter_option`.
pub fn choose_option(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    army: &ArmyId,
    site: u32,
    option: usize,
    events: &mut Vec<GameEvent>,
) -> Result<(), EncounterError> {
    let index = state
        .encounters
        .pending
        .iter()
        .position(|p| p.site == site && &p.army == army)
        .ok_or(EncounterError::NoPendingEncounter(site))?;
    let pending = state.encounters.pending[index].clone();
    if &pending.faction != faction {
        return Err(EncounterError::NotYours(faction.clone()));
    }
    let Some(encounter) = data.encounters.get(&pending.encounter) else {
        state.encounters.pending.remove(index);
        remove_site(state, site);
        return Err(EncounterError::UnknownEncounter(pending.encounter));
    };
    if option >= encounter.options.len() {
        return Err(EncounterError::InvalidOption(option));
    }
    option_availability(state, data, encounter, option, army, &pending.province)
        .map_err(EncounterError::OptionUnavailable)?;
    state.encounters.pending.remove(index);
    let site_state = site_or_stub(state, &pending);
    apply_option(state, data, &site_state, army, option, events);
    Ok(())
}

/// The site of `pending` (a stub when it vanished).
fn site_or_stub(state: &CampaignState, pending: &PendingEncounter) -> EncounterSite {
    state
        .encounters
        .sites
        .iter()
        .find(|s| s.id == pending.site)
        .cloned()
        .unwrap_or_else(|| EncounterSite {
            id: pending.site,
            encounter: pending.encounter.clone(),
            cell: Cell::new(0, 0),
            province: pending.province.clone(),
            expires_turn: state.turn,
            claimed_by: Some(pending.army.clone()),
        })
}

/// End of the player's turn: encounters left unanswered take their default
/// option (the first available one when the default is closed).
pub(crate) fn resolve_unanswered(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let pending = std::mem::take(&mut state.encounters.pending);
    for entry in pending {
        let site = site_or_stub(state, &entry);
        let Some(encounter) = data.encounters.get(&entry.encounter) else {
            remove_site(state, entry.site);
            continue;
        };
        let open = |i: usize| {
            option_availability(state, data, encounter, i, &entry.army, &entry.province).is_ok()
        };
        let default = encounter.default_option();
        let choice = if open(default) {
            Some(default)
        } else {
            (0..encounter.options.len()).find(|i| open(*i))
        };
        match choice {
            Some(option) => apply_option(state, data, &site, &entry.army, option, events),
            None => remove_site(state, entry.site),
        }
    }
}

fn remove_site(state: &mut CampaignState, site: u32) {
    state.encounters.sites.retain(|s| s.id != site);
}

// =========================================================================
// Effects and outcomes
// =========================================================================

fn combine(value: f64, effect: &Effect, max: f64) -> f64 {
    let changed = match effect.mode {
        EffectMode::Add => value + effect.value,
        EffectMode::Percent => value * (1.0 + effect.value / 100.0),
    };
    changed.round().clamp(0.0, max)
}

/// Applies `effects` (`army_morale`, `supply`, `army_experience`; other
/// kinds are ignored) to `army_id`.
pub fn apply_army_effects(state: &mut CampaignState, army_id: &ArmyId, effects: &[Effect]) {
    let Some(army) = state.armies.get_mut(army_id) else {
        return;
    };
    for effect in effects {
        match effect.effect {
            EffectKind::ArmyMorale => {
                for unit in &mut army.units {
                    unit.morale = combine(f64::from(unit.morale), effect, 100.0) as u8;
                }
            }
            EffectKind::Supply => {
                army.supply = combine(f64::from(army.supply), effect, 100.0) as u8;
            }
            EffectKind::ArmyExperience => {
                for unit in &mut army.units {
                    unit.experience = combine(f64::from(unit.experience), effect, 10.0) as u8;
                }
            }
            _ => {}
        }
    }
}

fn describe_army_effect(effect: &Effect) -> String {
    let label = match effect.effect {
        EffectKind::ArmyMorale => "Moral de l'armée",
        EffectKind::Supply => "Ravitaillement de l'armée",
        EffectKind::ArmyExperience => "Expérience des régiments",
        _ => "Effet sans objet",
    };
    let sign = if effect.value >= 0.0 { "+" } else { "" };
    match effect.mode {
        EffectMode::Add => format!("{label} {sign}{}", effect.value),
        EffectMode::Percent => format!("{label} {sign}{} %", effect.value),
    }
}

/// French summary of an option, one line per effect.
fn describe_option(
    state: &CampaignState,
    data: &GameData,
    option: &EncounterOption,
    ctx: &EventContext,
) -> String {
    let mut lines: Vec<String> = option
        .effects
        .iter()
        .map(|e| state.describe_effect(data, e, ctx))
        .collect();
    lines.extend(option.army_effects.iter().map(describe_army_effect));
    match &option.outcome {
        Some(EncounterOutcome::Battle { units, .. }) => {
            lines.push(format!("Bataille contre : {}", units_text(data, units)));
        }
        Some(EncounterOutcome::Join { units }) => {
            lines.push(format!("Rejoignent l'armée : {}", units_text(data, units)));
        }
        None => {}
    }
    lines.join("\n")
}

#[allow(clippy::too_many_arguments)]
fn apply_result_effects(
    state: &mut CampaignState,
    data: &GameData,
    army_id: &ArmyId,
    effects: &[data_model::EventEffect],
    army_effects: &[Effect],
    ctx: &EventContext,
    events: &mut Vec<GameEvent>,
) {
    for effect in effects {
        apply_effect(state, data, effect, ctx, events);
    }
    apply_army_effects(state, army_id, army_effects);
}

/// Applies option `index` of `site`'s encounter for `army_id` and removes
/// the site.
fn apply_option(
    state: &mut CampaignState,
    data: &GameData,
    site: &EncounterSite,
    army_id: &ArmyId,
    index: usize,
    events: &mut Vec<GameEvent>,
) {
    remove_site(state, site.id);
    let Some(encounter) = data.encounters.get(&site.encounter) else {
        return;
    };
    let Some(option) = encounter.options.get(index) else {
        return;
    };
    let Some(faction) = state.armies.get(army_id).map(|a| a.faction.clone()) else {
        return;
    };
    let ctx = EventContext {
        faction: Some(faction.clone()),
        province: Some(site.province.clone()),
    };
    events.push(
        GameEvent::new(
            EventKind::Chronicle,
            format!(
                "{} — {} : {}.",
                encounter.title,
                crate::events::capitalize(&state.army_name(data, army_id)),
                option.label
            ),
        )
        .province(&site.province)
        .army(army_id)
        .faction(&faction),
    );
    apply_result_effects(
        state,
        data,
        army_id,
        &option.effects,
        &option.army_effects,
        &ctx,
        events,
    );
    match &option.outcome {
        None => {}
        Some(EncounterOutcome::Join { units }) => {
            join_units(state, data, army_id, units, &site.province, events);
        }
        Some(EncounterOutcome::Battle { units, .. }) => {
            start_battle(state, data, site, army_id, index, units, events);
        }
    }
}

/// Fresh regiments of `units` (unknown unit types skipped).
fn fresh_units(data: &GameData, units: &[EncounterUnits]) -> Vec<Unit> {
    units
        .iter()
        .filter_map(|u| data.unit_types.get(&u.unit_type).map(|t| (t, u.count)))
        .flat_map(|(t, count)| (0..count).map(move |_| Unit::fresh(t)))
        .collect()
}

/// Outcome `join`: regiments join `army_id` up to `max_army_units`.
fn join_units(
    state: &mut CampaignState,
    data: &GameData,
    army_id: &ArmyId,
    units: &[EncounterUnits],
    province: &ProvinceId,
    events: &mut Vec<GameEvent>,
) {
    let max = data.encounter_rules.max_army_units as usize;
    let Some(army) = state.armies.get_mut(army_id) else {
        return;
    };
    let room = max.saturating_sub(army.units.len());
    let recruits: Vec<Unit> = fresh_units(data, units).into_iter().take(room).collect();
    let joined = recruits.len();
    army.units.extend(recruits);
    let faction = army.faction.clone();
    if joined > 0 {
        events.push(
            GameEvent::new(
                EventKind::Chronicle,
                format!(
                    "{joined} régiment(s) rejoignent {}.",
                    state.army_name(data, army_id)
                ),
            )
            .province(province)
            .army(army_id)
            .faction(&faction),
        );
    }
}

/// Outcome `battle`: a `fac_rebels` troop is raised on the site and fought
/// by `army_id` through the usual circuit (`movement::fight`: a pending 3D
/// battle for the player, auto-resolved otherwise).
fn start_battle(
    state: &mut CampaignState,
    data: &GameData,
    site: &EncounterSite,
    army_id: &ArmyId,
    option: usize,
    units: &[EncounterUnits],
    events: &mut Vec<GameEvent>,
) {
    let rebels = rebels();
    let Some(army) = state.armies.get(army_id) else {
        return;
    };
    let faction = army.faction.clone();
    let troop = fresh_units(data, units);
    if troop.is_empty() || !state.factions.contains_key(&rebels) {
        return;
    }
    let grid = data.navgrid();
    let army_point = state.army_point(data, army);
    let site_point = site.cell.center(grid);
    // The troop stands on the site, or beside the army when the site is a
    // stub or out of reach.
    let engage = km_px(data, data.free_movement_rules().engage_radius_km);
    let point = if site.cell != Cell::new(0, 0) && distance(site_point, army_point) <= engage {
        site_point
    } else {
        [army_point[0] + engage * 0.5, army_point[1]]
    };
    let rebels_id = state.allocate_army_id();
    state.armies.insert(
        rebels_id.clone(),
        Army::new(rebels.clone(), ArmyPosition::field(point), troop),
    );
    let declared_war = !state.is_at_war(&faction, &rebels);
    if declared_war {
        set_war(state, &faction, &rebels, true);
    }
    state.encounters.battles.push(EncounterBattle {
        site: site.id,
        encounter: site.encounter.clone(),
        option,
        army: army_id.clone(),
        faction,
        province: site.province.clone(),
        rebels: rebels_id.clone(),
        declared_war,
    });
    crate::movement::fight(state, data, army_id, &rebels_id, events);
    settle_orphan_battles(state);
}

fn set_war(state: &mut CampaignState, a: &FactionId, b: &FactionId, war: bool) {
    for (x, y) in [(a, b), (b, a)] {
        if let Some(f) = state.factions.get_mut(x) {
            if war {
                f.at_war_with.insert(y.clone());
            } else {
                f.at_war_with.remove(y);
                f.war_scores.remove(y);
            }
        }
    }
}

/// Removes the troop of `link` (already out of the battle list) and ends
/// the war declared for it once no other encounter battle needs it.
fn close_battle(state: &mut CampaignState, link: &EncounterBattle) {
    state.armies.remove(&link.rebels);
    let still_fighting = state
        .encounters
        .battles
        .iter()
        .any(|b| b.faction == link.faction && b.declared_war);
    if link.declared_war && !still_fighting {
        set_war(state, &link.faction, &rebels(), false);
    }
}

/// Drops the encounter battles that are neither pending nor fought (the
/// battle became impossible or went stale): the troop leaves, no result.
fn settle_orphan_battles(state: &mut CampaignState) {
    let orphans: Vec<EncounterBattle> = state
        .encounters
        .battles
        .iter()
        .filter(|link| {
            !state
                .pending_battles
                .iter()
                .any(|b| b.attacker == link.rebels || b.defender == link.rebels)
        })
        .cloned()
        .collect();
    for link in orphans {
        state.encounters.battles.retain(|b| b.rebels != link.rebels);
        close_battle(state, &link);
    }
}

/// End of a battle (`movement::apply_battle_result`): applies `on_win` /
/// `on_loss` when it was an encounter battle and removes the troop.
pub(crate) fn after_battle(
    state: &mut CampaignState,
    data: &GameData,
    attackers: &[ArmyId],
    defenders: &[ArmyId],
    winner: Winner,
    events: &mut Vec<GameEvent>,
) {
    if state.encounters.battles.is_empty() {
        return;
    }
    let (Some(attacker), Some(defender)) = (attackers.first(), defenders.first()) else {
        return;
    };
    let Some(index) = state
        .encounters
        .battles
        .iter()
        .position(|b| &b.rebels == attacker || &b.rebels == defender)
    else {
        return;
    };
    let link = state.encounters.battles.remove(index);
    let rebels_attacked = &link.rebels == attacker;
    let won = match winner {
        Winner::Attacker => !rebels_attacked,
        Winner::Defender => rebels_attacked,
    };
    close_battle(state, &link);
    let Some(encounter) = data.encounters.get(&link.encounter) else {
        return;
    };
    let Some(EncounterOutcome::Battle {
        on_win, on_loss, ..
    }) = encounter
        .options
        .get(link.option)
        .and_then(|o| o.outcome.as_ref())
    else {
        return;
    };
    let result: &EncounterResult = if won { on_win } else { on_loss };
    let ctx = EventContext {
        faction: Some(link.faction.clone()),
        province: Some(link.province.clone()),
    };
    let text = result.text.clone().unwrap_or_else(|| {
        if won {
            format!("{} : la troupe est dispersée.", encounter.title)
        } else {
            format!("{} : l'ost est repoussé.", encounter.title)
        }
    });
    events.push(
        GameEvent::new(EventKind::Chronicle, text)
            .province(&link.province)
            .army(&link.army)
            .faction(&link.faction),
    );
    apply_result_effects(
        state,
        data,
        &link.army,
        &result.effects,
        &result.army_effects,
        &ctx,
        events,
    );
}

// =========================================================================
// Views
// =========================================================================

impl CampaignState {
    /// Sites `faction` sees (their cell inside its vision), in creation order.
    pub fn encounter_site_views(
        &self,
        data: &GameData,
        faction: &FactionId,
    ) -> Vec<EncounterSiteView> {
        if self.encounters.sites.is_empty() {
            return Vec::new();
        }
        let vision = self.vision(data, faction);
        self.visible_encounter_sites(data, &vision)
    }

    /// Sites inside `vision`, in creation order.
    pub fn visible_encounter_sites(
        &self,
        data: &GameData,
        vision: &crate::vision::Vision,
    ) -> Vec<EncounterSiteView> {
        let grid = data.navgrid();
        self.encounters
            .sites
            .iter()
            .filter(|s| vision.mask.sees_point(data, s.cell.center(grid)))
            .map(|s| EncounterSiteView {
                id: s.id,
                encounter: s.encounter.clone(),
                title: data
                    .encounters
                    .get(&s.encounter)
                    .map_or_else(|| s.encounter.to_string(), |e| e.title.clone()),
                point: s.cell.center(grid),
                province: s.province.clone(),
                province_name: province_name(data, &s.province),
                expires_turn: s.expires_turn,
                expires_in: s.expires_turn.saturating_sub(self.turn),
                claimed: s.claimed_by.is_some(),
            })
            .collect()
    }

    /// Encounters of `faction` awaiting a choice, oldest first.
    pub fn pending_encounter_views(
        &self,
        data: &GameData,
        faction: &FactionId,
    ) -> Vec<PendingEncounterView> {
        self.encounters
            .pending
            .iter()
            .filter(|p| &p.faction == faction)
            .filter_map(|p| {
                let encounter = data.encounters.get(&p.encounter)?;
                let ctx = EventContext {
                    faction: Some(p.faction.clone()),
                    province: Some(p.province.clone()),
                };
                let default = encounter.default_option();
                let options = encounter
                    .options
                    .iter()
                    .enumerate()
                    .map(|(index, option)| {
                        let availability =
                            option_availability(self, data, encounter, index, &p.army, &p.province);
                        EncounterOptionView {
                            index,
                            label: option.label.clone(),
                            available: availability.is_ok(),
                            reason: availability.err(),
                            effects_text: describe_option(self, data, option, &ctx),
                            outcome: match option.outcome {
                                Some(EncounterOutcome::Battle { .. }) => "battle",
                                Some(EncounterOutcome::Join { .. }) => "join",
                                None => "",
                            }
                            .to_owned(),
                            default: index == default,
                        }
                    })
                    .collect();
                Some(PendingEncounterView {
                    site: p.site,
                    encounter: p.encounter.clone(),
                    army: p.army.clone(),
                    army_name: self.army_name(data, &p.army),
                    title: encounter.title.clone(),
                    text: encounter.text.clone(),
                    province: p.province.clone(),
                    province_name: province_name(data, &p.province),
                    options,
                })
            })
            .collect()
    }
}
