//! Sieges, captures and chevauchées (spec § 1.3 steps 3 and 4; spec § 2 for
//! the besieging general's `SiegeSpeed` and the `trait_siege_master`/
//! `trait_cruel` triggers).

use data_model::{FactionId, GameData, ProvinceId};

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
/// Unrest added to a province when it changes hands.
pub const CAPTURE_UNREST: u8 = 20;

fn province_name(data: &GameData, id: &ProvinceId) -> String {
    data.provinces
        .get(id)
        .map_or_else(|| id.to_string(), |p| p.name.display.clone())
}

fn faction_name(data: &GameData, id: &FactionId) -> String {
    data.factions
        .get(id)
        .map_or_else(|| id.to_string(), |f| f.short_or_display_name().to_owned())
}

/// Armies in `province` besieging its controller, sorted by id.
fn besiegers(state: &CampaignState, province: &ProvinceId, controller: &FactionId) -> Vec<ArmyId> {
    state
        .armies
        .iter()
        .filter(|(_, army)| {
            &army.location == province
                && army.stance == Stance::Siege
                && state.is_at_war(&army.faction, controller)
        })
        .map(|(id, _)| id.clone())
        .collect()
}

/// Phase 3: progress, start or lift sieges; capture provinces.
pub(crate) fn resolve_sieges(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let ids: Vec<ProvinceId> = state.provinces.keys().cloned().collect();
    for province_id in ids {
        let controller = state.provinces[&province_id].controller.clone();
        let besiegers = besiegers(state, &province_id, &controller);
        let defenders = state.friendly_armies_in(&controller, &province_id);
        if besiegers.is_empty() || !defenders.is_empty() {
            if state
                .provinces
                .get_mut(&province_id)
                .and_then(|p| p.siege.take())
                .is_some()
            {
                events.push(
                    GameEvent::new(
                        EventKind::SiegeLifted,
                        format!(
                            "Le siège {} est levé.",
                            crate::events::de(&province_name(data, &province_id))
                        ),
                    )
                    .province(&province_id)
                    .faction(&controller),
                );
            }
            continue;
        }
        let attacker = state.armies[&besiegers[0]].faction.clone();
        let garrison_empty = state.provinces[&province_id].garrison.is_empty();
        if garrison_empty {
            capture(state, data, &province_id, &attacker, events);
            continue;
        }
        let fortification = state.fortification_level(data, &province_id);
        let besieging_general = state.armies[&besiegers[0]].general.clone();
        let siege_speed_percent = besieging_general.as_ref().map_or(0.0, |g| {
            skills::character_effects(state, data, g)
                .siege_speed
                .apply(0.0)
        });
        // M8: the garrison sallies out when it outmatches the besiegers.
        if sortie(state, data, &province_id, &besiegers, events) {
            continue;
        }
        let resistance = state.siege_resistance(data, &province_id, &attacker);
        let breach_gain = (f64::from(breach_per_turn(state, data, &besiegers, fortification))
            * (1.0 - resistance / 100.0))
            .round() as u8;
        let drain = supplies_drain(fortification, siege_speed_percent);
        let province = state.provinces.get_mut(&province_id).expect("exists");
        match &mut province.siege {
            Some(siege) if siege.attacker == attacker => {
                siege.turns_elapsed += 1;
                siege.breach = siege.breach.saturating_add(breach_gain).min(100);
                siege.supplies = siege.supplies.saturating_sub(drain);
                siege.turns_left = turns_to_starve(siege.supplies, drain);
                if siege.supplies == 0 {
                    province.garrison.clear();
                    events.push(
                        GameEvent::new(
                            EventKind::ProvinceCaptured,
                            format!(
                                "Affamée, la garnison de {} capitule.",
                                province_name(data, &province_id)
                            ),
                        )
                        .province(&province_id)
                        .faction(&attacker),
                    );
                    capture(state, data, &province_id, &attacker, events);
                    if let Some(general) = besieging_general {
                        dynasty::on_siege_won(state, data, &general);
                    }
                }
            }
            _ => {
                let supplies = 100u8.saturating_sub(province.devastation / 2).max(10);
                let turns_left = turns_to_starve(supplies, drain);
                province.siege = Some(SiegeState {
                    attacker: attacker.clone(),
                    turns_left,
                    turns_elapsed: 0,
                    supplies,
                    breach: 0,
                });
                events.push(
                    GameEvent::new(
                        EventKind::SiegeStarted,
                        format!(
                            "{} met le siège devant {}.",
                            faction_name(data, &attacker),
                            province_name(data, &province_id)
                        ),
                    )
                    .province(&province_id)
                    .army(&besiegers[0])
                    .faction(&attacker),
                );
            }
        }
    }
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

/// A field army made of a province's garrison, for siege battles.
pub(crate) fn garrison_army(
    state: &CampaignState,
    province: &ProvinceId,
) -> Option<crate::state::Army> {
    let p = state.provinces.get(province)?;
    Some(crate::state::Army {
        faction: p.controller.clone(),
        general: state.province_governor(province).cloned(),
        location: province.clone(),
        units: p.garrison.clone(),
        movement_points: 0,
        supply: 100,
        stance: Stance::Normal,
        path: Vec::new(),
    })
}

fn apply_garrison_losses(
    state: &mut CampaignState,
    province: &ProvinceId,
    outcome: &crate::battle_auto::SideOutcome,
) {
    if let Some(p) = state.provinces.get_mut(province) {
        for (unit, losses) in p.garrison.iter_mut().zip(&outcome.losses) {
            unit.strength = unit.strength.saturating_sub(*losses);
            unit.morale = (i32::from(unit.morale) + outcome.morale_delta).clamp(0, 100) as u8;
        }
        p.garrison
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
}

impl CampaignState {
    /// Odds (0-100) that `army` storms the town it besieges, and whether the
    /// walls still stand (for the UI; a quick estimate, not the resolution).
    pub fn assault_odds(&self, data: &GameData, army: &ArmyId) -> Option<(u32, bool)> {
        let a = self.armies.get(army)?;
        let siege = self.provinces.get(&a.location)?.siege.as_ref()?;
        if siege.attacker != a.faction {
            return None;
        }
        let walls = siege.breach < 50 && !has_siege_towers(self, data, army);
        let attack = self.army_power(data, army) * if walls { 0.7 } else { 1.0 };
        let defence = crate::state::unit_power(data, &self.provinces[&a.location].garrison)
            * (1.0 + f64::from(self.fortification_level(data, &a.location)) * 0.1);
        let odds = (100.0 * attack / (attack + defence).max(1.0)).round() as u32;
        Some((odds, walls))
    }

    /// `assault { army }`: storm the walls now (M8). Victory takes the town;
    /// defeat bloodies the besiegers, the siege goes on. With
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
        let province = a.location.clone();
        let besieging = self
            .provinces
            .get(&province)
            .and_then(|p| p.siege.as_ref())
            .is_some_and(|s| &s.attacker == faction);
        if !besieging {
            return Err(AssaultError::NotBesieging);
        }
        let controller = self.provinces[&province].controller.clone();
        // G1: also when the player only sends allied armies to the assault.
        let player_ally = assault_coalition(self, army).iter().any(|id| {
            self.armies
                .get(id)
                .is_some_and(|a| a.faction == self.player_faction)
        });
        let player_involved = player_ally || controller == self.player_faction;
        if self.interactive_battles && player_involved {
            let already = self
                .pending_battles
                .iter()
                .any(|r| r.siege && &r.attacker == army);
            if !already {
                self.pending_battles.push(crate::state::BattleRequest {
                    attacker: army.clone(),
                    defender: army.clone(),
                    province: province.clone(),
                    attacker_origin: None,
                    siege: true,
                });
                self.pending_events.push(
                    GameEvent::new(
                        EventKind::Battle,
                        format!(
                            "{} se prépare à donner l'assaut à {}.",
                            faction_name(data, faction),
                            province_name(data, &province)
                        ),
                    )
                    .province(&province)
                    .army(army)
                    .faction(faction),
                );
            }
            return Ok(());
        }
        let mut events = Vec::new();
        auto_assault(self, data, army, &mut events);
        self.pending_events.extend(events);
        Ok(())
    }
}

/// `true` while the walls of `province` still count against `army`'s
/// assault (breach under 50 and no siege tower).
pub(crate) fn walls_stand(
    state: &CampaignState,
    data: &GameData,
    army: &ArmyId,
    province: &ProvinceId,
) -> bool {
    let breach = state
        .provinces
        .get(province)
        .and_then(|p| p.siege.as_ref())
        .map_or(0, |s| s.breach);
    breach < 50 && !has_siege_towers(state, data, army)
}

/// G1: the armies storming the town with `army`: itself first, then the
/// armies of its faction or of its allies in the province, at war with the
/// town's controller (as `movement::battle_coalition` for field battles).
pub fn assault_coalition(state: &CampaignState, army: &ArmyId) -> Vec<ArmyId> {
    let Some(controller) = state
        .armies
        .get(army)
        .and_then(|a| state.provinces.get(&a.location))
        .map(|p| p.controller.clone())
    else {
        return Vec::new();
    };
    crate::movement::battle_coalition(state, army, &controller)
}

/// Auto-resolved assault of `army` (and its allies, G1) on the town it
/// besieges.
pub(crate) fn auto_assault(
    state: &mut CampaignState,
    data: &GameData,
    army: &ArmyId,
    events: &mut Vec<GameEvent>,
) {
    let Some(province) = state.armies.get(army).map(|a| a.location.clone()) else {
        return;
    };
    let walls = walls_stand(state, data, army, &province);
    let Some(garrison) = garrison_army(state, &province) else {
        return;
    };
    let attackers = assault_coalition(state, army);
    let attacker_side = crate::movement::coalition_side(state, data, &attackers);
    let defender_side = crate::movement::side_from_army(state, data, &garrison);
    let context = crate::battle_auto::BattleContext {
        defender_terrain_bonus: false,
        river_crossing: false,
        walls,
    };
    let result =
        crate::battle_auto::resolve_auto(&attacker_side, &defender_side, &context, &mut state.rng);
    apply_assault_result(state, data, &attackers, &province, &result, walls, events);
}

/// Applies an assault result (auto-resolved or fought in 3D): losses on
/// both sides (spread over the storming armies, G1), journal line, capture
/// of the town on victory. `attackers` starts with the besieging army.
pub(crate) fn apply_assault_result(
    state: &mut CampaignState,
    data: &GameData,
    attackers: &[ArmyId],
    province: &ProvinceId,
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
    let defender_faction = state.provinces[province].controller.clone();
    let won = result.winner == crate::battle_auto::Winner::Attacker;
    let general = crate::movement::coalition_commander(state, attackers)
        .and_then(|id| state.armies.get(&id))
        .and_then(|a| a.general.clone());
    for (id, outcome) in crate::movement::split_outcome(state, attackers, &result.attacker) {
        crate::movement::apply_outcome(state, data, &id, &outcome, events);
    }
    apply_garrison_losses(state, province, &result.defender);
    let allies = if attackers.len() > 1 {
        format!(" (+{} armée(s) alliée(s))", attackers.len() - 1)
    } else {
        String::new()
    };
    let text = format!(
        "Assaut {}{allies} contre {}{} : {}. Pertes : {} contre {}.",
        crate::events::de(&faction_name(data, &faction)),
        province_name(data, province),
        if walls {
            " (murailles intactes)"
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
            .province(province)
            .faction(&faction),
    );
    if won {
        state.record_battle(&faction, &defender_faction, true);
        capture(state, data, province, &faction, events);
        if let Some(general) = general {
            dynasty::on_siege_won(state, data, &general);
        }
    } else {
        state.record_battle(&defender_faction, &faction, false);
    }
}

/// The garrison attacks the besiegers when clearly stronger; returns `true`
/// when the siege is broken.
fn sortie(
    state: &mut CampaignState,
    data: &GameData,
    province: &ProvinceId,
    besiegers: &[ArmyId],
    events: &mut Vec<GameEvent>,
) -> bool {
    let Some(target) = besiegers.first() else {
        return false;
    };
    let Some(garrison) = garrison_army(state, province) else {
        return false;
    };
    let garrison_power = crate::state::unit_power(data, &garrison.units);
    if garrison_power <= 1.3 * state.army_power(data, target) {
        return false;
    }
    let sallying = crate::movement::side_from_army(state, data, &garrison);
    let besieging = crate::movement::side_from_army(state, data, &state.armies[target]);
    let result = crate::battle_auto::resolve_auto(
        &sallying,
        &besieging,
        &crate::battle_auto::BattleContext::default(),
        &mut state.rng,
    );
    let besieger_faction = state.armies[target].faction.clone();
    apply_garrison_losses(state, province, &result.attacker);
    crate::movement::apply_outcome(state, data, target, &result.defender, events);
    let won = result.winner == crate::battle_auto::Winner::Attacker;
    events.push(
        GameEvent::new(
            EventKind::Battle,
            format!(
                "Sortie de la garnison de {} : {}.",
                province_name(data, province),
                if won {
                    "les assiégeants sont mis en fuite"
                } else {
                    "elle est repoussée"
                }
            ),
        )
        .province(province)
        .faction(&garrison.faction),
    );
    if won {
        state.record_battle(&garrison.faction, &besieger_faction, false);
        if let Some(p) = state.provinces.get_mut(province) {
            p.siege = None;
        }
        if let Some(army) = state.armies.get_mut(target) {
            army.stance = Stance::Normal;
        }
    }
    won
}

/// Hands `province` to `new_controller` (occupation: the de jure owner is kept).
pub(crate) fn capture(
    state: &mut CampaignState,
    data: &GameData,
    province_id: &ProvinceId,
    new_controller: &FactionId,
    events: &mut Vec<GameEvent>,
) {
    let province = state.provinces.get_mut(province_id).expect("exists");
    let previous = std::mem::replace(&mut province.controller, new_controller.clone());
    province.siege = None;
    province.garrison.clear();
    province.recruit_queue.clear();
    province.unrest = province.unrest.saturating_add(CAPTURE_UNREST).min(100);
    events.push(
        GameEvent::new(
            EventKind::ProvinceCaptured,
            format!(
                "{} tombe aux mains de {} (auparavant {}).",
                province_name(data, province_id),
                faction_name(data, new_controller),
                faction_name(data, &previous)
            ),
        )
        .province(province_id)
        .faction(new_controller),
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
        let province_id = army.location.clone();
        if !state.is_hostile_territory(&faction, &province_id) {
            continue;
        }
        let general = state.armies[&army_id].general.clone();
        let province = state.provinces.get_mut(&province_id).expect("exists");
        let loot = (province_income(province) * RAID_LOOT_SHARE).round() as i64;
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
