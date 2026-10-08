//! Field battles: sides, coalitions and the application of a result.

use std::collections::BTreeMap;

use super::{apply_outcome, assign_captor, retreat_beaten_army};
use crate::battle_auto::{resolve_field, BattleContext, BattleUnit, Side, Winner};
use crate::dynasty;
use crate::events::{EventKind, GameEvent};
use crate::research;
use crate::skills;
use crate::state::{Army, ArmyId, CampaignState};
use data_model::EffectKind;
use data_model::{FactionId, GameData, Terrain};
use sim_battle::{BattleOpening, SideId};

/// Builds the pure battle description of an army.
pub fn side_from_army(state: &CampaignState, data: &GameData, army: &Army) -> Side {
    let general_command = army
        .general
        .as_ref()
        .and_then(|id| state.characters.get(id))
        .map_or(0, |c| c.skills.command);
    let general_effects = army
        .general
        .as_ref()
        .map(|id| skills::character_effects(state, data, id))
        .unwrap_or_default();
    // TW2-T5: discipline (morale) and shooting traditions of the army.
    let traditions = crate::traditions::army_tradition_effects(data, army);
    Side {
        units: army
            .units
            .iter()
            .map(|unit| {
                let stats = data.unit_types.get(&unit.unit_type);
                // M6: technology bonuses of the army's faction, per category.
                let tech = stats.map_or_else(Default::default, |t| {
                    research::tech_unit_bonus(state, data, &army.faction, t.category)
                });
                BattleUnit {
                    strength: unit.strength,
                    max_strength: unit.max_strength,
                    experience: unit.experience,
                    morale: research::boosted(
                        unit.morale,
                        tech.morale + f64::from(traditions.morale),
                        100,
                    ),
                    melee: research::boosted(stats.map_or(30, |t| t.stats.melee), tech.melee, 255),
                    ranged: research::boosted(
                        stats.map_or(0, |t| t.stats.ranged),
                        tech.ranged
                            + f64::from(unit.levy_ranged)
                            + if stats.is_some_and(|t| t.stats.ranged > 0) {
                                f64::from(traditions.ranged)
                            } else {
                                0.0
                            },
                        255,
                    ),
                    // G1: plus the levying province's buildings (armoury, butts).
                    armor: research::boosted(
                        stats.map_or(20, |t| t.stats.armor),
                        tech.armor + f64::from(unit.levy_armor),
                        100,
                    ),
                    is_ranged: stats.is_some_and(|t| {
                        matches!(
                            t.category,
                            data_model::UnitCategory::Ranged | data_model::UnitCategory::Siege
                        )
                    }),
                }
            })
            .collect(),
        general_command,
        supply: army.supply,
        // CV3: plus the army's morale modifiers (battle outcomes).
        // JR1: and the crusade's zeal (0 for every other faction).
        general_morale_bonus: general_effects[EffectKind::ArmyMorale].apply(0.0)
            + f64::from(army.morale_modifier())
            + f64::from(crate::crusade::zeal_morale(state, data, &army.faction)),
        general_charge_percent: general_effects[EffectKind::BattleCharge].apply(0.0),
        general_ranged_percent: general_effects[EffectKind::BattleRanged].apply(0.0),
        general_defense_percent: general_effects[EffectKind::BattleDefense].apply(0.0),
        general_intrigue: general_effects[EffectKind::Intrigue].apply(0.0),
    }
}

/// A field battle between two armies within reach of each other: deferred
/// to the player when interactive battles are on and the player takes part
/// (M7, see `battle_request`), auto-resolved otherwise. Neither side moves
/// again this turn (lot M2).
pub(crate) fn fight(
    state: &mut CampaignState,
    data: &GameData,
    attacker_id: &ArmyId,
    defender_id: &ArmyId,
    events: &mut Vec<GameEvent>,
) {
    fight_with_opening(
        state,
        data,
        attacker_id,
        defender_id,
        BattleOpening::Standard,
        events,
    );
}

/// [`fight`] with a given opening (CV3: a sprung ambush).
pub(crate) fn fight_with_opening(
    state: &mut CampaignState,
    data: &GameData,
    attacker_id: &ArmyId,
    defender_id: &ArmyId,
    opening: BattleOpening,
    events: &mut Vec<GameEvent>,
) {
    for id in [attacker_id, defender_id] {
        if let Some(army) = state.armies.get_mut(id) {
            army.movement_left = 0;
            army.clear_plan();
        }
    }
    if crate::battle_request::defer_player_battle(
        state,
        data,
        attacker_id,
        defender_id,
        opening,
        events,
    ) {
        return;
    }
    auto_fight_with_opening(state, data, attacker_id, defender_id, opening, events);
}

/// Armies fighting on `lead`'s side (F1): `lead` first, then every other
/// army of `lead`'s faction or of an ally of it, at war with `enemy`, and
/// standing in the same settlement or within `engage_radius_km` of `lead`
/// (lot M2), in id order.
pub fn battle_coalition(
    state: &CampaignState,
    data: &GameData,
    lead: &ArmyId,
    enemy: &FactionId,
) -> Vec<ArmyId> {
    let Some(lead_army) = state.armies.get(lead) else {
        return Vec::new();
    };
    let engage = data.free_movement_rules().engage_radius_km;
    let mut ids = vec![lead.clone()];
    ids.extend(
        state
            .armies
            .iter()
            .filter(|(id, army)| {
                *id != lead
                    && state.is_allied(&lead_army.faction, &army.faction)
                    && state.is_at_war(&army.faction, enemy)
                    && match (army.settlement(), lead_army.settlement()) {
                        (Some(a), Some(b)) => a == b,
                        _ => state.army_distance_km(data, army, lead_army) <= engage,
                    }
            })
            .map(|(id, _)| id.clone()),
    );
    ids
}

/// Armies of `lead`'s side stationed in its settlement (sieges, G1):
/// `lead` first, then the armies of its faction or allies there at war with
/// `enemy`, in id order.
pub fn settlement_coalition(
    state: &CampaignState,
    lead: &ArmyId,
    enemy: &FactionId,
) -> Vec<ArmyId> {
    let Some(lead_army) = state.armies.get(lead) else {
        return Vec::new();
    };
    let Some(place) = lead_army.settlement() else {
        return vec![lead.clone()];
    };
    let mut ids = vec![lead.clone()];
    ids.extend(
        state
            .armies
            .iter()
            .filter(|(id, army)| {
                *id != lead
                    && army.is_at(place)
                    && state.is_allied(&lead_army.faction, &army.faction)
                    && state.is_at_war(&army.faction, enemy)
            })
            .map(|(id, _)| id.clone()),
    );
    ids
}

/// The army of a coalition whose general commands it (F1): the best
/// general by command skill (ties: the first in coalition order).
pub fn coalition_commander(state: &CampaignState, ids: &[ArmyId]) -> Option<ArmyId> {
    let mut best: Option<(&ArmyId, u8)> = None;
    for id in ids {
        let Some(command) = state
            .armies
            .get(id)
            .and_then(|a| a.general.as_ref())
            .and_then(|g| state.characters.get(g))
            .filter(|c| c.alive)
            .map(|c| c.skills.command)
        else {
            continue;
        };
        if best.is_none_or(|(_, b)| command > b) {
            best = Some((id, command));
        }
    }
    best.map(|(id, _)| id.clone())
}

/// One army standing for a whole coalition (F1): every regiment in
/// coalition order, the commander's general, the lead's faction and the
/// strength-weighted supply. Used for the 3D battle setup and to validate
/// its result.
pub(crate) fn coalition_army(state: &CampaignState, ids: &[ArmyId]) -> Option<Army> {
    let mut combined = state.armies.get(ids.first()?)?.clone();
    combined.general = coalition_commander(state, ids)
        .and_then(|id| state.armies.get(&id))
        .and_then(|a| a.general.clone());
    let mut weighted_supply = f64::from(combined.supply) * f64::from(combined.total_strength());
    let mut strength = f64::from(combined.total_strength());
    for id in &ids[1..] {
        let Some(army) = state.armies.get(id) else {
            continue;
        };
        weighted_supply += f64::from(army.supply) * f64::from(army.total_strength());
        strength += f64::from(army.total_strength());
        combined.units.extend(army.units.iter().cloned());
    }
    if strength > 0.0 {
        combined.supply = (weighted_supply / strength).round().clamp(0.0, 100.0) as u8;
    }
    Some(combined)
}

/// Battle description of a coalition (F1): the regiments of every army
/// (each with its own faction's technologies), the commander's general and
/// the strength-weighted supply.
pub(crate) fn coalition_side(state: &CampaignState, data: &GameData, ids: &[ArmyId]) -> Side {
    let commander = coalition_commander(state, ids);
    let mut side = Side::default();
    let mut weighted_supply = 0.0;
    let mut strength = 0.0;
    for id in ids {
        let Some(army) = state.armies.get(id) else {
            continue;
        };
        let part = side_from_army(state, data, army);
        if commander.as_ref() == Some(id) {
            side.general_command = part.general_command;
            side.general_morale_bonus = part.general_morale_bonus;
            side.general_charge_percent = part.general_charge_percent;
            side.general_ranged_percent = part.general_ranged_percent;
            side.general_defense_percent = part.general_defense_percent;
            side.general_intrigue = part.general_intrigue;
        }
        let men = f64::from(army.total_strength());
        weighted_supply += f64::from(part.supply) * men;
        strength += men;
        side.units.extend(part.units);
    }
    if strength > 0.0 {
        side.supply = (weighted_supply / strength).round().clamp(0.0, 100.0) as u8;
    }
    side
}

/// Everything an auto-resolved field battle is fought with, built once for
/// the resolver and for the pre-battle forecast (LR-13: the two used to
/// drift apart).
pub(crate) struct FieldBattleSetup<'a> {
    pub attackers: Vec<ArmyId>,
    pub defenders: Vec<ArmyId>,
    pub attacker_side: Side,
    pub defender_side: Side,
    pub context: BattleContext,
    /// The defender's province (terrain of the battle).
    pub province: Option<&'a data_model::Province>,
    /// RC: the real river crossing between the two armies, if any.
    pub crossing: Option<crate::river_crossing::CrossingSite>,
}

/// Sides and situation of a field battle between two armies within reach
/// of each other; the allied armies nearby join either side (F1).
/// CV3: `opening` (an ambush: the ambusher of a sprung ambush charges
/// harder); an entrenched side defends better. DF1: the AI's morale against
/// the player follows the difficulty.
pub(crate) fn field_battle_setup<'a>(
    state: &CampaignState,
    data: &'a GameData,
    attacker_id: &ArmyId,
    defender_id: &ArmyId,
    opening: BattleOpening,
) -> Option<FieldBattleSetup<'a>> {
    let attacker = state.armies.get(attacker_id)?;
    let defender = state.armies.get(defender_id)?;
    let province = state
        .army_province(data, defender)
        .and_then(|p| data.provinces.get(&p));
    let crossing = crate::river_crossing::crossing_site(state, data, attacker_id, defender_id);
    let context = BattleContext {
        defender_terrain_bonus: province.is_some_and(|p| {
            matches!(
                p.terrain,
                Terrain::Hills | Terrain::Forest | Terrain::Mountains
            )
        }),
        // RC: a real crossing replaces the province river flag.
        river_crossing: crossing.is_none() && province.is_some_and(|p| !p.rivers.is_empty()),
        walls: false,
        assault_bonus_percent: 0,
        wall: Default::default(),
        crossing: crossing.as_ref().map(|c| c.effect()),
    };
    let attackers = battle_coalition(state, data, attacker_id, &defender.faction);
    let defenders = battle_coalition(state, data, defender_id, &attacker.faction);
    let mut attacker_side = coalition_side(state, data, &attackers);
    let mut defender_side = coalition_side(state, data, &defenders);
    // CV3: stances (entrenched camp) and the ambush opening.
    for (side, lead, id) in [
        (&mut attacker_side, attacker, SideId::Attacker),
        (&mut defender_side, defender, SideId::Defender),
    ] {
        let ambusher = opening.ambush_victim() == Some(id.other());
        let (charge, defense) = crate::posture::auto_resolve_bonus(data, Some(lead), ambusher);
        side.general_charge_percent += charge;
        side.general_defense_percent += defense;
    }
    // DF1: the AI's morale against the player follows the difficulty.
    state.apply_difficulty_morale(
        data,
        &mut attacker_side,
        state.coalition_has_player(&attackers),
        &mut defender_side,
        state.coalition_has_player(&defenders),
    );
    Some(FieldBattleSetup {
        attackers,
        defenders,
        attacker_side,
        defender_side,
        context,
        province,
        crossing,
    })
}

/// Auto-resolves a field battle between two armies within reach of each
/// other (see [`field_battle_setup`]).
pub(crate) fn auto_fight_with_opening(
    state: &mut CampaignState,
    data: &GameData,
    attacker_id: &ArmyId,
    defender_id: &ArmyId,
    opening: BattleOpening,
    events: &mut Vec<GameEvent>,
) {
    let Some(setup) = field_battle_setup(state, data, attacker_id, defender_id, opening) else {
        return;
    };
    // N1: phased auto-resolve on the province's terrain, season and weather.
    let result = resolve_field(
        state,
        data,
        &setup.attackers,
        &setup.defenders,
        &setup.attacker_side,
        &setup.defender_side,
        &setup.context,
        setup.province,
    );
    apply_battle_result(
        state,
        data,
        &setup.attackers,
        &setup.defenders,
        &result,
        events,
    );
}

/// Splits a coalition's outcome into one outcome per army (F1): each army
/// takes the losses of its own regiments; only the commander's general can
/// be captured.
pub(crate) fn split_outcome(
    state: &CampaignState,
    ids: &[ArmyId],
    outcome: &crate::battle_auto::SideOutcome,
) -> Vec<(ArmyId, crate::battle_auto::SideOutcome)> {
    let commander = coalition_commander(state, ids);
    let mut offset = 0;
    ids.iter()
        .filter_map(|id| {
            let count = state.armies.get(id)?.units.len();
            let end = (offset + count).min(outcome.losses.len());
            let losses = outcome.losses[offset.min(end)..end].to_vec();
            offset += count;
            Some((
                id.clone(),
                crate::battle_auto::SideOutcome {
                    power: outcome.power,
                    total_losses: losses.iter().sum(),
                    losses,
                    morale_delta: outcome.morale_delta,
                    routed: outcome.routed,
                    general_captured: outcome.general_captured && commander.as_ref() == Some(id),
                    general_killed: outcome.general_killed && commander.as_ref() == Some(id),
                },
            ))
        })
        .collect()
}

/// Applies a battle result (auto-resolved or fought in 3D, M7): journal,
/// losses (spread over each coalition's armies, F1), captures, the
/// commanding generals' XP/traits (M4 hooks) and the losers' retreat.
/// `attackers` / `defenders` start with the two armies of the encounter.
pub(crate) fn apply_battle_result(
    state: &mut CampaignState,
    data: &GameData,
    attackers: &[ArmyId],
    defenders: &[ArmyId],
    result: &crate::battle_auto::BattleResult,
    events: &mut Vec<GameEvent>,
) {
    let (Some(attacker_id), Some(defender_id)) = (attackers.first(), defenders.first()) else {
        return;
    };
    let (Some(attacker), Some(defender)) =
        (state.armies.get(attacker_id), state.armies.get(defender_id))
    else {
        return;
    };
    let Some(province_id) = state.army_province(data, defender) else {
        return;
    };
    let battlefield = state.army_point(data, defender);
    let province = data.provinces.get(&province_id);
    let attacker_faction = attacker.faction.clone();
    let defender_faction = defender.faction.clone();
    let general_of = |state: &CampaignState, ids: &[ArmyId]| {
        coalition_commander(state, ids)
            .and_then(|id| state.armies.get(&id))
            .and_then(|a| a.general.clone())
    };
    let attacker_general = general_of(state, attackers);
    let defender_general = general_of(state, defenders);

    let province_name =
        province.map_or_else(|| province_id.to_string(), |p| p.name.display.clone());
    let faction_name = |id: &FactionId| -> String { data.faction_name(id) };
    let winner_faction = match result.winner {
        Winner::Attacker => &attacker_faction,
        Winner::Defender => &defender_faction,
    };
    let allies_note = |ids: &[ArmyId]| {
        if ids.len() > 1 {
            format!(" (+{} armée(s) alliée(s))", ids.len() - 1)
        } else {
            String::new()
        }
    };
    events.push(
        GameEvent::new(
            EventKind::Battle,
            format!(
                "Bataille {} : {}{} attaque {}{}. Vainqueur : {}. Pertes : {} contre {}.",
                crate::events::de(&province_name),
                faction_name(&attacker_faction),
                allies_note(attackers),
                faction_name(&defender_faction),
                allies_note(defenders),
                faction_name(winner_faction),
                result.attacker.total_losses,
                result.defender.total_losses
            ),
        )
        .province(&province_id)
        .army(attacker_id)
        .faction(winner_faction),
    );

    // Lot M5b: strength before the battle, to weigh the defeat of the losers.
    let strength_before: BTreeMap<ArmyId, u32> = attackers
        .iter()
        .chain(defenders)
        .filter_map(|id| Some((id.clone(), state.armies.get(id)?.total_strength())))
        .collect();
    let attacker_parts = split_outcome(state, attackers, &result.attacker);
    let defender_parts = split_outcome(state, defenders, &result.defender);
    for (id, outcome) in attacker_parts.iter().chain(defender_parts.iter()) {
        apply_outcome(state, data, id, outcome, events);
    }
    // F1: a captured commander is held by the victor.
    assign_captor(state, attacker_general.as_ref(), &defender_faction);
    assign_captor(state, defender_general.as_ref(), &attacker_faction);
    // M5 war score: a lopsided battle counts double.
    let (winner, loser, winner_losses, loser_losses) = match result.winner {
        Winner::Attacker => (
            &attacker_faction,
            &defender_faction,
            result.attacker.total_losses,
            result.defender.total_losses,
        ),
        Winner::Defender => (
            &defender_faction,
            &attacker_faction,
            result.defender.total_losses,
            result.attacker.total_losses,
        ),
    };
    state.record_battle(winner, loser, loser_losses > 2 * winner_losses.max(1));
    // JR1: the crusade's fervour follows its battles.
    let (winner, loser) = (winner.clone(), loser.clone());
    crate::crusade::on_battle(state, data, &winner, &loser, &attacker_faction);

    // CV3: nuanced outcome (heroic, decisive, Pyrrhic, disaster...).
    let tally = |ids: &[ArmyId], outcome: &crate::battle_auto::SideOutcome| {
        crate::battle_outcome::SideTally {
            strength: ids
                .iter()
                .filter_map(|id| strength_before.get(id))
                .sum::<u32>(),
            losses: outcome.total_losses,
            general_lost: outcome.general_captured || outcome.general_killed,
        }
    };
    // TB: the battle enters the history kept for the map's battlefield marks.
    crate::battle_history::record(
        state,
        data,
        crate::battle_history::BattleKind::Field,
        &province_id,
        battlefield,
        (
            (
                &attacker_faction,
                tally(attackers, &result.attacker).strength,
            ),
            (
                &defender_faction,
                tally(defenders, &result.defender).strength,
            ),
        ),
        result,
    );
    let place = crate::march::nearest_settlement(data, battlefield)
        .map_or_else(|| province_name.clone(), |s| data.settlement_name(&s));
    let (attacker_xp, defender_xp) = crate::battle_outcome::apply(
        state,
        data,
        &province_id,
        &place,
        &crate::battle_outcome::OutcomeSide {
            faction: &attacker_faction,
            armies: attackers,
            tally: tally(attackers, &result.attacker),
            won: result.winner == Winner::Attacker,
        },
        &crate::battle_outcome::OutcomeSide {
            faction: &defender_faction,
            armies: defenders,
            tally: tally(defenders, &result.defender),
            won: result.winner == Winner::Defender,
        },
        events,
    );

    // Spec § 2: XP, `trait_veteran`, wounded and death chance for both
    // commanding generals (only if they weren't captured, which already
    // removed them from command).
    if let Some(general) = &attacker_general {
        if state.characters.get(general).is_some_and(|c| c.alive) {
            dynasty::on_battle_resolved(
                state,
                data,
                general,
                result.winner == Winner::Attacker,
                attacker_xp,
                events,
            );
        }
    }
    if let Some(general) = &defender_general {
        if state.characters.get(general).is_some_and(|c| c.alive) {
            dynasty::on_battle_resolved(
                state,
                data,
                general,
                result.winner == Winner::Defender,
                defender_xp,
                events,
            );
        }
    }

    // TW2-T5: army experience of both sides (traditions).
    let side_strength = |ids: &[ArmyId]| {
        ids.iter()
            .filter_map(|id| strength_before.get(id))
            .sum::<u32>()
    };
    let sides = [
        (attackers, defenders, Winner::Attacker, attacker_xp),
        (defenders, attackers, Winner::Defender, defender_xp),
    ];
    for (ids, enemies, side, multiplier) in sides {
        crate::traditions::on_battle(
            state,
            data,
            &crate::traditions::BattleSide {
                armies: ids,
                strength_before: &strength_before,
                won: result.winner == side,
                enemy_strength: side_strength(enemies),
                multiplier,
            },
            events,
        );
    }

    let losers = match result.winner {
        Winner::Attacker => defenders,
        Winner::Defender => attackers,
    };
    for loser_id in losers {
        let before = strength_before.get(loser_id).copied().unwrap_or(0);
        let after = state.armies.get(loser_id).map_or(0, |a| a.total_strength());
        let losses_percent = if before == 0 {
            0
        } else {
            (u64::from(before.saturating_sub(after)) * 100 / u64::from(before)) as u32
        };
        retreat_beaten_army(state, data, loser_id, battlefield, losses_percent, events);
    }
    // CV3-3: consequences of an encounter battle.
    crate::encounter::after_battle(state, data, attackers, defenders, result.winner, events);
}
