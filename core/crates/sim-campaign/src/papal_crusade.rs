//! The pope's call to crusade (ADR 0327, Medieval II's papal crusades).
//!
//! From time to time the pope designates a target: a province of the Holy
//! Land (`data/rules/crusade.json` `holy_land`) or a province of Christian
//! faith held by an infidel. A Catholic that goes to war with the holder
//! during the window earns papal favour and prestige; the first Catholic to
//! hold the target receives the reward and ends the call. Everything is read
//! from `data/rules/religion.json` `papal_crusade`; without that file nothing
//! happens. Distinct from the crusader faction's fervour ([`crate::crusade`],
//! ADR 0165), which it feeds only through the war it provokes.

use std::collections::BTreeSet;

use data_model::{FactionId, GameData, ProvinceId, ReligionId, ReligionKind};
use serde::{Deserialize, Serialize};

use crate::diplomacy::PAPACY_FACTION;
use crate::events::{EventKind, GameEvent};
use crate::religion::{self, FaithRelation};
use crate::state::CampaignState;

/// A call to crusade in progress.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PapalCrusade {
    pub target: ProvinceId,
    pub called_turn: u32,
    pub expires_turn: u32,
    /// Catholics that went to war with the holder of the target.
    #[serde(default)]
    pub participants: BTreeSet<FactionId>,
}

/// A Church of the data (the Catholic one), reference for « Christian ».
fn church(data: &GameData) -> Option<&ReligionId> {
    data.religions
        .iter()
        .find(|(_, r)| r.kind == ReligionKind::Church)
        .map(|(id, _)| id)
}

/// `true` when `faction` is a living lord of another faith than the Church
/// (neither Catholic nor a kindred church).
fn is_infidel(state: &CampaignState, data: &GameData, faction: &FactionId) -> bool {
    if faction.is_rebels() || faction.as_str() == PAPACY_FACTION {
        return false;
    }
    if !state.factions.get(faction).is_some_and(|f| f.alive) {
        return false;
    }
    let (Some(church), Some(faith)) = (
        church(data),
        religion::faction_religion(state, data, faction),
    ) else {
        return false;
    };
    religion::religions_relation(data, &faith, church) == FaithRelation::Different
}

/// The province the pope would designate now: Holy Land first, then a
/// Christian province held by an infidel; ties by identifier.
pub fn pick_target(state: &CampaignState, data: &GameData) -> Option<ProvinceId> {
    let church = church(data)?;
    let holy: BTreeSet<&ProvinceId> = data
        .crusade_rules
        .as_ref()
        .map(|c| c.holy_land.iter().collect())
        .unwrap_or_default();
    state
        .provinces
        .keys()
        .filter_map(|id| {
            let holder = state.province_controller(id)?;
            if !is_infidel(state, data, holder) {
                return None;
            }
            let christian = state.province_faith(data, id).is_some_and(|f| {
                matches!(
                    religion::religions_relation(data, &f, church),
                    FaithRelation::Same | FaithRelation::RivalObedience
                )
            });
            let is_holy = holy.contains(id);
            (is_holy || christian).then(|| (u8::from(is_holy), id.clone()))
        })
        .max_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.cmp(&a.1)))
        .map(|(_, id)| id)
}

/// Current holder of the target of the call, if it is a lord to fight.
pub fn target_holder(state: &CampaignState) -> Option<FactionId> {
    let call = state.papal_crusade.as_ref()?;
    state.province_controller(&call.target).cloned()
}

/// AI: the holder `faction` is called to fight, when it should answer
/// (Catholic, in the pope's grace, not already its master).
pub fn ai_call_target(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> Option<(FactionId, f64)> {
    let rules = &data.religion_rules.as_ref()?.papal_crusade;
    let holder = target_holder(state)?;
    let favor = state.factions.get(faction)?.papal_favor;
    // The call draws a few princes, not all of Christendom.
    let full = state.papal_crusade.as_ref().is_some_and(|c| {
        c.participants.len() as u32 >= rules.ai_max_participants
            && !c.participants.contains(faction)
    });
    (&holder != faction
        && !full
        && religion::is_catholic(state, data, faction)
        && !religion::is_excommunicated(state, faction)
        && favor >= rules.ai_min_favor
        && is_infidel(state, data, &holder))
    .then_some((holder, rules.ai_min_ratio))
}

/// Hook of a declaration of war: a Catholic attacking the holder of the
/// target during the call earns favour and prestige (once per call).
pub(crate) fn on_war_declared(
    state: &mut CampaignState,
    data: &GameData,
    attacker: &FactionId,
    target: &FactionId,
) {
    let Some(rules) = data
        .religion_rules
        .as_ref()
        .map(|r| r.papal_crusade.clone())
    else {
        return;
    };
    if target_holder(state).as_ref() != Some(target)
        || !religion::is_catholic(state, data, attacker)
    {
        return;
    }
    join(state, attacker, &rules);
}

fn join(state: &mut CampaignState, faction: &FactionId, rules: &data_model::PapalCrusadeRules) {
    let fresh = state
        .papal_crusade
        .as_mut()
        .is_some_and(|c| c.participants.insert(faction.clone()));
    if fresh {
        religion::change_favor(state, faction, i32::from(rules.join_favor));
        state.change_ruler_prestige(faction, rules.join_prestige);
    }
}

/// Phase (with the religion): a call ends in victory or lapses; a new one is
/// made when the interval has passed.
pub(crate) fn resolve_papal_crusade(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let Some(rules) = data
        .religion_rules
        .as_ref()
        .map(|r| r.papal_crusade.clone())
    else {
        return;
    };
    if let Some(call) = state.papal_crusade.clone() {
        let holder = state.province_controller(&call.target).cloned();
        let winner = holder
            .filter(|h| state.factions.get(h).is_some_and(|f| f.alive))
            .filter(|h| religion::is_catholic(state, data, h));
        if let Some(winner) = winner {
            reward(state, data, &winner, &call, &rules, events);
            state.papal_crusade = None;
            state.last_papal_call_end = Some(state.turn);
        } else if state.turn >= call.expires_turn {
            events.push(GameEvent::new(
                EventKind::Crusade,
                format!(
                    "L'appel à la croisade pour {} reste sans suite : la chrétienté s'est détournée.",
                    data.province_name(&call.target)
                ),
            ));
            state.papal_crusade = None;
            state.last_papal_call_end = Some(state.turn);
        }
        return;
    }
    let ready = state.turn >= rules.first_call_turn
        && state
            .last_papal_call_end
            .is_none_or(|end| state.turn >= end + rules.interval_turns);
    let pope_lives = FactionId::new(PAPACY_FACTION)
        .ok()
        .and_then(|p| state.factions.get(&p))
        .is_some_and(|f| f.alive);
    if !ready || !pope_lives {
        return;
    }
    let Some(target) = pick_target(state, data) else {
        return;
    };
    let holder = state.province_controller(&target).cloned();
    state.papal_crusade = Some(PapalCrusade {
        target: target.clone(),
        called_turn: state.turn,
        expires_turn: state.turn + rules.window_turns,
        participants: BTreeSet::new(),
    });
    // Catholics already at war with the holder answer at once.
    if let Some(holder) = holder {
        let belligerents: Vec<FactionId> = state
            .factions
            .iter()
            .filter(|(id, f)| {
                f.alive && f.at_war_with.contains(&holder) && religion::is_catholic(state, data, id)
            })
            .map(|(id, _)| id.clone())
            .collect();
        for faction in &belligerents {
            join(state, faction, &rules);
        }
    }
    events.push(GameEvent::new(
        EventKind::Crusade,
        format!(
            "Le pape appelle la chrétienté à la croisade : reprendre {} ({}). Le premier prince à la tenir sera comblé.",
            data.province_name(&target),
            turns_text(rules.window_turns)
        ),
    ).province(&target));
}

fn turns_text(turns: u32) -> String {
    format!("{turns} saisons pour répondre")
}

fn reward(
    state: &mut CampaignState,
    data: &GameData,
    winner: &FactionId,
    call: &PapalCrusade,
    rules: &data_model::PapalCrusadeRules,
    events: &mut Vec<GameEvent>,
) {
    religion::change_favor(state, winner, i32::from(rules.reward_favor));
    state.change_ruler_prestige(winner, rules.reward_prestige);
    let papal_gift = FactionId::new(PAPACY_FACTION)
        .ok()
        .and_then(|p| state.factions.get_mut(&p))
        .map_or(0, |papacy| {
            let paid = rules.reward_gold.min(papacy.treasury).max(0);
            papacy.treasury -= paid;
            paid
        });
    if let Some(f) = state.factions.get_mut(winner) {
        f.treasury += papal_gift;
    }
    events.push(
        GameEvent::new(
            EventKind::Crusade,
            format!(
                "La croisade est victorieuse : {} tient {} et reçoit la bénédiction du pape.",
                data.faction_name(winner),
                data.province_name(&call.target)
            ),
        )
        .province(&call.target)
        .faction(winner),
    );
}
