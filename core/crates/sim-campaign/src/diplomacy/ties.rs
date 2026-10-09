//! Embargoes, broken pacts, released vassals and gifts.

use super::*;

impl CampaignState {
    pub fn set_embargo(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        target: &FactionId,
        active: bool,
    ) -> Result<(), DiplomacyError> {
        self.check_pair(faction, target)?;
        let f = self.factions.get_mut(faction).expect("checked");
        let changed = if active {
            f.embargoes.insert(target.clone())
        } else {
            f.embargoes.remove(target)
        };
        if changed {
            let text = if active {
                format!(
                    "{} impose un embargo commercial à {}.",
                    data.faction_name(faction),
                    data.faction_name(target)
                )
            } else {
                format!(
                    "{} lève son embargo contre {}.",
                    data.faction_name(faction),
                    data.faction_name(target)
                )
            };
            self.push_order_event(GameEvent::new(EventKind::Embargo, text).faction(faction));
        }
        Ok(())
    }

    pub fn break_alliance(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        target: &FactionId,
    ) -> Result<(), DiplomacyError> {
        self.check_pair(faction, target)?;
        if self.relation(faction, target) != RelationKind::Alliance {
            return Err(DiplomacyError::NotAllied);
        }
        self.factions
            .get_mut(faction)
            .expect("checked")
            .allies
            .remove(target);
        self.factions
            .get_mut(target)
            .expect("checked")
            .allies
            .remove(faction);
        self.add_modifier(target, faction, -30, "Trahison de l'alliance", 40);
        let text = format!(
            "{} rompt son alliance avec {}.",
            data.faction_name(faction),
            data.faction_name(target)
        );
        self.push_order_event(GameEvent::new(EventKind::AllianceBroken, text).faction(faction));
        Ok(())
    }

    /// Lot C5: ends a trade agreement concluded by a DP1 treaty article
    /// ([`crate::negotiation::Article::TradeAgreement`]); the player's call,
    /// the AI never breaks one on its own.
    pub fn break_trade_agreement(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        target: &FactionId,
    ) -> Result<(), DiplomacyError> {
        self.check_pair(faction, target)?;
        if !self.has_trade_agreement(faction, target) {
            return Err(DiplomacyError::NoTradeAgreement);
        }
        self.factions
            .get_mut(faction)
            .expect("checked")
            .ledger
            .trade_agreements
            .remove(target);
        self.factions
            .get_mut(target)
            .expect("checked")
            .ledger
            .trade_agreements
            .remove(faction);
        let text = format!(
            "{} rompt son accord commercial avec {}.",
            data.faction_name(faction),
            data.faction_name(target)
        );
        self.push_order_event(GameEvent::new(EventKind::Trade, text).faction(faction));
        Ok(())
    }

    pub fn release_vassal(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        target: &FactionId,
    ) -> Result<(), DiplomacyError> {
        self.check_pair(faction, target)?;
        if self.relation(faction, target) != RelationKind::Vassal {
            return Err(DiplomacyError::NotVassal);
        }
        self.cut_vassal_tie(faction, target);
        self.add_modifier(target, faction, 40, "Indépendance accordée", 80);
        let text = format!(
            "{} rend son indépendance à {}.",
            data.faction_name(faction),
            data.faction_name(target)
        );
        self.push_order_event(GameEvent::new(EventKind::Vassalage, text).faction(faction));
        Ok(())
    }

    pub fn send_gift(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        target: &FactionId,
        amount: i64,
    ) -> Result<(), DiplomacyError> {
        self.check_pair(faction, target)?;
        if amount <= 0 {
            return Err(DiplomacyError::InvalidAmount);
        }
        if self.factions[faction].treasury < amount {
            return Err(DiplomacyError::InsufficientFunds);
        }
        self.factions.get_mut(faction).expect("checked").treasury -= amount;
        self.factions.get_mut(target).expect("checked").treasury += amount;
        let value = ((amount / 100) as i32).clamp(1, 30);
        self.add_capped_modifier(data, target, faction, value, GIFT_REASON, 20);
        let text = format!(
            "{} envoie {amount} livres de présents à {}.",
            data.faction_name(faction),
            data.faction_name(target)
        );
        self.push_order_event(GameEvent::new(EventKind::Diplomacy, text).faction(faction));
        Ok(())
    }

    /// Income multiplier from embargoes (spec § 2.3).
    pub fn embargo_income_factor(&self, faction: &FactionId) -> f64 {
        let imposed = self.factions.get(faction).map_or(0, |f| f.embargoes.len());
        let suffered = self
            .factions
            .iter()
            .filter(|(id, f)| *id != faction && f.alive && f.embargoes.contains(faction))
            .count();
        (1.0 - EMBARGO_TARGET_PENALTY * suffered as f64 - EMBARGO_IMPOSER_PENALTY * imposed as f64)
            .max(0.5)
    }
}
