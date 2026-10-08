//! Lot DP2 (ADR 0075): right of passage and trespass.
//!
//! An army that ends its season in lands controlled by a faction at peace
//! with its own (not allied, not vassal nor suzerain, no military access by
//! treaty) trespasses. Each season of trespass leaves the victim an opinion
//! malus ([`TRESPASS_REASON`]) that grows with the number of consecutive
//! seasons; from `casus_belli_seasons` the victim holds a casus belli
//! ([`has_grievance`], read by `CampaignState::casus_belli`). A truce gives
//! the armies of the former enemy `truce_grace_seasons` to leave.
//!
//! Detection happens once per season, at the end of the turn
//! ([`resolve_trespass`]), on the armies' positions: the march itself
//! (`movement.rs`) is untouched. The same test serves the player's path
//! preview ([`trespass_along`]) and the AI's routes ([`ai_may_trespass`]).
//!
//! Tuning: `data/ai/diplomacy.json` § `passage` ([`PassageRules`]).

use std::collections::{BTreeMap, BTreeSet};

use data_model::{FactionId, GameData, PassageRules, ProvinceId};
use serde::{Deserialize, Serialize};

use crate::diplomacy::{RelationKind, REBELS_FACTION};
use crate::events::{EventKind, GameEvent};
use crate::plan_cache::PlanCache;
use crate::state::{ArmyId, CampaignState};

/// Reason of the opinion modifier a trespass leaves with the victim.
pub const TRESPASS_REASON: &str = "Violation de nos terres";
/// Distance (map pixels) between two samples of a path.
const PATH_SAMPLE_PX: f32 = 6.0;

/// Trespass of one faction's armies on a victim's lands, kept in the
/// victim's ledger (`DiplomaticLedger::trespassers`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trespass {
    /// Consecutive seasons of trespass (0: the armies have left).
    #[serde(default)]
    pub seasons: u32,
    /// Seasons of trespass in all.
    #[serde(default)]
    pub total: u32,
    /// Last season an army of the intruder stood on our lands.
    #[serde(default)]
    pub last_turn: u32,
    /// The victim holds a casus belli until this turn (0: none).
    #[serde(default)]
    pub grievance_until: u32,
    /// Provinces trespassed during the last season.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provinces: Vec<ProvinceId>,
}

/// A province of a path crossed without right of passage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathTrespass {
    pub province: ProvinceId,
    pub owner: FactionId,
    /// A season of the march ends in this province (an incident).
    pub halt: bool,
}

pub fn rules(data: &GameData) -> &PassageRules {
    &data.ai_diplomacy.passage
}

/// The faction whose lands `faction`'s armies trespass on in `province`, if
/// any: its controller, at peace with `faction`, neither ally, vassal nor
/// suzerain, and without military access granted to `faction`. Pure; does
/// not read `enabled`.
pub fn trespassed_owner(
    state: &CampaignState,
    faction: &FactionId,
    province: &ProvinceId,
) -> Option<FactionId> {
    let controller = state.province_controller(province)?;
    if controller == faction || controller.as_str() == REBELS_FACTION {
        return None;
    }
    if !state.factions.get(controller).is_some_and(|f| f.alive) {
        return None;
    }
    match state.relation(faction, controller) {
        RelationKind::Peace | RelationKind::Truce => {}
        _ => return None,
    }
    if crate::negotiation::has_military_access(state, controller, faction) {
        return None;
    }
    Some(controller.clone())
}

/// Does `victim` hold a casus belli against `intruder` for trespass?
pub fn has_grievance(state: &CampaignState, victim: &FactionId, intruder: &FactionId) -> bool {
    state
        .factions
        .get(victim)
        .and_then(|f| f.ledger.trespassers.get(intruder))
        .is_some_and(|t| t.grievance_until >= state.turn && t.grievance_until > 0)
}

/// Malus (positive) of `seasons` counted seasons of trespass.
pub fn penalty(rules: &PassageRules, seasons: u32) -> i32 {
    if seasons == 0 {
        return 0;
    }
    let extra = rules
        .per_season_penalty
        .saturating_mul(i32::try_from(seasons - 1).unwrap_or(i32::MAX));
    rules
        .base_penalty
        .saturating_add(extra)
        .min(rules.max_penalty)
}

/// Seasons of a trespass that count (the truce grace is subtracted).
fn counted_seasons(
    state: &CampaignState,
    rules: &PassageRules,
    victim: &FactionId,
    intruder: &FactionId,
    seasons: u32,
) -> u32 {
    if state.has_truce(victim, intruder) {
        seasons.saturating_sub(rules.truce_grace_seasons)
    } else {
        seasons
    }
}

/// End of season: records the trespasses of the armies standing on foreign
/// lands, sets the victims' opinion malus and casus belli, forgets the old
/// incidents. Deterministic (ordered maps only).
pub fn resolve_trespass(state: &mut CampaignState, data: &GameData, events: &mut Vec<GameEvent>) {
    let rules = rules(data).clone();
    if !rules.enabled {
        return;
    }
    let turn = state.turn;
    // (victim, intruder) -> (provinces, armies)
    type Found = BTreeMap<(FactionId, FactionId), (BTreeSet<ProvinceId>, Vec<ArmyId>)>;
    let mut found: Found = BTreeMap::new();
    for (id, army) in &state.armies {
        if army.units.is_empty() || army.faction.as_str() == REBELS_FACTION {
            continue;
        }
        let Some(province) = state.army_province(data, army) else {
            continue;
        };
        if let Some(owner) = trespassed_owner(state, &army.faction, &province) {
            let entry = found.entry((owner, army.faction.clone())).or_default();
            entry.0.insert(province);
            entry.1.push(id.clone());
        }
    }
    // Armies gone: the count restarts; old incidents are forgotten.
    let victims: Vec<FactionId> = state.factions.keys().cloned().collect();
    for victim in &victims {
        let at_war: BTreeSet<FactionId> = state.factions[victim].at_war_with.clone();
        let f = state.factions.get_mut(victim).expect("victim");
        f.ledger.trespassers.retain(|intruder, t| {
            if at_war.contains(intruder) {
                return false;
            }
            if !found.contains_key(&(victim.clone(), intruder.clone())) {
                t.seasons = 0;
                t.provinces.clear();
            }
            t.seasons > 0
                || t.grievance_until >= turn
                || t.last_turn.saturating_add(rules.memory_turns) >= turn
        });
    }
    let player = state.player_faction.clone();
    for ((victim, intruder), (provinces, armies)) in found {
        let record = {
            let f = state.factions.get_mut(&victim).expect("victim");
            let t = f.ledger.trespassers.entry(intruder.clone()).or_default();
            t.seasons += 1;
            t.total += 1;
            t.last_turn = turn;
            t.provinces = provinces.iter().cloned().collect();
            t.clone()
        };
        let counted = counted_seasons(state, &rules, &victim, &intruder, record.seasons);
        if counted == 0 {
            continue;
        }
        let malus = penalty(&rules, counted);
        if let Some(f) = state.factions.get_mut(&victim) {
            f.modifiers
                .retain(|m| !(m.with == intruder && m.reason_fr == TRESPASS_REASON));
        }
        state.add_modifier(
            &victim,
            &intruder,
            -malus,
            TRESPASS_REASON,
            rules.memory_turns,
        );
        let grievance = counted >= rules.casus_belli_seasons;
        if grievance {
            if let Some(t) = state
                .factions
                .get_mut(&victim)
                .and_then(|f| f.ledger.trespassers.get_mut(&intruder))
            {
                t.grievance_until = turn.saturating_add(rules.memory_turns);
            }
        }
        if victim != player && intruder != player {
            continue;
        }
        let where_ = provinces
            .iter()
            .map(|p| data.province_name(p))
            .collect::<Vec<_>>()
            .join(", ");
        let mut text = format!(
            "Les armées de {} campent sans droit de passage sur les terres de {} ({}) : \
             incident diplomatique ({:+}).",
            data.faction_name(&intruder),
            data.faction_name(&victim),
            where_,
            -malus
        );
        if grievance && counted == rules.casus_belli_seasons {
            text.push_str(&format!(
                " {} tient désormais un casus belli.",
                data.faction_name(&victim)
            ));
        }
        let mut event = GameEvent::new(EventKind::Diplomacy, text).faction(&victim);
        if let Some(first) = provinces.iter().next() {
            event.province = Some(first.clone());
        }
        event.army = armies.first().cloned();
        events.push(event);
    }
}

/// Provinces of the polyline `points` (map pixels) where `faction`'s army
/// would trespass, in the order of the march; `halts` are the indices of
/// `points` where a season ends. Empty when the rules are off.
pub fn trespass_along(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    points: &[[f32; 2]],
    halts: &[usize],
) -> Vec<PathTrespass> {
    if !rules(data).enabled || points.is_empty() {
        return Vec::new();
    }
    let mut result: Vec<PathTrespass> = Vec::new();
    let visit = |p: [f32; 2], halt: bool, result: &mut Vec<PathTrespass>| {
        let Some(province) = data.province_at_point(p[0], p[1]) else {
            return;
        };
        let Some(owner) = trespassed_owner(state, faction, province) else {
            return;
        };
        if let Some(existing) = result.iter_mut().find(|t| &t.province == province) {
            existing.halt |= halt;
        } else {
            result.push(PathTrespass {
                province: province.clone(),
                owner,
                halt,
            });
        }
    };
    // The army's own position (index 0) is where it already stands.
    for (i, window) in points.windows(2).enumerate() {
        let (a, b) = (window[0], window[1]);
        let length = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
        let steps = (length / PATH_SAMPLE_PX).ceil().max(1.0) as usize;
        for s in 1..=steps {
            let t = s as f32 / steps as f32;
            let p = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
            let halt = s == steps && halts.contains(&(i + 1));
            visit(p, halt, &mut result);
        }
    }
    if let Some(last) = points.last() {
        visit(*last, true, &mut result);
    }
    result
}

/// Would `faction`'s AI march through `owner`'s lands without right of
/// passage? Never at peace; at war, when its temper is aggressive, when it
/// hates the owner, or when the owner is much weaker and not liked.
pub fn ai_may_trespass(
    cache: &PlanCache,
    data: &GameData,
    faction: &FactionId,
    owner: &FactionId,
) -> bool {
    let state = cache.state();
    let rules = rules(data);
    if !rules.enabled {
        return true;
    }
    let at_war = state
        .factions
        .get(faction)
        .is_some_and(|f| f.at_war_with.iter().any(|e| e.as_str() != REBELS_FACTION));
    if !at_war {
        return false;
    }
    let aggression = data
        .factions
        .get(faction)
        .and_then(|f| f.ai_personality.as_ref())
        .and_then(|p| p.aggression)
        .map_or(50, i32::from);
    if aggression >= rules.ai_violate_aggression {
        return true;
    }
    let (attitude, _) = cache.attitude(data, faction, owner);
    if attitude <= rules.ai_violate_attitude {
        return true;
    }
    let ratio = cache.faction_power(faction) / cache.faction_power(owner).max(1.0);
    ratio >= rules.ai_violate_power_ratio && attitude < 0
}
