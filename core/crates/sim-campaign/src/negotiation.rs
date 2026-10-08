//! Lot DP1 (ADR 0025): diplomatic negotiation.
//!
//! Every diplomatic proposal is a [`Treaty`] of [`Article`]s (ADR 0202): an
//! alliance, a feudal call or a ten-article peace alike. Each article checks
//! itself (`check`), values itself for the recipient (`value`), executes
//! itself (`apply`) and words itself (`label`); the treaty is their sum plus
//! the general considerations of `context`.
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
    TitleId, TreatyWeights,
};
use serde::{Deserialize, Serialize};

mod apply;
mod check;
mod context;
mod label;
mod reasons;
mod value;

pub use reasons::ReasonList;

use crate::diplomacy::{self, DiplomacyError, RelationKind};
use crate::events::{EventKind, GameEvent};
use crate::orders::Order;
use crate::plan_cache::PlanCache;
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
/// Opinion reason of a signed treaty (capped motive `treaty`, RS-C).
pub const TREATY_REASON: &str = "Traité signé";

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
    /// White peace of `turns` seasons obtained through the pope's or a
    /// herald's mediation: the recipient owes it a good turn.
    Mediation {
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
    /// Great Schism: the recipient switches to `religion`. Not a bargain:
    /// created by the core, answered by the player.
    Obedience {
        religion: data_model::ReligionId,
    },
    /// FE (§ 4.3): the proposer, a direct vassal of the recipient, is
    /// attacked by `aggressor` and calls for protection. Accept: intervene;
    /// refuse or let expire: shirk. Only sent to the player.
    Protection {
        aggressor: FactionId,
    },
    /// FE (§ 4.3.5): private war of `attacker` against `target`, both direct
    /// vassals of the recipient. Accept: impose peace; refuse or let expire:
    /// let be; `arbitrate` also takes a side. Only sent to the player.
    Arbitration {
        attacker: FactionId,
        target: FactionId,
    },
    /// ADR 0146: the proposer, lord of both the player and `target`, summons
    /// the player to end the private war it declared on `target`. Accept:
    /// imposed peace; refuse or let expire: the war goes on, at a cost in
    /// loyalty. Only sent to the player.
    PeaceSummons {
        target: FactionId,
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
        matches!(
            self,
            Article::Peace | Article::Truce { .. } | Article::Mediation { .. }
        )
    }

    /// A lord's order created by the core (feudal call, obedience), which
    /// no faction proposes and which carries no treaty bookkeeping.
    pub fn is_imposed(&self) -> bool {
        matches!(
            self,
            Article::Obedience { .. }
                | Article::Protection { .. }
                | Article::Arbitration { .. }
                | Article::PeaceSummons { .. }
        )
    }

    /// Key of the article kind (`serde` tag).
    pub fn key(&self) -> &'static str {
        match self {
            Article::Peace => "peace",
            Article::Truce { .. } => "truce",
            Article::Mediation { .. } => "mediation",
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
            Article::Obedience { .. } => "obedience",
            Article::Protection { .. } => "protection",
            Article::Arbitration { .. } => "arbitration",
            Article::PeaceSummons { .. } => "peace_summons",
        }
    }
}

/// A treaty: what one faction proposes to another, made of articles. A
/// simple alliance or a lord's call is a treaty of one article.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Treaty {
    pub articles: Vec<Article>,
}

impl Treaty {
    pub fn new(articles: Vec<Article>) -> Self {
        Self { articles }
    }

    pub fn single(article: Article) -> Self {
        Self::new(vec![article])
    }

    /// Peace ending the war, with the `provinces` that change hands (each
    /// passes from its owner to the other party) and a `tribute` paid once:
    /// by the recipient to the proposer if positive, the reverse if negative.
    pub fn peace_terms(
        state: &CampaignState,
        proposer: &FactionId,
        provinces: &[ProvinceId],
        tribute: i64,
    ) -> Self {
        let mut articles = vec![Article::Peace];
        for province in provinces {
            let giver = if state.province_owner(province) == Some(proposer) {
                Party::Proposer
            } else {
                Party::Recipient
            };
            articles.push(Article::CedeProvince {
                giver,
                province: province.clone(),
            });
        }
        if tribute != 0 {
            articles.push(Article::Gold {
                giver: if tribute > 0 {
                    Party::Recipient
                } else {
                    Party::Proposer
                },
                amount: tribute.abs(),
            });
        }
        Self::new(articles)
    }

    /// Orders of a lord (feudal call, obedience) are answered, not bargained.
    pub fn is_imposed(&self) -> bool {
        self.articles.iter().any(Article::is_imposed)
    }

    /// Feudal calls are created by the core, never proposed by a faction.
    pub fn is_feudal_call(&self) -> bool {
        self.articles.iter().any(|a| {
            matches!(
                a,
                Article::Protection { .. }
                    | Article::Arbitration { .. }
                    | Article::PeaceSummons { .. }
            )
        })
    }

    pub fn is_obedience(&self) -> bool {
        self.articles
            .iter()
            .any(|a| matches!(a, Article::Obedience { .. }))
    }

    /// Key of the treaty for the UI: that of its article when it has one.
    pub fn kind(&self) -> &'static str {
        match self.articles.as_slice() {
            [only] => only.key(),
            _ => "treaty",
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
    pub reasons: ReasonList,
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
    pub context: ReasonList,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocked: Option<String>,
}

impl TreatyEvaluation {
    /// Every reason weighed, article by article, after the general
    /// considerations.
    pub fn detailed_reasons(&self) -> ReasonList {
        let mut reasons = self.context.clone();
        for article in &self.articles {
            reasons.extend(article.reasons.clone());
        }
        reasons
    }

    /// Flat list of reasons (context first), for [`diplomacy::Evaluation`].
    pub fn reasons(&self) -> ReasonList {
        let mut reasons = self.context.clone();
        for article in &self.articles {
            reasons.push(article.label.clone(), article.value);
        }
        if let Some(blocked) = &self.blocked {
            reasons.push(blocked.clone(), -100);
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

fn party_id<'a>(party: Party, proposer: &'a FactionId, recipient: &'a FactionId) -> &'a FactionId {
    match party {
        Party::Proposer => proposer,
        Party::Recipient => recipient,
    }
}

/// Score at or above which a proposal of the player is accepted (ADR 0182).
pub const ACCEPT_SCORE: i32 = 0;

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
// Validation and evaluation (the treaty is the sum of its articles)
// =========================================================================

/// The world and the two parties a treaty is judged or checked in.
struct Deal<'a> {
    pub state: &'a CampaignState,
    pub cache: &'a PlanCache<'a>,
    pub data: &'a GameData,
    pub proposer: &'a FactionId,
    pub recipient: &'a FactionId,
    pub articles: &'a [Article],
}

impl<'a> Deal<'a> {
    fn new(
        cache: &'a PlanCache<'a>,
        data: &'a GameData,
        proposer: &'a FactionId,
        recipient: &'a FactionId,
        articles: &'a [Article],
    ) -> Self {
        Self {
            state: cache.state(),
            cache,
            data,
            proposer,
            recipient,
            articles,
        }
    }

    pub fn rules(&self) -> &'a NegotiationRules {
        rules(self.data)
    }

    pub fn weights(&self) -> &'a TreatyWeights {
        &self.data.ai_diplomacy.treaty_weights
    }

    /// Faction giving `article` (`None`: mutual).
    pub fn giver(&self, article: &Article) -> Option<&'a FactionId> {
        article
            .giver()
            .map(|g| party_id(g, self.proposer, self.recipient))
    }

    /// Faction receiving `article` (`None`: mutual).
    pub fn taker(&self, article: &Article) -> Option<&'a FactionId> {
        article
            .giver()
            .map(|g| party_id(g.other(), self.proposer, self.recipient))
    }

    /// Articles of the treaty of the same kind and from the same giver as
    /// `article` (itself included).
    pub fn count_from_same_giver(
        &self,
        article: &Article,
        same_kind: fn(&Article) -> bool,
    ) -> usize {
        self.articles
            .iter()
            .filter(|a| same_kind(a) && a.giver() == article.giver())
            .count()
    }

    /// Treaty points of a sum of livres, capped.
    pub fn gold_points(&self, amount: i64) -> i32 {
        let rules = self.rules();
        ((amount / rules.livres_per_point.max(1)) as i32).min(rules.max_gold_points)
    }
}

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
    let cache = PlanCache::new(state);
    let deal = Deal::new(&cache, data, proposer, recipient, articles);
    for (index, article) in articles.iter().enumerate() {
        if articles[..index].contains(article) {
            return Err(DiplomacyError::Refused("article en double".to_owned()));
        }
        article.check(&deal, index)?;
    }
    Ok(())
}

/// How `recipient` values `articles` offered by `proposer`. Pure.
pub fn evaluate_treaty(
    state: &CampaignState,
    data: &GameData,
    proposer: &FactionId,
    recipient: &FactionId,
    articles: &[Article],
) -> TreatyEvaluation {
    evaluate_treaty_with(&PlanCache::new(state), data, proposer, recipient, articles)
}

/// [`evaluate_treaty`] reading the state through the planner's `cache`.
pub fn evaluate_treaty_with(
    cache: &PlanCache,
    data: &GameData,
    proposer: &FactionId,
    recipient: &FactionId,
    articles: &[Article],
) -> TreatyEvaluation {
    let state = cache.state();
    let deal = Deal::new(cache, data, proposer, recipient, articles);
    let values: Vec<ArticleValue> = articles.iter().map(|a| a.value(&deal)).collect();
    let context = context::context_reasons(&deal);
    let blocked = check_treaty(state, data, proposer, recipient, articles)
        .err()
        .map(|e| e.to_string())
        .or_else(|| values.iter().find_map(|v| v.blocked.clone()));
    let score = values.iter().map(|v| v.value).sum::<i32>() + context.total();
    let chance = if blocked.is_some() {
        0
    } else {
        chance_of(data, score)
    };
    TreatyEvaluation {
        chance,
        accept: blocked.is_none() && score >= ACCEPT_SCORE,
        score,
        articles: values,
        context,
        blocked,
    }
}

/// A realm down to this many provinces is not besieged there by an enemy
/// without a claim on them: peace decides its fate (F4, the AI planner's
/// rule; Scotland survives Edward III).
pub const LAST_BASTIONS: usize = 2;

/// LR-11: is the war between `a` and `b` a stalemate before last bastions?
/// One side holds at most [`LAST_BASTIONS`] provinces, the other has no
/// claim on it (so never besieges them), and neither wins clearly (war
/// score under `demand_score` either way).
pub fn bastion_stalemate(
    state: &CampaignState,
    data: &GameData,
    a: &FactionId,
    b: &FactionId,
) -> bool {
    let held = |f: &FactionId| {
        state
            .provinces
            .keys()
            .filter(|p| state.holds_province(f, p))
            .take(LAST_BASTIONS + 1)
            .count()
    };
    let cornered = |small: &FactionId, big: &FactionId| {
        held(small) <= LAST_BASTIONS && !diplomacy::claim_stakes(state, big, small).any()
    };
    (cornered(a, b) || cornered(b, a))
        && state.war_score(data, a, b).abs() < rules(data).demand_score
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
        .map(|a| a.label(state, data, proposer, recipient))
        .collect();
    format!(
        "Traité entre {} et {} : {}.",
        data.faction_name(proposer),
        data.faction_name(recipient),
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
            let affordable = me.last_budget.income.max(0) / 3;
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
    // The player's proposals are deterministic (ADR 0182): accepted iff the
    // score reaches the threshold. Between AIs, a seeded roll remains.
    let accepted = if proposer == &state.player_faction {
        verdict.accept
    } else {
        verdict.blocked.is_none() && answer_roll(state, proposer, recipient) < verdict.chance
    };
    if !accepted {
        record(state, data, proposer, recipient, &articles, false);
        return Err(DiplomacyError::Refused(format!(
            "{} refuse (score {:+}, il en fallait {:+}) : {}",
            data.faction_name(recipient),
            verdict.score,
            ACCEPT_SCORE,
            verdict.reasons().heaviest_objections(3)
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

/// Executes a signed treaty (peace first, then transfers). Orders of a lord
/// (feudal call, obedience) are executed without treaty bookkeeping.
pub fn apply_treaty(
    state: &mut CampaignState,
    data: &GameData,
    proposer: &FactionId,
    recipient: &FactionId,
    articles: &[Article],
) -> Result<(), DiplomacyError> {
    let imposed = articles.iter().any(Article::is_imposed);
    if !imposed {
        check_treaty(state, data, proposer, recipient, articles)?;
        record(state, data, proposer, recipient, articles, true);
    }
    let text = treaty_text(state, data, proposer, recipient, articles);
    let mut ordered: Vec<&Article> = articles.iter().collect();
    ordered.sort_by_key(|a| !a.ends_war());
    let parties = apply::Parties {
        proposer,
        recipient,
    };
    for article in ordered {
        article.apply(state, data, &parties)?;
    }
    if !imposed {
        state.add_capped_modifier(data, recipient, proposer, 5, TREATY_REASON, 20);
        state.add_capped_modifier(data, proposer, recipient, 5, TREATY_REASON, 20);
        state.push_order_event(GameEvent::new(EventKind::Diplomacy, text).faction(proposer));
    }
    Ok(())
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
        if !state.factions[id].alive || id.is_rebels() {
            continue;
        }
        let enemies: Vec<FactionId> = state.factions[id]
            .at_war_with
            .iter()
            .filter(|e| !e.is_rebels())
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
        if !state.factions[id].alive || id.is_rebels() {
            continue;
        }
        // Only wars that press on us tire the realm: a bordering enemy, or
        // one holding our lands or beating us.
        let enemies: Vec<FactionId> = state.factions[id]
            .at_war_with
            .iter()
            .filter(|e| !e.is_rebels())
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
        && me.treasury >= me.last_budget.upkeep().max(0)
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
pub fn plan_peace(cache: &PlanCache, data: &GameData, faction: &FactionId) -> Option<Order> {
    let state = cache.state();
    let rules = rules(data);
    let me = state.factions.get(faction)?;
    let min_chance = rules.ai_min_chance;
    let mut enemies: Vec<&FactionId> = me.at_war_with.iter().filter(|e| !e.is_rebels()).collect();
    // The strongest enemy first.
    enemies.sort_by(|a, b| {
        state
            .faction_power(b)
            .total_cmp(&cache.faction_power(a))
            .then_with(|| a.cmp(b))
    });
    let cornered = diplomacy::is_cornered(state, data, faction);
    for enemy in enemies {
        let enemy_is_player = enemy == &state.player_faction;
        // JR4: an AI-led crusade never treats with the master of its goal.
        if crate::crusade::ai_vow_forbids_peace(state, data, faction, enemy) {
            continue;
        }
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
            evaluate_treaty_with(cache, data, faction, enemy, articles).chance
        };
        // Would we sign a white peace ourselves (enemy's view of us)?
        let white = vec![Article::Peace];
        let we_accept_white =
            evaluate_treaty_with(cache, data, enemy, faction, &white).chance >= 50;
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
