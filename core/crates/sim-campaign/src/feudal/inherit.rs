//! Inheritance of titles (lot F3, spec § 4.5).
//!
//! - **Contested succession**: when the heir designated by the late ruler
//!   and the heir by law differ, and the faction has a suzerain, the
//!   suzerain arbitrates (`feudal_rules.arbitration`). The losing claimant
//!   flees to the strongest enemy of the arbiter, which presses his claim by
//!   war (Brittany 1341: Blois for France, Montfort for England).
//! - **Extinct line**: when no heir remains in the faction, each title goes
//!   to the nearest kin in another faction by the title's own law (or the
//!   faction's): personal union, the faction left without title vanishing
//!   into the heir's. Without kin, a title escheats to the holder of the
//!   title above; a sovereign title stays with the faction, which finds a
//!   new ruler as before (Burgundy 1361: the duchy to John II, the county
//!   to Marguerite).

use data_model::{CharacterId, ClaimKind, FactionId, GameData, Sex, SuccessionLaw, TitleId};

use super::{holder_of, liege_of, titles_of, SuccessionDispute};
use crate::diplomacy::{faction_name, Claim};
use crate::events::{EventKind, GameEvent};
use crate::state::{CampaignState, CharacterState};

fn title_name(data: &GameData, title: &TitleId) -> String {
    data.titles
        .get(title)
        .map_or_else(|| title.to_string(), |t| t.name.display.clone())
}

/// `claimant` (or the claimant's spouse) belongs to `arbiter`'s court or is
/// kin of its ruler (same house, or a mother of that house).
fn tied_to(state: &CampaignState, claimant: &CharacterId, arbiter: &FactionId) -> bool {
    let ruler_house = state
        .factions
        .get(arbiter)
        .and_then(|f| f.ruler.as_ref())
        .and_then(|r| state.characters.get(r))
        .map(|c| c.house.clone());
    let tie = |c: &CharacterState| {
        &c.faction == arbiter
            || ruler_house.as_ref().is_some_and(|house| {
                &c.house == house
                    || c.mother
                        .as_ref()
                        .and_then(|m| state.characters.get(m))
                        .is_some_and(|m| &m.house == house)
            })
    };
    let Some(character) = state.characters.get(claimant) else {
        return false;
    };
    tie(character)
        || character
            .spouse
            .as_ref()
            .and_then(|s| state.characters.get(s))
            .is_some_and(|s| s.alive && tie(s))
}

/// Arbitrates a contested succession of `faction` (see the module doc).
/// Returns the new ruler when there was a dispute, `None` otherwise (the
/// caller keeps its usual succession).
pub(crate) fn contested_succession(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    dead_ruler: Option<&CharacterId>,
    designated: Option<&CharacterId>,
    events: &mut Vec<GameEvent>,
) -> Option<CharacterId> {
    let designated = designated?.clone();
    let law_heir = crate::dynasty::pick_heir_by_law(state, data, faction, dead_ruler?)?;
    if law_heir == designated {
        return None;
    }
    let title = state.feudal.primary.get(faction)?.clone();
    let arbiter = liege_of(state, data, faction)?;
    if !state.factions.get(&arbiter).is_some_and(|f| f.alive) {
        return None;
    }
    let weights = &data.feudal_rules.arbitration;
    let score = |claimant: &CharacterId, base: i32| {
        base + if tied_to(state, claimant, &arbiter) {
            weights.family_tie
        } else {
            0
        }
    };
    let designated_score = score(&designated, weights.designated_heir);
    let law_score = score(&law_heir, weights.law_heir);
    let (winner, loser) = if law_score > designated_score {
        (law_heir, designated)
    } else {
        (designated, law_heir)
    };
    events.push(
        GameEvent::new(
            EventKind::Succession,
            format!(
                "Succession contestée en {} : {} et {} se disputent {} ; {} tranche en faveur de {}.",
                faction_name(data, faction),
                state.character_name(data, &winner),
                state.character_name(data, &loser),
                title_name(data, &title),
                faction_name(data, &arbiter),
                state.character_name(data, &winner)
            ),
        )
        .faction(faction),
    );
    super::record_rival_claimant(state, data, faction);
    let sponsor = state
        .factions
        .iter()
        .filter(|(id, f)| {
            f.alive
                && *id != faction
                && **id != arbiter
                && id.as_str() != crate::diplomacy::REBELS_FACTION
                && f.at_war_with.contains(&arbiter)
        })
        .map(|(id, _)| id.clone())
        .max_by(|a, b| {
            state
                .faction_power(a)
                .total_cmp(&state.faction_power(b))
                .then_with(|| b.cmp(a))
        });
    if let Some(sponsor) = &sponsor {
        state.detach_general(&loser);
        if let Some(c) = state.characters.get_mut(&loser) {
            c.faction = sponsor.clone();
            c.governor_of = None;
        }
        let text = format!(
            "{} se réfugie auprès de {} et revendique {}.",
            state.character_name(data, &loser),
            faction_name(data, sponsor),
            title_name(data, &title)
        );
        state
            .factions
            .get_mut(sponsor)
            .expect("alive")
            .claims
            .push(Claim {
                kind: ClaimKind::Throne,
                faction: Some(faction.clone()),
                province: None,
                text_fr: text.clone(),
                expires_turn: None,
            });
        events.push(GameEvent::new(EventKind::Diplomacy, text).faction(sponsor));
        if !state.is_at_war(sponsor, faction) {
            // A claim is a casus belli: the succession war begins.
            let _ = state.declare_war(data, sponsor, faction);
        }
    }
    state.feudal.disputes.push(SuccessionDispute {
        faction: faction.clone(),
        title,
        claimants: vec![winner.clone(), loser],
        arbiter,
        winner: winner.clone(),
        sponsor,
        turn: state.turn,
    });
    Some(winner)
}

/// Orders heirs by `law` (spec § 2 laws, applied to kin abroad). The Salic
/// law (and the default) only knows agnates: men of `house` itself, never
/// the sons of its daughters (1328).
fn best_by_law(
    state: &CampaignState,
    law: Option<SuccessionLaw>,
    house: &str,
    mut candidates: Vec<CharacterId>,
) -> Option<CharacterId> {
    let get = |id: &CharacterId| state.characters.get(id).expect("listed");
    candidates.sort_by_key(|id| (get(id).birth_year, id.clone()));
    match law {
        Some(SuccessionLaw::Salic) | None => candidates
            .into_iter()
            .find(|id| get(id).sex == Sex::Male && get(id).house == house),
        Some(SuccessionLaw::MalePreferencePrimogeniture) => candidates
            .iter()
            .find(|id| get(id).sex == Sex::Male)
            .or_else(|| candidates.first())
            .cloned(),
        Some(SuccessionLaw::CognaticPrimogeniture) => candidates.into_iter().next(),
        Some(SuccessionLaw::Elective) => candidates
            .into_iter()
            .max_by_key(|id| (get(id).skills.court, std::cmp::Reverse(id.clone()))),
    }
}

/// Kin of `house` living in other factions: members of the house, or
/// children of a mother or father of the house.
fn kin_abroad(state: &CampaignState, faction: &FactionId, house: &str) -> Vec<CharacterId> {
    let of_house = |id: &Option<CharacterId>| {
        id.as_ref()
            .and_then(|p| state.characters.get(p))
            .is_some_and(|p| p.house == house)
    };
    state
        .characters
        .iter()
        .filter(|(_, c)| {
            c.alive
                && &c.faction != faction
                && c.faction.as_str() != crate::diplomacy::REBELS_FACTION
                && state.factions.get(&c.faction).is_some_and(|f| f.alive)
                && (c.house == house || of_house(&c.mother) || of_house(&c.father))
        })
        .map(|(id, _)| id.clone())
        .collect()
}

/// Titles of `faction` when its line dies out (see the module doc).
/// Returns `true` when the faction, left without title, has vanished.
pub(crate) fn inherit_titles_on_extinction(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    dead_ruler: Option<&CharacterId>,
    events: &mut Vec<GameEvent>,
) -> bool {
    let Some(house) = dead_ruler
        .and_then(|r| state.characters.get(r))
        .map(|c| c.house.clone())
    else {
        return false;
    };
    // The primary title last: the remaining lands follow its heir.
    let mut titles = titles_of(state, faction);
    if !titles.is_empty() {
        titles.rotate_left(1);
    }
    let faction_law = data.factions.get(faction).map(|f| f.succession_law);
    for title in titles {
        if !state.factions.get(faction).is_some_and(|f| f.alive) {
            break;
        }
        let law = data
            .titles
            .get(&title)
            .and_then(|t| t.succession_law)
            .or(faction_law);
        let heir = best_by_law(state, law, &house, kin_abroad(state, faction, &house));
        if let Some(heir) = heir {
            let to = state.characters[&heir].faction.clone();
            events.push(
                GameEvent::new(
                    EventKind::Succession,
                    format!(
                        "La lignée de {} s'éteint : {} hérite de {}, uni aux possessions de {}.",
                        faction_name(data, faction),
                        state.character_name(data, &heir),
                        title_name(data, &title),
                        faction_name(data, &to)
                    ),
                )
                .faction(&to),
            );
            let _ = super::transfer::transfer(state, data, &title, &to, events);
            continue;
        }
        let liege = data
            .titles
            .get(&title)
            .and_then(|t| t.de_jure_liege.as_ref())
            .and_then(|l| holder_of(state, l))
            .filter(|h| *h != faction && state.factions.get(*h).is_some_and(|f| f.alive))
            .cloned();
        if let Some(liege) = liege {
            events.push(
                GameEvent::new(
                    EventKind::Succession,
                    format!(
                        "Déshérence : faute d'héritier, {} revient à {}.",
                        title_name(data, &title),
                        faction_name(data, &liege)
                    ),
                )
                .faction(&liege),
            );
            let _ = super::transfer::transfer(state, data, &title, &liege, events);
        }
    }
    !state.factions.get(faction).is_some_and(|f| f.alive)
}

/// The successor `heir` of `faction` lives in another faction (spec § 4.5,
/// personal union). From a lesser realm, the heir comes home: the realm he
/// rules is united to `faction` (Charles d'Alençon king of France brings
/// Alençon back to the crown), or he simply leaves the court he served.
/// From an equal or greater realm, `faction`'s titles pass to the heir's
/// realm. Returns `true` when `faction` has thereby vanished.
pub(crate) fn heir_comes_home(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    heir: &CharacterId,
    events: &mut Vec<GameEvent>,
) -> bool {
    let Some(home) = state.characters.get(heir).map(|c| c.faction.clone()) else {
        return false;
    };
    if &home == faction || !state.factions.get(&home).is_some_and(|f| f.alive) {
        return false;
    }
    let own_rank = super::primary_rank(state, data, faction);
    let home_rank = super::primary_rank(state, data, &home);
    if home_rank.is_some() && home_rank >= own_rank && own_rank.is_some() {
        let mut titles = titles_of(state, faction);
        if !titles.is_empty() {
            titles.rotate_left(1);
        }
        events.push(
            GameEvent::new(
                EventKind::Succession,
                format!(
                    "{} hérite de {}, uni aux possessions de {}.",
                    state.character_name(data, heir),
                    faction_name(data, faction),
                    faction_name(data, &home)
                ),
            )
            .faction(&home),
        );
        for title in titles {
            let _ = super::transfer::transfer(state, data, &title, &home, events);
        }
        return !state.factions.get(faction).is_some_and(|f| f.alive);
    }
    let rules_home = state.factions[&home].ruler.as_ref() == Some(heir);
    if rules_home {
        events.push(
            GameEvent::new(
                EventKind::Succession,
                format!(
                    "{} hérite de {} : {} y est réuni.",
                    state.character_name(data, heir),
                    faction_name(data, faction),
                    faction_name(data, &home)
                ),
            )
            .faction(faction),
        );
        let mut titles = titles_of(state, &home);
        if !titles.is_empty() {
            titles.rotate_left(1);
        }
        for title in titles {
            let _ = super::transfer::transfer(state, data, &title, faction, events);
        }
    }
    if state.characters.get(heir).map(|c| &c.faction) != Some(faction) {
        state.detach_general(heir);
        if let Some(c) = state.characters.get_mut(heir) {
            c.faction = faction.clone();
            c.governor_of = None;
        }
        // The realm he left keeps a ruler and needs a new heir.
        if state.factions.get(&home).is_some_and(|f| f.alive) {
            if rules_home {
                crate::characters::succeed(state, data, &home, events);
            } else if state.factions[&home].heir.as_ref() == Some(heir) {
                let next = state.factions[&home]
                    .ruler
                    .clone()
                    .and_then(|r| crate::dynasty::pick_heir_by_law(state, data, &home, &r));
                state.factions.get_mut(&home).expect("alive").heir = next;
            }
        }
    }
    false
}
