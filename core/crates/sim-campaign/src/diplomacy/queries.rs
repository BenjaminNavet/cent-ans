//! Pure queries and opinion modifiers on the campaign state.

use super::*;

impl CampaignState {
    /// Military weight of a faction: field armies count fully, garrisons half.
    pub fn faction_power(&self, faction: &FactionId) -> f64 {
        let armies: u32 = self
            .armies
            .values()
            .filter(|a| &a.faction == faction)
            .flat_map(|a| a.units.iter())
            .map(|u| u.strength)
            .sum();
        let garrisons: u32 = self
            .settlements
            .values()
            .filter(|s| &s.controller == faction)
            .map(|s| s.garrison_strength())
            .sum();
        f64::from(armies) + f64::from(garrisons) / 2.0
    }

    /// Power of `faction` plus that of its coalition
    /// ([`Self::coalition_members`]: the feudal tie stands for an alliance,
    /// ADR 0114).
    pub fn coalition_power(&self, faction: &FactionId) -> f64 {
        self.faction_power(faction)
            + self
                .coalition_members(faction)
                .iter()
                .map(|a| self.faction_power(a))
                .sum::<f64>()
    }

    /// Living allies, direct suzerain and direct vassals of `faction` (its
    /// coalition, itself left out; ADR 0114). The feudal ties are read from
    /// the `suzerain` cache refreshed every turn by the feudal pass.
    pub fn coalition_members(&self, faction: &FactionId) -> BTreeSet<FactionId> {
        let mut members: BTreeSet<FactionId> = self
            .factions
            .get(faction)
            .map(|f| f.allies.iter().chain(f.suzerain.iter()).cloned().collect())
            .unwrap_or_default();
        members.extend(
            self.factions
                .iter()
                .filter(|(_, f)| f.suzerain.as_ref() == Some(faction))
                .map(|(id, _)| id.clone()),
        );
        members.remove(faction);
        members.retain(|a| self.factions.get(a).is_some_and(|f| f.alive));
        members
    }

    /// `true` while a truce between `a` and `b` is running.
    pub fn has_truce(&self, a: &FactionId, b: &FactionId) -> bool {
        self.factions
            .get(a)
            .and_then(|f| f.truces.get(b))
            .is_some_and(|until| *until > self.turn)
    }

    /// `true` while a non-aggression pact between `a` and `b` runs (WH
    /// `diplob`).
    pub fn has_non_aggression(&self, a: &FactionId, b: &FactionId) -> bool {
        self.factions
            .get(a)
            .and_then(|f| f.ledger.non_aggression.get(b))
            .is_some_and(|until| *until > self.turn)
    }

    /// `true` when `a` and `b` are allied by a defensive alliance only (WH
    /// `diplob`): they answer each other's calls to arms but do not follow
    /// each other's offensive wars.
    pub fn is_defensive_alliance(&self, a: &FactionId, b: &FactionId) -> bool {
        self.factions
            .get(a)
            .is_some_and(|f| f.allies.contains(b) && f.ledger.defensive_allies.contains(b))
    }

    /// `true` when `a` and `b` control provinces bordering on the map (land
    /// borders as armies walk them, [`crate::movement::land_neighbors`]: the
    /// geometry graph, not the province files' partial `neighbors`).
    pub fn are_neighbors(&self, data: &GameData, a: &FactionId, b: &FactionId) -> bool {
        // OM I1: called for most pairs of factions each turn (O(F² × P)); read
        // each province's city directly instead of looking the province up
        // again, and compare the controller before walking the neighbours.
        self.provinces.iter().any(|(id, province)| {
            self.settlements
                .get(&province.city)
                .is_some_and(|city| &city.controller == a)
                && crate::movement::land_neighbors(data, id)
                    .iter()
                    .any(|n| self.controls_province(b, n))
        })
    }

    /// Every `b` for which [`Self::are_neighbors`]`(data, a, b)` holds, in one
    /// pass over the provinces (OM I1: loops over all factions call this
    /// once instead of `are_neighbors` for each pair). May contain `a`.
    pub fn neighbour_factions(&self, data: &GameData, a: &FactionId) -> BTreeSet<FactionId> {
        let mut out = BTreeSet::new();
        for (id, province) in &self.provinces {
            if self
                .settlements
                .get(&province.city)
                .is_none_or(|city| &city.controller != a)
            {
                continue;
            }
            for n in crate::movement::land_neighbors(data, id) {
                if let Some(controller) = self.province_controller(n) {
                    if !out.contains(controller) {
                        out.insert(controller.clone());
                    }
                }
            }
        }
        out
    }

    /// Relation of `a` with `b`.
    pub fn relation(&self, a: &FactionId, b: &FactionId) -> RelationKind {
        let Some(fa) = self.factions.get(a) else {
            return RelationKind::Peace;
        };
        if fa.at_war_with.contains(b) {
            return RelationKind::War;
        }
        if fa.suzerain.as_ref() == Some(b) {
            return RelationKind::Suzerain;
        }
        if self
            .factions
            .get(b)
            .is_some_and(|fb| fb.suzerain.as_ref() == Some(a))
        {
            return RelationKind::Vassal;
        }
        if fa.allies.contains(b) {
            return RelationKind::Alliance;
        }
        if self.has_truce(a, b) {
            return RelationKind::Truce;
        }
        RelationKind::Peace
    }

    pub(crate) fn ruler_house(&self, faction: &FactionId) -> Option<String> {
        let ruler = self.factions.get(faction)?.ruler.as_ref()?;
        self.characters.get(ruler).map(|c| c.house.clone())
    }

    /// `true` when a living member of `a`'s ruling house is married to a
    /// living member of `b`'s ruling house.
    pub fn marriage_tie(&self, a: &FactionId, b: &FactionId) -> bool {
        let (Some(house_a), Some(house_b)) = (self.ruler_house(a), self.ruler_house(b)) else {
            return false;
        };
        self.characters.values().any(|c| {
            c.alive
                && &c.faction == a
                && c.house == house_a
                && c.spouse
                    .as_ref()
                    .and_then(|s| self.characters.get(s))
                    .is_some_and(|s| s.alive && &s.faction == b && s.house == house_b)
        })
    }

    /// Full war score of `a` against `b` (-100..100): battles plus the
    /// provinces each side occupies of the other.
    pub fn war_score(&self, data: &GameData, a: &FactionId, b: &FactionId) -> i32 {
        let battles = self
            .factions
            .get(a)
            .and_then(|f| f.war_scores.get(b))
            .copied()
            .unwrap_or(0);
        let occupation = |taker: &FactionId, loser: &FactionId| -> i32 {
            let capital = self.factions.get(loser).map(|f| f.capital.clone());
            self.provinces
                .keys()
                .filter(|id| {
                    self.province_owner(id) == Some(loser) && self.controls_province(taker, id)
                })
                .map(|id| if Some(id) == capital.as_ref() { 28 } else { 8 })
                .sum()
        };
        let goals = crate::negotiation::goal_war_score(self, data, a, b);
        (battles + occupation(a, b) - occupation(b, a) + goals).clamp(-100, 100)
    }

    /// The casus belli `a` holds against `b`, if any (French label).
    pub fn casus_belli(&self, data: &GameData, a: &FactionId, b: &FactionId) -> Option<String> {
        let fa = self.factions.get(a)?;
        if crate::feudal::has_forfeiture(self, a, b) {
            return Some("commise".to_owned()); // FE (F3)
        }
        // WH `diplob`: a refused ultimatum is the casus belli of its sender.
        if fa.ledger.ultimatum_refused.contains_key(b) {
            return Some("ultimatum refusé".to_owned());
        }
        for claim in &fa.claims {
            match claim.kind {
                ClaimKind::Throne if claim.faction.as_ref() == Some(b) => {
                    return Some(format!("prétention au trône ({})", claim.text_fr));
                }
                ClaimKind::Province => {
                    if let Some(p) = &claim.province {
                        if self.province_owner(p) == Some(b) {
                            return Some(format!("prétention sur {}", data.province_name(p)));
                        }
                    }
                }
                ClaimKind::Throne => {}
            }
        }
        if self
            .factions
            .get(b)
            .is_some_and(|fb| fb.embargoes.contains(a))
        {
            return Some("embargo".to_owned());
        }
        if fa.allies.iter().any(|ally| self.is_at_war(ally, b)) {
            return Some("défense d'un allié".to_owned());
        }
        // WH `diplob`: nobody needs another motive against the hegemon.
        if self.league_target_of(data, a) == Some(b) {
            return Some(crate::diplomacy::LEAGUE_CASUS_BELLI.to_owned());
        }
        // DP2: armies camping on our lands without right of passage.
        if crate::passage::has_grievance(self, a, b) {
            return Some("violation de nos frontières".to_owned());
        }
        if religion::faith_relation(self, data, a, b) == religion::FaithRelation::Different {
            return Some("guerre de religion".to_owned());
        }
        None
    }

    /// Value of one effect of `faction`'s living ruler (traits and skills),
    /// rounded and bounded to ±20 (F1 `Diplomacy`, `Loyalty`).
    pub fn ruler_effect_points(
        &self,
        data: &GameData,
        faction: &FactionId,
        pick: impl Fn(&crate::buildings::EffectTotals) -> f64,
    ) -> i32 {
        let Some(ruler) = self
            .factions
            .get(faction)
            .and_then(|f| f.ruler.as_ref())
            .filter(|r| self.characters.get(*r).is_some_and(|c| c.alive))
        else {
            return 0;
        };
        pick(&crate::skills::character_effects(self, data, ruler))
            .round()
            .clamp(-20.0, 20.0) as i32
    }

    /// Attitude of `a` towards `b` (-100..100) with its reasons.
    pub fn attitude(
        &self,
        data: &GameData,
        a: &FactionId,
        b: &FactionId,
    ) -> (i32, Vec<(String, i32)>) {
        PlanCache::new(self).attitude(data, a, b)
    }

    /// Part of the attitude of `a` towards `b` owed to kinship: marriage
    /// ties, a shared ruling house and the goodwill of marriages between
    /// the two houses (EQ6, `war.claim_war_ignores_kinship`).
    pub fn kinship_attitude(&self, data: &GameData, a: &FactionId, b: &FactionId) -> i32 {
        self.attitude(data, a, b)
            .1
            .iter()
            .filter(|(reason, _)| KINSHIP_REASONS.contains(&reason.as_str()))
            .map(|(_, value)| value)
            .sum()
    }

    /// Adds an opinion modifier held by `holder` about `with`.
    pub(crate) fn add_modifier(
        &mut self,
        holder: &FactionId,
        with: &FactionId,
        value: i32,
        reason: &str,
        duration: u32,
    ) {
        let expires_turn = if duration == FOREVER {
            FOREVER
        } else {
            self.turn.saturating_add(duration)
        };
        if let Some(f) = self.factions.get_mut(holder) {
            f.modifiers.push(OpinionModifier {
                with: with.clone(),
                value,
                reason_fr: reason.to_owned(),
                expires_turn,
            });
        }
    }

    /// RS-C: adds an opinion modifier of a capped motive
    /// (`data/rules/diplomacy.json` `opinion_caps`, looked up from `reason`
    /// by [`opinion_motive`]). The running modifiers of the same motive held
    /// by `holder` about `with` never total more than the cap (in absolute
    /// value): the new one is cut to what is left, and when nothing is left
    /// it only renews the running ones up to its own expiry.
    pub(crate) fn add_capped_modifier(
        &mut self,
        data: &GameData,
        holder: &FactionId,
        with: &FactionId,
        value: i32,
        reason: &str,
        duration: u32,
    ) {
        let Some(cap) = opinion_motive(reason).and_then(|m| data.diplomacy_rules.opinion_cap(m))
        else {
            self.add_modifier(holder, with, value, reason, duration);
            return;
        };
        let turn = self.turn;
        let expires_turn = if duration == FOREVER {
            FOREVER
        } else {
            turn.saturating_add(duration)
        };
        let Some(f) = self.factions.get_mut(holder) else {
            return;
        };
        let running: i32 = f
            .modifiers
            .iter()
            .filter(|m| &m.with == with && m.reason_fr == reason && m.expires_turn > turn)
            .map(|m| m.value)
            .sum();
        let room = if value >= 0 {
            (cap - running).clamp(0, value)
        } else {
            (-cap - running).clamp(value, 0)
        };
        if room == 0 {
            for m in f
                .modifiers
                .iter_mut()
                .filter(|m| &m.with == with && m.reason_fr == reason && m.expires_turn > turn)
            {
                m.expires_turn = m.expires_turn.max(expires_turn);
            }
            return;
        }
        f.modifiers.push(OpinionModifier {
            with: with.clone(),
            value: room,
            reason_fr: reason.to_owned(),
            expires_turn,
        });
    }

    /// Records a battle between two factions in their war scores.
    pub(crate) fn record_battle(&mut self, winner: &FactionId, loser: &FactionId, decisive: bool) {
        let points = if decisive { 10 } else { 5 };
        if let Some(f) = self.factions.get_mut(winner) {
            let entry = f.war_scores.entry(loser.clone()).or_insert(0);
            *entry = (*entry + points).clamp(-100, 100);
        }
        if let Some(f) = self.factions.get_mut(loser) {
            let entry = f.war_scores.entry(winner.clone()).or_insert(0);
            *entry = (*entry - points).clamp(-100, 100);
        }
    }

    pub(super) fn living_faction(&self, id: &FactionId) -> Result<(), DiplomacyError> {
        if id.is_rebels() {
            return Err(DiplomacyError::Rebels);
        }
        if self.factions.get(id).is_some_and(|f| f.alive) {
            Ok(())
        } else {
            Err(DiplomacyError::UnknownFaction(id.clone()))
        }
    }

    pub(super) fn check_pair(&self, a: &FactionId, b: &FactionId) -> Result<(), DiplomacyError> {
        if a == b {
            return Err(DiplomacyError::SelfTarget);
        }
        self.living_faction(a)?;
        self.living_faction(b)
    }
}
