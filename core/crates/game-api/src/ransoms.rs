//! Ransom view (`CampaignSim.get_ransoms` of `godot-bridge`): the captives of both sides and the
//! installment debts of the player, for the « Captifs et rançons » window. Amounts, plans and
//! ranks come from `sim_campaign::ransom`; the orders (`PayRansom`, `SetRansomTerms`,
//! `ReleaseOnParole`, `ExecuteCaptive`) go through `diplomacy::submit_order`.

use data_model::{CharacterId, GameData};
use serde::Serialize;
use sim_campaign::{ransom, CampaignState, RansomTerms};

/// One captive (ours, held by a captor; or a prisoner of the player).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Captive {
    pub id: String,
    pub name: String,
    pub faction: String,
    pub captor: String,
    pub rank: String,
    pub ransom: i64,
    /// `money`, `province`, `hold` or `parole`.
    pub terms: &'static str,
    /// Province demanded (terms `province`).
    pub province: Option<String>,
    /// `(installments, total, installment)` for 2..=`MAX_INSTALLMENTS`.
    pub plans: Vec<(u32, i64, i64)>,
}

/// An installment debt of the player.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Debt {
    pub name: String,
    pub remaining: i64,
    pub installment: i64,
    pub next_due_turn: u32,
    pub missed: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct RansomView {
    /// Captives of the player's faction.
    pub ours: Vec<Captive>,
    /// Prisoners held by the player.
    pub held: Vec<Captive>,
    pub debts: Vec<Debt>,
}

impl RansomView {
    pub fn is_empty(&self) -> bool {
        self.ours.is_empty() && self.held.is_empty() && self.debts.is_empty()
    }
}

fn captive(state: &CampaignState, data: &GameData, id: &CharacterId) -> Captive {
    let c = &state.characters[id];
    let amount = ransom::ransom_amount(state, data, id);
    let terms = c.ransom_terms.clone().unwrap_or_default();
    Captive {
        id: id.as_str().to_owned(),
        name: state.character_name(data, id),
        faction: data.faction_name(&c.faction),
        captor: c
            .captor
            .as_ref()
            .map_or_else(String::new, |f| data.faction_name(f)),
        rank: ransom::captive_rank(state, id).label_fr().to_owned(),
        ransom: amount,
        terms: terms.key(),
        province: match &terms {
            RansomTerms::Province { province } => Some(province.as_str().to_owned()),
            _ => None,
        },
        plans: (2..=ransom::MAX_INSTALLMENTS)
            .map(|n| {
                let (total, installment) = ransom::installment_plan(amount, n);
                (n, total, installment)
            })
            .collect(),
    }
}

/// Captives of both sides and debts of the player (`get_ransoms`).
pub fn get_ransoms(state: &CampaignState, data: &GameData) -> RansomView {
    let player = state.player_faction().clone();
    let mut out = RansomView::default();
    for (id, c) in &state.characters {
        if !c.alive || !c.captive {
            continue;
        }
        if c.faction == player {
            out.ours.push(captive(state, data, id));
        } else if c.captor.as_ref() == Some(&player) {
            out.held.push(captive(state, data, id));
        }
    }
    if let Some(f) = state.faction_state(&player) {
        out.debts = f
            .ransom_debts
            .iter()
            .map(|d| Debt {
                name: state.character_name(data, &d.character),
                remaining: d.remaining,
                installment: d.installment,
                next_due_turn: d.next_due_turn,
                missed: d.missed,
            })
            .collect();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quiet_start_has_no_ransoms() {
        let data = data_model::test_support::game_data();
        let state = sim_campaign::test_support::start_quiet(data, "fac_france", 1337);
        assert!(get_ransoms(&state, data).is_empty());
    }

    #[test]
    fn captive_of_the_player_is_listed_with_plans() {
        let data = data_model::test_support::game_data();
        let mut state = sim_campaign::test_support::start_quiet(data, "fac_france", 1337);
        let player = state.player_faction().clone();
        let captor = state
            .factions
            .keys()
            .find(|f| **f != player)
            .cloned()
            .expect("another faction");
        let id = state
            .characters
            .iter()
            .find(|(_, c)| c.alive && c.faction == player)
            .map(|(id, _)| id.clone())
            .expect("a character");
        {
            let c = state.characters.get_mut(&id).unwrap();
            c.captive = true;
            c.captor = Some(captor);
        }
        let view = get_ransoms(&state, data);
        assert_eq!(view.ours.len(), 1);
        assert_eq!(view.ours[0].id, id.as_str());
        assert!(view.ours[0].ransom > 0);
        assert_eq!(
            view.ours[0].plans.len(),
            (ransom::MAX_INSTALLMENTS - 1) as usize
        );
        assert!(view.held.is_empty());
    }
}
