//! Lot DP1 (ADR 0025): diplomatic negotiation.
//!
//! - **Treaties** with several articles (peace, truce, alliance, military
//!   access, trade agreement, marriage, tribute, gold, cession of a province
//!   or of a settlement, vassalage, release of a captive, hostage), each given
//!   by one party ([`Party`]).
//! - **Valuation** of every article by the recipient ([`evaluate_treaty`]):
//!   value for it, attitude, trust, threat, honour, war weariness; the sum
//!   gives an **acceptance chance** (logistic). The decision is a
//!   deterministic roll per (seed, turn, pair): the same offer gets the same
//!   answer during a season.
//! - **Counter-proposals** ([`counter_proposal`]): what the recipient would
//!   need to accept (captives, occupied lands, gold, tribute, fewer demands).
//! - **War goals, war score and war weariness** (audit A2, lot N2): each
//!   belligerent targets provinces ([`DiplomaticLedger::war_goals`]); holding
//!   them fills the war score (`CampaignState::war_score`); weariness grows
//!   with every season of war and pushes towards peace (and unrest).
//! - The AI's **treaty peace** ([`plan_peace`]): the winner demands the
//!   provinces it holds, the loser buys peace with them.
//!
//! All rules live here; the UI (`diplomacy_panel.gd`) only shows the
//! evaluation returned by the bridge.

use std::collections::{BTreeMap, BTreeSet};

use data_model::{
    CharacterId, FactionId, GameData, NegotiationRules, ProvinceId, SettlementId, SettlementKind,
    TitleId,
};
use serde::{Deserialize, Serialize};

use crate::diplomacy::{self, faction_name, DiplomacyError, Proposal, RelationKind};
use crate::events::{EventKind, GameEvent};
use crate::orders::Order;
use crate::state::CampaignState;

/// Seasons a hostage stays with the other party.
pub const HOSTAGE_TURNS: u32 = 40;
/// Treaty records kept per faction.
pub const HISTORY_LENGTH: usize = 40;
/// Chance (percent) a counter-proposal aims at.
pub const COUNTER_TARGET_CHANCE: u8 = 65;
/// Seasons over which a counter-proposal spreads a tribute.
pub const COUNTER_TRIBUTE_SEASONS: u32 = 8;
/// Longest tribute (seasons).
pub const MAX_TRIBUTE_SEASONS: u32 = 40;
/// Reason of the modifier left by a war declared on a hostage holder.
pub const HOSTAGE_BETRAYAL_REASON: &str = "Otages abandonnés : parole trahie";

// =========================================================================
// Types
// =========================================================================

/// The party giving an article.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Party {
    Proposer,
    Recipient,
}

impl Party {
    pub fn other(self) -> Self {
        match self {
            Party::Proposer => Party::Recipient,
            Party::Recipient => Party::Proposer,
        }
    }
}

/// One article of a treaty.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Article {
    /// Ends the war; truce of [`diplomacy::TRUCE_TURNS`].
    Peace,
    /// Ends the war with a short truce.
    Truce {
        turns: u32,
    },
    Alliance,
    /// `giver` opens its lands to the other party's armies.
    MilitaryAccess {
        giver: Party,
    },
    /// Both parties trade freely (income bonus).
    TradeAgreement,
    /// `character` (the proposer's) marries `spouse` (the recipient's).
    Marriage {
        character: CharacterId,
        spouse: CharacterId,
    },
    /// `giver` pays `per_season` livres for `seasons` seasons.
    Tribute {
        giver: Party,
        per_season: i64,
        seasons: u32,
    },
    /// `giver` pays `amount` livres at once.
    Gold {
        giver: Party,
        amount: i64,
    },
    /// `giver` cedes a province it owns (every settlement it holds there).
    CedeProvince {
        giver: Party,
        province: ProvinceId,
    },
    /// `giver` cedes one secondary settlement (castle, town, abbey...).
    CedeSettlement {
        giver: Party,
        settlement: SettlementId,
    },
    /// `giver` becomes the other party's vassal.
    Vassalage {
        giver: Party,
    },
    /// `giver` frees a captive of the other party.
    ReleaseCaptive {
        giver: Party,
        character: CharacterId,
    },
    /// `giver` hands one of its characters as a hostage.
    Hostage {
        giver: Party,
        character: CharacterId,
    },
    /// FE (F3, spec § 4.6): `giver` gives up a feudal title it holds; the
    /// other party usurps it, or grants a lesser title to a direct vassal
    /// (`feudal::conquer_title`).
    DemandTitle {
        giver: Party,
        title: TitleId,
    },
}

impl Article {
    /// Party giving the article (`None`: mutual).
    pub fn giver(&self) -> Option<Party> {
        match self {
            Article::MilitaryAccess { giver }
            | Article::Tribute { giver, .. }
            | Article::Gold { giver, .. }
            | Article::CedeProvince { giver, .. }
            | Article::CedeSettlement { giver, .. }
            | Article::Vassalage { giver }
            | Article::ReleaseCaptive { giver, .. }
            | Article::Hostage { giver, .. }
            | Article::DemandTitle { giver, .. } => Some(*giver),
            _ => None,
        }
    }

    /// Ends a war between the parties.
    pub fn ends_war(&self) -> bool {
        matches!(self, Article::Peace | Article::Truce { .. })
    }

    /// Key of the article kind (`serde` tag).
    pub fn key(&self) -> &'static str {
        match self {
            Article::Peace => "peace",
            Article::Truce { .. } => "truce",
            Article::Alliance => "alliance",
            Article::MilitaryAccess { .. } => "military_access",
            Article::TradeAgreement => "trade_agreement",
            Article::Marriage { .. } => "marriage",
            Article::Tribute { .. } => "tribute",
            Article::Gold { .. } => "gold",
            Article::CedeProvince { .. } => "cede_province",
            Article::CedeSettlement { .. } => "cede_settlement",
            Article::Vassalage { .. } => "vassalage",
            Article::ReleaseCaptive { .. } => "release_captive",
            Article::Hostage { .. } => "hostage",
            Article::DemandTitle { .. } => "demand_title",
        }
    }
}

/// A tribute a faction pays each season.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TributeDue {
    pub to: FactionId,
    pub per_season: i64,
    pub until_turn: u32,
}

/// A hostage a faction holds as a pledge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostagePledge {
    pub character: CharacterId,
    pub from: FactionId,
    pub until_turn: u32,
}

/// One treaty in a faction's history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreatyRecord {
    pub turn: u32,
    pub with: FactionId,
    /// Did this faction propose it?
    pub proposed: bool,
    pub accepted: bool,
    /// Article keys.
    pub articles: Vec<String>,
    pub text_fr: String,
}

/// Diplomatic memory of a faction (DP1): war goals, weariness, standing
/// treaties and their history.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiplomaticLedger {
    /// Provinces targeted in each war.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub war_goals: BTreeMap<FactionId, Vec<ProvinceId>>,
    /// War weariness 0-100.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub weariness: u32,
    /// Unrest the weariness adds in each province (set each season).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub weariness_unrest: u32,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub trade_agreements: BTreeSet<FactionId>,
    /// Factions whose armies may cross our lands.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub military_access: BTreeSet<FactionId>,
    /// Tributes this faction pays.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tributes: Vec<TributeDue>,
    /// Hostages this faction holds.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hostages: Vec<HostagePledge>,
    /// Latest treaties, oldest first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub history: Vec<TreatyRecord>,
    /// Lot DP2: armies of other factions trespassing on our lands.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub trespassers: BTreeMap<FactionId, crate::passage::Trespass>,
}

fn is_zero(value: &u32) -> bool {
    *value == 0
}

impl DiplomaticLedger {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// Value of one article for the recipient.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArticleValue {
    pub label: String,
    pub value: i32,
    pub reasons: Vec<(String, i32)>,
    /// Why the article cannot be accepted at all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocked: Option<String>,
}

/// Verdict of the recipient on a treaty.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreatyEvaluation {
    /// Chance of acceptance 0-100 (0 when blocked).
    pub chance: u8,
    /// The recipient would accept at even odds (`chance >= 50`).
    pub accept: bool,
    pub score: i32,
    pub articles: Vec<ArticleValue>,
    /// General considerations (attitude, trust, threat, honour, weariness).
    pub context: Vec<(String, i32)>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocked: Option<String>,
}

impl TreatyEvaluation {
    /// Flat list of reasons (context first), for [`diplomacy::Evaluation`].
    pub fn reasons(&self) -> Vec<(String, i32)> {
        let mut reasons = self.context.clone();
        for article in &self.articles {
            reasons.push((article.label.clone(), article.value));
        }
        if let Some(blocked) = &self.blocked {
            reasons.push((blocked.clone(), -100));
        }
        reasons
    }
}

// =========================================================================
// Helpers
// =========================================================================

fn rules(data: &GameData) -> &NegotiationRules {
    &data.ai_diplomacy.negotiation
}

fn province_name(data: &GameData, id: &ProvinceId) -> String {
    data.provinces
        .get(id)
        .map_or_else(|| id.to_string(), |p| p.name.display.clone())
}

fn settlement_name(data: &GameData, id: &SettlementId) -> String {
    data.settlements
        .get(id)
        .map_or_else(|| id.to_string(), |s| s.name.display.clone())
}

fn party_id<'a>(party: Party, proposer: &'a FactionId, recipient: &'a FactionId) -> &'a FactionId {
    match party {
        Party::Proposer => proposer,
        Party::Recipient => recipient,
    }
}

fn is_rebels(id: &FactionId) -> bool {
    id.as_str() == diplomacy::REBELS_FACTION
}

/// Acceptance chance (percent) of a score.
pub fn chance_of(data: &GameData, score: i32) -> u8 {
    let scale = rules(data).chance_scale.max(0.1);
    let chance = 100.0 / (1.0 + (-f64::from(score) / scale).exp());
    chance.round().clamp(0.0, 100.0) as u8
}

/// Score needed for `chance` percent.
fn score_for(data: &GameData, chance: u8) -> i32 {
    let p = f64::from(chance.clamp(1, 99)) / 100.0;
    (rules(data).chance_scale * (p / (1.0 - p)).ln()).ceil() as i32
}

/// Deterministic roll 0-99 of `recipient` answering `proposer` this season.
pub fn answer_roll(state: &CampaignState, proposer: &FactionId, recipient: &FactionId) -> u8 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut feed = |bytes: &[u8]| {
        for b in bytes {
            hash ^= u64::from(*b);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
        hash ^= 0xff;
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    };
    feed(&state.seed.to_le_bytes());
    feed(&state.turn.to_le_bytes());
    feed(proposer.as_str().as_bytes());
    feed(recipient.as_str().as_bytes());
    (hash % 100) as u8
}

/// Name of the ruler of `faction` (for the UI).
pub fn ruler_name(state: &CampaignState, data: &GameData, faction: &FactionId) -> String {
    state
        .factions
        .get(faction)
        .and_then(|f| f.ruler.as_ref())
        .map(|r| state.character_name(data, r))
        .unwrap_or_default()
}

/// Is `province` one of `faction`'s war goals against `enemy`?
pub fn is_war_goal(
    state: &CampaignState,
    faction: &FactionId,
    enemy: &FactionId,
    province: &ProvinceId,
) -> bool {
    state
        .factions
        .get(faction)
        .and_then(|f| f.ledger.war_goals.get(enemy))
        .is_some_and(|goals| goals.contains(province))
}

/// War weariness of `faction` (0-100).
pub fn weariness(state: &CampaignState, faction: &FactionId) -> u32 {
    state
        .factions
        .get(faction)
        .map_or(0, |f| f.ledger.weariness)
}

/// Unrest a war-weary realm suffers in each province it controls.
pub fn weariness_unrest(state: &CampaignState, faction: &FactionId) -> f64 {
    state
        .factions
        .get(faction)
        .map_or(0.0, |f| f64::from(f.ledger.weariness_unrest))
}

/// War score bonus of `a` against `b` (DP1): war goals and secondary
/// settlements `a` occupies, minus those of `b`.
pub fn goal_war_score(state: &CampaignState, data: &GameData, a: &FactionId, b: &FactionId) -> i32 {
    let rules = rules(data);
    if !rules.enabled {
        return 0;
    }
    let side = |taker: &FactionId, loser: &FactionId| -> i32 {
        let goals = state
            .factions
            .get(taker)
            .and_then(|f| f.ledger.war_goals.get(loser))
            .map_or(0, |goals| {
                goals
                    .iter()
                    .filter(|p| {
                        state.province_owner(p) == Some(loser) && state.controls_province(taker, p)
                    })
                    .count() as i32
            });
        let settlements = state
            .settlements
            .values()
            .filter(|s| {
                s.kind != SettlementKind::City && &s.owner == loser && &s.controller == taker
            })
            .count() as i32;
        goals * rules.war_goal_score + (settlements * rules.settlement_score).min(20)
    };
    side(a, b) - side(b, a)
}

// =========================================================================
// Validation
// =========================================================================

/// Checks that every article can be executed (ownership, funds, war).
pub fn check_treaty(
    state: &CampaignState,
    data: &GameData,
    proposer: &FactionId,
    recipient: &FactionId,
    articles: &[Article],
) -> Result<(), DiplomacyError> {
    if articles.is_empty() {
        return Err(DiplomacyError::Refused("traité vide".to_owned()));
    }
    let at_war = state.is_at_war(proposer, recipient);
    let ends_war = articles.iter().any(Article::ends_war);
    for (i, article) in articles.iter().enumerate() {
        if articles[..i].contains(article) {
            return Err(DiplomacyError::Refused("article en double".to_owned()));
        }
        let giver = article.giver().map(|g| party_id(g, proposer, recipient));
        let taker = article
            .giver()
            .map(|g| party_id(g.other(), proposer, recipient));
        match article {
            Article::Peace | Article::Truce { .. } => {
                if !at_war {
                    return Err(DiplomacyError::NotAtWar);
                }
            }
            Article::Alliance => {
                if state.is_allied(proposer, recipient) {
                    return Err(DiplomacyError::AlreadyAllied);
                }
                if at_war && !ends_war {
                    return Err(DiplomacyError::AlreadyAtWar);
                }
            }
            Article::MilitaryAccess { .. } | Article::TradeAgreement => {
                if at_war && !ends_war {
                    return Err(DiplomacyError::AlreadyAtWar);
                }
                let (g, t) = match article {
                    Article::MilitaryAccess { .. } => (giver.expect("giver"), taker.expect("t")),
                    _ => (proposer, recipient),
                };
                let ledger = &state.factions[g].ledger;
                let already = match article {
                    Article::MilitaryAccess { .. } => ledger.military_access.contains(t),
                    _ => ledger.trade_agreements.contains(t),
                };
                if already {
                    return Err(DiplomacyError::Refused("accord déjà en vigueur".to_owned()));
                }
            }
            Article::Marriage { character, spouse } => {
                if at_war && !ends_war {
                    return Err(DiplomacyError::AlreadyAtWar);
                }
                let own = |c: &CharacterId, f: &FactionId| {
                    state
                        .characters
                        .get(c)
                        .is_some_and(|c| c.alive && &c.faction == f)
                };
                if !own(character, proposer) || !own(spouse, recipient) {
                    return Err(DiplomacyError::Refused("époux invalides".to_owned()));
                }
                // Checked here so that the treaty never applies by halves.
                crate::dynasty::check_marriage(state, character, spouse)
                    .map_err(|e| DiplomacyError::Refused(e.to_string()))?;
                let wed_twice = articles[..i].iter().any(|other| {
                    matches!(other, Article::Marriage { character: c, spouse: s }
                        if [c, s].iter().any(|x| *x == character || *x == spouse))
                });
                if wed_twice {
                    return Err(DiplomacyError::Refused(
                        "un même époux dans deux mariages".to_owned(),
                    ));
                }
            }
            Article::Tribute {
                per_season,
                seasons,
                ..
            } => {
                if *per_season <= 0 || *seasons == 0 || *seasons > MAX_TRIBUTE_SEASONS {
                    return Err(DiplomacyError::InvalidAmount);
                }
            }
            Article::Gold { amount, .. } => {
                if *amount <= 0 {
                    return Err(DiplomacyError::InvalidAmount);
                }
                if state.factions[giver.expect("giver")].treasury < *amount {
                    return Err(DiplomacyError::InsufficientFunds);
                }
            }
            Article::CedeProvince { province, .. } => {
                if state.province_owner(province) != giver {
                    return Err(DiplomacyError::InvalidProvince(province.clone()));
                }
                let owned = state.owned_provinces(giver.expect("giver")).len();
                let ceded = articles
                    .iter()
                    .filter(|a| {
                        matches!(a, Article::CedeProvince { .. }) && a.giver() == article.giver()
                    })
                    .count();
                if ceded >= owned {
                    return Err(DiplomacyError::Refused(
                        "on ne cède pas sa dernière province".to_owned(),
                    ));
                }
            }
            Article::DemandTitle { title, .. } => {
                let giver = giver.expect("giver");
                if crate::feudal::holder_of(state, title) != Some(giver) {
                    return Err(DiplomacyError::Refused(format!(
                        "{} ne détient pas ce titre",
                        faction_name(data, giver)
                    )));
                }
                let held = crate::feudal::titles_of(state, giver).len();
                let demanded = articles
                    .iter()
                    .filter(|a| {
                        matches!(a, Article::DemandTitle { .. }) && a.giver() == article.giver()
                    })
                    .count();
                if demanded >= held {
                    return Err(DiplomacyError::Refused(
                        "on ne cède pas son dernier titre".to_owned(),
                    ));
                }
            }
            Article::CedeSettlement { settlement, .. } => {
                let Some(s) = state.settlements.get(settlement) else {
                    return Err(DiplomacyError::Refused("place inconnue".to_owned()));
                };
                if Some(&s.owner) != giver || s.kind == SettlementKind::City {
                    return Err(DiplomacyError::Refused(format!(
                        "{} ne peut pas être cédée seule",
                        settlement_name(data, settlement)
                    )));
                }
            }
            Article::Vassalage { .. } => {
                let (vassal, lord) = (giver.expect("giver"), taker.expect("taker"));
                if state.factions[vassal].suzerain.is_some()
                    || state.factions[lord].suzerain.as_ref() == Some(vassal)
                {
                    return Err(DiplomacyError::Refused(
                        "déjà lié par la vassalité".to_owned(),
                    ));
                }
            }
            Article::ReleaseCaptive { character, .. } => {
                let ok = state.characters.get(character).is_some_and(|c| {
                    c.alive && c.captive && c.captor.as_ref() == giver && Some(&c.faction) == taker
                });
                if !ok {
                    return Err(DiplomacyError::Refused("captif invalide".to_owned()));
                }
            }
            Article::Hostage { character, .. } => {
                let g = giver.expect("giver");
                let f = &state.factions[g];
                let ok =
                    state.characters.get(character).is_some_and(|c| {
                        c.alive && !c.captive && &c.faction == g && c.army.is_none()
                    }) && f.ruler.as_ref() != Some(character);
                if !ok {
                    return Err(DiplomacyError::Refused("otage invalide".to_owned()));
                }
            }
        }
    }
    Ok(())
}

// =========================================================================
// Evaluation
// =========================================================================

/// How `recipient` values `articles` offered by `proposer`. Pure.
pub fn evaluate_treaty(
    state: &CampaignState,
    data: &GameData,
    proposer: &FactionId,
    recipient: &FactionId,
    articles: &[Article],
) -> TreatyEvaluation {
    let mut values: Vec<ArticleValue> = articles
        .iter()
        .map(|a| article_value(state, data, proposer, recipient, a))
        .collect();
    let context = context_reasons(state, data, proposer, recipient, articles);
    let mut blocked = check_treaty(state, data, proposer, recipient, articles)
        .err()
        .map(|e| e.to_string());
    if blocked.is_none() {
        blocked = values.iter().find_map(|v| v.blocked.clone());
    }
    for v in &mut values {
        v.value = v.reasons.iter().map(|(_, x)| x).sum();
    }
    let score: i32 =
        values.iter().map(|v| v.value).sum::<i32>() + context.iter().map(|(_, v)| v).sum::<i32>();
    let chance = if blocked.is_some() {
        0
    } else {
        chance_of(data, score)
    };
    TreatyEvaluation {
        chance,
        accept: chance >= 50,
        score,
        articles: values,
        context,
        blocked,
    }
}

/// General considerations of `recipient` about a treaty with `proposer`.
fn context_reasons(
    state: &CampaignState,
    data: &GameData,
    proposer: &FactionId,
    recipient: &FactionId,
    articles: &[Article],
) -> Vec<(String, i32)> {
    let rules = rules(data);
    let mut reasons: Vec<(String, i32)> = Vec::new();
    let (attitude, _) = state.attitude(data, recipient, proposer);
    // A peace is judged on the war, not on the hatred the war feeds.
    let divisor = if articles.iter().any(Article::ends_war) {
        5
    } else {
        3
    };
    reasons.push(("Attitude".to_owned(), attitude / divisor));
    // Trust: the proposer's word (perjuries, hostages, marriages, treaties).
    let rec = &state.factions[recipient];
    let perjuries = rec
        .modifiers
        .iter()
        .filter(|m| {
            &m.with == proposer
                && m.expires_turn > state.turn
                && (m.reason_fr == diplomacy::PERJURY_REASON
                    || m.reason_fr == HOSTAGE_BETRAYAL_REASON)
        })
        .count() as i32;
    let hostages = rec
        .ledger
        .hostages
        .iter()
        .filter(|h| &h.from == proposer)
        .count() as i32;
    let standing = i32::from(rec.ledger.trade_agreements.contains(proposer))
        + i32::from(state.is_allied(recipient, proposer));
    let trust = (-15 * perjuries
        + 10 * hostages
        + 5 * i32::from(state.marriage_tie(recipient, proposer))
        + 5 * standing)
        .clamp(-30, 20);
    reasons.push(("Confiance".to_owned(), trust));
    // Threat: a much stronger neighbour obtains concessions more easily.
    let gives = articles.iter().any(|a| {
        a.giver() == Some(Party::Recipient)
            || (a.ends_war() && state.war_score(data, recipient, proposer) < 0)
    });
    if gives {
        let ratio = state.faction_power(proposer) / state.faction_power(recipient).max(1.0);
        if ratio > 1.5 && state.are_neighbors(data, recipient, proposer) {
            reasons.push((
                "Menace de sa puissance".to_owned(),
                ((ratio - 1.0) * 5.0).round().min(15.0) as i32,
            ));
        } else if ratio < 0.5 {
            reasons.push(("Faiblesse du demandeur".to_owned(), -8));
        }
    }
    let ends_war = articles.iter().any(Article::ends_war);
    if ends_war {
        // Honour: a separate peace abandons the allies still fighting.
        let abandoned = rec
            .allies
            .iter()
            .filter(|a| state.is_at_war(a, proposer))
            .count() as i32;
        if abandoned > 0 {
            reasons.push((
                "Honneur : ne pas abandonner nos alliés".to_owned(),
                -6 * abandoned.min(3),
            ));
        }
        if rules.enabled {
            reasons.push((
                "Fatigue de guerre".to_owned(),
                (rec.ledger.weariness / rules.weariness_peace_divisor.max(1)) as i32,
            ));
            // EQ6: a war nobody wins outright ends in a truce, the winner
            // weary of it too.
            if rules.long_war_years > 0 {
                let years = rec
                    .war_started
                    .get(proposer)
                    .map_or(0, |started| state.turn.saturating_sub(*started) / 4);
                let beyond = years.saturating_sub(rules.long_war_years) as i32;
                reasons.push((
                    "Guerre interminable".to_owned(),
                    (beyond * rules.long_war_points_per_year).min(rules.long_war_max_points),
                ));
            }
            // A pretender does not give up a crown for nothing.
            let gains_land = articles.iter().any(|a| {
                matches!(
                    a,
                    Article::CedeProvince {
                        giver: Party::Proposer,
                        ..
                    }
                )
            });
            if diplomacy::claim_stakes(state, recipient, proposer).throne
                && !gains_land
                && state.war_score(data, recipient, proposer) > -40
            {
                reasons.push((
                    "Prétention à la couronne".to_owned(),
                    -rules.pretender_reluctance,
                ));
            }
            // War goals: a belligerent not beaten keeps fighting for them.
            if let Some(goals) = rec.ledger.war_goals.get(proposer) {
                let obtained = articles.iter().any(|a| match a {
                    Article::CedeProvince {
                        giver: Party::Proposer,
                        province,
                    } => goals.contains(province),
                    _ => false,
                });
                let held = goals.iter().any(|p| state.controls_province(recipient, p));
                if !goals.is_empty()
                    && !obtained
                    && state.war_score(data, recipient, proposer) > -30
                {
                    let value = if held {
                        rules.unmet_goals_reluctance
                    } else {
                        rules.unmet_goals_reluctance / 2
                    };
                    reasons.push(("Buts de guerre non atteints".to_owned(), -value));
                }
            }
        }
    }
    reasons.push(("Prudence".to_owned(), -3));
    reasons.retain(|(_, v)| *v != 0);
    reasons
}

/// Value of one article for `recipient`.
fn article_value(
    state: &CampaignState,
    data: &GameData,
    proposer: &FactionId,
    recipient: &FactionId,
    article: &Article,
) -> ArticleValue {
    let rules = rules(data);
    let mut reasons: Vec<(String, i32)> = Vec::new();
    let mut blocked: Option<String> = None;
    let gives = article.giver() == Some(Party::Recipient);
    let sign = if gives { -1 } else { 1 };
    let lpp = rules.livres_per_point.max(1);
    let gold_points = |amount: i64| -> i32 { ((amount / lpp) as i32).min(rules.max_gold_points) };
    let label = article_label(state, data, proposer, recipient, article);
    // Reasons of the legacy evaluation of a proposal, without attitude
    // (counted once in the context) nor the turn-based lassitude (replaced
    // by the war weariness when DP1 rules are on).
    let legacy = |proposal: &Proposal| -> (Vec<(String, i32)>, bool) {
        let eval = diplomacy::evaluate(state, data, proposer, recipient, proposal);
        let hard = eval.reasons.iter().any(|(_, v)| *v <= -100);
        let reasons = eval
            .reasons
            .into_iter()
            .filter(|(t, v)| {
                t != "Attitude" && *v > -100 && !(rules.enabled && t == "Lassitude de la guerre")
            })
            .collect();
        (reasons, hard)
    };
    match article {
        Article::Peace | Article::Truce { .. } => {
            let (mut r, _) = legacy(&Proposal::Peace {
                provinces: Vec::new(),
                tribute: 0,
            });
            if let Article::Truce { turns } = article {
                if *turns < diplomacy::TRUCE_TURNS {
                    r.push(("Trêve courte, sans engagement".to_owned(), 4));
                }
            }
            reasons.extend(r);
        }
        Article::Alliance => {
            let (r, hard) = legacy(&Proposal::Alliance);
            reasons.extend(r);
            if hard && !state.is_at_war(proposer, recipient) {
                blocked = Some("alliance impossible".to_owned());
            }
        }
        Article::Marriage { character, spouse } => {
            let (r, hard) = legacy(&Proposal::Marriage {
                character: character.clone(),
                spouse: spouse.clone(),
            });
            reasons.extend(r);
            if hard {
                blocked = Some("mariage impossible".to_owned());
            }
        }
        Article::Vassalage { giver } => {
            if *giver == Party::Recipient {
                let (r, hard) = legacy(&Proposal::Vassalage);
                reasons.extend(r);
                if hard {
                    blocked = Some("vassalité refusée : pas assez puissant".to_owned());
                }
            } else {
                reasons.push(("Hommage d'un nouveau vassal".to_owned(), 25));
                let income = state.factions[proposer].income_last_turn.max(0);
                reasons.push((
                    "Tribut du vassal".to_owned(),
                    gold_points(income * data.feudal_rules.vassal_tribute_percent / 100 * 20),
                ));
            }
        }
        Article::MilitaryAccess { giver } => {
            if *giver == Party::Recipient {
                reasons.push(("Passage d'armées étrangères".to_owned(), -10));
                if diplomacy::rivals(state, recipient).contains(proposer) {
                    reasons.push(("Armées d'un rival".to_owned(), -20));
                }
                if state.is_allied(recipient, proposer) {
                    reasons.push(("Entre alliés".to_owned(), 8));
                }
            } else {
                reasons.push(("Libre passage de nos armées".to_owned(), 6));
            }
        }
        Article::TradeAgreement => {
            reasons.push(("Commerce".to_owned(), 6));
            // C5: the agreement raises the routes linking our marketplaces.
            let routes = crate::trade::common_routes(state, data, proposer, recipient) as i32;
            if routes > 0 {
                reasons.push((
                    "Routes commerciales communes".to_owned(),
                    (4 * routes).min(12),
                ));
            }
            let partner = state.factions[proposer].income_last_turn.max(0);
            reasons.push((
                "Richesse du partenaire".to_owned(),
                ((partner / 1500) as i32).min(10),
            ));
            if diplomacy::rivals(state, recipient).contains(proposer) {
                reasons.push(("Enrichir un rival".to_owned(), -12));
            }
            let embargo = state.factions[proposer].embargoes.contains(recipient)
                || state.factions[recipient].embargoes.contains(proposer);
            if embargo {
                reasons.push(("Embargo en cours".to_owned(), -15));
            }
        }
        Article::Tribute {
            per_season,
            seasons,
            ..
        } => {
            let total = per_season.saturating_mul(i64::from(*seasons));
            let points = gold_points(total * 4 / 5);
            reasons.push((
                if gives {
                    "Tribut à verser".to_owned()
                } else {
                    "Tribut reçu".to_owned()
                },
                sign * points,
            ));
            if gives && *per_season * 2 > state.factions[recipient].income_last_turn.max(1) {
                reasons.push(("Tribut ruineux".to_owned(), -15));
            }
        }
        Article::Gold { amount, .. } => {
            let mut points = gold_points(*amount);
            if !gives && state.factions[recipient].treasury < 0 {
                points = points * 3 / 2;
                reasons.push(("Trésor vide".to_owned(), 5));
            }
            reasons.push((
                if gives {
                    "Or à verser".to_owned()
                } else {
                    "Or reçu".to_owned()
                },
                sign * points,
            ));
        }
        Article::CedeProvince { province, .. } => {
            let taker = if gives { proposer } else { recipient };
            let giver_id = if gives { recipient } else { proposer };
            let capital = state.factions[giver_id].capital == *province;
            let mut cost = if capital {
                rules.capital_cost
            } else {
                rules.province_cost
            };
            if gives {
                if data.ai_diplomacy.peace.keep_capital && capital {
                    blocked = Some(format!(
                        "{} ne cédera jamais sa capitale",
                        faction_name(data, recipient)
                    ));
                }
                let occupied = state.controls_province(taker, province);
                if occupied {
                    cost = cost * rules.occupied_cost_percent / 100;
                }
                reasons.push((
                    if occupied {
                        "Province déjà occupée par l'ennemi".to_owned()
                    } else {
                        "Perte d'une province".to_owned()
                    },
                    -cost,
                ));
                let claimed = state.factions[recipient].claims.iter().any(|c| {
                    c.province.as_ref() == Some(province)
                        && c.kind == data_model::ClaimKind::Province
                });
                if claimed {
                    reasons.push(("Terre revendiquée de longue date".to_owned(), -5));
                }
            } else {
                reasons.push((
                    "Gain d'une province".to_owned(),
                    rules.province_cost * 3 / 4,
                ));
                let wanted = is_war_goal(state, recipient, proposer, province)
                    || diplomacy::claimed_provinces(state, recipient).contains(province);
                if wanted {
                    reasons.push(("Terre convoitée".to_owned(), rules.war_goal_bonus));
                }
                if state.controls_province(recipient, province) {
                    reasons.push(("Déjà tenue par nos troupes".to_owned(), 5));
                }
            }
        }
        Article::DemandTitle { title, .. } => {
            // The title's own provinces held by the giver, plus its rank.
            let giver_id = if gives { recipient } else { proposer };
            let capital = &state.factions[giver_id].capital;
            let provinces: i32 = data
                .titles
                .get(title)
                .map(|t| {
                    t.de_jure_provinces
                        .iter()
                        .filter(|p| state.province_owner(p) == Some(giver_id))
                        .map(|p| {
                            if p == capital {
                                rules.capital_cost
                            } else {
                                rules.province_cost
                            }
                        })
                        .sum()
                })
                .unwrap_or(0);
            let value = provinces + data.feudal_rules.title_loss_penalty;
            if gives {
                reasons.push(("Perte d'un titre".to_owned(), -value));
            } else {
                reasons.push(("Gain d'un titre".to_owned(), value * 3 / 4));
            }
        }
        Article::CedeSettlement { settlement, .. } => {
            let taker = if gives { proposer } else { recipient };
            let occupied = state
                .settlements
                .get(settlement)
                .is_some_and(|s| &s.controller == taker);
            let mut cost = rules.settlement_cost;
            if occupied {
                cost = cost * rules.occupied_cost_percent / 100;
            }
            reasons.push((
                if gives {
                    "Perte d'une place".to_owned()
                } else {
                    "Gain d'une place".to_owned()
                },
                sign * cost.max(1),
            ));
        }
        Article::ReleaseCaptive { character, .. } => {
            let ransom = crate::ransom::ransom_amount(state, data, character);
            let high = state
                .factions
                .values()
                .any(|f| f.ruler.as_ref() == Some(character) || f.heir.as_ref() == Some(character));
            let mut points = ((ransom / lpp) as i32).clamp(3, 40);
            if high {
                points += 10;
            }
            reasons.push((
                if gives {
                    "Libérer un captif sans rançon".to_owned()
                } else {
                    "Retour d'un des nôtres".to_owned()
                },
                sign * points,
            ));
        }
        Article::Hostage { character, .. } => {
            let heir = state
                .factions
                .values()
                .any(|f| f.heir.as_ref() == Some(character));
            if gives {
                reasons.push(("Livrer un otage".to_owned(), if heir { -30 } else { -12 }));
            } else {
                reasons.push((
                    "Otage en gage de parole".to_owned(),
                    if heir { 20 } else { 10 },
                ));
            }
        }
    }
    reasons.retain(|(_, v)| *v != 0);
    let value = reasons.iter().map(|(_, v)| v).sum();
    ArticleValue {
        label,
        value,
        reasons,
        blocked,
    }
}

/// French label of an article from the proposer's point of view.
pub fn article_label(
    state: &CampaignState,
    data: &GameData,
    proposer: &FactionId,
    recipient: &FactionId,
    article: &Article,
) -> String {
    let who = |p: &Party| faction_name(data, party_id(*p, proposer, recipient));
    let to = |p: &Party| faction_name(data, party_id(p.other(), proposer, recipient));
    match article {
        Article::Peace => format!(
            "Paix (trêve de {} ans)",
            (if rules(data).enabled {
                rules(data).peace_truce_turns
            } else {
                diplomacy::TRUCE_TURNS
            } / 4)
                .max(1)
        ),
        Article::Truce { turns } => format!("Trêve de {} an(s)", (turns / 4).max(1)),
        Article::Alliance => "Alliance défensive et offensive".to_owned(),
        Article::MilitaryAccess { giver } => {
            format!("Accès militaire : {} ouvre ses terres", who(giver))
        }
        Article::TradeAgreement => "Accord commercial".to_owned(),
        Article::Marriage { character, spouse } => format!(
            "Mariage de {} et de {}",
            state.character_name(data, character),
            state.character_name(data, spouse)
        ),
        Article::Tribute {
            giver,
            per_season,
            seasons,
        } => format!(
            "Tribut : {} verse {per_season} livres par saison pendant {seasons} saisons",
            who(giver)
        ),
        Article::Gold { giver, amount } => format!("{} verse {amount} livres", who(giver)),
        Article::CedeProvince { giver, province } => format!(
            "{} cède {} à {}",
            who(giver),
            province_name(data, province),
            to(giver)
        ),
        Article::CedeSettlement { giver, settlement } => format!(
            "{} cède la place de {} à {}",
            who(giver),
            settlement_name(data, settlement),
            to(giver)
        ),
        Article::Vassalage { giver } => {
            format!("{} devient vassal de {}", who(giver), to(giver))
        }
        Article::ReleaseCaptive { giver, character } => format!(
            "{} libère {}",
            who(giver),
            state.character_name(data, character)
        ),
        Article::Hostage { giver, character } => format!(
            "{} livre {} en otage",
            who(giver),
            state.character_name(data, character)
        ),
        Article::DemandTitle { giver, title } => format!(
            "{} remet le titre {} à {}",
            who(giver),
            data.titles
                .get(title)
                .map_or_else(|| title.to_string(), |t| t.name.display.clone()),
            to(giver)
        ),
    }
}

/// Summary of a treaty (journal, offers).
pub fn treaty_text(
    state: &CampaignState,
    data: &GameData,
    proposer: &FactionId,
    recipient: &FactionId,
    articles: &[Article],
) -> String {
    let list: Vec<String> = articles
        .iter()
        .map(|a| article_label(state, data, proposer, recipient, a))
        .collect();
    format!(
        "Traité entre {} et {} : {}.",
        faction_name(data, proposer),
        faction_name(data, recipient),
        list.join(" ; ")
    )
}

// =========================================================================
// Counter-proposal
// =========================================================================

/// What `recipient` would need to accept: the treaty completed with what
/// `proposer` can still give (captives, occupied lands, gold, tribute), or
/// stripped of its costliest demands. `None` if nothing reaches even odds.
pub fn counter_proposal(
    state: &CampaignState,
    data: &GameData,
    proposer: &FactionId,
    recipient: &FactionId,
    articles: &[Article],
) -> Option<Vec<Article>> {
    let rules = rules(data);
    let mut current: Vec<Article> = articles.to_vec();
    // Drop what cannot be executed at all.
    current.retain(|a| {
        check_treaty(state, data, proposer, recipient, std::slice::from_ref(a)).is_ok()
            || a.ends_war()
    });
    let judge = |list: &[Article]| evaluate_treaty(state, data, proposer, recipient, list);
    let mut verdict = judge(&current);
    let target = COUNTER_TARGET_CHANCE;
    let at_war = state.is_at_war(proposer, recipient);
    let me = &state.factions[proposer];
    // 1. Captives of the recipient held by the proposer.
    let captives: Vec<CharacterId> = state
        .characters
        .iter()
        .filter(|(_, c)| {
            c.alive && c.captive && c.captor.as_ref() == Some(proposer) && &c.faction == recipient
        })
        .map(|(id, _)| id.clone())
        .collect();
    for character in captives {
        if verdict.chance >= target {
            break;
        }
        let article = Article::ReleaseCaptive {
            giver: Party::Proposer,
            character,
        };
        if !current.contains(&article) {
            current.push(article);
            verdict = judge(&current);
        }
    }
    // 2. Our provinces the enemy already holds.
    if at_war {
        let capital = me.capital.clone();
        let mut lost: Vec<ProvinceId> = state
            .provinces
            .keys()
            .filter(|p| {
                state.province_owner(p) == Some(proposer)
                    && state.controls_province(recipient, p)
                    && **p != capital
            })
            .cloned()
            .collect();
        lost.sort();
        for province in lost {
            if verdict.chance >= target {
                break;
            }
            let article = Article::CedeProvince {
                giver: Party::Proposer,
                province,
            };
            if !current.contains(&article) {
                current.push(article);
                if check_treaty(state, data, proposer, recipient, &current).is_err() {
                    current.pop();
                    continue;
                }
                verdict = judge(&current);
            }
        }
    }
    // 3. Drop our costliest demands, one by one.
    while verdict.chance < target {
        let worst = current
            .iter()
            .enumerate()
            .filter(|(_, a)| a.giver() == Some(Party::Recipient))
            .map(|(i, _)| (i, verdict.articles.get(i).map_or(0, |v| v.value)))
            .filter(|(_, v)| *v < 0)
            .min_by_key(|(i, v)| (*v, *i));
        let Some((index, _)) = worst else {
            break;
        };
        // Try gold first when it would suffice without dropping.
        current.remove(index);
        verdict = judge(&current);
    }
    // 4. Gold, then tribute, for the remaining gap.
    if verdict.chance < target && verdict.blocked.is_none() {
        let gap = score_for(data, target) - verdict.score;
        let lpp = rules.livres_per_point.max(1);
        let already_gold = current.iter().any(|a| {
            matches!(
                a,
                Article::Gold {
                    giver: Party::Proposer,
                    ..
                }
            )
        });
        let treasury = me.treasury.max(0);
        let max_points = i64::from(rules.max_gold_points);
        let wanted = (i64::from(gap.max(1)) * lpp).min(max_points * lpp);
        let amount = (wanted.min(treasury) / 100) * 100;
        if !already_gold && amount >= 100 {
            current.push(Article::Gold {
                giver: Party::Proposer,
                amount,
            });
            verdict = judge(&current);
        }
        if verdict.chance < target {
            let gap = score_for(data, target) - verdict.score;
            let total = i64::from(gap.max(1)) * lpp * 5 / 4;
            let seasons = COUNTER_TRIBUTE_SEASONS;
            let per_season = ((total / i64::from(seasons)) / 50 + 1) * 50;
            let affordable = me.income_last_turn.max(0) / 3;
            if per_season <= affordable {
                current.push(Article::Tribute {
                    giver: Party::Proposer,
                    per_season,
                    seasons,
                });
                verdict = judge(&current);
            }
        }
    }
    (verdict.blocked.is_none() && verdict.chance >= 50 && current != articles).then_some(current)
}

// =========================================================================
// Proposal and application
// =========================================================================

/// `proposer` sends a treaty: an offer to the player, otherwise a roll
/// against the acceptance chance. `Ok(true)` if signed.
pub fn propose_treaty(
    state: &mut CampaignState,
    data: &GameData,
    proposer: &FactionId,
    recipient: &FactionId,
    articles: Vec<Article>,
) -> Result<bool, DiplomacyError> {
    check_treaty(state, data, proposer, recipient, &articles)?;
    let verdict = evaluate_treaty(state, data, proposer, recipient, &articles);
    let roll = answer_roll(state, proposer, recipient);
    let accepted = verdict.blocked.is_none() && roll < verdict.chance;
    if !accepted {
        record(state, data, proposer, recipient, &articles, false);
        let mut reasons = verdict.reasons();
        reasons.sort_by_key(|(_, v)| *v);
        let top: Vec<String> = reasons
            .iter()
            .take(3)
            .map(|(t, v)| format!("{t} ({v:+})"))
            .collect();
        return Err(DiplomacyError::Refused(format!(
            "{} refuse ({} % de chances) : {}",
            faction_name(data, recipient),
            verdict.chance,
            top.join(", ")
        )));
    }
    apply_treaty(state, data, proposer, recipient, &articles)?;
    Ok(true)
}

fn record(
    state: &mut CampaignState,
    data: &GameData,
    proposer: &FactionId,
    recipient: &FactionId,
    articles: &[Article],
    accepted: bool,
) {
    // Refusals between two AIs are not worth a line of history.
    let player = state.player_faction.clone();
    if !accepted && proposer != &player && recipient != &player {
        return;
    }
    let text = treaty_text(state, data, proposer, recipient, articles);
    let keys: Vec<String> = articles.iter().map(|a| a.key().to_owned()).collect();
    let turn = state.turn;
    for (me, other, proposed) in [(proposer, recipient, true), (recipient, proposer, false)] {
        if let Some(f) = state.factions.get_mut(me) {
            f.ledger.history.push(TreatyRecord {
                turn,
                with: other.clone(),
                proposed,
                accepted,
                articles: keys.clone(),
                text_fr: text.clone(),
            });
            let excess = f.ledger.history.len().saturating_sub(HISTORY_LENGTH);
            f.ledger.history.drain(..excess);
        }
    }
}

/// Executes a signed treaty (peace first, then transfers).
pub fn apply_treaty(
    state: &mut CampaignState,
    data: &GameData,
    proposer: &FactionId,
    recipient: &FactionId,
    articles: &[Article],
) -> Result<(), DiplomacyError> {
    check_treaty(state, data, proposer, recipient, articles)?;
    record(state, data, proposer, recipient, articles, true);
    let text = treaty_text(state, data, proposer, recipient, articles);
    let mut ordered: Vec<&Article> = articles.iter().collect();
    ordered.sort_by_key(|a| !a.ends_war());
    for article in ordered {
        let giver = article
            .giver()
            .map(|g| party_id(g, proposer, recipient).clone());
        let taker = article
            .giver()
            .map(|g| party_id(g.other(), proposer, recipient).clone());
        match article {
            Article::Peace => {
                let truce = if rules(data).enabled {
                    rules(data).peace_truce_turns
                } else {
                    diplomacy::TRUCE_TURNS
                };
                state.make_peace(data, proposer, recipient, &[], 0, truce.max(1));
            }
            Article::Truce { turns } => {
                state.make_peace(data, proposer, recipient, &[], 0, (*turns).max(1));
            }
            Article::Alliance => {
                if !state.is_allied(proposer, recipient) {
                    state.apply_proposal(data, proposer, recipient, &Proposal::Alliance)?;
                }
            }
            Article::Marriage { character, spouse } => {
                state.apply_proposal(
                    data,
                    proposer,
                    recipient,
                    &Proposal::Marriage {
                        character: character.clone(),
                        spouse: spouse.clone(),
                    },
                )?;
            }
            Article::Vassalage { .. } => {
                let (vassal, lord) = (giver.expect("giver"), taker.expect("taker"));
                state.apply_proposal(data, &lord, &vassal, &Proposal::Vassalage)?;
            }
            Article::MilitaryAccess { .. } => {
                let (g, t) = (giver.expect("giver"), taker.expect("taker"));
                state
                    .factions
                    .get_mut(&g)
                    .expect("checked")
                    .ledger
                    .military_access
                    .insert(t);
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
                let holder = taker.expect("taker");
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
                        from: giver.expect("giver"),
                        until_turn,
                    });
            }
        }
    }
    state.add_modifier(recipient, proposer, 5, "Traité signé", 20);
    state.add_modifier(proposer, recipient, 5, "Traité signé", 20);
    state.push_order_event(GameEvent::new(EventKind::Diplomacy, text).faction(proposer));
    Ok(())
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

// =========================================================================
// Turn phase: war goals, weariness, tributes, hostages, broken treaties
// =========================================================================

/// War goals of `faction` against `enemy`: provinces it claims, then the
/// enemy's provinces bordering its own (never the capital when capitals are
/// kept), at most `war_goal_count`.
pub fn compute_war_goals(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    enemy: &FactionId,
) -> Vec<ProvinceId> {
    let rules = rules(data);
    let claimed = diplomacy::claimed_provinces(state, faction);
    let capital = state.factions.get(enemy).map(|f| f.capital.clone());
    let keep_capital = data.ai_diplomacy.peace.keep_capital;
    let throne = diplomacy::claim_stakes(state, faction, enemy).throne;
    let mut candidates: Vec<(u8, ProvinceId)> = state
        .provinces
        .keys()
        .filter(|p| state.province_owner(p) == Some(enemy))
        .filter(|p| !(keep_capital && Some(*p) == capital.as_ref()))
        .filter_map(|p| {
            let border = crate::movement::land_neighbors(data, p)
                .iter()
                .any(|n| state.controls_province(faction, n));
            let rank = if claimed.contains(p) {
                0
            } else if border && throne {
                1
            } else if border {
                2
            } else {
                return None;
            };
            Some((rank, p.clone()))
        })
        .collect();
    candidates.sort();
    candidates
        .into_iter()
        .take(rules.war_goal_count)
        .map(|(_, p)| p)
        .collect()
}

/// Turn phase (called from `resolve_diplomacy`).
pub(crate) fn resolve_negotiation(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let rules = rules(data).clone();
    let ids: Vec<FactionId> = state.factions.keys().cloned().collect();
    // Broken treaties: war between parties ends their standing agreements.
    for id in &ids {
        let at_war = state.factions[id].at_war_with.clone();
        let betrayed: Vec<HostagePledge> = state.factions[id]
            .ledger
            .hostages
            .iter()
            .filter(|h| at_war.contains(&h.from))
            .cloned()
            .collect();
        // The broken word (`HOSTAGE_BETRAYAL_REASON`) weighs on the giver
        // only when it declared the war (`declare_war`).
        for pledge in betrayed {
            // Hostages of an enemy become plain prisoners (ransom rules).
            if let Some(c) = state
                .characters
                .get_mut(&pledge.character)
                .filter(|c| c.captor.as_ref() == Some(id))
            {
                c.ransom_terms = None;
            }
        }
        let f = state.factions.get_mut(id).expect("listed");
        f.ledger.trade_agreements.retain(|o| !at_war.contains(o));
        f.ledger.military_access.retain(|o| !at_war.contains(o));
        f.ledger.tributes.retain(|t| !at_war.contains(&t.to));
        // Hostages of an enemy become plain prisoners (ransom rules).
        f.ledger.hostages.retain(|h| !at_war.contains(&h.from));
        f.ledger.war_goals.retain(|e, _| at_war.contains(e));
    }
    // Tributes.
    let turn = state.turn;
    for id in &ids {
        // A vanished faction pays nothing more.
        if !state.factions[id].alive {
            state
                .factions
                .get_mut(id)
                .expect("listed")
                .ledger
                .tributes
                .clear();
            continue;
        }
        let dues: Vec<TributeDue> = state.factions[id].ledger.tributes.clone();
        for due in &dues {
            if !state.factions.get(&due.to).is_some_and(|f| f.alive) {
                continue;
            }
            state.factions.get_mut(id).expect("listed").treasury -= due.per_season;
            state.factions.get_mut(&due.to).expect("alive").treasury += due.per_season;
        }
        let f = state.factions.get_mut(id).expect("listed");
        f.ledger.tributes.retain(|t| t.until_turn > turn + 1);
    }
    // Hostages come home at the end of their term.
    for id in &ids {
        let (home, keep): (Vec<HostagePledge>, Vec<HostagePledge>) = state.factions[id]
            .ledger
            .hostages
            .iter()
            .cloned()
            .partition(|h| h.until_turn <= turn + 1);
        state.factions.get_mut(id).expect("listed").ledger.hostages = keep;
        for pledge in home {
            let still_held = state
                .characters
                .get(&pledge.character)
                .is_some_and(|c| c.alive && c.captive && c.captor.as_ref() == Some(id));
            if still_held {
                crate::chronicle::release_character(state, data, &pledge.character, 0, events);
            }
        }
        // A hostage ransomed or dead is no longer a pledge.
        let characters = &state.characters;
        let f = state.factions.get_mut(id).expect("listed");
        f.ledger.hostages.retain(|h| {
            characters
                .get(&h.character)
                .is_some_and(|c| c.alive && c.captive && c.captor.as_ref() == Some(id))
        });
    }
    if !rules.enabled {
        return;
    }
    // War goals of new wars.
    for id in &ids {
        if !state.factions[id].alive || is_rebels(id) {
            continue;
        }
        let enemies: Vec<FactionId> = state.factions[id]
            .at_war_with
            .iter()
            .filter(|e| !is_rebels(e))
            .filter(|e| !state.factions[id].ledger.war_goals.contains_key(*e))
            .cloned()
            .collect();
        for enemy in enemies {
            let goals = compute_war_goals(state, data, id, &enemy);
            state
                .factions
                .get_mut(id)
                .expect("listed")
                .ledger
                .war_goals
                .insert(enemy, goals);
        }
    }
    // War weariness.
    for id in &ids {
        if !state.factions[id].alive || is_rebels(id) {
            continue;
        }
        // Only wars that press on us tire the realm: a bordering enemy, or
        // one holding our lands or beating us.
        let enemies: Vec<FactionId> = state.factions[id]
            .at_war_with
            .iter()
            .filter(|e| !is_rebels(e))
            .filter(|e| {
                (state.are_neighbors(data, id, e)
                    && state.faction_power(e) >= 0.25 * state.faction_power(id))
                    || state.war_score(data, id, e) < 0
                    || state.provinces.keys().any(|p| {
                        state.province_owner(p) == Some(id) && state.controls_province(e, p)
                    })
            })
            .cloned()
            .collect();
        let current = state.factions[id].ledger.weariness;
        let next = if enemies.is_empty() {
            current.saturating_sub(rules.weariness_recovery)
        } else {
            let mut gain = rules.weariness_per_war * enemies.len().min(2) as u32;
            if enemies.iter().any(|e| state.war_score(data, id, e) < -20) {
                gain += rules.weariness_losing;
            }
            let occupied = state
                .provinces
                .keys()
                .filter(|p| {
                    state.province_owner(p) == Some(id)
                        && state
                            .province_controller(p)
                            .is_some_and(|c| enemies.contains(c))
                })
                .count()
                .min(3) as u32;
            gain += rules.weariness_occupied * occupied;
            if state.factions[id].treasury < 0 {
                gain += rules.weariness_bankrupt;
            }
            (current + gain.min(rules.max_weariness_gain)).min(100)
        };
        let ledger = &mut state.factions.get_mut(id).expect("listed").ledger;
        ledger.weariness = next;
        ledger.weariness_unrest = next / rules.weariness_unrest_divisor.max(1);
    }
}

/// A pretender to a throne goes to war on credit (Edward III and the
/// Bardi): one season of upkeep in the chest is enough, instead of the
/// two of `diplomacy::war_ready`.
pub fn pretender_ready(state: &CampaignState, data: &GameData, faction: &FactionId) -> bool {
    let Some(me) = state.factions.get(faction) else {
        return false;
    };
    let pretender = me
        .claims
        .iter()
        .any(|c| c.kind == data_model::ClaimKind::Throne);
    let ruler_free = me
        .ruler
        .as_ref()
        .and_then(|r| state.characters.get(r))
        .is_none_or(|r| !r.captive);
    rules(data).enabled
        && pretender
        && !me.regency
        && ruler_free
        && me.treasury > 0
        && me.treasury >= me.upkeep_last_turn.max(0)
}

/// Does `owner` let `army_faction`'s armies cross its lands?
pub fn has_military_access(
    state: &CampaignState,
    owner: &FactionId,
    army_faction: &FactionId,
) -> bool {
    owner == army_faction
        || state.is_allied(owner, army_faction)
        || state
            .factions
            .get(owner)
            .is_some_and(|f| f.ledger.military_access.contains(army_faction))
}

// =========================================================================
// AI: treaty peace
// =========================================================================

/// Peace treaty `faction` sends one of its enemies this turn, if any (DP1):
/// the winner demands the provinces it holds (war goals first) as long as
/// the loser would still likely sign; a weary or beaten side buys peace
/// with what the enemy already holds, gold and tribute.
pub fn plan_peace(state: &CampaignState, data: &GameData, faction: &FactionId) -> Option<Order> {
    let rules = rules(data);
    let me = state.factions.get(faction)?;
    let min_chance = rules.ai_min_chance;
    let mut enemies: Vec<&FactionId> = me.at_war_with.iter().filter(|e| !is_rebels(e)).collect();
    // The strongest enemy first.
    enemies.sort_by(|a, b| {
        state
            .faction_power(b)
            .total_cmp(&state.faction_power(a))
            .then_with(|| a.cmp(b))
    });
    let cornered = diplomacy::is_cornered(state, data, faction);
    for enemy in enemies {
        let enemy_is_player = enemy == &state.player_faction;
        // A campaign season does not end a war: no treaty before
        // `min_war_turns`, unless the realm is down to its last lands.
        let started = me.war_started.get(enemy).copied().unwrap_or(0);
        let score = state.war_score(data, faction, enemy);
        // EQ6: a cornered crown first fights a pretender to its throne
        // (bought off, he would be back after the truce); against anyone
        // else it treats at once (the Scots after Halidon Hill).
        let sues_early = cornered
            && (!data.ai_diplomacy.peace.cornered_waits_for_defeat
                || !diplomacy::claim_stakes(state, enemy, faction).throne
                || score <= diplomacy::SURRENDER_WAR_SCORE);
        if !sues_early && state.turn < started + rules.min_war_turns {
            continue;
        }
        let chance = |articles: &[Article]| -> u8 {
            evaluate_treaty(state, data, faction, enemy, articles).chance
        };
        // Would we sign a white peace ourselves (enemy's view of us)?
        let white = vec![Article::Peace];
        let we_accept_white = evaluate_treaty(state, data, enemy, faction, &white).chance >= 50;
        if score >= rules.demand_score {
            // Winner: demand what we hold of theirs, war goals first.
            let goals = me.ledger.war_goals.get(enemy).cloned().unwrap_or_default();
            let mut held: Vec<(bool, ProvinceId)> = state
                .provinces
                .keys()
                .filter(|p| {
                    state.province_owner(p) == Some(enemy) && state.controls_province(faction, p)
                })
                .map(|p| (!goals.contains(p), p.clone()))
                .collect();
            // The war goals themselves, even unheld: a clear victory buys
            // the lands it was fought for (Brétigny); they come before the
            // other provinces held (ADR 0025 § 5).
            for goal in &goals {
                if state.province_owner(goal) == Some(enemy) && !held.iter().any(|(_, p)| p == goal)
                {
                    held.push((false, goal.clone()));
                }
            }
            held.sort();
            let mut treaty = white.clone();
            for (_, province) in held {
                treaty.push(Article::CedeProvince {
                    giver: Party::Recipient,
                    province,
                });
                let ok = check_treaty(state, data, faction, enemy, &treaty).is_ok()
                    && (enemy_is_player || chance(&treaty) >= min_chance);
                if !ok {
                    treaty.pop();
                }
            }
            if treaty.len() > 1 {
                return Some(treaty_order(enemy, treaty));
            }
            if we_accept_white && (enemy_is_player || chance(&white) >= min_chance) {
                return Some(treaty_order(enemy, white));
            }
            continue;
        }
        if we_accept_white && !enemy_is_player && chance(&white) >= min_chance {
            return Some(treaty_order(enemy, white));
        }
        // Only a beaten or exhausted crown buys its peace (lands, gold,
        // tribute); otherwise the war goes on until one side prevails.
        let beaten = score <= 2 * diplomacy::SURRENDER_WAR_SCORE
            || me.ledger.weariness >= rules.sue_weariness
            || cornered;
        if beaten {
            if let Some(treaty) = counter_proposal(state, data, faction, enemy, &white) {
                if enemy_is_player || chance(&treaty) >= min_chance {
                    return Some(treaty_order(enemy, treaty));
                }
            }
            if enemy_is_player && we_accept_white {
                return Some(treaty_order(enemy, white));
            }
        }
    }
    None
}

fn treaty_order(target: &FactionId, articles: Vec<Article>) -> Order {
    Order::ProposeTreaty {
        target: target.clone(),
        articles,
    }
}

/// Is the relation of `a` to `b` one where treaties of friendship apply?
pub fn friendly(state: &CampaignState, a: &FactionId, b: &FactionId) -> bool {
    matches!(
        state.relation(a, b),
        RelationKind::Peace
            | RelationKind::Alliance
            | RelationKind::Vassal
            | RelationKind::Suzerain
    )
}
