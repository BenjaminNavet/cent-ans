//! Strategic doctrines of the AI (ADR 0782): which doctrine a faction follows now, and the
//! revenge it owes.
//!
//! The catalogue and the assignment list are data (`data/ai/strategic_doctrines.json`). The
//! assignment is read from top to bottom and the first line whose conditions all hold names the
//! doctrine: faction, sovereign, years, defeats suffered. A change of sovereign (Charles V after
//! Jean II) or a run of defeats therefore changes the doctrine without any code.
//!
//! Everything here is a pure function of the state: deterministic, no random roll.

use data_model::{DoctrineCondition, FactionId, ProvinceId, StrategicDoctrine, StrategicDoctrines};

use crate::state::CampaignState;

/// The doctrine a faction follows at this moment.
#[derive(Debug, Clone, Copy)]
pub struct ActiveDoctrine {
    /// Key of the catalogue (`chevauchee_pillage`...), also the key of the causes texts.
    pub id: &'static str,
    pub doctrine: &'static StrategicDoctrine,
}

/// Field battles `faction` has lost so far (the permanent tally, all rivals together).
pub fn defeats(state: &CampaignState, faction: &FactionId) -> u32 {
    state
        .chronicle
        .battle_tally
        .iter()
        .filter(|t| &t.loser == faction)
        .map(|t| t.victories)
        .sum()
}

fn holds(state: &CampaignState, faction: &FactionId, condition: &DoctrineCondition) -> bool {
    condition.faction.as_ref().is_none_or(|f| f == faction)
        && condition.ruler.as_ref().is_none_or(|r| {
            state
                .factions
                .get(faction)
                .is_some_and(|f| f.ruler.as_ref() == Some(r))
        })
        && condition.from_year.is_none_or(|y| state.year >= y)
        && condition.to_year.is_none_or(|y| state.year <= y)
        && condition
            .min_defeats
            .is_none_or(|n| defeats(state, faction) >= n)
}

/// The doctrine of `faction` in `catalogue` (testable with another catalogue than the bundled).
pub fn active_in<'a>(
    catalogue: &'a StrategicDoctrines,
    state: &CampaignState,
    faction: &FactionId,
) -> (&'a str, &'a StrategicDoctrine) {
    let id = catalogue
        .assignments
        .iter()
        .find(|a| holds(state, faction, &a.condition))
        .map_or(catalogue.default.as_str(), |a| a.doctrine.as_str());
    let doctrine = catalogue
        .doctrines
        .get(id)
        .or_else(|| catalogue.doctrines.get(&catalogue.default))
        .expect("the catalogue holds its default doctrine");
    (id, doctrine)
}

/// The doctrine `faction` follows now (bundled catalogue).
pub fn active(state: &CampaignState, faction: &FactionId) -> ActiveDoctrine {
    let catalogue = StrategicDoctrines::bundled();
    let (id, doctrine) = active_in(catalogue, state, faction);
    ActiveDoctrine { id, doctrine }
}

/// A rival the faction wants to avenge itself on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revenge {
    /// Battles the rival won over the faction, beyond those the faction won over it.
    pub net_defeats: u32,
    /// Province of the latest battle the rival won over the faction.
    pub place: Option<ProvinceId>,
}

/// Whether `target` beat `faction` in the field more often than `faction` beat it (by the
/// threshold of the data): the faction then owes it a revenge (Crécy, Poitiers).
pub fn revenge_on(
    state: &CampaignState,
    faction: &FactionId,
    target: &FactionId,
) -> Option<Revenge> {
    let tally = &state.chronicle;
    let lost = tally.victories(target, faction);
    let won = tally.victories(faction, target);
    let net = lost.saturating_sub(won);
    let min = StrategicDoctrines::bundled().tuning.revenge_min_net_defeats;
    (net >= min.max(1)).then(|| Revenge {
        net_defeats: net,
        place: tally
            .battle_tally
            .iter()
            .find(|t| &t.winner == target && &t.loser == faction)
            .and_then(|t| t.last_province.clone()),
    })
}

/// Factor on the power ratio a war on `target` demands, from the revenge the doctrine owes
/// (1.0 without revenge: `weight` of 0 or no defeat).
pub fn revenge_war_factor(state: &CampaignState, faction: &FactionId, target: &FactionId) -> f64 {
    if revenge_on(state, faction, target).is_none() {
        return 1.0;
    }
    let active = active(state, faction);
    let discount = f64::from(
        StrategicDoctrines::bundled()
            .tuning
            .revenge_war_discount_percent,
    ) / 100.0
        * f64::from(active.doctrine.objectives.revenge.min(100))
        / 100.0;
    1.0 - discount
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_catalogue_is_consistent() {
        let c = StrategicDoctrines::bundled();
        assert!(c.doctrines.contains_key(&c.default));
        for a in &c.assignments {
            assert!(c.doctrines.contains_key(&a.doctrine), "{}", a.doctrine);
        }
        let neutral = &c.doctrines[&c.default];
        assert_eq!(neutral.battle.engage_ratio_percent, 100);
        assert_eq!(neutral.objectives.conquest, 50);
    }
}
