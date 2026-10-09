//! Attitude of one faction towards another, with its reasons.

use super::*;
use std::collections::BTreeMap;

impl PlanCache<'_> {
    /// Attitude of `a` towards `b` (-100..100) with its reasons.
    pub fn attitude(
        &self,
        data: &GameData,
        a: &FactionId,
        b: &FactionId,
    ) -> (i32, Vec<(String, i32)>) {
        let state = self.state();
        let mut reasons: Vec<(String, i32)> = Vec::new();
        let mut add = |text: &str, value: i32| {
            if value != 0 {
                reasons.push((text.to_owned(), value));
            }
        };
        let Some(fa) = state.factions.get(a) else {
            return (0, reasons);
        };
        let personality = data
            .factions
            .get(a)
            .and_then(|f| f.ai_personality.as_ref())
            .and_then(|p| p.diplomacy)
            .map_or(0, |d| (i32::from(d) - 50) / 2);
        add("Tempérament diplomatique", personality);
        // F1: a charming (or haughty) ruler on the other side.
        add(
            "Diplomatie de son souverain",
            state.ruler_effect_points(data, b, |e| e[EffectKind::Diplomacy].apply(0.0) * 2.0),
        );
        match state.relation(a, b) {
            RelationKind::War => add(AT_WAR_REASON, -50),
            RelationKind::Truce => add("Trêve récente", -10),
            RelationKind::Alliance => add("Alliés", 30),
            RelationKind::Suzerain => add(
                "Loyauté envers le suzerain",
                (i32::from(fa.loyalty) - 50) / 2,
            ),
            RelationKind::Vassal => add("Notre vassal", 10),
            RelationKind::Peace => {}
        }
        if state.marriage_tie(a, b) {
            add(MARRIAGE_TIE_REASON, 15);
        }
        if let (Some(ha), Some(hb)) = (state.ruler_house(a), state.ruler_house(b)) {
            if ha == hb {
                add(SAME_HOUSE_REASON, 20);
            }
        }
        match religion::faith_relation(state, data, a, b) {
            religion::FaithRelation::Same => add("Même foi", 10),
            religion::FaithRelation::RivalObedience => add("Obédience rivale", -20),
            religion::FaithRelation::Kindred => add("Schismatiques", -25),
            religion::FaithRelation::Different => add("Religion différente", -40),
        }
        if religion::is_excommunicated(state, b) && religion::is_catholic(state, data, a) {
            add("Excommunié", -30);
        }
        let common_enemy = fa
            .at_war_with
            .iter()
            .any(|e| !e.is_rebels() && state.is_at_war(b, e));
        if common_enemy {
            add("Ennemi commun", 20);
        }
        // DF1: the AI's stance towards the player follows the difficulty.
        add(DIFFICULTY_REASON, state.difficulty_attitude(data, a, b));
        let menace = &data.ai_diplomacy.menacing_neighbour;
        if !state.is_allied(a, b)
            && self.faction_power(b) > menace.power_ratio * self.faction_power(a).max(1.0)
            && self.are_neighbors(data, a, b)
        {
            add("Voisin menaçant", menace.attitude);
        }
        // WH `diplob`: the league stands against the hegemon.
        if state.league_target_of(data, a) == Some(b) {
            add(LEAGUE_REASON, data.ai_diplomacy.league.attitude);
        }
        if state.has_non_aggression(a, b) {
            add(
                "Pacte de non-agression",
                data.ai_diplomacy.non_aggression.attitude,
            );
        }
        if let Some(fb) = state.factions.get(b) {
            let claims_on_us = fb.claims.iter().any(|c| match c.kind {
                ClaimKind::Throne => c.faction.as_ref() == Some(a),
                ClaimKind::Province => c
                    .province
                    .as_ref()
                    .is_some_and(|p| state.province_owner(p) == Some(a)),
            });
            if claims_on_us {
                add("Prétentions sur nos terres", -25);
            }
            if fb.embargoes.contains(a) {
                add("Embargo contre nous", -20);
            }
        }
        // LR-07: a capped motive (`opinion_caps`) also weighs at most its
        // cap when read, whatever path wrote its modifiers (events, saves
        // from before RS-C): one line per capped motive.
        let mut capped: Vec<(&str, i32, i32)> = Vec::new();
        for modifier in fa
            .modifiers
            .iter()
            .filter(|m| &m.with == b && m.expires_turn > state.turn)
        {
            let cap = opinion_motive(&modifier.reason_fr)
                .and_then(|m| data.diplomacy_rules.opinion_cap(m));
            match cap {
                Some(cap) => match capped
                    .iter_mut()
                    .find(|(reason, _, _)| *reason == modifier.reason_fr)
                {
                    Some(entry) => entry.1 += modifier.value,
                    None => capped.push((&modifier.reason_fr, modifier.value, cap)),
                },
                None => add(&modifier.reason_fr, modifier.value),
            }
        }
        for (reason, sum, cap) in capped {
            add(reason, sum.clamp(-cap.abs(), cap.abs()));
        }
        let total: i32 = reasons.iter().map(|(_, v)| v).sum();
        (total.clamp(-100, 100), reasons)
    }
}

impl CampaignState {
    /// WH `diploa`: turns left of each running timed reason of `a`'s opinion
    /// of `b` (the longest when a reason repeats; permanent ones and the
    /// structural reasons are absent). Keys are the reason texts of
    /// [`CampaignState::attitude`].
    pub fn reason_turns_left(&self, a: &FactionId, b: &FactionId) -> BTreeMap<String, u32> {
        let state = self;
        let mut left: BTreeMap<String, u32> = BTreeMap::new();
        let Some(fa) = state.factions.get(a) else {
            return left;
        };
        for modifier in fa
            .modifiers
            .iter()
            .filter(|m| &m.with == b && m.expires_turn > state.turn && m.expires_turn != FOREVER)
        {
            let remaining = modifier.expires_turn - state.turn;
            let entry = left.entry(modifier.reason_fr.clone()).or_insert(0);
            *entry = (*entry).max(remaining);
        }
        if let Some(until) = fa.truces.get(b).filter(|u| **u > state.turn) {
            left.insert("Trêve récente".to_owned(), until - state.turn);
        }
        left
    }
}
