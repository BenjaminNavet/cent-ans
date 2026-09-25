//! Diplomacy (M5 spec § 2.1-2.3): attitude, casus belli, war and peace,
//! alliances and calls to arms, embargoes, vassals, proposals and offers, and
//! the minimal diplomatic AI.
//!
//! Every proposal is judged by [`evaluate`], a pure function shared by the AI
//! and the UI (which shows the verdict and its reasons before sending).
//! Proposals to the player become [`Offer`]s answered with `answer_offer`.

use std::collections::BTreeSet;

use data_model::{CharacterId, ClaimKind, FactionId, GameData, ProvinceId};
use serde::{Deserialize, Serialize};

use crate::events::{EventKind, GameEvent};
use crate::orders::Order;
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
/// Reason of the opinion modifier a gift leaves with its recipient.
pub const GIFT_REASON: &str = "Présents diplomatiques";
/// Truce obtained through papal mediation (2 years).
pub const MEDIATION_TRUCE_TURNS: u32 = 8;
/// Turns an offer to the player stays open.
pub const OFFER_LIFETIME: u32 = 2;
/// Minimum turns between two offers of the same AI faction to the player.
pub const OFFER_COOLDOWN: u32 = 4;
/// Share of a vassal's income paid to its suzerain each turn.
pub const VASSAL_TRIBUTE_PERCENT: i64 = 10;
/// Vassals below this loyalty may rebel.
pub const REBELLION_LOYALTY: u8 = 20;
/// Vassals below this loyalty ignore their suzerain's calls to arms.
pub const CALL_TO_ARMS_LOYALTY: u8 = 30;
/// Chance per turn (‰) that a disloyal vassal rebels.
pub const REBELLION_PERMILLE: u32 = 250;
/// Income lost by the target of each embargo.
pub const EMBARGO_TARGET_PENALTY: f64 = 0.08;
/// Income lost by the faction imposing each embargo.
pub const EMBARGO_IMPOSER_PENALTY: f64 = 0.03;
/// Cost of a papal mediation, paid to the Papacy.
pub const MEDIATION_COST: i64 = 1000;
/// Papal favour needed to ask for a mediation.
pub const MEDIATION_MIN_FAVOR: u8 = 30;
/// Minimum power ratio to demand vassalage.
pub const VASSALAGE_POWER_RATIO: f64 = 3.0;
/// Modifier duration meaning "never expires".
pub const FOREVER: u32 = u32::MAX;

pub const REBELS_FACTION: &str = "fac_rebels";
pub const PAPACY_FACTION: &str = "fac_papacy";

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

/// Something one faction proposes to another.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Proposal {
    /// Peace: each listed province passes to the other party; `tribute` is
    /// paid by the recipient to the proposer (negative: the proposer pays).
    Peace {
        #[serde(default)]
        provinces: Vec<ProvinceId>,
        #[serde(default)]
        tribute: i64,
    },
    /// White peace with a short truce (papal mediation).
    Truce {
        turns: u32,
    },
    Alliance,
    /// The recipient becomes the proposer's vassal.
    Vassalage,
    /// `character` (proposer's) marries `spouse` (recipient's).
    Marriage {
        character: CharacterId,
        spouse: CharacterId,
    },
    /// Great Schism: the recipient switches to `religion`.
    Obedience {
        religion: data_model::ReligionId,
    },
    /// Lot DP1: a treaty of several articles.
    Treaty {
        articles: Vec<crate::negotiation::Article>,
    },
}

/// A proposal waiting for the player's answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Offer {
    pub id: u32,
    pub from: FactionId,
    pub proposal: Proposal,
    pub expires_turn: u32,
    pub text_fr: String,
}

/// Verdict on a proposal, with the reasons behind it (French).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evaluation {
    pub accept: bool,
    pub score: i32,
    pub reasons: Vec<(String, i32)>,
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

pub(crate) fn faction_name(data: &GameData, id: &FactionId) -> String {
    data.factions
        .get(id)
        .map_or_else(|| id.to_string(), |f| f.short_or_display_name().to_owned())
}

fn province_name(data: &GameData, id: &ProvinceId) -> String {
    data.provinces
        .get(id)
        .map_or_else(|| id.to_string(), |p| p.name.display.clone())
}

pub(crate) fn is_rebels(id: &FactionId) -> bool {
    id.as_str() == REBELS_FACTION
}

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

    /// Power of `faction` plus that of its allies.
    pub fn coalition_power(&self, faction: &FactionId) -> f64 {
        let allies = self
            .factions
            .get(faction)
            .map(|f| f.allies.clone())
            .unwrap_or_default();
        self.faction_power(faction)
            + allies
                .iter()
                .filter(|a| self.factions.get(*a).is_some_and(|f| f.alive))
                .map(|a| self.faction_power(a))
                .sum::<f64>()
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
        self.provinces.keys().any(|id| {
            self.controls_province(a, id)
                && crate::movement::land_neighbors(data, id)
                    .iter()
                    .any(|n| self.controls_province(b, n))
        })
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
        for claim in &fa.claims {
            match claim.kind {
                ClaimKind::Throne if claim.faction.as_ref() == Some(b) => {
                    return Some(format!("prétention au trône ({})", claim.text_fr));
                }
                ClaimKind::Province => {
                    if let Some(p) = &claim.province {
                        if self.province_owner(p) == Some(b) {
                            return Some(format!("prétention sur {}", province_name(data, p)));
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
        let mut reasons: Vec<(String, i32)> = Vec::new();
        let mut add = |text: &str, value: i32| {
            if value != 0 {
                reasons.push((text.to_owned(), value));
            }
        };
        let Some(fa) = self.factions.get(a) else {
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
            self.ruler_effect_points(data, b, |e| e.diplomacy.apply(0.0) * 2.0),
        );
        match self.relation(a, b) {
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
        if self.marriage_tie(a, b) {
            add("Liens matrimoniaux", 15);
        }
        if let (Some(ha), Some(hb)) = (self.ruler_house(a), self.ruler_house(b)) {
            if ha == hb {
                add("Même maison régnante", 20);
            }
        }
        match religion::faith_relation(self, data, a, b) {
            religion::FaithRelation::Same => add("Même foi", 10),
            religion::FaithRelation::RivalObedience => add("Obédience rivale", -20),
            religion::FaithRelation::Different => add("Religion différente", -40),
        }
        if religion::is_excommunicated(self, b) && religion::is_catholic(self, data, a) {
            add("Excommunié", -30);
        }
        let common_enemy = fa
            .at_war_with
            .iter()
            .any(|e| !is_rebels(e) && self.is_at_war(b, e));
        if common_enemy {
            add("Ennemi commun", 20);
        }
        // DF1: the AI's stance towards the player follows the difficulty.
        add(DIFFICULTY_REASON, self.difficulty_attitude(data, a, b));
        let menace = &data.ai_diplomacy.menacing_neighbour;
        if !self.is_allied(a, b)
            && self.faction_power(b) > menace.power_ratio * self.faction_power(a).max(1.0)
            && self.are_neighbors(data, a, b)
        {
            add("Voisin menaçant", menace.attitude);
        }
        if let Some(fb) = self.factions.get(b) {
            let claims_on_us = fb.claims.iter().any(|c| match c.kind {
                ClaimKind::Throne => c.faction.as_ref() == Some(a),
                ClaimKind::Province => c
                    .province
                    .as_ref()
                    .is_some_and(|p| self.province_owner(p) == Some(a)),
            });
            if claims_on_us {
                add("Prétentions sur nos terres", -25);
            }
            if fb.embargoes.contains(a) {
                add("Embargo contre nous", -20);
            }
        }
        for modifier in fa
            .modifiers
            .iter()
            .filter(|m| &m.with == b && m.expires_turn > self.turn)
        {
            add(&modifier.reason_fr, modifier.value);
        }
        let total: i32 = reasons.iter().map(|(_, v)| v).sum();
        (total.clamp(-100, 100), reasons)
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
        if is_rebels(id) {
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
// Evaluation (pure)
// =========================================================================

/// Would `recipient` accept `proposal` from `proposer`? Pure; used by the AI
/// and shown to the player before sending.
pub fn evaluate(
    state: &CampaignState,
    data: &GameData,
    proposer: &FactionId,
    recipient: &FactionId,
    proposal: &Proposal,
) -> Evaluation {
    let mut reasons: Vec<(String, i32)> = Vec::new();
    let mut hard_no = false;
    let (attitude, _) = state.attitude(data, recipient, proposer);
    let aggression = data
        .factions
        .get(recipient)
        .and_then(|f| f.ai_personality.as_ref())
        .and_then(|p| p.aggression)
        .map_or(50, i32::from);
    match proposal {
        Proposal::Peace { provinces, tribute } => {
            if !state.is_at_war(proposer, recipient) {
                reasons.push(("Pas en guerre".to_owned(), -100));
                hard_no = true;
            }
            peace_reasons(
                state,
                data,
                proposer,
                recipient,
                attitude,
                aggression,
                &mut reasons,
            );
            let capital = state.factions.get(recipient).map(|f| f.capital.clone());
            for province in provinces {
                let Some(owner) = state.province_owner(province) else {
                    continue;
                };
                if owner == recipient {
                    let cost = if Some(province) == capital.as_ref() {
                        60
                    } else {
                        20
                    };
                    reasons.push((
                        format!("Cession de {}", province_name(data, province)),
                        -cost,
                    ));
                } else if owner == proposer {
                    reasons.push((
                        format!("Obtention de {}", province_name(data, province)),
                        15,
                    ));
                }
            }
            // F4: a beaten realm that gives up everything we hold of it
            // settles the war score.
            let conquests: Vec<&ProvinceId> = state
                .provinces
                .keys()
                .filter(|id| {
                    state.province_owner(id) == Some(proposer)
                        && state.controls_province(recipient, id)
                })
                .collect();
            // G5: a crown keeping its capital (or its last land, the capital
            // being lost) under `peace.keep_capital` still settles the war.
            let proposer_capital = state.factions.get(proposer).map(|f| f.capital.clone());
            let missing: Vec<&&ProvinceId> = conquests
                .iter()
                .filter(|c| !provinces.contains(**c))
                .collect();
            let kept_land = data.ai_diplomacy.peace.keep_capital
                && missing.len() == 1
                && (proposer_capital.as_ref() == Some(*missing[0])
                    || !state
                        .provinces
                        .keys()
                        .any(|id| state.controls_province(proposer, id)));
            if !conquests.is_empty() && (missing.is_empty() || kept_land) {
                let score = state.war_score(data, recipient, proposer);
                if score > 0 {
                    reasons.push(("Conquêtes reconnues".to_owned(), score));
                }
            }
            if *tribute > 0 {
                reasons.push((
                    "Tribut exigé".to_owned(),
                    -((*tribute / 400) as i32).min(60),
                ));
            } else if *tribute < 0 {
                reasons.push((
                    "Tribut offert".to_owned(),
                    ((-*tribute / 400) as i32).min(60),
                ));
            }
        }
        Proposal::Truce { .. } => {
            if !state.is_at_war(proposer, recipient) {
                reasons.push(("Pas en guerre".to_owned(), -100));
                hard_no = true;
            }
            peace_reasons(
                state,
                data,
                proposer,
                recipient,
                attitude,
                aggression,
                &mut reasons,
            );
            reasons.push(("Médiation pontificale".to_owned(), 30));
        }
        Proposal::Alliance => {
            if state.is_allied(proposer, recipient) {
                reasons.push(("Déjà alliés".to_owned(), -100));
                hard_no = true;
            }
            if state.is_at_war(proposer, recipient) {
                reasons.push(("En guerre".to_owned(), -100));
                hard_no = true;
            }
            reasons.push(("Attitude".to_owned(), attitude / 2));
            reasons.push(("Engagement militaire".to_owned(), -15));
            let recipient_allies = state
                .factions
                .get(recipient)
                .map(|f| f.allies.clone())
                .unwrap_or_default();
            if recipient_allies
                .iter()
                .any(|a| state.is_at_war(proposer, a))
            {
                reasons.push(("En guerre contre nos alliés".to_owned(), -60));
            }
            let common = state.factions.get(proposer).is_some_and(|f| {
                f.at_war_with
                    .iter()
                    .any(|e| !is_rebels(e) && state.is_at_war(recipient, e))
            });
            if common {
                reasons.push(("Ennemi commun".to_owned(), 25));
            }
            let ratio = state.faction_power(proposer) / state.faction_power(recipient).max(1.0);
            if ratio > 1.0 {
                reasons.push(("Allié puissant".to_owned(), 10));
            } else if ratio < 0.3 {
                reasons.push(("Allié faible".to_owned(), -10));
            }
            // F4: shared rivals, counterweight against a menacing neighbour,
            // and no alliance with a rival's friend.
            let proposer_rivals = rivals(state, proposer);
            let recipient_rivals = rivals(state, recipient);
            if !proposer_rivals.is_disjoint(&recipient_rivals) {
                reasons.push(("Rival commun".to_owned(), 15));
            }
            let menace = &data.ai_diplomacy.menacing_neighbour;
            let menaced = proposer_rivals.iter().any(|r| {
                state.faction_power(r)
                    > menace.power_ratio * state.faction_power(recipient).max(1.0)
                    && state.are_neighbors(data, recipient, r)
            });
            if menaced {
                reasons.push((
                    "Contrepoids à un voisin menaçant".to_owned(),
                    menace.counterweight,
                ));
            }
            let friend_of_rival = state
                .factions
                .get(proposer)
                .is_some_and(|f| f.allies.iter().any(|a| recipient_rivals.contains(a)));
            if friend_of_rival {
                reasons.push(("Allié de nos rivaux".to_owned(), -40));
            }
        }
        Proposal::Vassalage => {
            let already = state
                .factions
                .get(recipient)
                .is_some_and(|f| f.suzerain.is_some());
            if already {
                reasons.push(("Déjà vassal".to_owned(), -100));
                hard_no = true;
            }
            let ratio = state.faction_power(proposer) / state.faction_power(recipient).max(1.0);
            if ratio < VASSALAGE_POWER_RATIO {
                reasons.push(("Pas assez puissant pour l'exiger".to_owned(), -100));
                hard_no = true;
            } else {
                reasons.push((
                    "Rapport de forces".to_owned(),
                    (((ratio - VASSALAGE_POWER_RATIO) * 10.0) as i32).min(40),
                ));
            }
            reasons.push(("Perte d'indépendance".to_owned(), -40));
            reasons.push(("Attitude".to_owned(), attitude / 2));
            if state.is_at_war(proposer, recipient)
                && state.war_score(data, recipient, proposer) < -30
            {
                reasons.push(("Défaite militaire".to_owned(), 30));
            }
        }
        Proposal::Marriage { character, spouse } => {
            let mut probe = state.clone();
            if let Err(error) =
                crate::dynasty::propose_marriage(&mut probe, data, character, spouse)
            {
                reasons.push((format!("Mariage impossible : {error}"), -100));
                hard_no = true;
            }
            reasons.push(("Attitude".to_owned(), attitude / 2));
            reasons.push(("Alliance matrimoniale".to_owned(), 10));
            let rank = |id: &CharacterId| state.characters.get(id).map_or(0, |c| c.prestige);
            reasons.push((
                "Prestige du prétendant".to_owned(),
                ((rank(character) - rank(spouse)) / 10).clamp(-20, 20),
            ));
        }
        Proposal::Obedience { .. } => {
            reasons.push(("Choix d'obédience".to_owned(), 0));
        }
        Proposal::Treaty { articles } => {
            let verdict =
                crate::negotiation::evaluate_treaty(state, data, proposer, recipient, articles);
            return Evaluation {
                accept: verdict.accept,
                score: verdict.score,
                reasons: verdict.reasons(),
            };
        }
    }
    reasons.retain(|(_, v)| *v != 0 || hard_no);
    let score: i32 = reasons.iter().map(|(_, v)| v).sum();
    Evaluation {
        accept: !hard_no && score > 0,
        score,
        reasons,
    }
}

fn peace_reasons(
    state: &CampaignState,
    data: &GameData,
    proposer: &FactionId,
    recipient: &FactionId,
    attitude: i32,
    aggression: i32,
    reasons: &mut Vec<(String, i32)>,
) {
    let score = state.war_score(data, recipient, proposer);
    reasons.push(("Score de guerre".to_owned(), -score));
    let started = state
        .factions
        .get(recipient)
        .and_then(|f| f.war_started.get(proposer))
        .copied()
        .unwrap_or(state.turn);
    reasons.push((
        "Lassitude de la guerre".to_owned(),
        (state.turn.saturating_sub(started) / 2).min(20) as i32,
    ));
    if state
        .factions
        .get(recipient)
        .is_some_and(|f| f.treasury < 0)
    {
        reasons.push(("Trésor vide".to_owned(), 15));
    }
    let devastated = state
        .provinces
        .iter()
        .filter(|(id, p)| state.province_owner(id) == Some(recipient) && p.devastation > 30)
        .count() as i32;
    reasons.push(("Provinces ravagées".to_owned(), (devastated * 3).min(20)));
    reasons.push(("Attitude".to_owned(), attitude / 5));
    reasons.push(("Tempérament belliqueux".to_owned(), -(aggression - 50) / 5));
    reasons.push(("Attrait de la victoire".to_owned(), -5));
    // F4: a stronger enemy on another front calls for peace here; a
    // pretender does not give up its claim lightly.
    let other_front = state.factions.get(recipient).is_some_and(|f| {
        f.at_war_with.iter().any(|e| {
            e != proposer && !is_rebels(e) && state.faction_power(e) > state.faction_power(proposer)
        })
    });
    if other_front {
        reasons.push(("Guerre sur un autre front".to_owned(), 15));
    }
    if score > -30 {
        let stakes = claim_stakes(state, recipient, proposer);
        if stakes.throne {
            reasons.push((
                "Prétention au trône".to_owned(),
                -PRETENDER_PEACE_RELUCTANCE,
            ));
        } else if stakes.provinces > 0 {
            reasons.push(("Provinces revendiquées".to_owned(), -5));
        }
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
            .filter(|f| *f != attacker && !is_rebels(f))
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
        // Break any vassal tie between the two.
        self.cut_vassal_tie(attacker, target);
        self.start_war(attacker, target);
        self.factions
            .get_mut(attacker)
            .expect("checked")
            .last_war_declared = Some(self.turn);
        let text = format!(
            "{} déclare la guerre à {} ({motive}).",
            faction_name(data, attacker),
            faction_name(data, target)
        );
        self.push_order_event(GameEvent::new(EventKind::WarDeclared, text).faction(attacker));
        if excommunicate {
            religion::excommunicate(self, data, attacker);
        }
        self.call_to_arms(data, target, attacker);
        self.rally_vassals(data, attacker, target);
        Ok(())
    }

    fn change_ruler_prestige(&mut self, faction: &FactionId, delta: i32) {
        if let Some(ruler) = self.factions.get(faction).and_then(|f| f.ruler.clone()) {
            if let Some(c) = self.characters.get_mut(&ruler) {
                c.prestige += delta;
            }
        }
    }

    fn cut_vassal_tie(&mut self, a: &FactionId, b: &FactionId) {
        for (vassal, suzerain) in [(a, b), (b, a)] {
            if self
                .factions
                .get(vassal)
                .is_some_and(|f| f.suzerain.as_ref() == Some(suzerain))
            {
                let v = self.factions.get_mut(vassal).expect("exists");
                v.suzerain = None;
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
    fn call_to_arms(&mut self, data: &GameData, defender: &FactionId, aggressor: &FactionId) {
        let allies: Vec<FactionId> = self.factions[defender].allies.iter().cloned().collect();
        for ally in allies {
            if &ally == aggressor
                || !self.factions.get(&ally).is_some_and(|f| f.alive)
                || self.is_at_war(&ally, aggressor)
                || is_rebels(&ally)
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
                    faction_name(data, &ally),
                    faction_name(data, defender),
                    faction_name(data, aggressor)
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
                    self.factions.get_mut(&ally).expect("exists").suzerain = None;
                }
                self.add_modifier(defender, &ally, -30, "A refusé l'appel aux armes", 40);
                let text = format!(
                    "{} refuse de soutenir {} : l'alliance est rompue.",
                    faction_name(data, &ally),
                    faction_name(data, defender)
                );
                self.push_order_event(
                    GameEvent::new(EventKind::AllianceBroken, text).faction(&ally),
                );
            }
        }
    }

    /// Loyal vassals of `aggressor` follow it to war against `target`.
    fn rally_vassals(&mut self, data: &GameData, aggressor: &FactionId, target: &FactionId) {
        let vassals: Vec<FactionId> = self
            .factions
            .iter()
            .filter(|(_, f)| f.alive && f.suzerain.as_ref() == Some(aggressor))
            .map(|(id, _)| id.clone())
            .collect();
        for vassal in vassals {
            if &vassal == target
                || self.is_at_war(&vassal, target)
                || self.is_allied(&vassal, target)
                || self.factions[&vassal].loyalty < CALL_TO_ARMS_LOYALTY
            {
                continue;
            }
            self.start_war(&vassal, target);
            let text = format!(
                "{} suit son suzerain dans la guerre contre {}.",
                faction_name(data, &vassal),
                faction_name(data, target)
            );
            self.push_order_event(GameEvent::new(EventKind::WarDeclared, text).faction(&vassal));
        }
    }

    /// Ends the war between `a` and `b`: each listed province passes to the
    /// other party, `tribute` is paid by `b` to `a` (negative: `a` pays),
    /// and a truce of `truce_turns` starts.
    pub(crate) fn make_peace(
        &mut self,
        data: &GameData,
        a: &FactionId,
        b: &FactionId,
        provinces: &[ProvinceId],
        tribute: i64,
        truce_turns: u32,
    ) {
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
                province_name(data, province),
                faction_name(data, &to)
            ));
        }
        // Occupied settlements return to their owner.
        for s in self.settlements.values_mut() {
            let between =
                (&s.owner == a && &s.controller == b) || (&s.owner == b && &s.controller == a);
            if between {
                s.controller = s.owner.clone();
                s.siege = None;
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
            faction_name(data, a),
            faction_name(data, b),
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

    fn form_alliance(&mut self, data: &GameData, a: &FactionId, b: &FactionId) {
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
            faction_name(data, a),
            faction_name(data, b)
        );
        self.push_order_event(GameEvent::new(EventKind::AllianceFormed, text).faction(a));
    }

    fn make_vassal(&mut self, data: &GameData, suzerain: &FactionId, vassal: &FactionId) {
        if self.is_at_war(suzerain, vassal) {
            self.make_peace(data, suzerain, vassal, &[], 0, TRUCE_TURNS);
        }
        let v = self.factions.get_mut(vassal).expect("exists");
        v.suzerain = Some(suzerain.clone());
        v.allies.insert(suzerain.clone());
        v.loyalty = 60;
        self.factions
            .get_mut(suzerain)
            .expect("exists")
            .allies
            .insert(vassal.clone());
        let text = format!(
            "{} devient vassal de {}.",
            faction_name(data, vassal),
            faction_name(data, suzerain)
        );
        self.push_order_event(GameEvent::new(EventKind::Vassalage, text).faction(vassal));
    }

    /// Applies an accepted proposal of `proposer` to `recipient`.
    pub(crate) fn apply_proposal(
        &mut self,
        data: &GameData,
        proposer: &FactionId,
        recipient: &FactionId,
        proposal: &Proposal,
    ) -> Result<(), DiplomacyError> {
        match proposal {
            Proposal::Peace { provinces, tribute } => {
                self.make_peace(data, proposer, recipient, provinces, *tribute, TRUCE_TURNS);
            }
            Proposal::Truce { turns } => {
                self.make_peace(data, proposer, recipient, &[], 0, *turns);
            }
            Proposal::Alliance => self.form_alliance(data, proposer, recipient),
            Proposal::Vassalage => self.make_vassal(data, proposer, recipient),
            Proposal::Marriage { character, spouse } => {
                crate::dynasty::propose_marriage(self, data, character, spouse)
                    .map_err(|e| DiplomacyError::Refused(e.to_string()))?;
                self.add_modifier(proposer, recipient, 15, "Mariage entre nos maisons", 80);
                self.add_modifier(recipient, proposer, 15, "Mariage entre nos maisons", 80);
                let text = format!(
                    "Mariage de {} et de {}.",
                    self.character_name(data, character),
                    self.character_name(data, spouse)
                );
                self.push_order_event(GameEvent::new(EventKind::Marriage, text).faction(proposer));
            }
            Proposal::Obedience { religion: target } => {
                religion::set_obedience(self, data, recipient, target)?;
            }
            Proposal::Treaty { articles } => {
                crate::negotiation::apply_treaty(self, data, proposer, recipient, articles)?;
            }
        }
        Ok(())
    }

    /// Validates a proposal before evaluation (war targets, provinces...).
    fn check_proposal(
        &self,
        data: &GameData,
        proposer: &FactionId,
        recipient: &FactionId,
        proposal: &Proposal,
    ) -> Result<(), DiplomacyError> {
        self.check_pair(proposer, recipient)?;
        match proposal {
            Proposal::Peace { provinces, tribute } => {
                if !self.is_at_war(proposer, recipient) {
                    return Err(DiplomacyError::NotAtWar);
                }
                for province in provinces {
                    let owner = self.province_owner(province);
                    if owner != Some(proposer) && owner != Some(recipient) {
                        return Err(DiplomacyError::InvalidProvince(province.clone()));
                    }
                }
                if *tribute < 0 && self.factions[proposer].treasury < -*tribute {
                    return Err(DiplomacyError::InsufficientFunds);
                }
            }
            Proposal::Truce { .. } => {
                if !self.is_at_war(proposer, recipient) {
                    return Err(DiplomacyError::NotAtWar);
                }
            }
            Proposal::Alliance => {
                if self.is_allied(proposer, recipient) {
                    return Err(DiplomacyError::AlreadyAllied);
                }
                if self.is_at_war(proposer, recipient) {
                    return Err(DiplomacyError::AlreadyAtWar);
                }
            }
            Proposal::Vassalage | Proposal::Marriage { .. } | Proposal::Obedience { .. } => {}
            Proposal::Treaty { articles } => {
                crate::negotiation::check_treaty(self, data, proposer, recipient, articles)?;
            }
        }
        Ok(())
    }

    /// Sends `proposal`: the player receives an offer, anyone else answers
    /// at once through [`evaluate`].
    pub fn propose(
        &mut self,
        data: &GameData,
        proposer: &FactionId,
        recipient: &FactionId,
        proposal: Proposal,
    ) -> Result<(), DiplomacyError> {
        self.check_proposal(data, proposer, recipient, &proposal)?;
        if recipient == &self.player_faction && proposer != &self.player_faction {
            self.create_offer(data, proposer, proposal);
            return Ok(());
        }
        if let Proposal::Treaty { articles } = proposal {
            crate::negotiation::propose_treaty(self, data, proposer, recipient, articles)?;
            return Ok(());
        }
        let evaluation = evaluate(self, data, proposer, recipient, &proposal);
        if !evaluation.accept {
            let mut reasons = evaluation.reasons.clone();
            reasons.sort_by_key(|(_, v)| *v);
            let top: Vec<String> = reasons
                .iter()
                .take(3)
                .map(|(t, v)| format!("{t} ({v:+})"))
                .collect();
            return Err(DiplomacyError::Refused(format!(
                "{} refuse : {}",
                faction_name(data, recipient),
                top.join(", ")
            )));
        }
        self.apply_proposal(data, proposer, recipient, &proposal)
    }

    fn create_offer(&mut self, data: &GameData, from: &FactionId, proposal: Proposal) {
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
        if (recent && !matches!(proposal, Proposal::Obedience { .. })) || duplicate {
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
            if !matches!(offer.proposal, Proposal::Obedience { .. }) {
                self.check_proposal(data, &offer.from, faction, &offer.proposal)?;
            }
            self.factions
                .get_mut(faction)
                .expect("exists")
                .offers
                .remove(index);
            self.apply_proposal(data, &offer.from, faction, &offer.proposal)
        } else {
            self.factions
                .get_mut(faction)
                .expect("exists")
                .offers
                .remove(index);
            if !matches!(offer.proposal, Proposal::Obedience { .. }) {
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
                    faction_name(data, faction),
                    faction_name(data, target)
                )
            } else {
                format!(
                    "{} lève son embargo contre {}.",
                    faction_name(data, faction),
                    faction_name(data, target)
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
            faction_name(data, faction),
            faction_name(data, target)
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
            faction_name(data, faction),
            faction_name(data, target)
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
            faction_name(data, faction),
            faction_name(data, target)
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
        self.add_modifier(target, faction, value, GIFT_REASON, 20);
        let text = format!(
            "{} envoie {amount} livres de présents à {}.",
            faction_name(data, faction),
            faction_name(data, target)
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
            .filter(|(id, f)| *id != faction && f.alive && !is_rebels(id))
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
    proposal: &Proposal,
) -> String {
    let name = faction_name(data, from);
    match proposal {
        Proposal::Peace { provinces, tribute } => {
            let mut text = format!("{name} propose la paix");
            if !provinces.is_empty() {
                let list: Vec<String> = provinces.iter().map(|p| province_name(data, p)).collect();
                text.push_str(&format!(" contre {}", list.join(", ")));
            }
            if *tribute > 0 {
                text.push_str(&format!(" et un tribut de {tribute} livres"));
            } else if *tribute < 0 {
                text.push_str(&format!(" et offre {} livres", -tribute));
            }
            text.push('.');
            text
        }
        Proposal::Truce { turns } => {
            format!(
                "Par la médiation du pape, {name} propose une trêve de {} ans.",
                turns / 4
            )
        }
        Proposal::Alliance => format!("{name} propose une alliance."),
        Proposal::Vassalage => format!("{name} exige que vous deveniez son vassal."),
        Proposal::Marriage { character, spouse } => format!(
            "{name} propose le mariage de {} et de {}.",
            state.character_name(data, character),
            state.character_name(data, spouse)
        ),
        Proposal::Treaty { articles } => {
            let player = state.player_faction.clone();
            format!(
                "{name} propose un traité. {}",
                crate::negotiation::treaty_text(state, data, from, &player, articles)
            )
        }
        Proposal::Obedience { religion } => format!(
            "Grand Schisme : rejoindre {} ?",
            data.religions
                .get(religion)
                .map_or_else(|| religion.to_string(), |r| r.name.display.clone())
        ),
    }
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
        if let Proposal::Obedience { .. } = offer.proposal {
            continue; // keeping the historical obedience is the default
        }
        events.push(
            GameEvent::new(
                EventKind::DiplomaticOffer,
                format!("L'offre de {} a expiré.", faction_name(data, &offer.from)),
            )
            .faction(&offer.from),
        );
    }

    // Vassals.
    let vassals: Vec<(FactionId, FactionId)> = state
        .factions
        .iter()
        .filter(|(_, f)| f.alive)
        .filter_map(|(id, f)| f.suzerain.clone().map(|s| (id.clone(), s)))
        .collect();
    for (vassal, suzerain) in vassals {
        if !state.factions.get(&suzerain).is_some_and(|f| f.alive) {
            let v = state.factions.get_mut(&vassal).expect("exists");
            v.suzerain = None;
            v.allies.remove(&suzerain);
            continue;
        }
        let tribute =
            (state.factions[&vassal].income_last_turn * VASSAL_TRIBUTE_PERCENT / 100).max(0);
        state.factions.get_mut(&vassal).expect("exists").treasury -= tribute;
        state.factions.get_mut(&suzerain).expect("exists").treasury += tribute;
        let target = loyalty_target(state, data, &vassal, &suzerain);
        let v = state.factions.get_mut(&vassal).expect("exists");
        v.loyalty = move_towards(v.loyalty, target, 5);
        let loyalty = v.loyalty;
        if loyalty < REBELLION_LOYALTY
            && !state.is_at_war(&vassal, &suzerain)
            && state.rng.chance_permille(REBELLION_PERMILLE)
        {
            state.cut_vassal_tie(&vassal, &suzerain);
            state.start_war(&vassal, &suzerain);
            events.push(
                GameEvent::new(
                    EventKind::VassalRebellion,
                    format!(
                        "{} se révolte contre son suzerain {} et proclame son indépendance.",
                        faction_name(data, &vassal),
                        faction_name(data, &suzerain)
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

/// Loyalty a vassal drifts towards (0-100).
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
    let mut target = 55 + (attitude - loyalty_term) / 2;
    if state.faction_power(suzerain) > 2.0 * state.faction_power(vassal) {
        target += 10;
    } else {
        target -= 10;
    }
    if religion::is_excommunicated(state, suzerain) {
        target -= 20;
    }
    // F1 `Loyalty`: a loyal vassal ruler, a generous or kind overlord.
    let loyalty = |e: &crate::buildings::EffectTotals| e.loyalty.apply(0.0);
    target += state.ruler_effect_points(data, vassal, loyalty)
        + state.ruler_effect_points(data, suzerain, loyalty);
    // Embargoed by an enemy of the suzerain: the vassal's trade suffers
    // for its overlord's quarrels (Flanders and English wool, 1336).
    let squeezed = state
        .factions
        .iter()
        .any(|(id, f)| f.alive && f.embargoes.contains(vassal) && state.is_at_war(id, suzerain));
    if squeezed {
        target -= 25;
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
    if is_rebels(&claimant_faction) {
        return;
    }
    let text = format!(
        "{} hérite d'une prétention au trône de {} par sa mère.",
        state.character_name(data, &character),
        faction_name(data, faction)
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
                    faction_name(data, faction),
                    faction_name(data, &claimant_faction)
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
        .filter(|(id, f)| *id != faction && f.alive && !is_rebels(id))
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
        && me.treasury >= 2 * me.upkeep_last_turn.max(0)
        && (me.income_last_turn >= me.upkeep_last_turn
            || me.treasury >= 8 * me.upkeep_last_turn.max(0))
}

/// Power of the enemies `faction` already fights (rebels excluded).
/// Only fronts that press on us count: bordering enemies and enemies
/// holding our provinces (a distant war of religion does not tie armies).
fn enemy_power(state: &CampaignState, data: &GameData, faction: &FactionId) -> f64 {
    state.factions.get(faction).map_or(0.0, |f| {
        f.at_war_with
            .iter()
            .filter(|e| !is_rebels(e))
            .filter(|e| {
                state.are_neighbors(data, faction, e)
                    || state.provinces.keys().any(|id| {
                        state.province_owner(id) == Some(faction) && state.controls_province(e, id)
                    })
            })
            .map(|e| state.faction_power(e))
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
        return ally_state.loyalty >= CALL_TO_ARMS_LOYALTY;
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
    let crippled = ally_state.treasury < 0;
    let grudge = rivals(state, ally).contains(aggressor);
    !crippled && (attitude > 0 || (grudge && attitude > -20))
}

/// Diplomatic orders of an AI faction for this turn (spec § 2.3, F4).
pub fn plan_diplomacy(state: &CampaignState, data: &GameData, faction: &FactionId) -> Vec<Order> {
    let mut orders = Vec::new();
    if is_rebels(faction) {
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
        orders.extend(crate::negotiation::plan_peace(state, data, faction));
    }
    if !treaties && ((turn + slot).is_multiple_of(2) || cornered(state, data, faction)) {
        for enemy in me.at_war_with.iter().filter(|e| !is_rebels(e)) {
            if let Some(provinces) = peace_terms(state, data, faction, enemy) {
                orders.push(Order::ProposePeace {
                    target: enemy.clone(),
                    provinces,
                    tribute: 0,
                });
                break;
            }
        }
    }

    plan_alliances(state, data, faction, slot, &mut orders);

    let rested = me
        .last_war_declared
        .is_none_or(|t| t + WAR_REST_TURNS <= turn);
    // DP1: a weary realm opens no new front; a pretender waits less.
    let weary = treaties && {
        let most = data.ai_diplomacy.negotiation.max_weariness_to_declare;
        let pretender = me.claims.iter().any(|c| c.kind == ClaimKind::Throne);
        me.ledger.weariness > if pretender { most + 20 } else { most }
    };
    let ready = turn >= 4
        && rested
        && !weary
        && (war_ready(state, faction) || crate::negotiation::pretender_ready(state, data, faction));
    let mut declared = false;
    if ready && (turn + slot).is_multiple_of(2) {
        if let Some(target) = war_target(state, data, faction, aggression) {
            orders.push(Order::DeclareWar { target });
            declared = true;
        }
    }
    if ready && !declared && (turn + slot) % 2 == 1 {
        if let Some(target) = ally_war_to_join(state, data, faction) {
            orders.push(Order::DeclareWar { target });
            declared = true;
        }
    }
    if ready && !declared {
        orders.extend(desert_losing_suzerain(state, data, faction, slot));
    }

    // Lift pointless embargoes, drop hated allies.
    for target in &me.embargoes {
        if !state.is_at_war(faction, target) && state.attitude(data, faction, target).0 > 10 {
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
        .filter(|a| state.attitude(data, faction, a).0 < -30)
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

/// Peace terms `faction` offers `enemy` this turn, if any would be accepted.
fn peace_terms(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    enemy: &FactionId,
) -> Option<Vec<ProvinceId>> {
    let score = state.war_score(data, faction, enemy);
    let capital = |f: &FactionId| state.factions.get(f).map(|s| s.capital.clone());
    let keep_capital = data.ai_diplomacy.peace.keep_capital;
    let held_by = |owner: &FactionId, holder: &FactionId, most: usize| -> Vec<ProvinceId> {
        let capital = capital(owner);
        let mut list: Vec<ProvinceId> = state
            .provinces
            .keys()
            .filter(|id| {
                state.province_owner(id) == Some(owner) && state.controls_province(holder, id)
            })
            // A crown survives its defeats: its capital is never ceded.
            .filter(|id| !keep_capital || Some(*id) != capital.as_ref())
            .cloned()
            .collect();
        // Capitals last: they are the costliest to give up.
        list.sort_by_key(|id| (Some(id) == capital.as_ref(), id.clone()));
        list.truncate(most);
        if keep_capital {
            // Nor its last land (Scotland's capital may lie in English hands).
            let owned = state
                .provinces
                .keys()
                .filter(|id| state.province_owner(id) == Some(owner))
                .count();
            list.truncate(owned.saturating_sub(1));
        }
        list
    };
    let answer_known = enemy != &state.player_faction;
    let accepted = |provinces: &[ProvinceId]| {
        let proposal = Proposal::Peace {
            provinces: provinces.to_vec(),
            tribute: 0,
        };
        !answer_known || evaluate(state, data, faction, enemy, &proposal).accept
    };
    if score > 40 {
        let taken = held_by(enemy, faction, 2);
        return accepted(&taken).then_some(taken);
    }
    let white = Proposal::Peace {
        provinces: Vec::new(),
        tribute: 0,
    };
    if evaluate(state, data, enemy, faction, &white).accept && accepted(&[]) {
        return Some(Vec::new());
    }
    if score < SURRENDER_WAR_SCORE || cornered(state, data, faction) {
        let lost = held_by(faction, enemy, usize::MAX);
        if !lost.is_empty() && accepted(&lost) {
            return Some(lost);
        }
    }
    None
}

/// The war `faction` declares this turn, if any: a pretender presses its
/// claim even against a stronger crown when it has allies or a bridgehead;
/// aggressive realms fall on weaker rivals they hold a casus belli against.
fn war_target(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    aggression: i32,
) -> Option<FactionId> {
    let my_power = state.coalition_power(faction);
    let rules = &data.ai_diplomacy.war;
    // Never a new front while the current wars weigh (DP1: a pretender
    // to a throne tolerates a heavier border war, as Edward III kept
    // fighting the Scots while claiming France).
    let pressing = enemy_power(state, data, faction);
    let own = state.faction_power(faction);
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
    state
        .factions
        .iter()
        .filter(|(id, f)| {
            *id != faction
                && f.alive
                && !is_rebels(id)
                && id.as_str() != PAPACY_FACTION
                && !state.is_allied(faction, id)
                && !state.is_at_war(faction, id)
                && !state.has_truce(faction, id)
        })
        .filter_map(|(id, _)| {
            let stakes = claim_stakes(state, faction, id);
            // DF1: a harder campaign lowers the odds an AI wants before
            // falling on the player.
            let demand = state.difficulty_war_ratio_factor(data, id);
            if stakes.any() && aggression >= PRETENDER_AGGRESSION {
                let ratio = my_power / state.faction_power(id).max(1.0);
                let supported = has_allies || state.are_neighbors(data, faction, id);
                let needed = demand
                    * if supported {
                        rules.pretender_ratio
                    } else {
                        rules.pretender_ratio_alone
                    };
                let weight = if stakes.throne { 3.0 } else { 0.0 } + stakes.provinces as f64;
                return (ratio >= needed && state.attitude(data, faction, id).0 < 20)
                    // A claim outranks any opportunistic war.
                    .then(|| (id.clone(), CLAIM_WAR_PRIORITY + weight + ratio));
            }
            if aggression >= OPPORTUNIST_AGGRESSION
                && state.casus_belli(data, faction, id).is_some()
                && state.attitude(data, faction, id).0 < 0
            {
                let ratio = my_power / state.coalition_power(id).max(1.0);
                return (ratio >= OPPORTUNIST_RATIO * demand).then(|| (id.clone(), ratio));
            }
            None
        })
        .max_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
        .map(|(id, _)| id)
}

/// An ally's war `faction` joins (co-belligerence, F4): the Low Countries
/// follow Edward III into France, Scotland falls on the English border.
fn ally_war_to_join(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> Option<FactionId> {
    let me = state.factions.get(faction)?;
    let front_share = data.ai_diplomacy.war.front_share;
    let rules = &data.ai_diplomacy.join_war;
    if enemy_power(state, data, faction) > front_share * state.faction_power(faction) {
        return None;
    }
    let my_power = state.faction_power(faction);
    for ally in &me.allies {
        let Some(ally_state) = state.factions.get(ally) else {
            continue;
        };
        if !ally_state.alive
            || state.attitude(data, faction, ally).0 <= rules.min_attitude
            || state.faction_power(ally) < rules.min_ally_power_ratio * my_power
        {
            continue;
        }
        for enemy in ally_state.at_war_with.iter().filter(|e| !is_rebels(e)) {
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
            let border = state.are_neighbors(data, faction, enemy)
                && (claim_war || !rules.border_only_claim_wars);
            let reachable = border || claim_stakes(state, faction, enemy).any();
            let ratio = state.coalition_power(faction) / state.coalition_power(enemy).max(1.0);
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
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    slot: u32,
    orders: &mut Vec<Order>,
) {
    if (state.turn + slot) % 4 != 1 || alliance_count(state, faction) >= MAX_ALLIANCES {
        return;
    }
    let my_rivals = rivals(state, faction);
    if my_rivals.is_empty() {
        return;
    }
    let candidate = state
        .factions
        .iter()
        .filter(|(id, f)| {
            *id != faction
                && f.alive
                && !is_rebels(id)
                && id.as_str() != PAPACY_FACTION
                && !state.is_allied(faction, id)
                && !state.is_at_war(faction, id)
                && !my_rivals.contains(*id)
                && f.allies.iter().all(|a| !my_rivals.contains(a))
        })
        .filter(|(id, _)| {
            let theirs = rivals(state, id);
            !theirs.is_disjoint(&my_rivals)
                || my_rivals.iter().any(|r| {
                    let towards_rival = state.attitude(data, id, r).0;
                    towards_rival < 0 && state.attitude(data, id, faction).0 > towards_rival + 10
                })
        })
        .map(|(id, _)| (id.clone(), state.attitude(data, id, faction).0))
        .filter(|(id, _)| state.attitude(data, faction, id).0 > 0)
        .filter(|(id, _)| {
            id == &state.player_faction
                || evaluate(state, data, faction, id, &Proposal::Alliance).accept
        })
        .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)));
    if let Some((target, _)) = candidate {
        orders.push(Order::ProposeAlliance { target });
    }
}

/// An opportunistic vassal (Burgundy) deserts a suzerain that is losing to
/// a stronger coalition: white peace with the winner, then independence.
fn desert_losing_suzerain(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    slot: u32,
) -> Vec<Order> {
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
        .filter(|e| !is_rebels(e) && *e != faction)
        .find(|e| {
            state.war_score(data, &lord, e) <= DESERTION_WAR_SCORE
                && state.coalition_power(e) > state.coalition_power(&lord)
                && state.attitude(data, faction, e).0 > -40
        })
        .cloned();
    let Some(winner) = winner else {
        return Vec::new();
    };
    let mut orders = Vec::new();
    if state.is_at_war(faction, &winner) {
        let white = Proposal::Peace {
            provinces: Vec::new(),
            tribute: 0,
        };
        if winner == state.player_faction || !evaluate(state, data, faction, &winner, &white).accept
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
