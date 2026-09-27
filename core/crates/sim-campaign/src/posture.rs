//! Army stances of the living campaign (lot CV3-1, spec
//! `docs/design/2026-09-27-campagne-vivante.md` § 1): ambush, forced march,
//! entrenched camp. Numbers in `data/rules/postures.json`
//! ([`data_model::PostureRules`]).
//!
//! - **Ambush**: set up in cover (forest, bocage, marsh: [`data_model::CoverClass`])
//!   with at least `min_movement_left_percent` of the turn's movement left
//!   (spent at once). Hidden from enemies ([`is_hidden_from`]) unless one of
//!   their armies or spies comes close. An enemy march entering its zone of
//!   control springs it ([`spring_ambush`], called by `march`): success opens
//!   the battle with [`BattleOpening::Ambush`], failure gives a normal
//!   battle. Moving or attacking leaves the stance.
//! - **Forced march**: taken before moving; more movement at once
//!   ([`CampaignState::army_grid_allowance`]); no attack, siege, entry into a
//!   hostile settlement nor ambush this turn; supply cost at the end of the
//!   turn; back to `Normal` at the start of the next.
//! - **Entrenched**: taken before moving, outside a settlement; stays until
//!   a move order; less supply lost, `BattleDefense` in auto-resolve.

use data_model::{CoverClass, FactionId, GameData, PostureRules};
use sim_battle::{BattleOpening, SideId};

use crate::events::{EventKind, GameEvent};
use crate::orders::OrderError;
use crate::state::{Army, ArmyId, CampaignState, Stance};

fn rules(data: &GameData) -> &PostureRules {
    &data.posture_rules
}

fn refused(reason: impl Into<String>) -> OrderError {
    OrderError::StanceRefused(reason.into())
}

/// Movement points (grid costs) of the turn for `army` without its stance
/// bonus: the "full" movement of a fresh turn.
pub fn base_allowance(state: &CampaignState, data: &GameData, army: &Army) -> u32 {
    state.army_base_grid_allowance(data, army)
}

/// `base` grid points with the forced march bonus when `stance` is one.
pub fn with_stance_bonus(data: &GameData, stance: Stance, base: u32) -> u32 {
    if stance == Stance::ForcedMarch {
        let bonus = rules(data).forced_march.movement_bonus_percent;
        (f64::from(base) * (1.0 + bonus / 100.0)).round() as u32
    } else {
        base
    }
}

/// `true` when `army` has not moved this turn (movement left at least its
/// base allowance).
fn has_full_movement(state: &CampaignState, data: &GameData, army: &Army) -> bool {
    army.movement_left >= base_allowance(state, data, army).max(1)
}

/// Cover class of the cell `army` stands on.
pub fn army_cover(state: &CampaignState, data: &GameData, army: &Army) -> CoverClass {
    data.cover_class_at(state.army_point(data, army), &rules(data).cover)
}

/// Checks that `army` may switch to `stance` now (French reasons).
pub fn validate_stance_change(
    state: &CampaignState,
    data: &GameData,
    army_id: &ArmyId,
    stance: Stance,
) -> Result<(), OrderError> {
    let army = state
        .armies
        .get(army_id)
        .ok_or_else(|| OrderError::UnknownArmy(army_id.clone()))?;
    if army.stance == stance {
        return Ok(());
    }
    let forced = army.stance == Stance::ForcedMarch;
    match stance {
        Stance::Normal | Stance::Raid => Ok(()),
        Stance::Siege => {
            if forced {
                Err(refused(
                    "une armée en marche forcée ne peut assiéger ce tour",
                ))
            } else {
                Ok(())
            }
        }
        Stance::Ambush => {
            if forced {
                return Err(refused(
                    "pas d'embuscade le tour d'une marche forcée : les hommes sont fourbus",
                ));
            }
            if army.settlement().is_some() {
                return Err(refused(
                    "une embuscade se tend en rase campagne, pas dans une place",
                ));
            }
            let cover = army_cover(state, data, army);
            if !cover.is_covered() {
                return Err(refused(format!(
                    "pas de couvert ici ({}) : il faut une forêt, un bocage ou un marais",
                    cover.label_fr()
                )));
            }
            let percent = rules(data).ambush.min_movement_left_percent;
            let needed =
                (f64::from(base_allowance(state, data, army)) * percent / 100.0).ceil() as u32;
            if army.movement_left < needed.max(1) {
                return Err(refused(format!(
                    "il faut encore {percent:.0} % du mouvement du tour pour s'embusquer"
                )));
            }
            Ok(())
        }
        Stance::ForcedMarch => {
            if !has_full_movement(state, data, army) {
                return Err(refused(
                    "la marche forcée se décide avant de bouger ce tour",
                ));
            }
            Ok(())
        }
        Stance::Entrenched => {
            if forced {
                return Err(refused("pas de camp retranché le tour d'une marche forcée"));
            }
            if army.settlement().is_some() {
                return Err(refused(
                    "on ne se retranche pas dans une place : le camp se dresse en rase campagne",
                ));
            }
            if !has_full_movement(state, data, army) {
                return Err(refused(
                    "l'armée a déjà bougé ce tour : trop tard pour se retrancher",
                ));
            }
            Ok(())
        }
    }
}

/// Every stance with `Ok` when `army` may take it now, or the reason why not.
pub fn stance_options(
    state: &CampaignState,
    data: &GameData,
    army: &ArmyId,
) -> Vec<(Stance, Result<(), OrderError>)> {
    Stance::ALL
        .iter()
        .map(|stance| (*stance, validate_stance_change(state, data, army, *stance)))
        .collect()
}

/// Order `SetStance`: validates, then applies the stance and its immediate
/// effect on the movement points.
pub(crate) fn set_stance(
    state: &mut CampaignState,
    data: &GameData,
    army_id: &ArmyId,
    stance: Stance,
) -> Result<(), OrderError> {
    validate_stance_change(state, data, army_id, stance)?;
    let base = {
        let army = &state.armies[army_id];
        if army.stance == stance {
            return Ok(());
        }
        base_allowance(state, data, army)
    };
    let army = state.armies.get_mut(army_id).expect("checked");
    let previous = army.stance;
    army.stance = stance;
    match stance {
        Stance::Ambush | Stance::Entrenched => {
            army.movement_left = 0;
            army.clear_plan();
        }
        Stance::ForcedMarch => {
            army.movement_left = with_stance_bonus(data, stance, base);
        }
        _ => {
            // Giving up a forced march before moving takes the bonus back.
            if previous == Stance::ForcedMarch {
                army.movement_left = army.movement_left.min(base);
            }
        }
    }
    Ok(())
}

/// A move or attack order leaves the ambush and the entrenched camp.
pub(crate) fn leave_static_stance(state: &mut CampaignState, army_id: &ArmyId) {
    if let Some(army) = state.armies.get_mut(army_id) {
        if matches!(army.stance, Stance::Ambush | Stance::Entrenched) {
            army.stance = Stance::Normal;
        }
    }
}

/// Refuses what a forced march forbids (`what`: "attaquer", ...).
pub(crate) fn check_forced_march(army: &Army, what: &'static str) -> Result<(), OrderError> {
    if army.stance == Stance::ForcedMarch {
        Err(OrderError::ForcedMarchForbids(what))
    } else {
        Ok(())
    }
}

/// `true` when `army` is hidden (ambush) from `viewer`: neither the owner,
/// an ally, an army of `viewer` (or ally) within `detect_radius_army_km`,
/// nor a spy of theirs within `detect_radius_spy_km` of it.
pub fn is_hidden_from(
    state: &CampaignState,
    data: &GameData,
    army: &Army,
    viewer: &FactionId,
) -> bool {
    if army.stance != Stance::Ambush || state.is_allied(viewer, &army.faction) {
        return false;
    }
    let ambush = &rules(data).ambush;
    let watcher = |faction: &FactionId| state.is_allied(viewer, faction);
    let near_army = state.armies.values().any(|other| {
        watcher(&other.faction)
            && state.army_distance_km(data, other, army) <= ambush.detect_radius_army_km
    });
    if near_army {
        return false;
    }
    let point = state.army_point(data, army);
    let px_per_km = f64::from(crate::march::px_per_km(data));
    let near_spy = state.agents.agents.values().any(|agent| {
        agent.kind == data_model::AgentKind::Spy
            && watcher(&agent.faction)
            && data.settlement_point(&agent.location).is_some_and(|p| {
                let (dx, dy) = (f64::from(p[0] - point[0]), f64::from(p[1] - point[1]));
                (dx * dx + dy * dy).sqrt() / px_per_km <= ambush.detect_radius_spy_km
            })
    });
    !near_spy
}

/// Ambush skill of `army`'s general: the best of his Command and of his
/// `Intrigue` (traits, skills, companions).
pub fn ambush_skill(state: &CampaignState, data: &GameData, army: &Army) -> f64 {
    let Some(general) = army
        .general
        .as_ref()
        .filter(|g| state.characters.get(*g).is_some_and(|c| c.alive))
    else {
        return 0.0;
    };
    let command = f64::from(state.characters[general].skills.command);
    let intrigue = crate::skills::character_effects(state, data, general)
        .intrigue
        .apply(0.0);
    command.max(intrigue)
}

/// Share (0-1) of `army`'s men in scout units (light horse).
pub fn scout_share(data: &GameData, army: &Army) -> f64 {
    let scouts = &rules(data).ambush.scout_unit_types;
    let total = army.total_strength();
    if total == 0 {
        return 0.0;
    }
    let men: u32 = army
        .units
        .iter()
        .filter(|u| scouts.iter().any(|s| s == u.unit_type.as_str()))
        .map(|u| u.strength)
        .sum();
    f64::from(men) / f64::from(total)
}

/// Success chance (0-1) of the ambush of `ambusher` on `victim` (spec §
/// 1.2): base + cover bonus + skill − scouts (+ forced march of the victim),
/// clamped.
pub fn ambush_chance(
    state: &CampaignState,
    data: &GameData,
    ambusher: &ArmyId,
    victim: &ArmyId,
) -> f64 {
    let (Some(a), Some(v)) = (state.armies.get(ambusher), state.armies.get(victim)) else {
        return 0.0;
    };
    let ambush = &rules(data).ambush;
    let cover = army_cover(state, data, a);
    let mut chance = ambush.base_chance
        + ambush.terrain_bonus.get(&cover).copied().unwrap_or(0.0)
        + ambush.per_skill * ambush_skill(state, data, a)
        - ambush.scout_malus * scout_share(data, v);
    if v.stance == Stance::ForcedMarch {
        chance += rules(data).forced_march.ambush_bonus;
    }
    chance.clamp(ambush.chance_min, ambush.chance_max)
}

/// An enemy march (`victim`) entered the zone of control of `ambusher`, an
/// army in ambush at war with it: draws the success with the campaign RNG.
/// Success: battle with the ambush opening (the ambusher attacks). Failure:
/// the ambush is revealed and the marching army falls on it (normal
/// battle). Returns `true` when the ambush was sprung.
pub(crate) fn spring_ambush(
    state: &mut CampaignState,
    data: &GameData,
    ambusher: &ArmyId,
    victim: &ArmyId,
    events: &mut Vec<GameEvent>,
) -> bool {
    let (Some(a), Some(v)) = (state.armies.get(ambusher), state.armies.get(victim)) else {
        return false;
    };
    if a.stance != Stance::Ambush || !state.is_at_war(&a.faction, &v.faction) {
        return false;
    }
    let chance = ambush_chance(state, data, ambusher, victim);
    let cover = army_cover(state, data, a);
    let (a_faction, v_faction) = (a.faction.clone(), v.faction.clone());
    let province = state.army_province(data, a);
    let place = crate::march::nearest_settlement(data, state.army_point(data, a))
        .map(|p| crate::siege::settlement_name(data, &p));
    let permille = (chance * 1000.0).round().clamp(0.0, 1000.0) as u32;
    let sprung = state.rng.chance_permille(permille);
    if let Some(army) = state.armies.get_mut(ambusher) {
        army.stance = Stance::Normal;
    }
    let name = |f: &FactionId| {
        data.factions
            .get(f)
            .map_or_else(|| f.to_string(), |f| f.short_or_display_name().to_owned())
    };
    let near = place.map_or_else(String::new, |p| format!(" près de {p}"));
    let text = if sprung {
        format!(
            "Embuscade ! L'ost {} surgit {}{near} et tombe sur l'ost {} en colonne de marche.",
            sim_battle::sim::of_faction(&name(&a_faction)),
            from_cover(cover),
            sim_battle::sim::of_faction(&name(&v_faction)),
        )
    } else {
        format!(
            "Embuscade éventée{near} : les éclaireurs de l'ost {} découvrent l'ost {} tapi {}.",
            sim_battle::sim::of_faction(&name(&v_faction)),
            sim_battle::sim::of_faction(&name(&a_faction)),
            in_cover(cover),
        )
    };
    let mut event = GameEvent::new(EventKind::Battle, text)
        .army(ambusher)
        .faction(&a_faction);
    if let Some(province) = &province {
        event = event.province(province);
    }
    events.push(event);
    if sprung {
        crate::movement::fight_with_opening(
            state,
            data,
            ambusher,
            victim,
            BattleOpening::Ambush {
                victim: SideId::Defender,
            },
            events,
        );
    } else {
        crate::movement::fight(state, data, victim, ambusher, events);
    }
    sprung
}

fn from_cover(cover: CoverClass) -> &'static str {
    match cover {
        CoverClass::Forest => "de la forêt",
        CoverClass::Bocage => "des haies du bocage",
        CoverClass::Marsh => "des roseaux du marais",
        CoverClass::Open => "des fourrés",
    }
}

fn in_cover(cover: CoverClass) -> &'static str {
    match cover {
        CoverClass::Forest => "dans la forêt",
        CoverClass::Bocage => "derrière les haies du bocage",
        CoverClass::Marsh => "dans les roseaux du marais",
        CoverClass::Open => "dans les fourrés",
    }
}

/// End of turn: forced marches pay their supply, morale modifiers count
/// down.
pub(crate) fn end_of_turn(state: &mut CampaignState, data: &GameData) {
    let cost = rules(data).forced_march.supply_cost;
    for army in state.armies.values_mut() {
        if army.stance == Stance::ForcedMarch {
            army.supply = army.supply.saturating_sub(cost);
        }
        for modifier in &mut army.morale_modifiers {
            modifier.turns = modifier.turns.saturating_sub(1);
        }
        army.morale_modifiers.retain(|m| m.turns > 0);
    }
}

/// Start of a new turn (before the movement points are refilled): forced
/// marches end.
pub(crate) fn start_of_turn(state: &mut CampaignState) {
    for army in state.armies.values_mut() {
        if army.stance == Stance::ForcedMarch {
            army.stance = Stance::Normal;
        }
    }
}

/// Supply loss of an entrenched army: `loss` minus the saving.
pub fn entrenched_loss(data: &GameData, army: &Army, loss: u8) -> u8 {
    if army.stance != Stance::Entrenched {
        return loss;
    }
    let saving = rules(data).entrenched.supply_saving_percent;
    (f64::from(loss) * (1.0 - saving / 100.0))
        .round()
        .clamp(0.0, 255.0) as u8
}

/// Auto-resolve bonuses of a side led by `lead` (`BattleCharge`,
/// `BattleDefense` percents): the entrenched camp defends better; the
/// ambusher of a sprung ambush charges harder.
pub fn auto_resolve_bonus(data: &GameData, lead: Option<&Army>, ambusher: bool) -> (f64, f64) {
    let rules = rules(data);
    let charge = if ambusher {
        rules.ambush.auto_attack_percent
    } else {
        0.0
    };
    let defense = if lead.is_some_and(|a| a.stance == Stance::Entrenched) {
        rules.entrenched.auto_defense_percent
    } else {
        0.0
    };
    (charge, defense)
}
