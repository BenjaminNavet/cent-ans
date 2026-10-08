//! Lot JR1 « Ferveur » (ADR 0165): the crusader faction lives on the zeal of
//! its vow instead of land.
//!
//! Everything is read from `data/rules/crusade.json`
//! ([`data_model::CrusadeRules`]): the faction, its base, the province of
//! the vow and the whole scale. Without the file, or without the faction,
//! [`CampaignState::crusade`] stays `None` and every function here is a
//! no-op.
//!
//! Spec `docs/superpowers/specs/2026-10-02-jr-croises-jerusalem-design.md`.

use data_model::{CrusadeRules, FactionId, GameData, ProvinceId, SettlementId, UnitTypeId};
use serde::{Deserialize, Serialize};

use crate::events::{EventKind, GameEvent};
use crate::orders::Order;
use crate::religion::{self, FaithRelation};
use crate::rng::CampaignRng;
use crate::state::{CampaignState, Unit};

/// A contingent of volunteers at sea.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingPassage {
    /// Turn whose end lands the contingent.
    pub arrival_turn: u32,
    /// Port it was preached for.
    pub port: SettlementId,
    pub units: u32,
}

/// Dynamic state of the crusade (absent from older saves and when the
/// rules or the faction are missing).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrusadeState {
    /// 0-100.
    pub fervor: u8,
    /// Turns before the passage can be preached again.
    #[serde(default)]
    pub preach_cooldown: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending_passages: Vec<PendingPassage>,
    /// The target province has been taken and is still held (floor).
    #[serde(default)]
    pub target_taken: bool,
    /// Causes (French) and points of the latest changes, for the tooltip.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub last_changes: Vec<(String, i32)>,
    /// Turn `last_changes` belongs to.
    #[serde(default)]
    pub changes_turn: u32,
    /// Alms paid at the last end of turn.
    #[serde(default)]
    pub alms_last_turn: i64,
    /// JR4b: turns before the master of a besieged place of the Holy Land
    /// can call its defence again.
    #[serde(default)]
    pub relief_cooldown: u32,
    /// JR4b: places whose current siege by the crusade was already relieved.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relieved: Vec<SettlementId>,
    /// JR5: the target was delivered at least once (its fervour and
    /// prestige are won only the first time).
    #[serde(default)]
    pub target_ever_taken: bool,
    /// JR5: capital before the deliverance, restored when the target is lost.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub former_capital: Option<ProvinceId>,
    /// JR5: places of the Holy Land whose capture already lifted the fervour.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub counted_places: Vec<SettlementId>,
}

/// Why `preach_passage` was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CrusadeError {
    #[error("Seule la faction croisée peut prêcher le passage.")]
    NotCrusaders,
    #[error("Aucun port tenu pour accueillir les volontaires.")]
    NoPort,
    #[error("Trésor insuffisant : {needed} livres nécessaires, {available} disponibles.")]
    InsufficientFunds { needed: i64, available: i64 },
    #[error("Passage déjà prêché : nouvel appel dans {}.", count_noun(*.0, "tour", "tours"))]
    Cooldown(u32),
}

/// `n` and the noun agreed with it (« 1 tour », « 8 tours »).
fn count_noun(n: u32, singular: &str, plural: &str) -> String {
    format!("{n} {}", if n > 1 { plural } else { singular })
}

/// One line of the fervour tooltip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FervorChange {
    pub cause: String,
    pub delta: i32,
}

/// A contingent at sea, for the interface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PendingPassageView {
    pub turns_left: u32,
    pub port: SettlementId,
    pub port_name: String,
    pub units: u32,
}

/// The « Ferveur » section of the faction panel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CrusadeView {
    pub fervor: u8,
    /// Level fervour cannot fall below (0 unless the target is held).
    pub floor: u8,
    /// Alms the next end of turn pays at the current fervour.
    pub alms: i64,
    pub alms_last_turn: i64,
    pub changes: Vec<FervorChange>,
    /// Morale points the faction's armies get from the current fervour.
    pub zeal_morale: i32,
    /// Percent of the men who leave each turn at the current fervour.
    pub desertion_percent: u32,
    /// The thresholds of the rules, for the tooltip: zeal bonus at or above
    /// `zeal_high_threshold`, malus below `zeal_low_threshold`, desertion
    /// of `desertion_men_percent` % a turn below `desertion_threshold`.
    pub zeal_high_threshold: u8,
    pub zeal_low_threshold: u8,
    pub zeal_high_morale: i32,
    pub zeal_low_morale: i32,
    pub desertion_threshold: u8,
    pub desertion_men_percent: u32,
    pub target_taken: bool,
    pub target_name: String,
    pub passage_cost: i64,
    pub passage_cooldown: u32,
    pub passage_available: bool,
    /// French reason the passage cannot be preached now (empty: it can).
    pub passage_blocker: String,
    /// Units a contingent preached now would bring.
    pub passage_units: u32,
    pub passage_delay: u32,
    pub pending: Vec<PendingPassageView>,
}

/// Campaign setup: opens the crusade when the rules and the faction exist.
pub(crate) fn init_crusade(state: &mut CampaignState, data: &GameData) {
    let Some(rules) = &data.crusade_rules else {
        return;
    };
    if state.factions.contains_key(&rules.faction) {
        state.crusade = Some(CrusadeState {
            fervor: rules.fervor.start.min(100),
            ..CrusadeState::default()
        });
    }
}

/// Position and units of the starting army of `faction` when the crusade
/// rules give it one (its base settlement instead of the capital's city).
pub(crate) fn starting_army(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> Option<(SettlementId, Vec<Unit>)> {
    let rules = data.crusade_rules.as_ref()?;
    if &rules.faction != faction || !state.settlements.contains_key(&rules.base_settlement) {
        return None;
    }
    let units = rules
        .starting_army
        .iter()
        .filter_map(|id| data.unit_types.get(id))
        .map(Unit::fresh)
        .collect();
    Some((rules.base_settlement.clone(), units))
}

/// The rules of the open crusade (`None`: no rules, no state, or the
/// crusader faction is dead — its state is then frozen).
fn active<'a>(state: &CampaignState, data: &'a GameData) -> Option<&'a CrusadeRules> {
    let rules = data.crusade_rules.as_ref()?;
    state.crusade.as_ref()?;
    state
        .factions
        .get(&rules.faction)
        .is_some_and(|f| f.alive)
        .then_some(rules)
}

/// `true` when `faction` is the living crusader faction of an open crusade.
pub fn is_crusader(state: &CampaignState, data: &GameData, faction: &FactionId) -> bool {
    active(state, data).is_some_and(|rules| &rules.faction == faction)
}

/// The vow binds the AI: the crusader faction, when the player does not
/// lead it, neither offers nor signs a peace with `other` while `other`
/// holds the target province. (The player stays free to treat, and pays
/// for it in fervour each turn.)
pub fn ai_vow_forbids_peace(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    other: &FactionId,
) -> bool {
    faction != &state.player_faction
        && active(state, data).is_some_and(|rules| {
            &rules.faction == faction
                && state.province_controller(&rules.target_province) == Some(other)
        })
}

/// Level fervour cannot fall below.
fn floor(rules: &CrusadeRules, crusade: &CrusadeState) -> u8 {
    if crusade.target_taken {
        rules.fervor.target_floor.min(100)
    } else {
        0
    }
}

/// Moves the gauge by `delta` within `floor..=100` and notes the points
/// really gained or lost under `cause` (the notes of an earlier turn are
/// dropped first).
fn change(state: &mut CampaignState, rules: &CrusadeRules, cause: &str, delta: i32) {
    let turn = state.turn;
    let Some(crusade) = state.crusade.as_mut() else {
        return;
    };
    let before = i32::from(crusade.fervor);
    let low = i32::from(floor(rules, crusade));
    let after = (before + delta).clamp(low, 100);
    crusade.fervor = after as u8;
    let applied = after - before;
    if applied == 0 {
        return;
    }
    if crusade.changes_turn != turn {
        crusade.last_changes.clear();
        crusade.changes_turn = turn;
    }
    match crusade.last_changes.iter_mut().find(|(c, _)| c == cause) {
        Some(entry) => entry.1 += applied,
        None => crusade.last_changes.push((cause.to_owned(), applied)),
    }
}

/// Name of the vow's goal: the city of the target province.
fn target_name(state: &CampaignState, data: &GameData, rules: &CrusadeRules) -> String {
    match state.province_city_id(&rules.target_province) {
        Some(city) => data.settlement_name(city),
        None => rules.target_province.to_string(),
    }
}

/// Alms `faction` receives this turn (0 unless it is the crusaders):
/// `base + per_fervor × fervour`. Part of
/// [`CampaignState::faction_income`], so the treasury, the
/// projection, the budget and the AI all see them.
pub fn alms(state: &CampaignState, data: &GameData, faction: &FactionId) -> i64 {
    let Some(rules) = &data.crusade_rules else {
        return 0;
    };
    if &rules.faction != faction {
        return 0;
    }
    match (&state.crusade, active(state, data)) {
        (Some(crusade), Some(_)) => alms_at(rules, crusade.fervor),
        _ => 0,
    }
}

fn alms_at(rules: &CrusadeRules, fervor: u8) -> i64 {
    rules.alms.base + rules.alms.per_fervor * i64::from(fervor)
}

/// Ports `faction` holds, the best landing first: a port of the coastal
/// Holy Land, then the base settlement, then the others in id order.
fn held_ports(state: &CampaignState, data: &GameData, rules: &CrusadeRules) -> Vec<SettlementId> {
    let mut ports: Vec<(u8, SettlementId)> = state
        .settlements
        .iter()
        .filter(|(id, s)| {
            s.controller == rules.faction && data.settlements.get(*id).is_some_and(|d| d.port)
        })
        .map(|(id, s)| {
            let rank = if rules.coastal_holy_land.contains(&s.province) {
                0
            } else if *id == rules.base_settlement {
                1
            } else {
                2
            };
            (rank, id.clone())
        })
        .collect();
    ports.sort();
    ports.into_iter().map(|(_, id)| id).collect()
}

/// Takes or loses the target province according to who holds its city.
fn sync_target(state: &mut CampaignState, data: &GameData, events: &mut Vec<GameEvent>) {
    let Some(rules) = active(state, data) else {
        return;
    };
    let held = state.controls_province(&rules.faction, &rules.target_province);
    let taken = state.crusade.as_ref().is_some_and(|c| c.target_taken);
    let name = target_name(state, data, rules);
    let faction = data.faction_name(&rules.faction);
    if held && !taken {
        let capital = state
            .factions
            .get(&rules.faction)
            .map(|f| f.capital.clone());
        let first = !state.crusade.as_ref().is_some_and(|c| c.target_ever_taken);
        if let Some(crusade) = state.crusade.as_mut() {
            crusade.target_taken = true;
            crusade.target_ever_taken = true;
            crusade.former_capital = capital.filter(|c| *c != rules.target_province);
        }
        // The gain and the prestige come with the first deliverance only.
        if first {
            change(
                state,
                rules,
                &format!("{name} délivrée"),
                rules.fervor.target_taken,
            );
            state.change_ruler_prestige(&rules.faction, rules.target_taken_prestige);
        }
        // The floor also lifts a gauge the gain left below it.
        change(state, rules, &format!("{name} délivrée"), 0);
        if let Some(f) = state.factions.get_mut(&rules.faction) {
            f.capital = rules.target_province.clone();
        }
        events.push(
            GameEvent::new(
                EventKind::Crusade,
                format!(
                    "{name} délivrée ! {} tient la cité de son vœu et y établit son siège.",
                    crate::events::capitalize(&faction)
                ),
            )
            .province(&rules.target_province)
            .faction(&rules.faction)
            .public(),
        );
    } else if !held && taken {
        let former = state.crusade.as_mut().and_then(|crusade| {
            crusade.target_taken = false;
            crusade.former_capital.take()
        });
        // Back to the former seat.
        if let (Some(former), Some(f)) = (former, state.factions.get_mut(&rules.faction)) {
            if f.capital == rules.target_province {
                f.capital = former;
            }
        }
        events.push(
            GameEvent::new(
                EventKind::Crusade,
                format!("{name} est perdue : la ferveur de {faction} n'a plus de plancher."),
            )
            .province(&rules.target_province)
            .faction(&rules.faction)
            .public()
            .loss(),
        );
    }
}

/// End of turn: the target's status, wear of the vow, peace with the
/// target's holder, landings, desertion, cooldown. The alms are paid with
/// the taxes ([`alms`]); the amount of the season is only noted here.
pub(crate) fn resolve_crusade(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let Some(rules) = active(state, data) else {
        return;
    };
    // The economy has just paid the alms at this fervour.
    let paid = state
        .crusade
        .as_ref()
        .map_or(0, |c| alms_at(rules, c.fervor));
    if let Some(crusade) = state.crusade.as_mut() {
        crusade.alms_last_turn = paid;
    }
    sync_target(state, data, events);
    change(
        state,
        rules,
        "Le vœu s'use",
        -i32::from(rules.fervor.decay_per_turn),
    );
    // JR4: exaltation does not last; above the high threshold it falls back
    // faster.
    let exalted = state
        .crusade
        .as_ref()
        .is_some_and(|c| c.fervor >= rules.zeal.high_threshold);
    if exalted {
        change(
            state,
            rules,
            "L'exaltation retombe",
            -i32::from(rules.fervor.decay_above_high),
        );
    }
    // Peace or truce with the master of the target, without holding it.
    let holder = state.province_controller(&rules.target_province).cloned();
    if let Some(holder) = holder {
        if holder != rules.faction
            && !holder.is_rebels()
            && !state.is_at_war(&rules.faction, &holder)
        {
            change(
                state,
                rules,
                &format!("Paix avec le maître de {}", target_name(state, data, rules)),
                rules.fervor.truce_with_target_holder_per_turn,
            );
        }
    }
    land_contingents(state, data, rules, events);
    desert(state, data, rules, events);
    relieve_sieges(state, data, rules, events);
    if let Some(crusade) = state.crusade.as_mut() {
        crusade.preach_cooldown = crusade.preach_cooldown.saturating_sub(1);
        crusade.relief_cooldown = crusade.relief_cooldown.saturating_sub(1);
    }
}

/// Generator of the contingents: seeded by the campaign seed and the turn,
/// it never draws from [`CampaignState::rng`].
fn passage_rng(seed: u64, turn: u32, index: usize) -> CampaignRng {
    CampaignRng::from_seed(
        seed ^ 0x4352_5553_4144_4553
            ^ (u64::from(turn) + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ (index as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F),
    )
}

/// `count` unit types drawn from the weighted table (types missing from the
/// data are left out of the draw).
fn draw_units(
    data: &GameData,
    unit_table: &[data_model::CrusadePassageUnit],
    rng: &mut CampaignRng,
    count: u32,
) -> Vec<UnitTypeId> {
    let table: Vec<(&UnitTypeId, u32)> = unit_table
        .iter()
        .filter(|e| e.weight > 0 && data.unit_types.contains_key(&e.unit))
        .map(|e| (&e.unit, e.weight))
        .collect();
    let total: u32 = table.iter().map(|(_, w)| w).sum();
    if total == 0 {
        return Vec::new();
    }
    (0..count)
        .map(|_| {
            let mut roll = rng.below(total);
            for (unit, weight) in &table {
                if roll < *weight {
                    return (*unit).clone();
                }
                roll -= weight;
            }
            table[0].0.clone()
        })
        .collect()
}

/// Lands the contingents due at the start of the coming turn: in their
/// port, else in another held port, else they are lost.
fn land_contingents(
    state: &mut CampaignState,
    data: &GameData,
    rules: &CrusadeRules,
    events: &mut Vec<GameEvent>,
) {
    let next_turn = state.turn + 1;
    let Some(crusade) = state.crusade.as_mut() else {
        return;
    };
    let (due, waiting): (Vec<PendingPassage>, Vec<PendingPassage>) =
        std::mem::take(&mut crusade.pending_passages)
            .into_iter()
            .partition(|p| p.arrival_turn <= next_turn);
    crusade.pending_passages = waiting;
    for (index, passage) in due.into_iter().enumerate() {
        let booked_held = state
            .settlements
            .get(&passage.port)
            .is_some_and(|s| s.controller == rules.faction);
        let port = if booked_held {
            Some(passage.port.clone())
        } else {
            held_ports(state, data, rules).into_iter().next()
        };
        let Some(port) = port else {
            events.push(
                GameEvent::new(
                    EventKind::Crusade,
                    format!(
                        "Les volontaires du passage ne trouvent aucun port où débarquer : \
                         le contingent ({}) se disperse.",
                        count_noun(passage.units, "unité", "unités")
                    ),
                )
                .faction(&rules.faction),
            );
            continue;
        };
        let mut rng = passage_rng(state.seed, state.turn, index);
        let units = draw_units(data, &rules.passage.unit_table, &mut rng, passage.units);
        let landed = disembark(state, data, rules, &port, &units);
        let mut event = GameEvent::new(
            EventKind::Crusade,
            format!(
                "Un contingent de volontaires débarque à {} : {}.",
                data.settlement_name(&port),
                count_noun(landed, "unité", "unités")
            ),
        )
        .faction(&rules.faction);
        if let Some(province) = state.settlement_province(&port) {
            event = event.province(province);
        }
        events.push(event);
    }
}

/// Free places in the garrison of `settlement` (its kind's cap).
fn garrison_room(state: &CampaignState, data: &GameData, settlement: &SettlementId) -> usize {
    let Some(place) = state.settlements.get(settlement) else {
        return 0;
    };
    data.settlement_rules
        .as_ref()
        .and_then(|r| r.garrison_cap.get(&place.kind).copied())
        .map_or(usize::MAX, |cap| cap.saturating_sub(place.garrison.len()))
}

/// JR5: lands `units` at `port` within its garrison cap; the surplus fills
/// the other held ports, then joins (or forms) a field army of the faction
/// at `port`. Returns the units landed.
fn disembark(
    state: &mut CampaignState,
    data: &GameData,
    rules: &CrusadeRules,
    port: &SettlementId,
    units: &[UnitTypeId],
) -> u32 {
    let mut left: Vec<UnitTypeId> = units
        .iter()
        .filter(|id| data.unit_types.contains_key(*id))
        .cloned()
        .collect();
    let total = left.len() as u32;
    let mut ports = vec![port.clone()];
    ports.extend(
        held_ports(state, data, rules)
            .into_iter()
            .filter(|p| p != port),
    );
    for place in ports {
        if left.is_empty() {
            break;
        }
        let room = garrison_room(state, data, &place).min(left.len());
        let batch: Vec<UnitTypeId> = left.drain(..room).collect();
        spawn_units_at_settlement(state, data, &place, &batch);
    }
    if left.is_empty() {
        return total;
    }
    let fresh: Vec<Unit> = left
        .iter()
        .filter_map(|id| data.unit_types.get(id))
        .map(Unit::fresh)
        .collect();
    let cap = data.army_rules.cap();
    let position = crate::state::ArmyPosition::Settlement(port.clone());
    let existing = state
        .armies
        .iter()
        .find(|(_, a)| a.faction == rules.faction && a.position == position && a.units.len() < cap)
        .map(|(id, _)| id.clone());
    let mut fresh = fresh.into_iter();
    if let Some(id) = existing {
        if let Some(army) = state.armies.get_mut(&id) {
            let room = cap.saturating_sub(army.units.len());
            army.units.extend(fresh.by_ref().take(room));
        }
    }
    let rest: Vec<Unit> = fresh.collect();
    if !rest.is_empty() {
        let id = state.allocate_army_id();
        let mut army = crate::state::Army::new(rules.faction.clone(), position, rest);
        army.movement_left = state.army_grid_allowance(data, &army);
        state.armies.insert(id, army);
    }
    total
}

/// JR4b « appel à défendre »: the master of a place of the Holy Land the
/// crusade besieges throws a relief levy into it, once per siege, at most
/// once every `cooldown_turns` turns, within the place's garrison cap. The
/// levy goes into the besieged garrison rather than the field: a besieged
/// place keeps its men when its master is in debt, a field levy would be
/// dismissed the next season by an indebted planner.
fn relieve_sieges(
    state: &mut CampaignState,
    data: &GameData,
    rules: &CrusadeRules,
    events: &mut Vec<GameEvent>,
) {
    let Some(relief) = &rules.relief else {
        return;
    };
    let besieged: Vec<SettlementId> = state
        .settlements
        .iter()
        .filter(|(_, s)| {
            rules.holy_land.contains(&s.province)
                && !s.controller.is_rebels()
                && s.controller != rules.faction
                && s.siege
                    .as_ref()
                    .is_some_and(|g| g.attacker == rules.faction)
        })
        .map(|(id, _)| id.clone())
        .collect();
    let Some(crusade) = state.crusade.as_mut() else {
        return;
    };
    // A siege lifted or ended: the next one may be relieved again.
    crusade.relieved.retain(|id| besieged.contains(id));
    if crusade.relief_cooldown > 0 {
        return;
    }
    let Some(place) = besieged
        .iter()
        .find(|id| !crusade.relieved.contains(id))
        .cloned()
    else {
        return;
    };
    let Some(settlement) = state.settlements.get(&place) else {
        return;
    };
    let master = settlement.controller.clone();
    let cap = data
        .settlement_rules
        .as_ref()
        .and_then(|r| r.garrison_cap.get(&settlement.kind).copied())
        .unwrap_or(usize::MAX);
    let room = cap.saturating_sub(settlement.garrison.len()) as u32;
    let count = relief.units.min(room);
    if let Some(crusade) = state.crusade.as_mut() {
        crusade.relieved.push(place.clone());
        crusade.relief_cooldown = relief.cooldown_turns;
    }
    if count == 0 {
        return;
    }
    let index = state
        .settlements
        .keys()
        .position(|id| *id == place)
        .unwrap_or(0);
    let mut rng = passage_rng(state.seed ^ 0x5245_4C49_4546, state.turn, index);
    let units = draw_units(data, &relief.unit_table, &mut rng, count);
    let landed = spawn_units_at_settlement(state, data, &place, &units);
    if landed == 0 {
        return;
    }
    let name = data.settlement_name(&place);
    let mut event = GameEvent::new(
        EventKind::Crusade,
        format!(
            "Appel à défendre {name} contre {} : {} de secours {} dans la place.",
            data.faction_name(&rules.faction),
            count_noun(landed, "unité", "unités"),
            if landed > 1 { "entrent" } else { "entre" },
        ),
    )
    .faction(&master)
    // The besieging crusade reads it too, whoever plays.
    .public();
    if let Some(province) = state.settlement_province(&place) {
        event = event.province(province);
    }
    events.push(event);
}

/// « Débandade »: below the threshold a share of each unit's men goes home.
fn desert(
    state: &mut CampaignState,
    data: &GameData,
    rules: &CrusadeRules,
    events: &mut Vec<GameEvent>,
) {
    let fervor = state.crusade.as_ref().map_or(100, |c| c.fervor);
    let percent = rules.desertion.percent_at(fervor);
    if percent == 0 {
        return;
    }
    let mut lost = 0;
    for army in state.armies.values_mut() {
        if army.faction != rules.faction {
            continue;
        }
        for unit in &mut army.units {
            // Rounded down: a company never deserts to the last man.
            let leaving = unit.strength * percent / 100;
            unit.strength -= leaving;
            lost += leaving;
        }
    }
    if lost > 0 {
        events.push(
            GameEvent::new(
                EventKind::Crusade,
                format!(
                    "Débandade : la ferveur retombe et {lost} hommes de {} rentrent chez eux.",
                    data.faction_name(&rules.faction)
                ),
            )
            .faction(&rules.faction),
        );
    }
}

/// A field battle, assault, sortie or sea fight between `winner` and
/// `loser` was decided; `attacker` is the one of the two that sought it. A
/// victory over another faith lifts the fervour and a defeat lowers it; a
/// battle the crusade itself sought against its own faith lowers it too
/// (attacked by brothers in faith, it only defends itself: no malus).
/// Rebels and kindred churches (schismatics, not infidels) count for
/// neither.
pub fn on_battle(
    state: &mut CampaignState,
    data: &GameData,
    winner: &FactionId,
    loser: &FactionId,
    attacker: &FactionId,
) {
    let Some(rules) = active(state, data) else {
        return;
    };
    let (won, other) = if winner == &rules.faction {
        (true, loser)
    } else if loser == &rules.faction {
        (false, winner)
    } else {
        return;
    };
    let relation = if other.is_rebels() {
        None
    } else {
        Some(religion::faith_relation(state, data, &rules.faction, other))
    };
    match relation {
        Some(FaithRelation::Same | FaithRelation::RivalObedience) if attacker == &rules.faction => {
            change(
                state,
                rules,
                "Bataille livrée contre des frères de foi",
                rules.fervor.battle_same_faith,
            )
        }
        Some(FaithRelation::Different) if won => change(
            state,
            rules,
            "Victoire sur une autre foi",
            rules.fervor.battle_won_other_faith,
        ),
        _ => {}
    }
    if !won {
        change(state, rules, "Bataille perdue", rules.fervor.battle_lost);
    }
}

/// `taker` took `settlement` by arms: a place of the Holy Land lifts the
/// fervour; the city of the target province delivers it (event for all,
/// capital moved, prestige, floor). Also notes the loss of the target.
pub fn on_settlement_taken(
    state: &mut CampaignState,
    data: &GameData,
    taker: &FactionId,
    settlement: &SettlementId,
    events: &mut Vec<GameEvent>,
) {
    let Some(rules) = active(state, data) else {
        return;
    };
    let taken_before = state.crusade.as_ref().is_some_and(|c| c.target_taken);
    sync_target(state, data, events);
    let delivered = !taken_before && state.crusade.as_ref().is_some_and(|c| c.target_taken);
    if taker != &rules.faction {
        return;
    }
    let in_holy_land = state
        .settlement_province(settlement)
        .is_some_and(|p| rules.holy_land.contains(p));
    // JR5: each place lifts the fervour once, however often it changes hands.
    let first = in_holy_land
        && state.crusade.as_mut().is_some_and(|c| {
            let new = !c.counted_places.contains(settlement);
            if new {
                c.counted_places.push(settlement.clone());
            }
            new
        });
    if in_holy_land && first && !delivered {
        change(
            state,
            rules,
            "Place prise en Terre sainte",
            rules.fervor.holy_land_settlement_taken,
        );
    }
}

/// `aggressor` declared war on `target`: a crusade that turns on its own
/// faith loses its fervour.
pub fn on_war_declared(
    state: &mut CampaignState,
    data: &GameData,
    aggressor: &FactionId,
    target: &FactionId,
) {
    let Some(rules) = active(state, data) else {
        return;
    };
    if aggressor != &rules.faction || target.is_rebels() {
        return;
    }
    if matches!(
        religion::faith_relation(state, data, aggressor, target),
        FaithRelation::Same | FaithRelation::RivalObedience
    ) {
        change(
            state,
            rules,
            "Guerre déclarée à des frères de foi",
            rules.fervor.war_declared_same_faith,
        );
    }
}

/// Morale points (0-100 scale) the armies of `faction` get from fervour
/// (« Élan de la Croix »): a bonus at or above the high threshold, a malus
/// below the low one.
pub fn zeal_morale(state: &CampaignState, data: &GameData, faction: &FactionId) -> i32 {
    let Some(rules) = &data.crusade_rules else {
        return 0;
    };
    if &rules.faction != faction || active(state, data).is_none() {
        return 0;
    }
    state
        .crusade
        .as_ref()
        .map_or(0, |c| zeal_at(rules, c.fervor))
}

fn zeal_at(rules: &CrusadeRules, fervor: u8) -> i32 {
    if fervor >= rules.zeal.high_threshold {
        rules.zeal.high_morale
    } else if fervor < rules.zeal.low_threshold {
        rules.zeal.low_morale
    } else {
        0
    }
}

/// Cost of preaching at the faction's current prices (H5).
pub fn passage_cost(state: &CampaignState, data: &GameData, faction: &FactionId) -> i64 {
    data.crusade_rules.as_ref().map_or(0, |rules| {
        crate::coinage::priced(state, faction, rules.passage.cost)
    })
}

/// Why `faction` cannot preach the passage now (`None`: it can).
pub fn preach_blocker(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> Option<CrusadeError> {
    let Some(rules) = active(state, data).filter(|rules| &rules.faction == faction) else {
        return Some(CrusadeError::NotCrusaders);
    };
    let crusade = state.crusade.as_ref()?;
    if crusade.preach_cooldown > 0 {
        return Some(CrusadeError::Cooldown(crusade.preach_cooldown));
    }
    if held_ports(state, data, rules).is_empty() {
        return Some(CrusadeError::NoPort);
    }
    let needed = passage_cost(state, data, faction);
    let available = state.factions.get(faction).map_or(0, |f| f.treasury);
    if available < needed {
        return Some(CrusadeError::InsufficientFunds { needed, available });
    }
    None
}

/// `preach_passage`: pays, raises fervour, books a contingent sized by the
/// fervour of the call for the best held port, and starts the cooldown.
pub fn preach_passage(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> Result<(), CrusadeError> {
    if let Some(error) = preach_blocker(state, data, faction) {
        return Err(error);
    }
    let rules = active(state, data).ok_or(CrusadeError::NotCrusaders)?;
    let port = held_ports(state, data, rules)
        .into_iter()
        .next()
        .ok_or(CrusadeError::NoPort)?;
    let cost = passage_cost(state, data, faction);
    if let Some(f) = state.factions.get_mut(faction) {
        f.treasury -= cost;
    }
    change(state, rules, "Passage prêché", rules.fervor.preach);
    let turn = state.turn;
    let mut units = 0;
    if let Some(crusade) = state.crusade.as_mut() {
        units = rules.passage.units_at(crusade.fervor);
        crusade.preach_cooldown = rules.passage.cooldown_turns;
        crusade.pending_passages.push(PendingPassage {
            arrival_turn: turn + rules.passage.delay_turns,
            port: port.clone(),
            units,
        });
    }
    let mut event = GameEvent::new(
        EventKind::Crusade,
        format!(
            "{} prêche le passage : {} de volontaires {} à {} dans {}.",
            crate::events::capitalize(&data.faction_name(faction)),
            count_noun(units, "unité", "unités"),
            if units > 1 {
                "sont attendues"
            } else {
                "est attendue"
            },
            data.settlement_name(&port),
            count_noun(rules.passage.delay_turns, "tour", "tours")
        ),
    )
    .faction(faction);
    if let Some(province) = state.settlement_province(&port) {
        event = event.province(province);
    }
    state.push_order_event(event);
    Ok(())
}

/// Adds fresh units of `units` to the garrison of `settlement`; returns how
/// many were added (unknown unit types and settlements add nothing).
pub fn spawn_units_at_settlement(
    state: &mut CampaignState,
    data: &GameData,
    settlement: &SettlementId,
    units: &[UnitTypeId],
) -> u32 {
    let Some(place) = state.settlements.get_mut(settlement) else {
        return 0;
    };
    let fresh: Vec<Unit> = units
        .iter()
        .filter_map(|id| data.unit_types.get(id))
        .map(Unit::fresh)
        .collect();
    let added = fresh.len() as u32;
    place.garrison.extend(fresh);
    added
}

/// The fervour panel of `faction` (`None` unless it is the crusaders).
pub fn crusade_view(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> Option<CrusadeView> {
    let rules = active(state, data).filter(|rules| &rules.faction == faction)?;
    let crusade = state.crusade.as_ref()?;
    let blocker = preach_blocker(state, data, faction);
    // A contingent is sized after the call has lifted the fervour.
    let after_call = (i32::from(crusade.fervor) + rules.fervor.preach).clamp(0, 100) as u8;
    Some(CrusadeView {
        fervor: crusade.fervor,
        floor: floor(rules, crusade),
        alms: alms_at(rules, crusade.fervor),
        alms_last_turn: crusade.alms_last_turn,
        changes: crusade
            .last_changes
            .iter()
            .map(|(cause, delta)| FervorChange {
                cause: cause.clone(),
                delta: *delta,
            })
            .collect(),
        zeal_morale: zeal_at(rules, crusade.fervor),
        desertion_percent: rules.desertion.percent_at(crusade.fervor),
        zeal_high_threshold: rules.zeal.high_threshold,
        zeal_low_threshold: rules.zeal.low_threshold,
        zeal_high_morale: rules.zeal.high_morale,
        zeal_low_morale: rules.zeal.low_morale,
        desertion_threshold: rules.desertion.threshold,
        desertion_men_percent: rules.desertion.men_percent_per_turn,
        target_taken: crusade.target_taken,
        target_name: target_name(state, data, rules),
        passage_cost: passage_cost(state, data, faction),
        passage_cooldown: crusade.preach_cooldown,
        passage_available: blocker.is_none(),
        passage_blocker: blocker.map(|b| b.to_string()).unwrap_or_default(),
        passage_units: rules.passage.units_at(after_call),
        passage_delay: rules.passage.delay_turns,
        pending: crusade
            .pending_passages
            .iter()
            .map(|p| PendingPassageView {
                turns_left: p.arrival_turn.saturating_sub(state.turn),
                port: p.port.clone(),
                port_name: data.settlement_name(&p.port),
                units: p.units,
            })
            .collect(),
    })
}

/// AI: gold the crusade keeps aside for the passage (its price once the
/// cooldown is about to end and a port is held; 0 for every other faction
/// and for a crusade the player leads). Without it the planner spent the
/// alms on recruits and buildings, the passage was never affordable again
/// and the fervour wore down to nothing.
pub fn ai_passage_reserve(state: &CampaignState, data: &GameData, faction: &FactionId) -> i64 {
    if faction == &state.player_faction {
        return 0;
    }
    let Some(rules) = active(state, data).filter(|rules| &rules.faction == faction) else {
        return 0;
    };
    let soon = state
        .crusade
        .as_ref()
        .is_some_and(|c| c.preach_cooldown <= 1);
    if soon && !held_ports(state, data, rules).is_empty() {
        passage_cost(state, data, faction)
    } else {
        0
    }
}

/// AI: preaches as soon as the action is available and affordable.
pub fn ai_preach(state: &CampaignState, data: &GameData, faction: &FactionId) -> Vec<Order> {
    if preach_blocker(state, data, faction).is_none() {
        vec![Order::PreachPassage]
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    //! The scale on synthetic rules: the real data of 1337 with a crusade
    //! handed to an existing faction, so the tests do not depend on the
    //! values (or the faction) of `data/rules/crusade.json`.

    use std::sync::OnceLock;

    use data_model::test_support::{fac, game_data, prov};

    use super::*;
    use crate::orders::OrderError;
    use crate::state::{Army, ArmyPosition};

    const CRUSADERS: &str = "fac_cyprus";
    const HOLDER: &str = "fac_mamluks";
    const BASE: &str = "set_famagusta";
    const TARGET: &str = "prov_jerusalem";

    fn set(id: &str) -> SettlementId {
        SettlementId::new(id).unwrap()
    }

    fn synthetic_rules() -> CrusadeRules {
        serde_json::from_value(serde_json::json!({
            "faction": CRUSADERS,
            "base_settlement": BASE,
            "target_province": TARGET,
            "holy_land": [TARGET, "prov_gaza", "prov_safad"],
            "coastal_holy_land": ["prov_gaza", "prov_safad"],
            "fervor": {
                "start": 60,
                "decay_per_turn": 1,
                "decay_above_high": 2,
                "battle_won_other_faith": 6,
                "battle_lost": -8,
                "holy_land_settlement_taken": 10,
                "target_taken": 40,
                "target_floor": 50,
                "preach": 10,
                "war_declared_same_faith": -30,
                "battle_same_faith": -10,
                "truce_with_target_holder_per_turn": -2
            },
            "alms": { "base": 100, "per_fervor": 10 },
            "passage": {
                "cost": 1000,
                "cooldown_turns": 6,
                "delay_turns": 2,
                "units_base": 1,
                "fervor_per_extra_unit": 25,
                "max_units": 4,
                "unit_table": [
                    { "unit": "unit_crossbowmen", "weight": 3 },
                    { "unit": "unit_knights", "weight": 1 }
                ]
            },
            "zeal": {
                "high_threshold": 70,
                "high_morale": 10,
                "low_threshold": 30,
                "low_morale": -10
            },
            "desertion": { "threshold": 20, "men_percent_per_turn": 5, "zero_multiplier": 2 },
            "starting_army": ["unit_knights", "unit_crossbowmen", "unit_crossbowmen"],
            "target_taken_prestige": 40,
            "relief": {
                "units": 3,
                "cooldown_turns": 5,
                "unit_table": [{ "unit": "unit_urban_militia", "weight": 1 }]
            }
        }))
        .expect("synthetic rules are well formed")
    }

    fn data() -> &'static GameData {
        static DATA: OnceLock<GameData> = OnceLock::new();
        DATA.get_or_init(|| {
            let mut data = game_data().clone();
            data.crusade_rules = Some(synthetic_rules());
            data
        })
    }

    /// A campaign played by the synthetic crusaders, at war with the holder
    /// of the target (no peace penalty unless a test asks for it).
    fn campaign() -> CampaignState {
        let mut state = CampaignState::new_1337(data(), fac(CRUSADERS), 7).expect("campaign");
        for (a, b) in [(CRUSADERS, HOLDER), (HOLDER, CRUSADERS)] {
            let f = state.factions.get_mut(&fac(a)).unwrap();
            f.at_war_with.insert(fac(b));
            f.truces.remove(&fac(b));
        }
        state
    }

    /// Units of the crusaders: garrisons of their places and armies.
    fn crusader_units(state: &CampaignState) -> usize {
        let garrisons: usize = state
            .settlements
            .values()
            .filter(|s| s.controller == fac(CRUSADERS))
            .map(|s| s.garrison.len())
            .sum();
        let armies: usize = state
            .armies
            .values()
            .filter(|a| a.faction == fac(CRUSADERS))
            .map(|a| a.units.len())
            .sum();
        garrisons + armies
    }

    fn set_fervor(state: &mut CampaignState, fervor: u8) {
        state.crusade.as_mut().unwrap().fervor = fervor;
    }

    fn fervor(state: &CampaignState) -> u8 {
        state.crusade.as_ref().unwrap().fervor
    }

    fn end_of_turn(state: &mut CampaignState) -> Vec<GameEvent> {
        let mut events = Vec::new();
        resolve_crusade(state, data(), &mut events);
        state.turn += 1;
        events
    }

    fn hand(state: &mut CampaignState, settlement: &SettlementId, to: &str) {
        let s = state.settlements.get_mut(settlement).unwrap();
        s.controller = fac(to);
    }

    fn target_city(state: &CampaignState) -> SettlementId {
        state.province_city_id(&prov(TARGET)).unwrap().clone()
    }

    #[test]
    fn setup_opens_the_crusade_and_bases_the_army() {
        let state = campaign();
        let crusade = state.crusade.as_ref().expect("opened at setup");
        assert_eq!(crusade.fervor, 60);
        assert!(!crusade.target_taken);
        assert!(is_crusader(&state, data(), &fac(CRUSADERS)));
        assert!(!is_crusader(&state, data(), &fac(HOLDER)));
        let army = state
            .armies
            .values()
            .find(|a| a.faction == fac(CRUSADERS))
            .expect("starting army");
        assert!(army.is_at(&set(BASE)), "based in the rules' settlement");
        let types: Vec<&str> = army.units.iter().map(|u| u.unit_type.as_str()).collect();
        assert_eq!(
            types,
            ["unit_knights", "unit_crossbowmen", "unit_crossbowmen"]
        );
    }

    #[test]
    fn fervor_wears_off_and_stays_within_bounds() {
        let mut state = campaign();
        end_of_turn(&mut state);
        assert_eq!(fervor(&state), 59, "the vow wears off");
        assert_eq!(
            state.crusade.as_ref().unwrap().last_changes,
            vec![("Le vœu s'use".to_owned(), -1)]
        );
        // 0 is a floor, 100 a ceiling.
        set_fervor(&mut state, 0);
        end_of_turn(&mut state);
        assert_eq!(fervor(&state), 0);
        set_fervor(&mut state, 98);
        on_battle(
            &mut state,
            data(),
            &fac(CRUSADERS),
            &fac(HOLDER),
            &fac(CRUSADERS),
        );
        assert_eq!(fervor(&state), 100);
        // Only the points really gained are noted, under the turn's causes.
        let changes = &state.crusade.as_ref().unwrap().last_changes;
        assert_eq!(changes, &vec![("Victoire sur une autre foi".to_owned(), 2)]);
    }

    #[test]
    fn exaltation_falls_back_faster_above_the_high_threshold() {
        let mut state = campaign();
        set_fervor(&mut state, 80);
        end_of_turn(&mut state);
        assert_eq!(fervor(&state), 77, "1 of wear + 2 of exaltation");
        assert!(state
            .crusade
            .as_ref()
            .unwrap()
            .last_changes
            .contains(&("L'exaltation retombe".to_owned(), -2)));
        // Below the threshold, the plain wear only.
        set_fervor(&mut state, 69);
        end_of_turn(&mut state);
        assert_eq!(fervor(&state), 68);
    }

    #[test]
    fn a_contingent_respects_the_garrison_cap() {
        let mut state = campaign();
        let rules = synthetic_rules();
        let base = set(BASE);
        let cap = data()
            .settlement_rules
            .as_ref()
            .and_then(|r| r.garrison_cap.get(&state.settlements[&base].kind).copied())
            .expect("a cap");
        // Every port full: the volunteers form an army in the booked port.
        let militia = data()
            .unit_types
            .get(&UnitTypeId::new("unit_urban_militia").unwrap())
            .unwrap();
        for port in held_ports(&state, data(), &rules) {
            let place = state.settlements.get_mut(&port).unwrap();
            let kind_cap = data()
                .settlement_rules
                .as_ref()
                .and_then(|r| r.garrison_cap.get(&place.kind).copied())
                .unwrap();
            while place.garrison.len() < kind_cap {
                place.garrison.push(Unit::fresh(militia));
            }
        }
        state.armies.retain(|_, a| a.faction != fac(CRUSADERS));
        let before = crusader_units(&state);
        state.factions.get_mut(&fac(CRUSADERS)).unwrap().treasury = 5000;
        preach_passage(&mut state, data(), &fac(CRUSADERS)).expect("preached");
        end_of_turn(&mut state);
        end_of_turn(&mut state);
        assert_eq!(state.settlements[&base].garrison.len(), cap);
        assert_eq!(crusader_units(&state), before + 3);
        let army: Vec<_> = state
            .armies
            .values()
            .filter(|a| a.faction == fac(CRUSADERS))
            .collect();
        assert_eq!(army.len(), 1);
        assert_eq!(army[0].units.len(), 3);
        assert_eq!(army[0].settlement(), Some(&base));
    }

    #[test]
    fn the_target_and_each_place_lift_the_fervour_once() {
        let mut state = campaign();
        let city = target_city(&state);
        let ruler = state.factions[&fac(CRUSADERS)].ruler.clone().unwrap();
        let mut events = Vec::new();
        hand(&mut state, &city, CRUSADERS);
        on_settlement_taken(&mut state, data(), &fac(CRUSADERS), &city, &mut events);
        assert_eq!(fervor(&state), 100);
        let prestige = state.characters[&ruler].prestige;
        // Lost and delivered again: the floor and the seat, no new gain.
        hand(&mut state, &city, HOLDER);
        on_settlement_taken(&mut state, data(), &fac(HOLDER), &city, &mut events);
        set_fervor(&mut state, 30);
        hand(&mut state, &city, CRUSADERS);
        on_settlement_taken(&mut state, data(), &fac(CRUSADERS), &city, &mut events);
        assert_eq!(fervor(&state), 50, "lifted to the floor only");
        assert_eq!(state.characters[&ruler].prestige, prestige);
        assert_eq!(state.factions[&fac(CRUSADERS)].capital, prov(TARGET));
        // A place of the Holy Land counts once, however often retaken.
        set_fervor(&mut state, 60);
        let acre = set("set_acre");
        for (taker, gain) in [(CRUSADERS, 10), (HOLDER, 0), (CRUSADERS, 0)] {
            let before = fervor(&state);
            hand(&mut state, &acre, taker);
            on_settlement_taken(&mut state, data(), &fac(taker), &acre, &mut events);
            assert_eq!(i32::from(fervor(&state)) - i32::from(before), gain);
        }
    }

    #[test]
    fn the_capital_goes_back_when_the_target_is_lost() {
        let mut state = campaign();
        let seat = state.factions[&fac(CRUSADERS)].capital.clone();
        let city = target_city(&state);
        let mut events = Vec::new();
        hand(&mut state, &city, CRUSADERS);
        on_settlement_taken(&mut state, data(), &fac(CRUSADERS), &city, &mut events);
        assert_eq!(state.factions[&fac(CRUSADERS)].capital, prov(TARGET));
        hand(&mut state, &city, HOLDER);
        on_settlement_taken(&mut state, data(), &fac(HOLDER), &city, &mut events);
        assert_eq!(state.factions[&fac(CRUSADERS)].capital, seat);
        let lost = events.last().unwrap();
        assert!(lost.public && lost.loss, "{lost:?}");
    }

    #[test]
    fn the_master_of_a_besieged_holy_place_calls_its_defence_once() {
        let mut state = campaign();
        let city = target_city(&state);
        let besiege = |state: &mut CampaignState| {
            state.settlements.get_mut(&city).unwrap().siege = Some(
                serde_json::from_value(serde_json::json!({
                    "attacker": CRUSADERS,
                    "turns_left": 9
                }))
                .unwrap(),
            );
        };
        // A rebel place besieged first in the order does not use the call.
        let acre = set("set_acre");
        assert!(acre < city);
        hand(&mut state, &acre, "fac_rebels");
        state.settlements.get_mut(&acre).unwrap().siege = Some(
            serde_json::from_value(serde_json::json!({
                "attacker": CRUSADERS,
                "turns_left": 9
            }))
            .unwrap(),
        );
        let rebels = state.settlements[&acre].garrison.len();
        besiege(&mut state);
        let before = state.settlements[&city].garrison.len();
        let events = end_of_turn(&mut state);
        assert_eq!(state.settlements[&acre].garrison.len(), rebels);
        state.settlements.get_mut(&acre).unwrap().siege = None;
        let after = state.settlements[&city].garrison.len();
        assert_eq!(after, before + 3, "{events:?}");
        let call: Vec<_> = events
            .iter()
            .filter(|e| e.text_fr.contains("Appel à défendre"))
            .collect();
        assert_eq!(call.len(), 1, "{events:?}");
        assert_eq!(call[0].faction, Some(fac(HOLDER)));
        assert!(call[0].public, "the besieging crusade reads it too");
        assert_eq!(call[0].province, Some(prov(TARGET)));
        // Once per siege.
        end_of_turn(&mut state);
        assert_eq!(state.settlements[&city].garrison.len(), after);
        // A new siege waits for the cooldown.
        state.settlements.get_mut(&city).unwrap().siege = None;
        end_of_turn(&mut state);
        besiege(&mut state);
        end_of_turn(&mut state);
        assert_eq!(state.settlements[&city].garrison.len(), after);
        for _ in 0..3 {
            end_of_turn(&mut state);
        }
        assert!(state.settlements[&city].garrison.len() > after);
        // A place outside the Holy Land is never relieved.
        let elsewhere = set("set_limassol");
        let mut other = campaign();
        other.settlements.get_mut(&elsewhere).unwrap().controller = fac(HOLDER);
        other.settlements.get_mut(&elsewhere).unwrap().siege = Some(
            serde_json::from_value(serde_json::json!({
                "attacker": CRUSADERS,
                "turns_left": 9
            }))
            .unwrap(),
        );
        let size = other.settlements[&elsewhere].garrison.len();
        end_of_turn(&mut other);
        assert_eq!(other.settlements[&elsewhere].garrison.len(), size);
    }

    #[test]
    fn peace_with_the_holder_of_the_target_costs_fervor() {
        let mut state = campaign();
        for (a, b) in [(CRUSADERS, HOLDER), (HOLDER, CRUSADERS)] {
            let f = state.factions.get_mut(&fac(a)).unwrap();
            f.at_war_with.remove(&fac(b));
        }
        end_of_turn(&mut state);
        assert_eq!(fervor(&state), 60 - 1 - 2);
    }

    #[test]
    fn battles_and_wars_move_the_gauge() {
        let mut state = campaign();
        let brothers = fac("fac_hospitallers");
        // Lost to another faith: only the defeat.
        on_battle(
            &mut state,
            data(),
            &fac(HOLDER),
            &fac(CRUSADERS),
            &fac(HOLDER),
        );
        assert_eq!(fervor(&state), 52);
        // Won against the same faith it attacked: the scandal, no gain.
        on_battle(
            &mut state,
            data(),
            &fac(CRUSADERS),
            &brothers,
            &fac(CRUSADERS),
        );
        assert_eq!(fervor(&state), 42);
        // Lost against the same faith it attacked: both.
        on_battle(
            &mut state,
            data(),
            &brothers,
            &fac(CRUSADERS),
            &fac(CRUSADERS),
        );
        assert_eq!(fervor(&state), 24);
        // Attacked by brothers in faith, it only defends itself: no
        // scandal, won or lost (the defeat still counts).
        on_battle(&mut state, data(), &fac(CRUSADERS), &brothers, &brothers);
        assert_eq!(fervor(&state), 24);
        on_battle(&mut state, data(), &brothers, &fac(CRUSADERS), &brothers);
        assert_eq!(fervor(&state), 16);
        set_fervor(&mut state, 24);
        // Other factions' battles and wars are none of its business.
        on_battle(&mut state, data(), &fac(HOLDER), &brothers, &fac(HOLDER));
        on_war_declared(&mut state, data(), &fac(HOLDER), &fac(CRUSADERS));
        on_war_declared(&mut state, data(), &fac(CRUSADERS), &fac(HOLDER));
        assert_eq!(fervor(&state), 24);
        set_fervor(&mut state, 60);
        on_war_declared(
            &mut state,
            data(),
            &fac(CRUSADERS),
            &fac("fac_hospitallers"),
        );
        assert_eq!(fervor(&state), 30);
        // Through the real order too.
        set_fervor(&mut state, 60);
        state
            .declare_war(data(), &fac(CRUSADERS), &fac("fac_papacy"))
            .expect("war declared");
        assert_eq!(fervor(&state), 30);
    }

    #[test]
    fn holy_land_places_and_the_target_floor() {
        let mut state = campaign();
        let mut events = Vec::new();
        // A place of the Holy Land.
        hand(&mut state, &set("set_acre"), CRUSADERS);
        on_settlement_taken(
            &mut state,
            data(),
            &fac(CRUSADERS),
            &set("set_acre"),
            &mut events,
        );
        assert_eq!(fervor(&state), 70);
        assert!(events.is_empty());
        // A place elsewhere gives nothing.
        on_settlement_taken(
            &mut state,
            data(),
            &fac(CRUSADERS),
            &set("set_limassol"),
            &mut events,
        );
        assert_eq!(fervor(&state), 70);
        // The city of the vow: +40 (not +10 more), capped, with its event.
        set_fervor(&mut state, 5);
        let ruler = state.factions[&fac(CRUSADERS)].ruler.clone().unwrap();
        let prestige = state.characters[&ruler].prestige;
        let city = target_city(&state);
        hand(&mut state, &city, CRUSADERS);
        on_settlement_taken(&mut state, data(), &fac(CRUSADERS), &city, &mut events);
        assert_eq!(fervor(&state), 50, "5 + 40, lifted to the floor");
        assert!(state.crusade.as_ref().unwrap().target_taken);
        assert_eq!(state.factions[&fac(CRUSADERS)].capital, prov(TARGET));
        assert_eq!(state.characters[&ruler].prestige, prestige + 40);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, EventKind::Crusade);
        assert_eq!(events[0].province, Some(prov(TARGET)));
        assert!(events[0].public && !events[0].loss, "{}", events[0].text_fr);
        // The floor holds against defeats and wear.
        set_fervor(&mut state, 53);
        on_battle(
            &mut state,
            data(),
            &fac(HOLDER),
            &fac(CRUSADERS),
            &fac(HOLDER),
        );
        assert_eq!(fervor(&state), 50);
        end_of_turn(&mut state);
        assert_eq!(fervor(&state), 50);
        assert_eq!(
            crusade_view(&state, data(), &fac(CRUSADERS)).unwrap().floor,
            50
        );
        // Delivered only once: a later capture there gives nothing more.
        let mut again = Vec::new();
        on_settlement_taken(&mut state, data(), &fac(CRUSADERS), &city, &mut again);
        assert!(again.is_empty());
        assert_eq!(
            fervor(&state),
            50,
            "the city was counted at its deliverance"
        );
        // Lost: the floor goes, with an event.
        hand(&mut state, &city, HOLDER);
        let lost = end_of_turn(&mut state);
        assert!(!state.crusade.as_ref().unwrap().target_taken);
        assert_eq!(fervor(&state), 49);
        // The loss is news for every faction too.
        assert!(lost.iter().any(|e| e.public && e.loss), "{lost:?}");
    }

    #[test]
    fn the_capture_of_the_target_city_goes_through_the_hook() {
        let mut state = campaign();
        let city = target_city(&state);
        let mut events = Vec::new();
        crate::siege::capture(&mut state, data(), &city, &fac(CRUSADERS), &mut events);
        assert!(state.crusade.as_ref().unwrap().target_taken);
        assert_eq!(fervor(&state), 100);
        assert!(events
            .iter()
            .any(|e| e.kind == EventKind::Crusade && e.text_fr.contains("délivrée")));
        // Retaken by its former master: the floor goes at once.
        crate::siege::capture(&mut state, data(), &city, &fac(HOLDER), &mut events);
        assert!(!state.crusade.as_ref().unwrap().target_taken);
    }

    #[test]
    fn alms_follow_fervor_and_are_income() {
        let mut state = campaign();
        assert_eq!(alms(&state, data(), &fac(CRUSADERS)), 100 + 10 * 60);
        assert_eq!(alms(&state, data(), &fac(HOLDER)), 0);
        let with = state.faction_income(data(), &fac(CRUSADERS));
        set_fervor(&mut state, 10);
        let poorer = state.faction_income(data(), &fac(CRUSADERS));
        assert_eq!(with - poorer, 500, "alms are part of the income");
        let view = crusade_view(&state, data(), &fac(CRUSADERS)).unwrap();
        assert_eq!(view.alms, 200);
        end_of_turn(&mut state);
        assert_eq!(state.crusade.as_ref().unwrap().alms_last_turn, 200);
        // A dead crusade pays nothing.
        state.factions.get_mut(&fac(CRUSADERS)).unwrap().alive = false;
        assert_eq!(alms(&state, data(), &fac(CRUSADERS)), 0);
    }

    #[test]
    fn preach_is_refused_with_a_reason() {
        let mut state = campaign();
        // Not the crusader faction.
        assert_eq!(
            preach_passage(&mut state, data(), &fac(HOLDER)),
            Err(CrusadeError::NotCrusaders)
        );
        assert!(crusade_view(&state, data(), &fac(HOLDER)).is_none());
        // Not enough money.
        state.factions.get_mut(&fac(CRUSADERS)).unwrap().treasury = 999;
        assert_eq!(
            preach_blocker(&state, data(), &fac(CRUSADERS)),
            Some(CrusadeError::InsufficientFunds {
                needed: 1000,
                available: 999
            })
        );
        let view = crusade_view(&state, data(), &fac(CRUSADERS)).unwrap();
        assert!(!view.passage_available);
        assert!(view.passage_blocker.contains("Trésor insuffisant"));
        assert!(ai_preach(&state, data(), &fac(CRUSADERS)).is_empty());
        // Through the order, in French.
        match state.submit_order(data(), Order::PreachPassage) {
            Err(OrderError::Crusade(CrusadeError::InsufficientFunds { .. })) => {}
            other => panic!("unexpected {other:?}"),
        }
        // No port held.
        state.factions.get_mut(&fac(CRUSADERS)).unwrap().treasury = 5000;
        let ports: Vec<SettlementId> = held_ports(&state, data(), &synthetic_rules());
        assert_eq!(ports.first(), Some(&set(BASE)), "the base comes first");
        for port in &ports {
            hand(&mut state, port, HOLDER);
        }
        assert_eq!(
            preach_blocker(&state, data(), &fac(CRUSADERS)),
            Some(CrusadeError::NoPort)
        );
        for port in &ports {
            hand(&mut state, port, CRUSADERS);
        }
        // Accepted once, then on cooldown.
        assert_eq!(
            ai_preach(&state, data(), &fac(CRUSADERS)),
            vec![Order::PreachPassage]
        );
        state
            .submit_order(data(), Order::PreachPassage)
            .expect("preached");
        assert_eq!(state.factions[&fac(CRUSADERS)].treasury, 4000);
        assert_eq!(fervor(&state), 70);
        assert_eq!(
            preach_blocker(&state, data(), &fac(CRUSADERS)),
            Some(CrusadeError::Cooldown(6))
        );
        assert_eq!(
            CrusadeError::Cooldown(6).to_string(),
            "Passage déjà prêché : nouvel appel dans 6 tours."
        );
        assert_eq!(
            CrusadeError::Cooldown(1).to_string(),
            "Passage déjà prêché : nouvel appel dans 1 tour."
        );
        for _ in 0..6 {
            end_of_turn(&mut state);
        }
        assert_eq!(preach_blocker(&state, data(), &fac(CRUSADERS)), None);
    }

    #[test]
    fn contingent_size_follows_fervor() {
        let passage = synthetic_rules().passage;
        assert_eq!(passage.units_at(0), 1);
        assert_eq!(passage.units_at(24), 1);
        assert_eq!(passage.units_at(25), 2);
        assert_eq!(passage.units_at(60), 3);
        assert_eq!(passage.units_at(100), 4, "capped by max_units");
        let state = campaign();
        let view = crusade_view(&state, data(), &fac(CRUSADERS)).unwrap();
        assert_eq!(view.passage_units, 3, "sized at 60 + 10");
        assert_eq!(view.passage_cost, 1000);
        assert_eq!(view.passage_delay, 2);
    }

    #[test]
    fn contingent_lands_two_turns_later() {
        let mut state = campaign();
        state.factions.get_mut(&fac(CRUSADERS)).unwrap().treasury = 5000;
        preach_passage(&mut state, data(), &fac(CRUSADERS)).expect("preached");
        let pending = state.crusade.as_ref().unwrap().pending_passages.clone();
        assert_eq!(
            pending,
            vec![PendingPassage {
                arrival_turn: state.turn + 2,
                port: set(BASE),
                units: 3
            }]
        );
        let view = crusade_view(&state, data(), &fac(CRUSADERS)).unwrap();
        assert_eq!(view.pending.len(), 1);
        assert_eq!(view.pending[0].turns_left, 2);
        let before = crusader_units(&state);
        // End of the turn of the call: still at sea.
        end_of_turn(&mut state);
        assert_eq!(crusader_units(&state), before);
        // End of the next one: ashore for the second turn after the call
        // (in the port's garrison within its cap, the rest elsewhere).
        let events = end_of_turn(&mut state);
        assert_eq!(crusader_units(&state), before + 3);
        assert!(state.crusade.as_ref().unwrap().pending_passages.is_empty());
        assert!(events.iter().any(|e| e.kind == EventKind::Crusade));
    }

    #[test]
    fn contingent_finds_another_port_or_is_lost() {
        let mut state = campaign();
        let rules = synthetic_rules();
        state.factions.get_mut(&fac(CRUSADERS)).unwrap().treasury = 5000;
        preach_passage(&mut state, data(), &fac(CRUSADERS)).expect("preached");
        // The booked port falls: another held port receives the volunteers.
        hand(&mut state, &set(BASE), HOLDER);
        let fallback = held_ports(&state, data(), &rules)
            .into_iter()
            .next()
            .expect("another port");
        let before = crusader_units(&state);
        let garrison = state.settlements[&fallback].garrison.len();
        end_of_turn(&mut state);
        end_of_turn(&mut state);
        assert_eq!(crusader_units(&state), before + 3);
        assert!(state.settlements[&fallback].garrison.len() > garrison);

        // No port at all: lost, with an event.
        let mut state = campaign();
        state.factions.get_mut(&fac(CRUSADERS)).unwrap().treasury = 5000;
        preach_passage(&mut state, data(), &fac(CRUSADERS)).expect("preached");
        for port in held_ports(&state, data(), &rules) {
            hand(&mut state, &port, HOLDER);
        }
        end_of_turn(&mut state);
        let events = end_of_turn(&mut state);
        assert!(state.crusade.as_ref().unwrap().pending_passages.is_empty());
        assert!(events
            .iter()
            .any(|e| e.kind == EventKind::Crusade && e.text_fr.contains("se disperse")));
    }

    #[test]
    fn zeal_and_desertion_follow_the_thresholds() {
        let mut state = campaign();
        let crusaders = fac(CRUSADERS);
        assert_eq!(zeal_morale(&state, data(), &crusaders), 0);
        set_fervor(&mut state, 70);
        assert_eq!(zeal_morale(&state, data(), &crusaders), 10);
        assert_eq!(zeal_morale(&state, data(), &fac(HOLDER)), 0);
        set_fervor(&mut state, 29);
        assert_eq!(zeal_morale(&state, data(), &crusaders), -10);

        // One army of 1 000 men in one unit.
        let unit_type = &data().unit_types[&UnitTypeId::new("unit_crossbowmen").unwrap()];
        let mut unit = Unit::fresh(unit_type);
        unit.strength = 1000;
        unit.max_strength = 1000;
        state.armies.retain(|_, a| a.faction != crusaders);
        let id = state.allocate_army_id();
        state.armies.insert(
            id.clone(),
            Army::new(
                crusaders.clone(),
                ArmyPosition::Settlement(set(BASE)),
                vec![unit],
            ),
        );
        // 21 → 20 after the wear: still at the threshold, nobody leaves.
        set_fervor(&mut state, 21);
        end_of_turn(&mut state);
        assert_eq!(state.armies[&id].total_strength(), 1000);
        // 20 → 19: 5 % go home.
        let events = end_of_turn(&mut state);
        assert_eq!(state.armies[&id].total_strength(), 950);
        assert!(events.iter().any(|e| e.text_fr.contains("Débandade")));
        assert_eq!(
            crusade_view(&state, data(), &crusaders)
                .unwrap()
                .desertion_percent,
            5
        );
        // At 0 the share doubles.
        set_fervor(&mut state, 0);
        end_of_turn(&mut state);
        assert_eq!(state.armies[&id].total_strength(), 855);
        // Other factions' armies are untouched.
        let other: u32 = state
            .armies
            .values()
            .filter(|a| a.faction == fac(HOLDER))
            .map(Army::total_strength)
            .sum();
        end_of_turn(&mut state);
        let after: u32 = state
            .armies
            .values()
            .filter(|a| a.faction == fac(HOLDER))
            .map(Army::total_strength)
            .sum();
        assert_eq!(other, after);
    }

    #[test]
    fn inert_without_rules_state_or_a_living_faction() {
        // Older save: no `crusade` field.
        let mut state = campaign();
        let mut json: serde_json::Value = serde_json::from_str(&state.save_json()).unwrap();
        assert!(json.as_object_mut().unwrap().remove("crusade").is_some());
        let mut old = CampaignState::load_json(&json.to_string()).expect("older save loads");
        assert!(old.crusade.is_none());
        assert!(end_of_turn(&mut old).is_empty());
        assert_eq!(alms(&old, data(), &fac(CRUSADERS)), 0);
        assert_eq!(
            preach_blocker(&old, data(), &fac(CRUSADERS)),
            Some(CrusadeError::NotCrusaders)
        );
        // The state survives a save, causes included.
        end_of_turn(&mut state);
        let reloaded = CampaignState::load_json(&state.save_json()).expect("save loads");
        assert_eq!(reloaded.crusade, state.crusade);
        // Dead faction: frozen.
        state.factions.get_mut(&fac(CRUSADERS)).unwrap().alive = false;
        let frozen = state.crusade.clone();
        assert!(end_of_turn(&mut state).is_empty());
        on_battle(
            &mut state,
            data(),
            &fac(HOLDER),
            &fac(CRUSADERS),
            &fac(HOLDER),
        );
        assert_eq!(state.crusade, frozen);
    }
}
