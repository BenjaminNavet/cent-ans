//! Executing one article of a signed treaty.

use data_model::{FactionId, GameData, ProvinceId};

use super::{Article, HostagePledge, Party, TributeDue, HOSTAGE_TURNS};
use crate::diplomacy::{self, DiplomacyError, MARRIAGE_REASON, TRUCE_TURNS};
use crate::events::{EventKind, GameEvent};
use crate::state::CampaignState;

/// The two parties of the treaty being executed.
pub(super) struct Parties<'a> {
    pub proposer: &'a FactionId,
    pub recipient: &'a FactionId,
}

impl Parties<'_> {
    fn id(&self, party: Party) -> &FactionId {
        match party {
            Party::Proposer => self.proposer,
            Party::Recipient => self.recipient,
        }
    }
}

impl Article {
    pub(super) fn apply(
        &self,
        state: &mut CampaignState,
        data: &GameData,
        parties: &Parties,
    ) -> Result<(), DiplomacyError> {
        let (proposer, recipient) = (parties.proposer, parties.recipient);
        let giver = self.giver().map(|g| parties.id(g).clone());
        let taker = self.giver().map(|g| parties.id(g.other()).clone());
        match self {
            Article::Peace => {
                let rules = &data.ai_diplomacy.negotiation;
                let truce = if rules.enabled {
                    rules.peace_truce_turns
                } else {
                    TRUCE_TURNS
                };
                state.make_peace(data, proposer, recipient, &[], 0, truce.max(1));
            }
            Article::Truce { turns } | Article::Mediation { turns } => {
                state.make_peace(data, proposer, recipient, &[], 0, (*turns).max(1));
            }
            Article::Alliance => {
                if !state.is_allied(proposer, recipient) {
                    state.form_alliance(data, proposer, recipient);
                }
            }
            Article::Marriage { character, spouse } => {
                marry(state, data, parties, character, spouse)?;
            }
            Article::Vassalage { .. } => {
                let (vassal, lord) = (giver.expect("giver"), taker.expect("taker"));
                state.make_vassal(data, &lord, &vassal);
            }
            Article::MilitaryAccess { .. } => {
                let (giver, taker) = (giver.expect("giver"), taker.expect("taker"));
                state
                    .factions
                    .get_mut(&giver)
                    .expect("checked")
                    .ledger
                    .military_access
                    .insert(taker);
            }
            Article::TradeAgreement => {
                for (a, b) in [(proposer, recipient), (recipient, proposer)] {
                    state
                        .factions
                        .get_mut(a)
                        .expect("checked")
                        .ledger
                        .trade_agreements
                        .insert(b.clone());
                }
            }
            Article::Tribute {
                per_season,
                seasons,
                ..
            } => {
                let until_turn = state.turn + seasons;
                state
                    .factions
                    .get_mut(&giver.expect("giver"))
                    .expect("checked")
                    .ledger
                    .tributes
                    .push(TributeDue {
                        to: taker.expect("taker"),
                        per_season: *per_season,
                        until_turn,
                    });
            }
            Article::Gold { amount, .. } => {
                state
                    .factions
                    .get_mut(&giver.expect("giver"))
                    .expect("checked")
                    .treasury -= amount;
                state
                    .factions
                    .get_mut(&taker.expect("taker"))
                    .expect("checked")
                    .treasury += amount;
            }
            Article::CedeProvince { province, .. } => {
                cede_province(state, province, &giver.expect("giver"), &taker.expect("t"));
            }
            Article::DemandTitle { title, .. } => {
                crate::feudal::conquer_title(state, data, &taker.expect("taker"), title)
                    .map_err(|e| DiplomacyError::Refused(e.to_string()))?;
            }
            Article::CedeSettlement { settlement, .. } => {
                let to = taker.expect("taker");
                // As `cede_province`: the giver's garrison, recruits and
                // building site do not pass to the taker.
                if let Some(s) = state.settlements.get_mut(settlement) {
                    s.owner = to.clone();
                    s.hand_over(&to);
                    s.garrison.clear();
                }
            }
            Article::ReleaseCaptive { character, .. } => {
                crate::chronicle::release_character(state, data, character, 0, &mut Vec::new());
            }
            Article::Hostage { character, .. } => {
                give_hostage(
                    state,
                    character,
                    giver.expect("giver"),
                    taker.expect("taker"),
                );
            }
            Article::Obedience { religion } => {
                crate::religion::set_obedience(state, data, recipient, religion)?;
            }
            Article::Protection { aggressor } => {
                if state.is_at_war(proposer, aggressor) {
                    crate::feudal::intervene(state, data, recipient, proposer, aggressor);
                }
            }
            Article::Arbitration { attacker, target } => {
                crate::feudal::apply_arbitration(
                    state,
                    data,
                    recipient,
                    attacker,
                    target,
                    &crate::feudal::Arbitration::ImposePeace,
                );
            }
            Article::PeaceSummons { target } => {
                crate::feudal::apply_arbitration(
                    state,
                    data,
                    proposer,
                    recipient,
                    target,
                    &crate::feudal::Arbitration::ImposePeace,
                );
            }
        }
        Ok(())
    }
}

fn marry(
    state: &mut CampaignState,
    data: &GameData,
    parties: &Parties,
    character: &data_model::CharacterId,
    spouse: &data_model::CharacterId,
) -> Result<(), DiplomacyError> {
    crate::dynasty::propose_marriage(state, data, character, spouse)
        .map_err(|e| DiplomacyError::Refused(e.to_string()))?;
    let (proposer, recipient) = (parties.proposer, parties.recipient);
    state.add_capped_modifier(data, proposer, recipient, 15, MARRIAGE_REASON, 80);
    state.add_capped_modifier(data, recipient, proposer, 15, MARRIAGE_REASON, 80);
    let text = format!(
        "Mariage de {} et de {}.",
        state.character_name(data, character),
        state.character_name(data, spouse)
    );
    state.push_order_event(GameEvent::new(EventKind::Marriage, text).faction(proposer));
    Ok(())
}

fn give_hostage(
    state: &mut CampaignState,
    character: &data_model::CharacterId,
    giver: FactionId,
    holder: FactionId,
) {
    if let Some(c) = state.characters.get_mut(character) {
        c.captive = true;
        c.captor = Some(holder.clone());
        c.governor_of = None;
        // ADR 0025 § 6: held for the whole term, not for sale.
        c.ransom_terms = Some(crate::ransom::RansomTerms::Hold);
    }
    let until_turn = state.turn + HOSTAGE_TURNS;
    state
        .factions
        .get_mut(&holder)
        .expect("checked")
        .ledger
        .hostages
        .push(HostagePledge {
            character: character.clone(),
            from: giver,
            until_turn,
        });
}

/// A province passes by treaty from `from` to `to` (every settlement `from`
/// holds there); `from` keeps a claim on it.
fn cede_province(
    state: &mut CampaignState,
    province: &ProvinceId,
    from: &FactionId,
    to: &FactionId,
) {
    state.cede_province(province, Some(from), to);
    for character in state.characters.values_mut() {
        if character.governor_of.as_ref() == Some(province) {
            character.governor_of = None;
        }
    }
    let turn = state.turn;
    if let Some(f) = state.factions.get_mut(from) {
        f.claims.push(diplomacy::Claim {
            kind: data_model::ClaimKind::Province,
            faction: None,
            province: Some(province.clone()),
            text_fr: "province perdue par traité".to_owned(),
            expires_turn: Some(turn + 80),
        });
    }
}
