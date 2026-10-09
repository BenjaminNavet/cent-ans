//! War, peace, alliances and vassal ties (orders apply immediately).

use super::*;

impl CampaignState {
    /// Queues an event produced by an order (orders apply immediately; the
    /// event joins the next turn's journal).
    pub(crate) fn push_order_event(&mut self, event: GameEvent) {
        self.pending_events.push(event);
    }

    /// `attacker` declares war on `target` (spec § 2.3), then calls to arms.
    pub fn declare_war(
        &mut self,
        data: &GameData,
        attacker: &FactionId,
        target: &FactionId,
    ) -> Result<(), DiplomacyError> {
        self.check_pair(attacker, target)?;
        if self.is_at_war(attacker, target) {
            return Err(DiplomacyError::AlreadyAtWar);
        }
        let vassal_link = matches!(
            self.relation(attacker, target),
            RelationKind::Vassal | RelationKind::Suzerain
        );
        if self.is_allied(attacker, target) && !vassal_link {
            return Err(DiplomacyError::Allied);
        }
        let truce_broken = self.has_truce(attacker, target);
        let casus_belli = self.casus_belli(data, attacker, target);
        let others: Vec<FactionId> = self
            .factions
            .keys()
            .filter(|f| *f != attacker && !f.is_rebels())
            .cloned()
            .collect();
        let mut motive = casus_belli
            .clone()
            .unwrap_or_else(|| "aucun motif".to_owned());
        if truce_broken {
            motive = "rupture de trêve".to_owned();
            for other in &others {
                self.add_modifier(other, attacker, -40, PERJURY_REASON, 40);
            }
            religion::change_favor(self, attacker, -30);
            self.change_ruler_prestige(attacker, -30);
            let text = format!(
                "{} rompt la trêve qui le liait à {} : parjure.",
                data.faction_name(attacker),
                data.faction_name(target)
            );
            crate::negotiation::record_rupture(
                self,
                attacker,
                target,
                crate::negotiation::Rupture::Perjury,
                &["truce"],
                &text,
            );
        } else if casus_belli.is_none() {
            for other in &others {
                self.add_modifier(other, attacker, -20, AGGRESSION_REASON, 40);
            }
            self.change_ruler_prestige(attacker, -20);
        }
        let excommunicate = religion::is_catholic(self, data, attacker)
            && religion::is_catholic(self, data, target)
            && self.factions[attacker].papal_favor < 10;
        // Attacking the holder of our hostages abandons them (ADR 0025 § 6):
        // our word is broken, not when the holder attacks us.
        let pledged = self.factions[target]
            .ledger
            .hostages
            .iter()
            .any(|h| &h.from == attacker);
        if pledged {
            self.add_modifier(
                target,
                attacker,
                -20,
                crate::negotiation::HOSTAGE_BETRAYAL_REASON,
                60,
            );
        }
        // Break any vassal tie between the two.
        self.cut_vassal_tie(attacker, target);
        self.start_war(attacker, target);
        self.factions
            .get_mut(attacker)
            .expect("checked")
            .last_war_declared = Some(self.turn);
        // FE § 4.3: the target's suzerain is called (or the common lord
        // arbitrates a private war) before the allies answer.
        let feudal_liege = crate::feudal::liege_of(self, data, target);
        let text = format!(
            "{} déclare la guerre à {} ({motive}).",
            data.faction_name(attacker),
            data.faction_name(target)
        );
        self.push_order_event(GameEvent::new(EventKind::WarDeclared, text).faction(attacker));
        if excommunicate {
            religion::excommunicate(self, data, attacker);
        }
        // JR1: a crusade that turns on its own faith loses its fervour.
        crate::crusade::on_war_declared(self, data, attacker, target);
        crate::feudal::escalate_war(self, data, attacker, target);
        self.call_to_arms(data, target, attacker, feudal_liege.as_ref());
        // The attacker summons its own host too (loyal direct vassals follow),
        // and so does the target: its vassals owe it the host, the feudal tie
        // standing for an alliance (ADR 0114).
        crate::feudal::summon_host(self, data, attacker, target);
        crate::feudal::summon_host(self, data, target, attacker);
        Ok(())
    }

    pub(crate) fn change_ruler_prestige(&mut self, faction: &FactionId, delta: i32) {
        if let Some(ruler) = self.factions.get(faction).and_then(|f| f.ruler.clone()) {
            if let Some(c) = self.characters.get_mut(&ruler) {
                c.prestige += delta;
            }
        }
    }

    pub(crate) fn cut_vassal_tie(&mut self, a: &FactionId, b: &FactionId) {
        for (vassal, suzerain) in [(a, b), (b, a)] {
            if self
                .factions
                .get(vassal)
                .is_some_and(|f| f.suzerain.as_ref() == Some(suzerain))
            {
                crate::feudal::release_from_liege(self, vassal);
                let v = self.factions.get_mut(vassal).expect("exists");
                v.allies.remove(suzerain);
                self.factions
                    .get_mut(suzerain)
                    .expect("exists")
                    .allies
                    .remove(vassal);
            }
        }
    }

    pub(crate) fn start_war(&mut self, a: &FactionId, b: &FactionId) {
        let turn = self.turn;
        for (x, y) in [(a, b), (b, a)] {
            let f = self.factions.get_mut(x).expect("exists");
            f.allies.remove(y);
            f.truces.remove(y);
            f.at_war_with.insert(y.clone());
            f.war_started.insert(y.clone(), turn);
            f.war_scores.insert(y.clone(), 0);
        }
    }

    /// The allies of `defender` decide whether to join its war against
    /// `aggressor` (spec § 2.3).
    /// `feudal_liege` (FE § 4.3) already answered as suzerain and is skipped.
    pub(super) fn call_to_arms(
        &mut self,
        data: &GameData,
        defender: &FactionId,
        aggressor: &FactionId,
        feudal_liege: Option<&FactionId>,
    ) {
        let allies: Vec<FactionId> = self.factions[defender].allies.iter().cloned().collect();
        for ally in allies {
            if Some(&ally) == feudal_liege
                || &ally == aggressor
                || !self.factions.get(&ally).is_some_and(|f| f.alive)
                || self.is_at_war(&ally, aggressor)
                || ally.is_rebels()
            {
                continue;
            }
            if self.is_allied(&ally, aggressor) {
                continue; // bound to both sides: stays out
            }
            let joins = answers_call_to_arms(self, data, &ally, defender, aggressor);
            if joins {
                self.start_war(&ally, aggressor);
                let text = format!(
                    "{} répond à l'appel aux armes de {} contre {}.",
                    data.faction_name(&ally),
                    data.faction_name(defender),
                    data.faction_name(aggressor)
                );
                self.push_order_event(GameEvent::new(EventKind::WarDeclared, text).faction(&ally));
            } else {
                self.factions
                    .get_mut(&ally)
                    .expect("exists")
                    .allies
                    .remove(defender);
                self.factions
                    .get_mut(defender)
                    .expect("exists")
                    .allies
                    .remove(&ally);
                if self.factions[&ally].suzerain.as_ref() == Some(defender) {
                    crate::feudal::release_from_liege(self, &ally);
                }
                self.add_modifier(defender, &ally, -30, "A refusé l'appel aux armes", 40);
                let refusal = format!(
                    "{} refuse l'appel aux armes de {} contre {}.",
                    data.faction_name(&ally),
                    data.faction_name(defender),
                    data.faction_name(aggressor)
                );
                crate::negotiation::record_rupture(
                    self,
                    &ally,
                    defender,
                    crate::negotiation::Rupture::RefusedCall,
                    &["alliance"],
                    &refusal,
                );
                let text = format!(
                    "{} refuse de soutenir {} : l'alliance est rompue.",
                    data.faction_name(&ally),
                    data.faction_name(defender)
                );
                self.push_order_event(
                    GameEvent::new(EventKind::AllianceBroken, text).faction(&ally),
                );
            }
        }
    }

    /// Ends the war between `a` and `b`: each listed province passes to the
    /// other party, `tribute` is paid by `b` to `a` (negative: `a` pays),
    /// and a truce of `truce_turns` starts. EQ3 (`negotiation.truce_binds_allies`):
    /// the allies and vassals who joined this war on either side sign the
    /// same truce (Leulinghem 1389 bound Scotland and the allies of both crowns).
    pub(crate) fn make_peace(
        &mut self,
        data: &GameData,
        a: &FactionId,
        b: &FactionId,
        provinces: &[ProvinceId],
        tribute: i64,
        truce_turns: u32,
    ) {
        let bound = if data.ai_diplomacy.negotiation.truce_binds_allies {
            self.cobelligerents(a, b)
        } else {
            Vec::new()
        };
        self.make_peace_between(data, a, b, provinces, tribute, truce_turns);
        for (ally, enemy) in bound {
            if self.is_at_war(&ally, &enemy) {
                self.make_peace_between(data, &ally, &enemy, &[], 0, truce_turns);
            }
        }
    }

    /// EQ3: the (ally or vassal, enemy) pairs bound by a peace between `a`
    /// and `b`: a weaker faction allied to (or vassal of) one side, at war
    /// with the other since this war began or later (it answered the call),
    /// not rebels.
    pub(super) fn cobelligerents(
        &self,
        a: &FactionId,
        b: &FactionId,
    ) -> Vec<(FactionId, FactionId)> {
        let mut pairs = Vec::new();
        for (side, enemy) in [(a, b), (b, a)] {
            let Some(began) = self
                .factions
                .get(side)
                .and_then(|f| f.war_started.get(enemy))
                .copied()
            else {
                continue;
            };
            for (id, f) in &self.factions {
                if id == a || id == b || id.is_rebels() || !f.alive {
                    continue;
                }
                let joined = f.war_started.get(enemy).is_some_and(|t| *t >= began);
                // Only the junior partner follows: a great crown is not bound
                // by the peace of a lesser ally it came to help.
                let junior = self.faction_power(id) < self.faction_power(side);
                let bound =
                    junior && (self.is_allied(id, side) || f.suzerain.as_ref() == Some(side));
                if joined && bound && f.at_war_with.contains(enemy) {
                    pairs.push((id.clone(), enemy.clone()));
                }
            }
        }
        pairs
    }

    /// Ends the war between `a` and `b` alone (see [`Self::make_peace`]).
    pub(crate) fn make_peace_between(
        &mut self,
        data: &GameData,
        a: &FactionId,
        b: &FactionId,
        provinces: &[ProvinceId],
        tribute: i64,
        truce_turns: u32,
    ) {
        self.settle_forfeitures_at_peace(data, a, b); // FE (F3), before war scores clear
                                                      // FE (F1): the side that cedes land or pays remembers its defeat
                                                      // through its direct vassals' loyalty.
        let losses = |x: &FactionId, pays: bool| {
            provinces
                .iter()
                .filter(|p| self.province_owner(p) == Some(x))
                .count()
                + usize::from(pays)
        };
        let (loss_a, loss_b) = (losses(a, tribute < 0), losses(b, tribute > 0));
        if loss_a != loss_b {
            let loser = if loss_a > loss_b { a } else { b };
            crate::feudal::record_liege_defeat(self, data, loser);
        }
        let until = self.turn + truce_turns;
        for (x, y) in [(a, b), (b, a)] {
            let f = self.factions.get_mut(x).expect("exists");
            f.at_war_with.remove(y);
            f.war_scores.remove(y);
            f.war_started.remove(y);
            f.truces.insert(y.clone(), until);
        }
        let mut ceded_names = Vec::new();
        for province in provinces {
            let Some(owner) = self.province_owner(province).cloned() else {
                continue;
            };
            let (from, to) = if &owner == a {
                (a.clone(), b.clone())
            } else if &owner == b {
                (b.clone(), a.clone())
            } else {
                continue;
            };
            // Every settlement `from` owns in the province is ceded.
            self.cede_province(province, Some(&from), &to);
            for character in self.characters.values_mut() {
                if character.governor_of.as_ref() == Some(province) {
                    character.governor_of = None;
                }
            }
            let turn = self.turn;
            self.factions
                .get_mut(&from)
                .expect("exists")
                .claims
                .push(Claim {
                    kind: ClaimKind::Province,
                    faction: None,
                    province: Some(province.clone()),
                    text_fr: "province perdue par traité".to_owned(),
                    expires_turn: Some(turn + 80),
                });
            ceded_names.push(format!(
                "{} à {}",
                data.province_name(province),
                data.faction_name(&to)
            ));
        }
        // Occupied settlements return to their owner.
        for s in self.settlements.values_mut() {
            let between =
                (&s.owner == a && &s.controller == b) || (&s.owner == b && &s.controller == a);
            if between {
                let owner = s.owner.clone();
                s.hand_over(&owner);
            }
            if s.siege.as_ref().is_some_and(|siege| {
                (&siege.attacker == a && &s.controller == b)
                    || (&siege.attacker == b && &s.controller == a)
            }) {
                s.siege = None;
            }
        }
        if tribute != 0 {
            let (payer, payee, amount) = if tribute > 0 {
                (b, a, tribute)
            } else {
                (a, b, -tribute)
            };
            self.factions.get_mut(payer).expect("exists").treasury -= amount;
            self.factions.get_mut(payee).expect("exists").treasury += amount;
        }
        let mut text = format!(
            "Paix entre {} et {} ; trêve de {} ans.",
            data.faction_name(a),
            data.faction_name(b),
            truce_turns / 4
        );
        if !ceded_names.is_empty() {
            text.push_str(&format!(" Cessions : {}.", ceded_names.join(", ")));
        }
        if tribute != 0 {
            text.push_str(&format!(" Tribut : {} livres.", tribute.abs()));
        }
        self.push_order_event(GameEvent::new(EventKind::PeaceSigned, text).faction(a));
    }

    pub(crate) fn form_alliance(&mut self, data: &GameData, a: &FactionId, b: &FactionId) {
        self.factions
            .get_mut(a)
            .expect("exists")
            .allies
            .insert(b.clone());
        self.factions
            .get_mut(b)
            .expect("exists")
            .allies
            .insert(a.clone());
        let text = format!(
            "Alliance conclue entre {} et {}.",
            data.faction_name(a),
            data.faction_name(b)
        );
        self.push_order_event(GameEvent::new(EventKind::AllianceFormed, text).faction(a));
    }

    /// `vassal` pays homage to `suzerain`: the effective liege of its
    /// primary title changes (ADR 0098).
    pub(crate) fn make_vassal(
        &mut self,
        data: &GameData,
        suzerain: &FactionId,
        vassal: &FactionId,
    ) {
        if self.is_at_war(suzerain, vassal) {
            self.make_peace(data, suzerain, vassal, &[], 0, TRUCE_TURNS);
        }
        if !crate::feudal::pay_homage(self, data, vassal, suzerain) {
            return;
        }
        let v = self.factions.get_mut(vassal).expect("exists");
        v.loyalty = data.feudal_rules.loyalty.homage_start;
        // ADR 0114: the feudal tie stands for an alliance.
        crate::feudal::drop_alliance(self, vassal, suzerain);
        let text = format!(
            "{} devient vassal de {}.",
            data.faction_name(vassal),
            data.faction_name(suzerain)
        );
        self.push_order_event(GameEvent::new(EventKind::Vassalage, text).faction(vassal));
    }
}
