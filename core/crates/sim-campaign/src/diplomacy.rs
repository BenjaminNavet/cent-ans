//! Diplomacy (M5 spec § 2.1-2.3): attitude, casus belli, war and peace,
//! alliances and calls to arms, embargoes, vassals, proposals and offers, and
//! the minimal diplomatic AI.
//!
//! Every proposal is judged by [`evaluate`], a pure function shared by the AI
//! and the UI (which shows the verdict and its reasons before sending).
//! Proposals to the player become [`Offer`]s answered with `answer_offer`.

use data_model::EffectKind;
use std::collections::BTreeSet;

use data_model::{ClaimKind, FactionId, GameData, ProvinceId};
use serde::{Deserialize, Serialize};

use crate::events::{EventKind, GameEvent};
use crate::negotiation::{Article, Treaty};
use crate::orders::Order;
use crate::plan_cache::PlanCache;
use crate::religion;
use crate::state::CampaignState;

/// Truce after a negotiated peace (5 years).
pub const TRUCE_TURNS: u32 = 20;
/// Reputation modifier every faction holds against a truce breaker.
pub const PERJURY_REASON: &str = "Parjure : trêve rompue";
/// Reputation modifier every faction holds against an unprovoked attacker.
pub const AGGRESSION_REASON: &str = "Agression sans motif";
/// Attitude reason of two factions at war.
pub const AT_WAR_REASON: &str = "En guerre";
/// Attitude reason of the campaign difficulty (lot DF1), AI towards the player.
pub const DIFFICULTY_REASON: &str = "Niveau de difficulté";
/// Opinion reason of a marriage between two ruling houses.
pub const MARRIAGE_REASON: &str = "Mariage entre nos maisons";
/// Attitude reason of rulers bound by marriage.
pub const MARRIAGE_TIE_REASON: &str = "Liens matrimoniaux";
/// Attitude reason of rulers of the same house.
pub const SAME_HOUSE_REASON: &str = "Même maison régnante";
/// Attitude reasons owed to kinship (EQ6, `war.claim_war_ignores_kinship`).
pub const KINSHIP_REASONS: [&str; 3] = [MARRIAGE_REASON, MARRIAGE_TIE_REASON, SAME_HOUSE_REASON];
/// Reason of the opinion modifier a gift leaves with its recipient.
pub const GIFT_REASON: &str = "Présents diplomatiques";
/// Truce obtained through papal mediation (2 years).
pub const MEDIATION_TRUCE_TURNS: u32 = 8;
/// Turns an offer to the player stays open.
pub const OFFER_LIFETIME: u32 = 2;
/// Minimum turns between two offers of the same AI faction to the player.
pub const OFFER_COOLDOWN: u32 = 4;
/// Income lost by the target of each embargo.
pub const EMBARGO_TARGET_PENALTY: f64 = 0.08;
/// Income lost by the faction imposing each embargo.
pub const EMBARGO_IMPOSER_PENALTY: f64 = 0.03;
/// Cost of a papal mediation, paid to the Papacy.
pub const MEDIATION_COST: i64 = 1000;
/// Papal favour needed to ask for a mediation.
pub const MEDIATION_MIN_FAVOR: u8 = 30;
/// Modifier duration meaning "never expires".
pub const FOREVER: u32 = u32::MAX;

pub const REBELS_FACTION: &str = "fac_rebels";
pub const PAPACY_FACTION: &str = "fac_papacy";

/// RS-C: the capped motive (`data/rules/diplomacy.json`) of an opinion
/// modifier's reason, if any.
pub fn opinion_motive(reason: &str) -> Option<data_model::OpinionMotive> {
    use data_model::OpinionMotive;
    match reason {
        MARRIAGE_REASON => Some(OpinionMotive::Marriage),
        crate::agents::PARLEY_REASON => Some(OpinionMotive::HeraldEmbassy),
        GIFT_REASON => Some(OpinionMotive::Gift),
        crate::negotiation::TREATY_REASON => Some(OpinionMotive::Treaty),
        _ => None,
    }
}

// =========================================================================
// Types
// =========================================================================

/// A claim (casus belli) held by a faction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claim {
    pub kind: ClaimKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faction: Option<FactionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub province: Option<ProvinceId>,
    pub text_fr: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_turn: Option<u32>,
}

/// A remembered event changing how the holder sees `with`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpinionModifier {
    pub with: FactionId,
    pub value: i32,
    pub reason_fr: String,
    pub expires_turn: u32,
}

/// A proposal waiting for the player's answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Offer {
    pub id: u32,
    pub from: FactionId,
    pub proposal: Treaty,
    pub expires_turn: u32,
    pub text_fr: String,
}

/// Relation of a faction with another, as shown by the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationKind {
    War,
    Truce,
    Peace,
    Alliance,
    /// The other faction is our vassal.
    Vassal,
    /// The other faction is our suzerain.
    Suzerain,
}

/// Why a diplomatic order was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DiplomacyError {
    #[error("faction inconnue ou disparue : {0}")]
    UnknownFaction(FactionId),
    #[error("une faction ne peut pas traiter avec elle-même")]
    SelfTarget,
    #[error("déjà en guerre")]
    AlreadyAtWar,
    #[error("pas en guerre avec cette faction")]
    NotAtWar,
    #[error("déjà alliés")]
    AlreadyAllied,
    #[error("rompez d'abord l'alliance ou la vassalité")]
    Allied,
    #[error("pas d'alliance à rompre")]
    NotAllied,
    #[error("cette faction n'est pas votre vassale")]
    NotVassal,
    #[error("{0}")]
    Refused(String),
    #[error("trésor insuffisant")]
    InsufficientFunds,
    #[error("montant invalide")]
    InvalidAmount,
    #[error("offre inconnue ou expirée")]
    UnknownOffer,
    #[error("province non concernée par cette guerre : {0}")]
    InvalidProvince(ProvinceId),
    #[error("faveur pontificale insuffisante")]
    PapalFavorTooLow,
    #[error("seules les factions catholiques peuvent solliciter le pape")]
    NotCatholic,
    #[error("aucun schisme en cours")]
    NoSchism,
    #[error("obédience invalide")]
    InvalidObedience,
    #[error("pas d'accord commercial à rompre")]
    NoTradeAgreement,
    #[error("la faction virtuelle des rebelles ne négocie pas")]
    Rebels,
}

// =========================================================================
// Helpers
// =========================================================================

/// Refusal of an alliance between a suzerain and its direct vassal (ADR 0114).
pub const FEUDAL_TIE_ALLIANCE: &str = "le lien féodal tient déjà lieu d'alliance";

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

    fn ruler_house(&self, faction: &FactionId) -> Option<String> {
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

    fn living_faction(&self, id: &FactionId) -> Result<(), DiplomacyError> {
        if id.is_rebels() {
            return Err(DiplomacyError::Rebels);
        }
        if self.factions.get(id).is_some_and(|f| f.alive) {
            Ok(())
        } else {
            Err(DiplomacyError::UnknownFaction(id.clone()))
        }
    }

    fn check_pair(&self, a: &FactionId, b: &FactionId) -> Result<(), DiplomacyError> {
        if a == b {
            return Err(DiplomacyError::SelfTarget);
        }
        self.living_faction(a)?;
        self.living_faction(b)
    }
}

// =========================================================================
// Actions
// =========================================================================

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
    fn call_to_arms(
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
    fn cobelligerents(&self, a: &FactionId, b: &FactionId) -> Vec<(FactionId, FactionId)> {
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
            // Lot C4: every settlement `from` owns in the province is ceded.
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
    /// primary title changes (lot FE, ADR 0098).
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

    fn create_offer(&mut self, data: &GameData, from: &FactionId, proposal: Treaty) {
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
        if recent || duplicate {
            return;
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
            } else if !offer.proposal.is_obedience() {
                self.add_modifier(&offer.from, faction, -10, "Offre repoussée", 20);
            }
            Ok(())
        }
    }

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

    /// Diplomatic view of every other living faction for `faction`.
    pub fn diplomacy_view(&self, data: &GameData, faction: &FactionId) -> Vec<DiplomacyEntry> {
        self.factions
            .iter()
            .filter(|(id, f)| *id != faction && f.alive && !id.is_rebels())
            .map(|(id, f)| {
                let (attitude, reasons) = self.attitude(data, id, faction);
                let claims = self.factions[faction]
                    .claims
                    .iter()
                    .filter(|c| match c.kind {
                        ClaimKind::Throne => c.faction.as_ref() == Some(id),
                        ClaimKind::Province => c
                            .province
                            .as_ref()
                            .is_some_and(|p| self.province_owner(p) == Some(id)),
                    })
                    .map(|c| c.text_fr.clone())
                    .collect();
                DiplomacyEntry {
                    faction: id.clone(),
                    relation: self.relation(faction, id),
                    attitude,
                    attitude_reasons: reasons,
                    truce_turns_left: self.factions[faction]
                        .truces
                        .get(id)
                        .map_or(0, |until| until.saturating_sub(self.turn)),
                    embargo_by_us: self.factions[faction].embargoes.contains(id),
                    embargo_on_us: f.embargoes.contains(faction),
                    war_score: if self.is_at_war(faction, id) {
                        self.war_score(data, faction, id)
                    } else {
                        0
                    },
                    casus_belli: self.casus_belli(data, faction, id),
                    claims,
                    loyalty: if f.suzerain.as_ref() == Some(faction) {
                        Some(f.loyalty)
                    } else {
                        None
                    },
                    trade_agreement: self.has_trade_agreement(faction, id),
                }
            })
            .collect()
    }
}

/// One line of the diplomacy panel (attitude is theirs towards us).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiplomacyEntry {
    pub faction: FactionId,
    pub relation: RelationKind,
    pub attitude: i32,
    pub attitude_reasons: Vec<(String, i32)>,
    pub truce_turns_left: u32,
    pub embargo_by_us: bool,
    pub embargo_on_us: bool,
    pub war_score: i32,
    pub casus_belli: Option<String>,
    pub claims: Vec<String>,
    pub loyalty: Option<u8>,
    /// Lot C5: a formal trade agreement is in force (erased by war, its
    /// routes suspended by an embargo — see [`CampaignState::has_trade_agreement`]).
    #[serde(default)]
    pub trade_agreement: bool,
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
    format!(
        "{} propose un traité. {}",
        data.faction_name(from),
        crate::negotiation::treaty_text(state, data, from, &player, &proposal.articles)
    )
}

// =========================================================================
// Turn phase: modifiers, offers, vassals, extinct lines
// =========================================================================

/// Phase (after the economy): expiry, vassal tribute, loyalty and rebellion.
pub(crate) fn resolve_diplomacy(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    crate::negotiation::resolve_negotiation(state, data, events);
    let turn = state.turn;
    for f in state.factions.values_mut() {
        f.modifiers.retain(|m| m.expires_turn > turn);
        f.claims.retain(|c| c.expires_turn.is_none_or(|t| t > turn));
        f.truces.retain(|_, until| *until > turn);
    }
    let player = state.player_faction.clone();
    let expired: Vec<Offer> = state
        .factions
        .get(&player)
        .map(|f| {
            f.offers
                .iter()
                .filter(|o| o.expires_turn <= turn + 1)
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    if let Some(f) = state.factions.get_mut(&player) {
        f.offers.retain(|o| o.expires_turn > turn + 1);
    }
    for offer in expired {
        if offer.proposal.is_obedience() {
            continue; // keeping the historical obedience is the default
        }
        if offer.proposal.is_feudal_call() {
            crate::feudal::refuse_feudal_call(state, data, &player, &offer);
            continue;
        }
        events.push(
            GameEvent::new(
                EventKind::DiplomaticOffer,
                format!("L'offre de {} a expiré.", data.faction_name(&offer.from)),
            )
            .faction(&offer.from),
        );
    }

    // Vassals: the suzerains are a view of the titles (lot FE).
    crate::feudal::forget_expired(state);
    crate::feudal::sync_suzerains(state, data);
    let vassals: Vec<(FactionId, FactionId)> = state
        .factions
        .iter()
        .filter(|(_, f)| f.alive)
        .filter_map(|(id, f)| f.suzerain.clone().map(|s| (id.clone(), s)))
        .collect();
    for (vassal, suzerain) in vassals {
        if !state.factions.get(&suzerain).is_some_and(|f| f.alive) {
            crate::feudal::release_from_liege(state, &vassal);
            let v = state.factions.get_mut(&vassal).expect("exists");
            v.allies.remove(&suzerain);
            continue;
        }
        // Tribute to the direct suzerain only (the maxim, spec § 4.1).
        let tribute = crate::feudal::tribute_due(state, data, &vassal)
            .filter(|(liege, _)| liege == &suzerain)
            .map_or(0, |(_, amount)| amount);
        state.factions.get_mut(&vassal).expect("exists").treasury -= tribute;
        state.factions.get_mut(&suzerain).expect("exists").treasury += tribute;
        let target = loyalty_target(state, data, &vassal, &suzerain);
        let v = state.factions.get_mut(&vassal).expect("exists");
        v.loyalty = move_towards(v.loyalty, target, data.feudal_rules.loyalty.drift_per_turn);
        let loyalty = v.loyalty;
        // FE5: AI vassals revolt by their own order when the policy plans it.
        let planned = crate::feudal::policy().planned_revolts && vassal != state.player_faction;
        if !planned
            && loyalty < data.feudal_rules.rebellion_loyalty
            && !state.is_at_war(&vassal, &suzerain)
            && state
                .rng
                .chance_permille(data.feudal_rules.rebellion_permille)
        {
            // FE (F3): the felony case first (FE5: once the tie is cut the
            // rebel no longer holds of its suzerain and no case opened).
            crate::feudal::on_revolt(state, data, &vassal, &suzerain);
            state.cut_vassal_tie(&vassal, &suzerain);
            state.start_war(&vassal, &suzerain);
            events.push(
                GameEvent::new(
                    EventKind::VassalRebellion,
                    format!(
                        "{} se révolte contre son suzerain {} et proclame son indépendance.",
                        data.faction_name(&vassal),
                        data.faction_name(&suzerain)
                    ),
                )
                .faction(&vassal),
            );
        }
    }
    // Events queued by orders during the previous planning phase join now.
    let queued = std::mem::take(&mut state.pending_events);
    events.splice(0..0, queued);
}

fn move_towards(current: u8, target: u8, step: u8) -> u8 {
    if current < target {
        current.saturating_add(step).min(target)
    } else {
        current.saturating_sub(step).max(target)
    }
}

/// Loyalty a vassal drifts towards (0-100), towards its direct suzerain,
/// with the terms of `feudal.json:loyalty` (spec § 4.2).
pub fn loyalty_target(
    state: &CampaignState,
    data: &GameData,
    vassal: &FactionId,
    suzerain: &FactionId,
) -> u8 {
    let (attitude, reasons) = state.attitude(data, vassal, suzerain);
    let loyalty_term: i32 = reasons
        .iter()
        .filter(|(t, _)| t == "Loyauté envers le suzerain")
        .map(|(_, v)| v)
        .sum();
    let weights = &data.feudal_rules.loyalty;
    let mut target = weights.base + (attitude - loyalty_term) / 2;
    if state.faction_power(suzerain) > weights.power_ratio * state.faction_power(vassal) {
        target += weights.power_favourable;
    } else {
        target += weights.power_unfavourable;
    }
    if religion::is_excommunicated(state, suzerain) {
        target += weights.excommunicated_liege;
    }
    // Kin: a marriage between the ruling houses, or the same house.
    let kin = state.marriage_tie(vassal, suzerain)
        || state.marriage_tie(suzerain, vassal)
        || state
            .ruler_house(vassal)
            .is_some_and(|h| state.ruler_house(suzerain) == Some(h));
    if kin {
        target += weights.family_tie;
    }
    let culture = |f: &FactionId| data.factions.get(f).map(|f| &f.culture);
    if culture(vassal).is_some() && culture(vassal) == culture(suzerain) {
        target += weights.shared_culture;
    }
    target += crate::feudal::remembered_loyalty(state, data, vassal, suzerain);
    // F1 `Loyalty`: a loyal vassal ruler, a generous or kind overlord.
    let loyalty = |e: &crate::buildings::EffectTotals| e[EffectKind::Loyalty].apply(0.0);
    target += state.ruler_effect_points(data, vassal, loyalty)
        + state.ruler_effect_points(data, suzerain, loyalty);
    // Embargoed by an enemy of the suzerain: the vassal's trade suffers
    // for its overlord's quarrels (Flanders and English wool, 1336).
    let squeezed = state
        .factions
        .iter()
        .any(|(id, f)| f.alive && f.embargoes.contains(vassal) && state.is_at_war(id, suzerain));
    if squeezed {
        target += weights.embargo_squeeze;
    }
    target.clamp(0, 100) as u8
}

/// When a ruling line dies out, relatives in other factions through a
/// foreign mother gain a claim on the throne; a friendly claimant at peace
/// takes the realm in personal union (it becomes a vassal).
pub(crate) fn on_line_extinct(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    house: &str,
    events: &mut Vec<GameEvent>,
) {
    let claimant = state
        .characters
        .iter()
        .filter(|(_, c)| c.alive && &c.faction != faction)
        .filter(|(_, c)| {
            c.mother
                .as_ref()
                .and_then(|m| state.characters.get(m))
                .is_some_and(|m| m.house == house)
        })
        .min_by_key(|(id, c)| (c.birth_year, (*id).clone()))
        .map(|(id, c)| (id.clone(), c.faction.clone()));
    let Some((character, claimant_faction)) = claimant else {
        return;
    };
    if claimant_faction.is_rebels() {
        return;
    }
    let text = format!(
        "{} hérite d'une prétention au trône de {} par sa mère.",
        state.character_name(data, &character),
        data.faction_name(faction)
    );
    state
        .factions
        .get_mut(&claimant_faction)
        .expect("exists")
        .claims
        .push(Claim {
            kind: ClaimKind::Throne,
            faction: Some(faction.clone()),
            province: None,
            text_fr: text.clone(),
            expires_turn: None,
        });
    events.push(GameEvent::new(EventKind::Diplomacy, text).faction(&claimant_faction));
    let friendly = !state.is_at_war(&claimant_faction, faction)
        && state.attitude(data, faction, &claimant_faction).0 > 50;
    if friendly && state.factions[faction].suzerain.is_none() {
        state.make_vassal(data, &claimant_faction, faction);
        events.push(
            GameEvent::new(
                EventKind::Vassalage,
                format!(
                    "Union personnelle : {} passe sous l'autorité de {}.",
                    data.faction_name(faction),
                    data.faction_name(&claimant_faction)
                ),
            )
            .faction(faction),
        );
    }
}

// =========================================================================
// Diplomatic AI (pure; M5 § 2.3, F4 « guerre de Cent Ans vivante »)
// =========================================================================

/// Score bonus making a claim war preferred to any opportunistic war.
const CLAIM_WAR_PRIORITY: f64 = 1000.0;
/// Peace reluctance of a pretender towards the crown it claims. (Power
/// ratios of pretenders and co-belligerents: `data/ai/diplomacy.json`.)
pub const PRETENDER_PEACE_RELUCTANCE: i32 = 10;
/// Power ratio of an opportunistic war without claim.
pub const OPPORTUNIST_RATIO: f64 = 1.5;
/// Minimum aggression to press a claim / to wage an opportunistic war.
pub const PRETENDER_AGGRESSION: i32 = 45;
pub const OPPORTUNIST_AGGRESSION: i32 = 60;
/// Turns between two war declarations of the same faction.
pub const WAR_REST_TURNS: u32 = 12;
/// Alliances (vassal ties excluded) an AI seeks at most.
pub const MAX_ALLIANCES: usize = 4;
/// War score below which a beaten realm offers what the enemy holds of it.
pub const SURRENDER_WAR_SCORE: i32 = -25;
/// War score below which a vassal deserts its losing suzerain.
pub const DESERTION_WAR_SCORE: i32 = -25;

/// What `a` claims from `b`: the crown, and how many of `b`'s provinces.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ClaimStakes {
    pub throne: bool,
    pub provinces: usize,
}

impl ClaimStakes {
    pub fn any(self) -> bool {
        self.throne || self.provinces > 0
    }
}

/// Claims `a` holds against `b` (F4).
pub fn claim_stakes(state: &CampaignState, a: &FactionId, b: &FactionId) -> ClaimStakes {
    let mut stakes = ClaimStakes::default();
    let Some(fa) = state.factions.get(a) else {
        return stakes;
    };
    let mut seen = BTreeSet::new();
    for claim in &fa.claims {
        match claim.kind {
            ClaimKind::Throne => stakes.throne |= claim.faction.as_ref() == Some(b),
            ClaimKind::Province => {
                if let Some(p) = &claim.province {
                    if state.province_owner(p) == Some(b) && seen.insert(p) {
                        stakes.provinces += 1;
                    }
                }
            }
        }
    }
    stakes
}

/// Provinces `faction` considers rightfully its own: claimed provinces and
/// every province of a crown it claims (F4: an English army lands anywhere
/// in France).
pub fn claimed_provinces(state: &CampaignState, faction: &FactionId) -> BTreeSet<ProvinceId> {
    let Some(f) = state.factions.get(faction) else {
        return BTreeSet::new();
    };
    let thrones: BTreeSet<&FactionId> = f
        .claims
        .iter()
        .filter(|c| c.kind == ClaimKind::Throne)
        .filter_map(|c| c.faction.as_ref())
        .collect();
    let mut provinces: BTreeSet<ProvinceId> =
        f.claims.iter().filter_map(|c| c.province.clone()).collect();
    provinces.extend(
        state
            .provinces
            .keys()
            .filter(|id| {
                state
                    .province_owner(id)
                    .is_some_and(|o| thrones.contains(o))
            })
            .cloned(),
    );
    provinces
}

/// Factions `faction` quarrels with: current enemies, the targets of its
/// claims and the factions claiming its lands.
pub fn rivals(state: &CampaignState, faction: &FactionId) -> BTreeSet<FactionId> {
    let Some(me) = state.factions.get(faction) else {
        return BTreeSet::new();
    };
    state
        .factions
        .iter()
        .filter(|(id, f)| *id != faction && f.alive && !id.is_rebels())
        .filter(|(id, _)| {
            me.at_war_with.contains(*id)
                || claim_stakes(state, faction, id).any()
                || claim_stakes(state, id, faction).any()
        })
        .map(|(id, _)| id.clone())
        .collect()
}

/// Can `faction` afford a new war: no regency, no debt, a season of upkeep
/// in the chest, a surplus and a free ruler (F4 tempo).
pub fn war_ready(state: &CampaignState, faction: &FactionId) -> bool {
    let Some(me) = state.factions.get(faction) else {
        return false;
    };
    let ruler_free = me
        .ruler
        .as_ref()
        .and_then(|r| state.characters.get(r))
        .is_none_or(|r| !r.captive);
    !me.regency
        && ruler_free
        && me.treasury > 0
        && me.treasury >= 2 * me.last_budget.upkeep().max(0)
        && (me.last_budget.income >= me.last_budget.upkeep()
            || me.treasury >= 8 * me.last_budget.upkeep().max(0))
}

/// Power of the enemies `faction` already fights (rebels excluded).
/// Only fronts that press on us count: bordering enemies and enemies
/// holding our provinces (a distant war of religion does not tie armies).
fn enemy_power(cache: &PlanCache, data: &GameData, faction: &FactionId) -> f64 {
    let state = cache.state();
    state.factions.get(faction).map_or(0.0, |f| {
        f.at_war_with
            .iter()
            .filter(|e| !e.is_rebels())
            .filter(|e| {
                cache.are_neighbors(data, faction, e)
                    || state.provinces.keys().any(|id| {
                        state.province_owner(id) == Some(faction) && state.controls_province(e, id)
                    })
            })
            .map(|e| cache.faction_power(e))
            .sum()
    })
}

/// Alliances proper (vassal ties excluded).
fn alliance_count(state: &CampaignState, faction: &FactionId) -> usize {
    state.factions.get(faction).map_or(0, |f| {
        f.allies
            .iter()
            .filter(|a| state.relation(faction, a) == RelationKind::Alliance)
            .count()
    })
}

/// Weariness above which `faction` declares no war of its own (DP1: a
/// pretender to a throne waits less).
pub fn weariness_to_declare(data: &GameData, faction: &crate::state::FactionState) -> u32 {
    let most = data.ai_diplomacy.negotiation.max_weariness_to_declare;
    let pretender = faction.claims.iter().any(|c| c.kind == ClaimKind::Throne);
    if pretender {
        most + 20
    } else {
        most
    }
}

/// Would `ally` answer the call to arms of `defender` attacked by
/// `aggressor` (M5 § 2.3, F4)? Vassals follow a loyal tie, overlords
/// protect their vassals, other allies march unless they resent the
/// defender or are crippled (empty treasury).
pub fn answers_call_to_arms(
    state: &CampaignState,
    data: &GameData,
    ally: &FactionId,
    defender: &FactionId,
    aggressor: &FactionId,
) -> bool {
    let Some(ally_state) = state.factions.get(ally) else {
        return false;
    };
    if ally_state.suzerain.as_ref() == Some(defender) {
        return crate::feudal::answers_host(state, data, ally, defender, aggressor);
    }
    if state
        .factions
        .get(defender)
        .is_some_and(|f| f.suzerain.as_ref() == Some(ally))
    {
        return true;
    }
    if ally == &state.player_faction {
        return true;
    }
    let attitude = state.attitude(data, ally, defender).0;
    // EQ6: an exhausted realm stays out, as a ruined one.
    let exhausted = data.ai_diplomacy.join_war.weary_stay_out
        && data.ai_diplomacy.negotiation.enabled
        && ally_state.ledger.weariness > weariness_to_declare(data, ally_state);
    let crippled = ally_state.treasury < 0 || exhausted;
    let grudge = rivals(state, ally).contains(aggressor);
    !crippled && (attitude > 0 || (grudge && attitude > -20))
}

/// Diplomatic orders of an AI faction for this turn (spec § 2.3, F4).
pub fn plan_diplomacy(cache: &PlanCache, data: &GameData, faction: &FactionId) -> Vec<Order> {
    let state = cache.state();
    let mut orders = Vec::new();
    if faction.is_rebels() {
        return orders;
    }
    let Some(me) = state.factions.get(faction) else {
        return orders;
    };
    let slot = faction
        .as_str()
        .bytes()
        .fold(0u32, |acc, b| acc.wrapping_add(u32::from(b)));
    let turn = state.turn;
    let aggression = data
        .factions
        .get(faction)
        .and_then(|f| f.ai_personality.as_ref())
        .and_then(|p| p.aggression)
        .map_or(50, i32::from);

    // Peace: offer a white peace when we would accept one ourselves; when
    // clearly winning, ask for the occupied provinces; when losing,
    // cede what the enemy holds rather than lose everything (F4).
    let treaties = data.ai_diplomacy.negotiation.enabled;
    if treaties && ((turn + slot).is_multiple_of(2) || cornered(state, data, faction)) {
        orders.extend(crate::negotiation::plan_peace(cache, data, faction));
    }

    plan_alliances(cache, data, faction, slot, &mut orders);

    let rested = me
        .last_war_declared
        .is_none_or(|t| t + WAR_REST_TURNS <= turn);
    // DP1: a weary realm opens no new front; a pretender waits less.
    let weary = treaties && me.ledger.weariness > weariness_to_declare(data, me);
    let able = turn >= 4
        && !weary
        && (war_ready(state, faction) || crate::negotiation::pretender_ready(state, data, faction));
    let ready = able && rested;
    // EQ6: the rest after another declaration does not hold back the war
    // for the main crown claimed.
    let main_first = data.ai_diplomacy.war.main_claim_first;
    let mut declared = false;
    if able && (rested || main_first) && (turn + slot).is_multiple_of(2) {
        if let Some(target) = war_target(cache, data, faction, aggression, rested) {
            orders.push(Order::DeclareWar { target });
            declared = true;
        }
    }
    if ready && !declared && (turn + slot) % 2 == 1 {
        if let Some(target) = ally_war_to_join(cache, data, faction) {
            orders.push(Order::DeclareWar { target });
            declared = true;
        }
    }
    if ready && !declared {
        orders.extend(desert_losing_suzerain(cache, data, faction, slot));
    }

    // Lift pointless embargoes, drop hated allies.
    for target in &me.embargoes {
        if !state.is_at_war(faction, target) && cache.attitude(data, faction, target).0 > 10 {
            orders.push(Order::SetEmbargo {
                target: target.clone(),
                active: false,
            });
        }
    }
    let hated: BTreeSet<FactionId> = me
        .allies
        .iter()
        .filter(|a| state.relation(faction, a) == RelationKind::Alliance)
        .filter(|a| cache.attitude(data, faction, a).0 < -30)
        .cloned()
        .collect();
    for target in hated {
        orders.push(Order::BreakAlliance { target });
    }

    // Excommunicated and rich: buy back the Pope's favour.
    if religion::is_excommunicated(state, faction) && me.treasury > 8000 {
        orders.push(Order::DonateToChurch { amount: 2000 });
    }
    orders
}

/// A crown down to `peace.cornered_provinces` provinces of its own (or
/// fewer) sues for peace every season, whatever the war score (G5: the
/// Scots after Halidon Hill treat rather than vanish).
pub fn is_cornered(state: &CampaignState, data: &GameData, faction: &FactionId) -> bool {
    cornered(state, data, faction)
}

fn cornered(state: &CampaignState, data: &GameData, faction: &FactionId) -> bool {
    let most = data.ai_diplomacy.peace.cornered_provinces;
    most > 0
        && state
            .provinces
            .keys()
            .filter(|id| {
                state.controls_province(faction, id) && state.province_owner(id) == Some(faction)
            })
            .count()
            <= most
}

/// EQ6: the main crown `faction` claims: the living realm with the most
/// provinces among those whose throne it claims (England: France, not a
/// small Italian lordship inherited through a marriage).
pub fn main_claim(state: &CampaignState, faction: &FactionId) -> Option<FactionId> {
    let me = state.factions.get(faction)?;
    let thrones: BTreeSet<&FactionId> = me
        .claims
        .iter()
        .filter(|c| c.kind == ClaimKind::Throne)
        .filter_map(|c| c.faction.as_ref())
        .filter(|f| *f != faction && state.factions.get(*f).is_some_and(|s| s.alive))
        .collect();
    thrones
        .into_iter()
        .map(|f| {
            let size = state
                .provinces
                .keys()
                .filter(|p| state.province_owner(p) == Some(f))
                .count();
            (size, f)
        })
        .max_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.cmp(a.1)))
        .map(|(_, f)| f.clone())
}

/// The war `faction` declares this turn, if any: a pretender presses its
/// claim even against a stronger crown when it has allies or a bridgehead;
/// aggressive realms fall on weaker rivals they hold a casus belli against.
/// When `faction` is not `rested` (it declared another war lately), only
/// its main claim is considered (EQ6, `war.main_claim_first`).
fn war_target(
    cache: &PlanCache,
    data: &GameData,
    faction: &FactionId,
    aggression: i32,
    rested: bool,
) -> Option<FactionId> {
    let state = cache.state();
    let my_power = cache.coalition_power(faction);
    let rules = &data.ai_diplomacy.war;
    // Never a new front while the current wars weigh (DP1: a pretender
    // to a throne tolerates a heavier border war, as Edward III kept
    // fighting the Scots while claiming France).
    let pressing = enemy_power(cache, data, faction);
    let own = cache.faction_power(faction);
    let pretender = data.ai_diplomacy.negotiation.enabled
        && state.factions[faction]
            .claims
            .iter()
            .any(|c| c.kind == ClaimKind::Throne);
    let share = if pretender {
        rules.front_share * 2.0
    } else {
        rules.front_share
    };
    if pressing > share * own {
        return None;
    }
    let has_allies = state.factions[faction]
        .allies
        .iter()
        .any(|a| state.factions.get(a).is_some_and(|f| f.alive));
    let main = if rules.main_claim_first || rules.claim_war_ignores_difficulty {
        main_claim(state, faction)
    } else {
        None
    };
    state
        .factions
        .iter()
        .filter(|(id, f)| {
            *id != faction
                && f.alive
                && !id.is_rebels()
                && id.as_str() != PAPACY_FACTION
                && !state.is_allied(faction, id)
                && !state.is_at_war(faction, id)
                && !state.has_truce(faction, id)
                && (rested || main.as_ref() == Some(*id))
        })
        .filter_map(|(id, _)| {
            let stakes = claim_stakes(state, faction, id);
            let is_main = main.as_ref() == Some(id);
            // EQ6: the main claim war does not depend on the difficulty.
            let neutral = is_main && rules.claim_war_ignores_difficulty;
            // DF1: a harder campaign lowers the odds an AI wants before
            // falling on the player.
            let demand = if neutral {
                1.0
            } else {
                state.difficulty_war_ratio_factor(data, id)
            };
            let attitude = cache.attitude(data, faction, id).0
                - if neutral {
                    state.difficulty_attitude(data, faction, id)
                } else {
                    0
                }
                // EQ6: kinship is the ground of the claim, not a restraint.
                - if is_main && rules.claim_war_ignores_kinship {
                    state.kinship_attitude(data, faction, id)
                } else {
                    0
                };
            // EQ6: the main crown outranks every lesser claim.
            let main_bonus = if is_main && rules.main_claim_first {
                CLAIM_WAR_PRIORITY
            } else {
                0.0
            };
            if stakes.any() && aggression >= PRETENDER_AGGRESSION {
                let ratio = my_power / cache.faction_power(id).max(1.0);
                let supported = has_allies || cache.are_neighbors(data, faction, id);
                let needed = demand
                    * if supported {
                        rules.pretender_ratio
                    } else {
                        rules.pretender_ratio_alone
                    };
                let weight = if stakes.throne { 3.0 } else { 0.0 } + stakes.provinces as f64;
                return (ratio >= needed && attitude < 20)
                    // A claim outranks any opportunistic war.
                    .then(|| (id.clone(), CLAIM_WAR_PRIORITY + main_bonus + weight + ratio));
            }
            if aggression >= OPPORTUNIST_AGGRESSION
                && state.casus_belli(data, faction, id).is_some()
                && cache.attitude(data, faction, id).0 < 0
            {
                let ratio = my_power / cache.coalition_power(id).max(1.0);
                return (ratio >= OPPORTUNIST_RATIO * demand).then(|| (id.clone(), ratio));
            }
            None
        })
        .max_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
        .map(|(id, _)| id)
}

/// An ally's war `faction` joins (co-belligerence, F4): the Low Countries
/// follow Edward III into France, Scotland falls on the English border.
fn ally_war_to_join(cache: &PlanCache, data: &GameData, faction: &FactionId) -> Option<FactionId> {
    let state = cache.state();
    let me = state.factions.get(faction)?;
    let front_share = data.ai_diplomacy.war.front_share;
    let rules = &data.ai_diplomacy.join_war;
    if enemy_power(cache, data, faction) > front_share * cache.faction_power(faction) {
        return None;
    }
    let my_power = cache.faction_power(faction);
    for ally in &me.allies {
        let Some(ally_state) = state.factions.get(ally) else {
            continue;
        };
        if !ally_state.alive
            || cache.attitude(data, faction, ally).0 <= rules.min_attitude
            || cache.faction_power(ally) < rules.min_ally_power_ratio * my_power
        {
            continue;
        }
        for enemy in ally_state.at_war_with.iter().filter(|e| !e.is_rebels()) {
            if enemy == faction
                || state.is_allied(faction, enemy)
                || state.is_at_war(faction, enemy)
                || state.has_truce(faction, enemy)
                || !state.factions.get(enemy).is_some_and(|f| f.alive)
            {
                continue;
            }
            // A border suffices for a war of claims (Scotland falls on the
            // English border while Edward III claims France); a border
            // quarrel of our ally's does not spread along every border.
            let claim_war =
                claim_stakes(state, ally, enemy).any() || claim_stakes(state, enemy, ally).any();
            let border = cache.are_neighbors(data, faction, enemy)
                && (claim_war || !rules.border_only_claim_wars);
            let reachable = border || claim_stakes(state, faction, enemy).any();
            let ratio = cache.coalition_power(faction) / cache.coalition_power(enemy).max(1.0);
            if reachable && ratio >= rules.ratio {
                return Some(enemy.clone());
            }
        }
    }
    None
}

/// Alliances against our rivals (F4): partners sharing a rival, or fearing
/// it more than they like it (the counterweight of distant England for the
/// Low Countries, of France for Scotland).
fn plan_alliances(
    cache: &PlanCache,
    data: &GameData,
    faction: &FactionId,
    slot: u32,
    orders: &mut Vec<Order>,
) {
    let state = cache.state();
    if (state.turn + slot) % 4 != 1 || alliance_count(state, faction) >= MAX_ALLIANCES {
        return;
    }
    let my_rivals = cache.rivals(faction);
    if my_rivals.is_empty() {
        return;
    }
    let candidate = state
        .factions
        .iter()
        .filter(|(id, f)| {
            *id != faction
                && f.alive
                && !id.is_rebels()
                && id.as_str() != PAPACY_FACTION
                && !state.is_allied(faction, id)
                && !state.is_at_war(faction, id)
                && !my_rivals.contains(*id)
                && f.allies.iter().all(|a| !my_rivals.contains(a))
        })
        .filter(|(id, _)| {
            let theirs = cache.rivals(id);
            !theirs.is_disjoint(&my_rivals)
                || my_rivals.iter().any(|r| {
                    let towards_rival = cache.attitude(data, id, r).0;
                    towards_rival < 0 && cache.attitude(data, id, faction).0 > towards_rival + 10
                })
        })
        .map(|(id, _)| (id.clone(), cache.attitude(data, id, faction).0))
        .filter(|(id, _)| cache.attitude(data, faction, id).0 > 0)
        .filter(|(id, _)| {
            id == &state.player_faction
                || crate::negotiation::evaluate_treaty_with(
                    cache,
                    data,
                    faction,
                    id,
                    &[Article::Alliance],
                )
                .accept
        })
        .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)));
    if let Some((target, _)) = candidate {
        orders.push(Order::ProposeAlliance { target });
    }
}

/// An opportunistic vassal (Burgundy) deserts a suzerain that is losing to
/// a stronger coalition: white peace with the winner, then independence.
fn desert_losing_suzerain(
    cache: &PlanCache,
    data: &GameData,
    faction: &FactionId,
    slot: u32,
) -> Vec<Order> {
    let state = cache.state();
    let me = &state.factions[faction];
    let Some(lord) = me.suzerain.clone() else {
        return Vec::new();
    };
    if (state.turn + slot) % 4 != 3 || me.loyalty >= 50 || faction == &state.player_faction {
        return Vec::new();
    }
    let winner = state.factions[&lord]
        .at_war_with
        .iter()
        .filter(|e| !e.is_rebels() && *e != faction)
        .find(|e| {
            state.war_score(data, &lord, e) <= DESERTION_WAR_SCORE
                && cache.coalition_power(e) > cache.coalition_power(&lord)
                && cache.attitude(data, faction, e).0 > -40
        })
        .cloned();
    let Some(winner) = winner else {
        return Vec::new();
    };
    let mut orders = Vec::new();
    if state.is_at_war(faction, &winner) {
        if winner == state.player_faction
            || !crate::negotiation::evaluate_treaty(
                state,
                data,
                faction,
                &winner,
                &[Article::Peace],
            )
            .accept
        {
            return Vec::new();
        }
        orders.push(Order::ProposePeace {
            target: winner.clone(),
            provinces: Vec::new(),
            tribute: 0,
        });
    }
    orders.push(Order::DeclareWar { target: lord });
    orders
}

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
