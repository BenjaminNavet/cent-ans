//! Sieges, captures and chevauchées (spec § 1.3 steps 3 and 4; spec § 2 for
//! the besieging general's `SiegeSpeed` and the `trait_siege_master`/
//! `trait_cruel` triggers).
//!
//! Lot C4: sieges and captures apply to settlements (M8 logic unchanged);
//! the chevauchée still devastates the whole province. A village held by an
//! enemy is taken as soon as an army enters it when it has no garrison;
//! otherwise it is stormed at once (no walls) and besieged by any hostile
//! army standing on it, whatever its stance.

use data_model::{FactionId, GameData, ProvinceId, SettlementId, SettlementKind};

use crate::dynasty;
use crate::economy::province_income;
use crate::events::{EventKind, GameEvent};
use crate::skills;
use crate::state::{ArmyId, CampaignState, SiegeState, Stance};

/// Base siege duration in turns, added to the fortification level.
pub const SIEGE_BASE_TURNS: u32 = 2;
/// Devastation added by one turn of chevauchée.
pub const RAID_DEVASTATION: u8 = 30;
/// Unrest added to a province by a chevauchée.
pub const RAID_UNREST: u8 = 10;
/// Share of the province's seasonal tax base taken as loot.
pub const RAID_LOOT_SHARE: f64 = 0.5;
/// Unrest added to a province when its city changes hands (half for
/// another settlement).
/// TW2-T1: the value now read is `occupy.unrest_city` of
/// `data/rules/capture.json`; kept for reference.
pub const CAPTURE_UNREST: u8 = 20;

fn province_name(data: &GameData, id: &ProvinceId) -> String {
    data.provinces
        .get(id)
        .map_or_else(|| id.to_string(), |p| p.name.display.clone())
}

/// Display name of a settlement (its id when unknown).
pub fn settlement_name(data: &GameData, id: &SettlementId) -> String {
    data.settlements
        .get(id)
        .map_or_else(|| id.to_string(), |s| s.name.display.clone())
}

fn faction_name(data: &GameData, id: &FactionId) -> String {
    data.factions
        .get(id)
        .map_or_else(|| id.to_string(), |f| f.short_or_display_name().to_owned())
}

/// Armies on `settlement` besieging its controller, sorted by id: armies in
/// `Siege` stance, or any hostile army when the settlement is a village.
pub(crate) fn besiegers(
    state: &CampaignState,
    settlement: &SettlementId,
    controller: &FactionId,
) -> Vec<ArmyId> {
    let village = state.settlement_kind(settlement) == SettlementKind::Village;
    state
        .armies
        .iter()
        .filter(|(_, army)| {
            army.is_at(settlement)
                && (village || army.stance == Stance::Siege)
                && state.is_at_war(&army.faction, controller)
        })
        .map(|(id, _)| id.clone())
        .collect()
}

/// Province of a settlement, for events (a placeholder when unknown).
pub(crate) fn province_of(state: &CampaignState, settlement: &SettlementId) -> ProvinceId {
    state
        .settlement_province(settlement)
        .cloned()
        .unwrap_or_else(|| ProvinceId::new("prov_unknown").expect("well-formed id"))
}

/// Ends the siege of `settlement`, if any, with a `SiegeLifted` event.
fn lift_siege(
    state: &mut CampaignState,
    data: &GameData,
    settlement: &SettlementId,
    events: &mut Vec<GameEvent>,
) {
    let Some(entry) = state.settlements.get_mut(settlement) else {
        return;
    };
    if entry.siege.take().is_none() {
        return;
    }
    let controller = entry.controller.clone();
    events.push(
        GameEvent::new(
            EventKind::SiegeLifted,
            format!(
                "Le siège {} est levé.",
                crate::events::de(&settlement_name(data, settlement))
            ),
        )
        .province(&province_of(state, settlement))
        .faction(&controller),
    );
}

/// `army_id` besieged `settlement` before an order: once it has marched
/// off, it leaves the siege stance, and the siege is lifted at once when
/// nobody else besieges the place (not at the end of the turn, which left
/// the camp drawn around the place and on the army).
pub(crate) fn leave_siege(
    state: &mut CampaignState,
    data: &GameData,
    army_id: &ArmyId,
    settlement: &SettlementId,
    events: &mut Vec<GameEvent>,
) {
    let Some(army) = state.armies.get_mut(army_id) else {
        return;
    };
    if army.is_at(settlement) {
        return;
    }
    if army.stance == Stance::Siege {
        army.stance = Stance::Normal;
    }
    let Some(controller) = state
        .settlements
        .get(settlement)
        .map(|s| s.controller.clone())
    else {
        return;
    };
    if besiegers(state, settlement, &controller).is_empty() {
        lift_siege(state, data, settlement, events);
    }
}

/// Phase 3: progress, start or lift sieges; capture settlements.
pub(crate) fn resolve_sieges(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let ids: Vec<SettlementId> = state.settlements.keys().cloned().collect();
    for settlement_id in ids {
        let controller = state.settlements[&settlement_id].controller.clone();
        let province_id = province_of(state, &settlement_id);
        let besiegers = besiegers(state, &settlement_id, &controller);
        let defenders = state.friendly_armies_at(&controller, &settlement_id);
        if besiegers.is_empty() || !defenders.is_empty() {
            lift_siege(state, data, &settlement_id, events);
            continue;
        }
        let attacker = siege_leader(state, &settlement_id, &besiegers);
        let lead = besiegers
            .iter()
            .find(|id| state.armies[*id].faction == attacker)
            .unwrap_or(&besiegers[0])
            .clone();
        let garrison_empty = state.settlements[&settlement_id].garrison.is_empty();
        if garrison_empty {
            capture(state, data, &settlement_id, &attacker, events);
            continue;
        }
        let fortification = state.fortification_level(data, &settlement_id);
        let besieging_general = state.armies[&lead].general.clone();
        let siege_speed_percent = siege_speed_percent(state, data, &lead);
        // NT5 (N7): the besiegers build their engines.
        let engine_gain = crate::siege_engines::work_per_turn(
            data,
            crate::siege_engines::men(state, &besiegers),
            siege_speed_percent,
        );
        // M8: the garrison sallies out when it outmatches the besiegers.
        if sortie(state, data, &settlement_id, &besiegers, events) {
            continue;
        }
        let resistance = state.siege_resistance(data, &settlement_id, &attacker);
        let breach_gain = (f64::from(breach_per_turn(state, data, &besiegers, fortification))
            * (1.0 - resistance / 100.0))
            .round() as u8;
        let drain = supplies_drain(fortification, siege_speed_percent);
        let turn = state.turn;
        let settlement = state.settlements.get_mut(&settlement_id).expect("exists");
        match &mut settlement.siege {
            // Lot M2: a siege begun during this turn (an army entered the
            // place) only starts counting at the next one. Its engines are
            // built from the start: marches are immediate, the army spends
            // the rest of the turn in camp (the ETA shown counts this turn).
            Some(siege) if siege.attacker == attacker && siege.started_turn == turn => {
                siege.engine_work = siege.engine_work.saturating_add(engine_gain);
            }
            Some(siege) if siege.attacker == attacker => {
                siege.turns_elapsed += 1;
                siege.engine_work = siege.engine_work.saturating_add(engine_gain);
                siege.breach = siege.breach.saturating_add(breach_gain).min(100);
                siege.supplies = siege.supplies.saturating_sub(drain);
                siege.turns_left = turns_to_starve(siege.supplies, drain);
                if siege.supplies == 0 {
                    settlement.garrison.clear();
                    events.push(
                        GameEvent::new(
                            EventKind::ProvinceCaptured,
                            format!(
                                "Affamée, la garnison de {} capitule.",
                                settlement_name(data, &settlement_id)
                            ),
                        )
                        .province(&province_id)
                        .faction(&attacker),
                    );
                    capture(state, data, &settlement_id, &attacker, events);
                    if let Some(general) = besieging_general {
                        dynasty::on_siege_won(state, data, &general);
                        crate::retinue::try_acquire(
                            state,
                            data,
                            &general,
                            data_model::AcquisitionTrigger::SiegeWon,
                            &[],
                            events,
                        );
                    }
                }
            }
            _ => begin_siege(state, data, &settlement_id, &attacker, &lead, events),
        }
    }
}

/// The faction leading the siege of `settlement` among `besiegers` (not
/// empty): the current attacker while one of its armies stays; else an ally
/// of it takes the siege over with its progress (breach, supplies); else the
/// first besieger.
fn siege_leader(
    state: &mut CampaignState,
    settlement: &SettlementId,
    besiegers: &[ArmyId],
) -> FactionId {
    let first = state.armies[&besiegers[0]].faction.clone();
    let Some(current) = state
        .settlements
        .get(settlement)
        .and_then(|s| s.siege.as_ref())
        .map(|s| s.attacker.clone())
    else {
        return first;
    };
    let factions: Vec<FactionId> = besiegers
        .iter()
        .map(|id| state.armies[id].faction.clone())
        .collect();
    if factions.contains(&current) {
        return current;
    }
    let Some(heir) = factions
        .into_iter()
        .find(|f| state.is_allied(f, &current) || state.is_allied(&current, f))
    else {
        return first;
    };
    if let Some(siege) = state
        .settlements
        .get_mut(settlement)
        .and_then(|s| s.siege.as_mut())
    {
        siege.attacker = heir.clone();
    }
    heir
}

/// Lays siege to `settlement_id` for `attacker` (its army `army` in the
/// lead): supplies from the province's devastation, an event. A siege of
/// the same attacker or of one of its allies already in place is kept.
pub(crate) fn begin_siege(
    state: &mut CampaignState,
    data: &GameData,
    settlement_id: &SettlementId,
    attacker: &FactionId,
    army: &ArmyId,
    events: &mut Vec<GameEvent>,
) {
    let province_id = province_of(state, settlement_id);
    let fortification = state.fortification_level(data, settlement_id);
    let siege_speed_percent = siege_speed_percent(state, data, army);
    let drain = supplies_drain(fortification, siege_speed_percent);
    let devastation = state
        .provinces
        .get(&province_id)
        .map_or(0, |p| p.devastation);
    let turn = state.turn;
    let kept = state
        .settlements
        .get(settlement_id)
        .and_then(|s| s.siege.as_ref())
        .is_some_and(|s| {
            state.is_allied(&s.attacker, attacker) || state.is_allied(attacker, &s.attacker)
        });
    let Some(settlement) = state.settlements.get_mut(settlement_id) else {
        return;
    };
    if kept {
        return;
    }
    let supplies = 100u8.saturating_sub(devastation / 2).max(10);
    let turns_left = turns_to_starve(supplies, drain);
    settlement.siege = Some(SiegeState {
        attacker: attacker.clone(),
        turns_left,
        turns_elapsed: 0,
        supplies,
        breach: 0,
        started_turn: turn,
        engine_work: 0,
    });
    events.push(
        GameEvent::new(
            EventKind::SiegeStarted,
            format!(
                "{} met le siège devant {}.",
                faction_name(data, attacker),
                settlement_name(data, settlement_id)
            ),
        )
        .province(&province_id)
        .army(army)
        .faction(attacker),
    );
}

/// `SiegeSpeed` (percent) of `army` besieging: its general's skills and
/// traits plus its traditions (0 for an unknown army).
pub(crate) fn siege_speed_percent(state: &CampaignState, data: &GameData, army: &ArmyId) -> f64 {
    let Some(a) = state.armies.get(army) else {
        return 0.0;
    };
    a.general.as_ref().map_or(0.0, |g| {
        skills::character_effects(state, data, g)
            .siege_speed
            .apply(0.0)
    }) + crate::traditions::siege_speed_percent(data, a)
}

/// Food lost per turn of siege: a town lasts `SIEGE_BASE_TURNS +
/// fortification` turns, shortened by the besieging general's `SiegeSpeed`.
pub fn supplies_drain(fortification: u32, siege_speed_percent: f64) -> u8 {
    let base_duration = f64::from(SIEGE_BASE_TURNS + fortification);
    let duration = (base_duration * (1.0 - siege_speed_percent / 100.0)).max(1.0);
    (100.0 / duration).ceil().min(100.0) as u8
}

fn turns_to_starve(supplies: u8, drain: u8) -> u32 {
    u32::from(supplies).div_ceil(u32::from(drain.max(1)))
}

/// Wall damage per turn from the besiegers' engines (`siege_attack`).
pub fn breach_per_turn(
    state: &CampaignState,
    data: &GameData,
    besiegers: &[ArmyId],
    fortification: u32,
) -> u8 {
    let attack: f64 = besiegers
        .iter()
        .filter_map(|id| state.armies.get(id))
        .flat_map(|a| a.units.iter())
        .filter_map(|u| {
            let t = data.unit_types.get(&u.unit_type)?;
            let ratio = f64::from(u.strength) / f64::from(u.max_strength.max(1));
            t.stats.siege_attack.map(|v| f64::from(v) * ratio)
        })
        .sum();
    (attack / (2.0 * (1.0 + f64::from(fortification))))
        .round()
        .min(100.0) as u8
}

/// A field army made of a settlement's garrison, for siege battles (the
/// province's governor leads the garrison of its city).
pub(crate) fn garrison_army(
    state: &CampaignState,
    settlement: &SettlementId,
) -> Option<crate::state::Army> {
    let s = state.settlements.get(settlement)?;
    let is_city = state.province_city_id(&s.province) == Some(settlement);
    let general = if is_city {
        state
            .province_governor(&s.province)
            .filter(|g| {
                state
                    .characters
                    .get(*g)
                    .is_some_and(|c| c.faction == s.controller)
            })
            .cloned()
    } else {
        None
    };
    let mut army = crate::state::Army::new(
        s.controller.clone(),
        crate::state::ArmyPosition::Settlement(settlement.clone()),
        s.garrison.clone(),
    );
    army.general = general;
    Some(army)
}

fn apply_garrison_losses(
    state: &mut CampaignState,
    settlement: &SettlementId,
    outcome: &crate::battle_auto::SideOutcome,
) {
    if let Some(s) = state.settlements.get_mut(settlement) {
        for (unit, losses) in s.garrison.iter_mut().zip(&outcome.losses) {
            unit.strength = unit.strength.saturating_sub(*losses);
            unit.morale = (i32::from(unit.morale) + outcome.morale_delta).clamp(0, 100) as u8;
        }
        s.garrison
            .retain(|u| u.strength > 0 && u.strength * 20 >= u.max_strength);
    }
}

/// `true` when the besiegers carry siege towers (`wall_assault`).
fn has_siege_towers(state: &CampaignState, data: &GameData, army: &ArmyId) -> bool {
    state.armies.get(army).is_some_and(|a| {
        a.units.iter().any(|u| {
            data.unit_types
                .get(&u.unit_type)
                .is_some_and(|t| t.abilities.contains(&data_model::Ability::WallAssault))
        })
    })
}

/// Why an assault order was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AssaultError {
    #[error("armée inconnue")]
    UnknownArmy,
    #[error("cette armée n'assiège pas cette place")]
    NotBesieging,
    /// NT5 (N7): walls standing and no engine ready yet.
    #[error("assaut impossible : {0}")]
    NoEngine(String),
}

impl CampaignState {
    /// Odds (0-100) that `army` storms the settlement it besieges, and
    /// whether the walls still stand (for the UI; a quick estimate, not the
    /// resolution).
    pub fn assault_odds(&self, data: &GameData, army: &ArmyId) -> Option<(u32, bool)> {
        let a = self.armies.get(army)?;
        let place = a.settlement()?;
        let settlement = self.settlements.get(place)?;
        let siege = settlement.siege.as_ref()?;
        if siege.attacker != a.faction {
            return None;
        }
        let walls = walls_stand(self, data, army, place);
        // NT9: a ready ram (gate broken) softens the walls, as in `auto_assault`.
        let walls_factor = if walls {
            0.7 * (1.0 + f64::from(self.engine_assault_bonus(data, place)) / 100.0)
        } else {
            1.0
        };
        let attack = self.army_power(data, army) * walls_factor;
        let defence = crate::state::unit_power(data, &settlement.garrison)
            * (1.0 + f64::from(self.fortification_level(data, place)) * 0.1);
        let odds = (100.0 * attack / (attack + defence).max(1.0)).round() as u32;
        Some((odds, walls))
    }

    /// `assault { army }`: storm the walls now (M8). Victory takes the
    /// settlement; defeat bloodies the besiegers, the siege goes on. With
    /// `interactive_battles`, an assault by or against the player becomes a
    /// pending siege battle (M8 § 2) fought in 3D or auto-resolved.
    pub fn assault(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        army: &ArmyId,
    ) -> Result<(), AssaultError> {
        let a = self.armies.get(army).ok_or(AssaultError::UnknownArmy)?;
        if &a.faction != faction {
            return Err(AssaultError::UnknownArmy);
        }
        let Some(settlement) = a.settlement().cloned() else {
            return Err(AssaultError::NotBesieging);
        };
        let besieging = self
            .settlements
            .get(&settlement)
            .and_then(|s| s.siege.as_ref())
            .is_some_and(|s| &s.attacker == faction);
        if !besieging {
            return Err(AssaultError::NotBesieging);
        }
        if let Some(reason) = self.assault_blocker(data, army) {
            return Err(AssaultError::NoEngine(reason));
        }
        let mut events = Vec::new();
        storm(self, data, army, &mut events);
        self.pending_events.extend(events);
        Ok(())
    }
}

/// Storms the settlement `army` stands on: a pending siege battle when the
/// player takes part and battles are interactive, an auto-resolved assault
/// otherwise.
fn storm(state: &mut CampaignState, data: &GameData, army: &ArmyId, events: &mut Vec<GameEvent>) {
    let Some(a) = state.armies.get(army) else {
        return;
    };
    let faction = a.faction.clone();
    let Some(settlement) = a.settlement().cloned() else {
        return;
    };
    let Some(controller) = state
        .settlements
        .get(&settlement)
        .map(|s| s.controller.clone())
    else {
        return;
    };
    // G1: also when the player only sends allied armies to the assault.
    let player_ally = assault_coalition(state, army).iter().any(|id| {
        state
            .armies
            .get(id)
            .is_some_and(|a| a.faction == state.player_faction)
    });
    let player_involved = player_ally || controller == state.player_faction;
    // Lot M3: an assault during an AI faction's turn is auto-resolved.
    if state.interactive_battles && player_involved && state.ai_turn.is_none() {
        let already = state
            .pending_battles
            .iter()
            .any(|r| r.siege && &r.attacker == army);
        if !already {
            let province = province_of(state, &settlement);
            state.pending_battles.push(crate::state::BattleRequest {
                attacker: army.clone(),
                defender: army.clone(),
                location: settlement.clone(),
                province: province.clone(),
                siege: true,
                opening: Default::default(),
            });
            events.push(
                GameEvent::new(
                    EventKind::Battle,
                    format!(
                        "{} se prépare à donner l'assaut à {}.",
                        faction_name(data, &faction),
                        settlement_name(data, &settlement)
                    ),
                )
                .province(&province)
                .army(army)
                .faction(&faction),
            );
        }
        return;
    }
    auto_assault(state, data, army, events);
}

/// An army entered a village held by an enemy (spec § 4.3): taken at once
/// without a garrison, stormed otherwise (a village has no walls).
pub(crate) fn enter_village(
    state: &mut CampaignState,
    data: &GameData,
    army: &ArmyId,
    events: &mut Vec<GameEvent>,
) {
    let Some(a) = state.armies.get(army) else {
        return;
    };
    let faction = a.faction.clone();
    let Some(settlement) = a.settlement().cloned() else {
        return;
    };
    let Some(village) = state.settlements.get(&settlement) else {
        return;
    };
    if !state.is_at_war(&faction, &village.controller) {
        return;
    }
    if !state
        .friendly_armies_at(&village.controller, &settlement)
        .is_empty()
    {
        return;
    }
    if village.garrison.is_empty() {
        capture(state, data, &settlement, &faction, events);
        return;
    }
    let turn = state.turn;
    let village = state.settlements.get_mut(&settlement).expect("exists");
    if village.siege.as_ref().is_none_or(|s| s.attacker != faction) {
        village.siege = Some(SiegeState {
            attacker: faction,
            turns_left: 1,
            turns_elapsed: 0,
            supplies: 100,
            breach: 100,
            started_turn: turn,
            engine_work: 0,
        });
    }
    storm(state, data, army, events);
}

/// `true` while the walls of `settlement` still count against `army`'s
/// assault (a fortified place, breach under 50 and no siege tower, recruited
/// or built on the spot, NT5).
pub(crate) fn walls_stand(
    state: &CampaignState,
    data: &GameData,
    army: &ArmyId,
    settlement: &SettlementId,
) -> bool {
    let breach = state
        .settlements
        .get(settlement)
        .and_then(|s| s.siege.as_ref())
        .map_or(0, |s| s.breach);
    state.fortification_level(data, settlement) > 0
        && breach < 50
        && !has_siege_towers(state, data, army)
        && state.built_towers(data, settlement) == 0
}

/// G1: the armies storming the settlement with `army`: itself first, then
/// the armies of its faction or of its allies standing there, at war with
/// the settlement's controller (as `movement::battle_coalition` for field
/// battles).
pub fn assault_coalition(state: &CampaignState, army: &ArmyId) -> Vec<ArmyId> {
    let Some(controller) = state
        .armies
        .get(army)
        .and_then(|a| a.settlement())
        .and_then(|id| state.settlements.get(id))
        .map(|s| s.controller.clone())
    else {
        return Vec::new();
    };
    crate::movement::settlement_coalition(state, army, &controller)
}

/// Auto-resolved assault of `army` (and its allies, G1) on the settlement
/// it besieges.
pub(crate) fn auto_assault(
    state: &mut CampaignState,
    data: &GameData,
    army: &ArmyId,
    events: &mut Vec<GameEvent>,
) {
    let Some(settlement) = state.armies.get(army).and_then(|a| a.settlement().cloned()) else {
        return;
    };
    let walls = walls_stand(state, data, army, &settlement);
    let Some(garrison) = garrison_army(state, &settlement) else {
        return;
    };
    let attackers = assault_coalition(state, army);
    let mut attacker_side = crate::movement::coalition_side(state, data, &attackers);
    let mut defender_side = crate::movement::side_from_army(state, data, &garrison);
    // DF1: the AI's morale against the player follows the difficulty.
    state.apply_difficulty_morale(
        data,
        &mut attacker_side,
        state.coalition_has_player(&attackers),
        &mut defender_side,
        garrison.faction == state.player_faction,
    );
    let context = crate::battle_auto::BattleContext {
        defender_terrain_bonus: false,
        river_crossing: false,
        walls,
        assault_bonus_percent: state.engine_assault_bonus(data, &settlement),
        crossing: None,
    };
    // N1: phased auto-resolve; walls stand for the terrain, the season
    // still brings its weather.
    let attacker_profiles = crate::battle_auto::coalition_profiles(state, data, &attackers);
    let defender_profiles = crate::battle_auto::army_profiles(data, &garrison);
    let result = crate::battle_auto::resolve_profiled(
        state,
        data,
        (&attacker_side, &attacker_profiles),
        (&defender_side, &defender_profiles),
        &context,
        None,
    );
    apply_assault_result(state, data, &attackers, &settlement, &result, walls, events);
}

/// Applies an assault result (auto-resolved or fought in 3D): losses on
/// both sides (spread over the storming armies, G1), journal line, capture
/// of the settlement on victory. `attackers` starts with the besieging army.
pub(crate) fn apply_assault_result(
    state: &mut CampaignState,
    data: &GameData,
    attackers: &[ArmyId],
    settlement: &SettlementId,
    result: &crate::battle_auto::BattleResult,
    walls: bool,
    events: &mut Vec<GameEvent>,
) {
    let Some(faction) = attackers
        .first()
        .and_then(|army| state.armies.get(army))
        .map(|a| a.faction.clone())
    else {
        return;
    };
    let Some(defender_faction) = state
        .settlements
        .get(settlement)
        .map(|s| s.controller.clone())
    else {
        return;
    };
    let province = province_of(state, settlement);
    let won = result.winner == crate::battle_auto::Winner::Attacker;
    // NT3: a won assault (taken or repelled) counts towards the missions.
    crate::missions::note_battle_won(state, if won { &faction } else { &defender_faction });
    let general = crate::movement::coalition_commander(state, attackers)
        .and_then(|id| state.armies.get(&id))
        .and_then(|a| a.general.clone());
    // TW2-T5: strengths before the assault, for the army experience.
    let strength_before = crate::traditions::strengths(state, attackers);
    let garrison_strength = state
        .settlements
        .get(settlement)
        .map_or(0, |s| s.garrison.iter().map(|u| u.strength).sum::<u32>());
    for (id, outcome) in crate::movement::split_outcome(state, attackers, &result.attacker) {
        crate::movement::apply_outcome(state, data, &id, &outcome, events);
    }
    crate::traditions::on_battle(
        state,
        data,
        &crate::traditions::BattleSide {
            armies: attackers,
            strength_before: &strength_before,
            won,
            enemy_strength: garrison_strength,
            multiplier: 1.0,
        },
        events,
    );
    // F1: a storming general taken on the walls is held by the defender.
    crate::movement::assign_captor(state, general.as_ref(), &defender_faction);
    capture_garrison_general(state, data, settlement, &result.defender, &faction, events);
    apply_garrison_losses(state, settlement, &result.defender);
    let allies = if attackers.len() > 1 {
        format!(" (+{} armée(s) alliée(s))", attackers.len() - 1)
    } else {
        String::new()
    };
    let text = format!(
        "Assaut {}{allies} contre {}{} : {}. Pertes : {} contre {}.",
        crate::events::de(&faction_name(data, &faction)),
        settlement_name(data, settlement),
        if walls {
            " (murailles intactes)"
        } else if state.fortification_level(data, settlement) == 0 {
            ""
        } else {
            " (par la brèche)"
        },
        if won {
            "la place est emportée"
        } else {
            "les assaillants sont repoussés"
        },
        result.attacker.total_losses,
        result.defender.total_losses
    );
    events.push(
        GameEvent::new(EventKind::Battle, text)
            .province(&province)
            .faction(&faction),
    );
    if won {
        state.record_battle(&faction, &defender_faction, true);
        crate::crusade::on_battle(state, data, &faction, &defender_faction, &faction);
        capture(state, data, settlement, &faction, events);
        if let Some(general) = general {
            dynasty::on_siege_won(state, data, &general);
            crate::retinue::try_acquire(
                state,
                data,
                &general,
                data_model::AcquisitionTrigger::SiegeWon,
                &[],
                events,
            );
        }
    } else {
        state.record_battle(&defender_faction, &faction, false);
        crate::crusade::on_battle(state, data, &defender_faction, &faction, &faction);
    }
}

/// The garrison attacks the besiegers when clearly stronger than the whole
/// besieging coalition (the lead besieger and its allies, as for an
/// assault); returns `true` when the siege is broken.
fn sortie(
    state: &mut CampaignState,
    data: &GameData,
    settlement: &SettlementId,
    besiegers: &[ArmyId],
    events: &mut Vec<GameEvent>,
) -> bool {
    let Some(lead) = besiegers.first() else {
        return false;
    };
    let Some(garrison) = garrison_army(state, settlement) else {
        return false;
    };
    let targets = crate::movement::settlement_coalition(state, lead, &garrison.faction);
    let garrison_power = crate::state::unit_power(data, &garrison.units);
    let besieging_power: f64 = targets.iter().map(|id| state.army_power(data, id)).sum();
    if garrison_power <= 1.3 * besieging_power {
        return false;
    }
    let mut sallying = crate::movement::side_from_army(state, data, &garrison);
    let mut besieging = crate::movement::coalition_side(state, data, &targets);
    // DF1: the AI's morale against the player follows the difficulty.
    state.apply_difficulty_morale(
        data,
        &mut sallying,
        garrison.faction == state.player_faction,
        &mut besieging,
        state.coalition_has_player(&targets),
    );
    // N1: a sortie is a field battle before the walls.
    let sallying_profiles = crate::battle_auto::army_profiles(data, &garrison);
    let besieging_profiles = crate::battle_auto::coalition_profiles(state, data, &targets);
    let province = state
        .settlement_province(settlement)
        .and_then(|p| data.provinces.get(p));
    let result = crate::battle_auto::resolve_profiled(
        state,
        data,
        (&sallying, &sallying_profiles),
        (&besieging, &besieging_profiles),
        &crate::battle_auto::BattleContext::default(),
        province,
    );
    let besieger_faction = state.armies[lead].faction.clone();
    let besieger_general = crate::movement::coalition_commander(state, &targets)
        .and_then(|id| state.armies.get(&id))
        .and_then(|a| a.general.clone());
    capture_garrison_general(
        state,
        data,
        settlement,
        &result.attacker,
        &besieger_faction,
        events,
    );
    apply_garrison_losses(state, settlement, &result.attacker);
    let strength_before = crate::traditions::strengths(state, &targets);
    for (id, outcome) in crate::movement::split_outcome(state, &targets, &result.defender) {
        crate::movement::apply_outcome(state, data, &id, &outcome, events);
    }
    // TW2-T5: the besiegers' experience of the sortie.
    crate::traditions::on_battle(
        state,
        data,
        &crate::traditions::BattleSide {
            armies: &targets,
            strength_before: &strength_before,
            won: result.winner == crate::battle_auto::Winner::Defender,
            enemy_strength: garrison.total_strength(),
            multiplier: 1.0,
        },
        events,
    );
    // F1: a besieging general taken in the sortie is held by the garrison.
    crate::movement::assign_captor(state, besieger_general.as_ref(), &garrison.faction);
    let won = result.winner == crate::battle_auto::Winner::Attacker;
    events.push(
        GameEvent::new(
            EventKind::Battle,
            format!(
                "Sortie de la garnison de {} : {}.",
                settlement_name(data, settlement),
                if won {
                    "les assiégeants sont mis en fuite"
                } else {
                    "elle est repoussée"
                }
            ),
        )
        .province(&province_of(state, settlement))
        .faction(&garrison.faction),
    );
    // NT9: a sortie is a battle for the missions (either side may win it).
    crate::missions::note_battle_won(
        state,
        if won {
            &garrison.faction
        } else {
            &besieger_faction
        },
    );
    // JR1: a sortie is a battle for the crusade's fervour too.
    if won {
        crate::crusade::on_battle(
            state,
            data,
            &garrison.faction,
            &besieger_faction,
            &garrison.faction,
        );
    } else {
        crate::crusade::on_battle(
            state,
            data,
            &besieger_faction,
            &garrison.faction,
            &garrison.faction,
        );
    }
    if won {
        state.record_battle(&garrison.faction, &besieger_faction, false);
        for id in &targets {
            if let Some(army) = state.armies.get_mut(id) {
                army.stance = Stance::Normal;
            }
        }
        // The siege is lifted only when no other army still besieges.
        if besiegers_of(state, settlement).is_empty() {
            if let Some(s) = state.settlements.get_mut(settlement) {
                s.siege = None;
            }
        }
    }
    won
}

/// The armies still besieging `settlement` (its current controller).
fn besiegers_of(state: &CampaignState, settlement: &SettlementId) -> Vec<ArmyId> {
    state
        .settlements
        .get(settlement)
        .map(|s| besiegers(state, settlement, &s.controller))
        .unwrap_or_default()
}

/// F1: the governor leading a garrison (see [`garrison_army`]) taken in a
/// siege battle becomes the prisoner of `captor`. Call before the losses
/// are applied and before any capture of the place.
fn capture_garrison_general(
    state: &mut CampaignState,
    data: &GameData,
    settlement: &SettlementId,
    outcome: &crate::battle_auto::SideOutcome,
    captor: &FactionId,
    events: &mut Vec<GameEvent>,
) {
    if !outcome.general_captured {
        return;
    }
    if let Some(general) = garrison_army(state, settlement).and_then(|a| a.general) {
        crate::chronicle::capture_character(state, data, &general, captor, events);
    }
}

/// Hands `settlement` to `new_controller` (occupation: the de jure owner is
/// kept). Taking a city hands over the province (derived control).
pub(crate) fn capture(
    state: &mut CampaignState,
    data: &GameData,
    settlement_id: &SettlementId,
    new_controller: &FactionId,
    events: &mut Vec<GameEvent>,
) {
    let province_id = province_of(state, settlement_id);
    let is_city = state.province_city_id(&province_id) == Some(settlement_id);
    let Some(settlement) = state.settlements.get_mut(settlement_id) else {
        return;
    };
    let previous = settlement.hand_over(new_controller);
    settlement.garrison.clear();
    // Q5: the besiegers stand down once the place is theirs (the army kept
    // its siege stance, and its "siège" label, for the rest of the game).
    for army in state.armies.values_mut() {
        if army.stance == Stance::Siege && army.settlement() == Some(settlement_id) {
            army.stance = Stance::Normal;
        }
    }
    // TW2-T1: the occupation's unrest comes from `data/rules/capture.json`.
    let unrest = crate::capture::occupation_unrest(state, data, settlement_id);
    if let Some(province) = state.provinces.get_mut(&province_id) {
        province.unrest = province.unrest.saturating_add(unrest).min(100);
    }
    let place = if is_city {
        province_name(data, &province_id)
    } else {
        format!(
            "{} ({})",
            settlement_name(data, settlement_id),
            province_name(data, &province_id)
        )
    };
    events.push(
        GameEvent::new(
            EventKind::ProvinceCaptured,
            format!(
                "{place} tombe aux mains de {} (auparavant {}).",
                faction_name(data, new_controller),
                faction_name(data, &previous)
            ),
        )
        .province(&province_id)
        .faction(new_controller),
    );
    // JR1: a place of the Holy Land, or the city of the vow.
    crate::crusade::on_settlement_taken(state, data, new_controller, settlement_id, events);
    // TW2-T1: the fate of the place (player's choice, or the AI's at once).
    crate::capture::on_captured(
        state,
        data,
        settlement_id,
        new_controller,
        &previous,
        events,
    );
}

/// Phase 4: armies in `Raid` stance devastate hostile provinces for loot.
pub(crate) fn resolve_raids(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let ids: Vec<ArmyId> = state.armies.keys().cloned().collect();
    for army_id in ids {
        let army = &state.armies[&army_id];
        if army.stance != Stance::Raid {
            continue;
        }
        let faction = army.faction.clone();
        let Some(province_id) = state.army_province(data, army) else {
            continue;
        };
        if !state.is_hostile_territory(&faction, &province_id) {
            continue;
        }
        let general = state.armies[&army_id].general.clone();
        let province = state.provinces.get_mut(&province_id).expect("exists");
        let loot =
            (province_income(&data.economy_rules, province) * RAID_LOOT_SHARE).round() as i64;
        province.devastation = province
            .devastation
            .saturating_add(RAID_DEVASTATION)
            .min(100);
        province.unrest = province.unrest.saturating_add(RAID_UNREST).min(100);
        if let Some(faction_state) = state.factions.get_mut(&faction) {
            faction_state.treasury += loot;
        }
        if let Some(general) = general {
            dynasty::on_raid_led(state, data, &general);
            crate::retinue::try_acquire(
                state,
                data,
                &general,
                data_model::AcquisitionTrigger::RaidLed,
                &[],
                events,
            );
        }
        events.push(
            GameEvent::new(
                EventKind::Raid,
                format!(
                    "Chevauchée de {} en {} : {loot} livres de butin.",
                    faction_name(data, &faction),
                    province_name(data, &province_id)
                ),
            )
            .province(&province_id)
            .army(&army_id)
            .faction(&faction),
        );
    }
}
