//! Proposals to the player: offers, their text and the answers.

use super::*;

impl CampaignState {
    /// Sends `treaty`: the player receives an offer, anyone else answers at
    /// once through [`crate::negotiation::propose_treaty`].
    pub fn propose(
        &mut self,
        data: &GameData,
        proposer: &FactionId,
        recipient: &FactionId,
        treaty: Treaty,
    ) -> Result<(), DiplomacyError> {
        self.check_pair(proposer, recipient)?;
        crate::negotiation::check_treaty(self, data, proposer, recipient, &treaty.articles)?;
        if recipient == &self.player_faction && proposer != &self.player_faction {
            self.create_offer(data, proposer, treaty);
            return Ok(());
        }
        crate::negotiation::propose_treaty(self, data, proposer, recipient, treaty.articles)?;
        Ok(())
    }

    pub(super) fn create_offer(&mut self, data: &GameData, from: &FactionId, proposal: Treaty) {
        let player = self.player_faction.clone();
        let recent = self
            .factions
            .get(&player)
            .and_then(|f| f.last_offer_turn.get(from))
            .is_some_and(|t| t + OFFER_COOLDOWN > self.turn);
        let duplicate = self.factions[&player]
            .offers
            .iter()
            .any(|o| &o.from == from && o.proposal == proposal);
        if recent || duplicate || self.alliance_offer_throttled(data, &proposal) {
            return;
        }
        if is_alliance(&proposal) {
            if let Some(f) = self.factions.get_mut(&player) {
                f.last_alliance_offer = Some(self.turn);
            }
        }
        self.push_offer(data, from, proposal);
    }

    /// One alliance offer pending at a time, and `player_alliance_gap`
    /// seasons between two of them, all senders together.
    fn alliance_offer_throttled(&self, data: &GameData, proposal: &Treaty) -> bool {
        if !is_alliance(proposal) {
            return false;
        }
        let Some(player) = self.factions.get(&self.player_faction) else {
            return false;
        };
        let gap = data.ai_diplomacy.ai_pacts.player_alliance_gap;
        player.offers.iter().any(|o| is_alliance(&o.proposal))
            || player
                .last_alliance_offer
                .is_some_and(|t| t + gap > self.turn)
    }

    /// Adds an offer to the player's pending ones, without the cooldown: the
    /// calls of an attacked ally cannot wait (WH `diplob`).
    pub(crate) fn push_offer(&mut self, data: &GameData, from: &FactionId, proposal: Treaty) {
        let player = self.player_faction.clone();
        if proposal.is_ultimatum() {
            let turn = self.turn;
            if let Some(f) = self.factions.get_mut(from) {
                f.ledger.ultimatum_sent.insert(player.clone(), turn);
            }
        }
        let text = offer_text(self, data, from, &proposal);
        let id = self.next_offer_id;
        self.next_offer_id += 1;
        let expires_turn = self.turn + OFFER_LIFETIME + 1;
        let turn = self.turn;
        let player_state = self.factions.get_mut(&player).expect("exists");
        player_state.last_offer_turn.insert(from.clone(), turn);
        player_state.offers.push(Offer {
            id,
            from: from.clone(),
            proposal,
            expires_turn,
            text_fr: text.clone(),
        });
        self.push_order_event(GameEvent::new(EventKind::DiplomaticOffer, text).faction(from));
    }

    /// The player answers an offer.
    pub fn answer_offer(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        offer_id: u32,
        accept: bool,
    ) -> Result<(), DiplomacyError> {
        let index = self
            .factions
            .get(faction)
            .and_then(|f| f.offers.iter().position(|o| o.id == offer_id))
            .ok_or(DiplomacyError::UnknownOffer)?;
        let offer = self.factions[faction].offers[index].clone();
        if accept {
            // A lord's order is executed as it stands; a bargain is checked
            // again against the world of today.
            if !offer.proposal.is_imposed() {
                self.check_pair(&offer.from, faction)?;
            }
            self.factions
                .get_mut(faction)
                .expect("exists")
                .offers
                .remove(index);
            crate::negotiation::apply_treaty(
                self,
                data,
                &offer.from,
                faction,
                &offer.proposal.articles,
            )
        } else {
            self.factions
                .get_mut(faction)
                .expect("exists")
                .offers
                .remove(index);
            if offer.proposal.is_feudal_call() {
                crate::feudal::refuse_feudal_call(self, data, faction, &offer);
            } else if offer.proposal.is_ally_call() {
                self.shirk_ally_call(data, faction, &offer);
            } else if offer.proposal.is_ultimatum() {
                self.ultimatum_declined(data, faction, &offer);
            } else if !offer.proposal.is_obedience() {
                self.add_modifier(&offer.from, faction, -10, "Offre repoussée", 20);
            }
            Ok(())
        }
    }
}

impl CampaignState {
    /// The player refused (or let expire) an ultimatum of `offer.from`: it
    /// takes offence, holds a casus belli and declares war next season
    /// (`plan_diplomacy`).
    pub(crate) fn ultimatum_declined(
        &mut self,
        data: &GameData,
        player: &FactionId,
        offer: &Offer,
    ) {
        let rules = &data.ai_diplomacy.ultimatum;
        let turn = self.turn;
        if let Some(f) = self.factions.get_mut(&offer.from) {
            f.ledger.ultimatum_refused.insert(player.clone(), turn);
        }
        self.add_modifier(
            &offer.from,
            player,
            rules.refused_attitude,
            "Ultimatum refusé",
            rules.refused_duration,
        );
        let text = format!(
            "{} tient votre refus pour une insulte : la guerre menace.",
            data.faction_name(&offer.from)
        );
        self.push_order_event(GameEvent::new(EventKind::Diplomacy, text).faction(&offer.from));
    }
}

fn offer_text(
    state: &CampaignState,
    data: &GameData,
    from: &FactionId,
    proposal: &Treaty,
) -> String {
    if let [only] = proposal.articles.as_slice() {
        if let Some(line) = only.offer_line(state, data, from) {
            return line;
        }
    }
    let player = state.player_faction.clone();
    if proposal.is_ultimatum() {
        return format!(
            "{} vous adresse un ultimatum : {}. Refusez, et ce sera la guerre.",
            data.faction_name(from),
            crate::negotiation::treaty_text(state, data, from, &player, &proposal.articles)
        );
    }
    format!(
        "{} propose un traité. {}",
        data.faction_name(from),
        crate::negotiation::treaty_text(state, data, from, &player, &proposal.articles)
    )
}

/// A military or defensive alliance proposal.
fn is_alliance(proposal: &Treaty) -> bool {
    proposal
        .articles
        .iter()
        .any(|a| matches!(a, Article::Alliance | Article::DefensiveAlliance))
}
