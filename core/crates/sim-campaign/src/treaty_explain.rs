//! Lot DP2 (ADR 0075): readable negotiations, « à la Total War Warhammer 3 ».
//!
//! [`explain_treaty`] turns the recipient's evaluation
//! ([`negotiation::evaluate_treaty`]) into one line per weighted reason
//! (« Ils se méfient de vous −12 », « Accord commercial — routes communes
//! +8 »), a short verdict, and — when a single point blocks the treaty — that
//! point and a counter-offer that removes or lowers it (or, for a general
//! consideration, compensates it with [`negotiation::counter_proposal`]).
//! Pure: the evaluation and the counter-offer are those of DP1.

use data_model::{FactionId, GameData};
use serde::{Deserialize, Serialize};

use crate::diplomacy::faction_name;
use crate::negotiation::{
    self, chance_of, counter_proposal, evaluate_treaty, Article, Party, TreatyEvaluation,
};
use crate::state::CampaignState;

/// Chance (percent) from which a treaty is said to be accepted.
pub const ACCEPT_CHANCE: u8 = 50;

/// One weighted reason of the recipient.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExplanationLine {
    /// Sentence shown to the player (French).
    pub text: String,
    pub value: i32,
    /// Index of the article it belongs to (`None`: general consideration).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub article: Option<usize>,
}

/// The one point that stands between the treaty and its acceptance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Blocker {
    pub text: String,
    pub value: i32,
    /// Index of the blocking article (`None`: a general consideration).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub article: Option<usize>,
}

/// A negotiation made readable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreatyExplanation {
    pub chance: u8,
    pub accept: bool,
    pub score: i32,
    /// Every weighted reason, the heaviest objections first, then the
    /// arguments in favour, strongest first.
    pub lines: Vec<ExplanationLine>,
    /// One-sentence verdict (French).
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocker: Option<Blocker>,
    /// Counter-offer answering the single blocking point.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub counter: Option<Vec<Article>>,
    #[serde(default)]
    pub counter_chance: u8,
    /// What the counter-offer changes (French).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub counter_text: String,
}

/// Sentence of a general consideration, in the recipient's voice.
fn context_sentence(label: &str, value: i32) -> String {
    let text = match (label, value >= 0) {
        ("Attitude", true) => "Ils vous voient d'un bon œil",
        ("Attitude", false) => "Ils ne vous aiment guère",
        ("Confiance", true) => "Ils se fient à votre parole",
        ("Confiance", false) => "Ils se méfient de vous",
        ("Menace de sa puissance", _) => "Ils craignent votre puissance",
        ("Faiblesse du demandeur", _) => "Ils vous jugent trop faible pour compter",
        ("Fatigue de guerre", _) => "Ils sont las de la guerre",
        ("Honneur : ne pas abandonner nos alliés", _) => "Ils refusent d'abandonner leurs alliés",
        ("Prétention à la couronne", _) => "Ils ne renoncent pas à la couronne",
        ("Buts de guerre non atteints", _) => "Leurs buts de guerre ne sont pas atteints",
        ("Prudence", _) => "Prudence devant tout engagement",
        _ => return label.to_owned(),
    };
    text.to_owned()
}

/// Lines of an evaluation (see [`TreatyExplanation::lines`]).
pub fn explanation_lines(verdict: &TreatyEvaluation) -> Vec<ExplanationLine> {
    let mut lines: Vec<ExplanationLine> = verdict
        .context
        .iter()
        .map(|(label, value)| ExplanationLine {
            text: context_sentence(label, *value),
            value: *value,
            article: None,
        })
        .collect();
    for (index, article) in verdict.articles.iter().enumerate() {
        for (reason, value) in &article.reasons {
            lines.push(ExplanationLine {
                text: format!("{} — {}", article.label, reason),
                value: *value,
                article: Some(index),
            });
        }
    }
    lines.retain(|l| l.value != 0);
    // Objections first (most negative first), then arguments (strongest
    // first); ties in the order of the evaluation.
    lines.sort_by_key(|l| (l.value >= 0, if l.value < 0 { l.value } else { -l.value }));
    lines
}

/// Chance of `articles` for the recipient.
fn chance(
    state: &CampaignState,
    data: &GameData,
    proposer: &FactionId,
    recipient: &FactionId,
    articles: &[Article],
) -> TreatyEvaluation {
    evaluate_treaty(state, data, proposer, recipient, articles)
}

/// The same article with a lower amount (gold, tribute), if it has one.
fn scaled(article: &Article, percent: i64) -> Option<Article> {
    let round = |x: i64, step: i64| (x * percent / 100 / step) * step;
    match article {
        Article::Gold { giver, amount } => {
            let amount = round(*amount, 50);
            (amount > 0).then_some(Article::Gold {
                giver: *giver,
                amount,
            })
        }
        Article::Tribute {
            giver,
            per_season,
            seasons,
        } => {
            let per_season = round(*per_season, 10);
            (per_season > 0).then_some(Article::Tribute {
                giver: *giver,
                per_season,
                seasons: *seasons,
            })
        }
        _ => None,
    }
}

/// A counter-offer on the single article at `index`: the demand lowered
/// (gold, tribute: the largest amount they would accept) or dropped.
fn counter_on_article(
    state: &CampaignState,
    data: &GameData,
    proposer: &FactionId,
    recipient: &FactionId,
    articles: &[Article],
    index: usize,
) -> Option<(Vec<Article>, u8, String)> {
    let label = negotiation::article_label(state, data, proposer, recipient, &articles[index]);
    // Lower amounts first: 75 %, 50 %, 25 % of the demand.
    if articles[index].giver() == Some(Party::Recipient) {
        for percent in [75, 50, 25] {
            let Some(lower) = scaled(&articles[index], percent) else {
                break;
            };
            let mut list = articles.to_vec();
            list[index] = lower.clone();
            let verdict = chance(state, data, proposer, recipient, &list);
            if verdict.blocked.is_none() && verdict.chance >= ACCEPT_CHANCE {
                let new_label =
                    negotiation::article_label(state, data, proposer, recipient, &lower);
                return Some((
                    list,
                    verdict.chance,
                    format!("« {label} » ramené à « {new_label} »"),
                ));
            }
        }
    }
    let mut list = articles.to_vec();
    list.remove(index);
    if list.is_empty() {
        return None;
    }
    let verdict = chance(state, data, proposer, recipient, &list);
    (verdict.blocked.is_none() && verdict.chance >= ACCEPT_CHANCE)
        .then(|| (list, verdict.chance, format!("sans « {label} »")))
}

/// The recipient's evaluation of `articles` made readable, with the single
/// blocking point and its counter-offer when there is one. Pure.
pub fn explain_treaty(
    state: &CampaignState,
    data: &GameData,
    proposer: &FactionId,
    recipient: &FactionId,
    articles: &[Article],
) -> TreatyExplanation {
    let verdict = chance(state, data, proposer, recipient, articles);
    let lines = explanation_lines(&verdict);
    let name = faction_name(data, recipient);
    let mut explanation = TreatyExplanation {
        chance: verdict.chance,
        accept: verdict.blocked.is_none() && verdict.chance >= ACCEPT_CHANCE,
        score: verdict.score,
        lines,
        summary: String::new(),
        blocker: None,
        counter: None,
        counter_chance: 0,
        counter_text: String::new(),
    };
    if articles.is_empty() {
        explanation.summary = "Ajoutez des clauses.".to_owned();
        return explanation;
    }
    if explanation.accept {
        let best = explanation
            .lines
            .iter()
            .filter(|l| l.value > 0)
            .max_by_key(|l| l.value);
        explanation.summary = match best {
            Some(line) => format!("{name} accepterait : {} ({:+}).", line.text, line.value),
            None => format!("{name} accepterait."),
        };
        return explanation;
    }
    // A single article that cannot be executed or blocks the treaty.
    let blocked_article = verdict.articles.iter().position(|a| a.blocked.is_some());
    let mut candidates: Vec<(usize, i32)> = verdict
        .articles
        .iter()
        .enumerate()
        .filter(|(_, a)| a.value < 0 || a.blocked.is_some())
        .map(|(i, a)| (i, a.value))
        .collect();
    candidates.sort_by_key(|(i, v)| (*v, *i));
    if let Some(index) = blocked_article {
        candidates.retain(|(i, _)| *i == index);
    }
    let mut solutions = Vec::new();
    for (index, value) in &candidates {
        if let Some(found) = counter_on_article(state, data, proposer, recipient, articles, *index)
        {
            solutions.push((*index, *value, found));
        }
    }
    let threshold = {
        // Lowest score of an acceptance.
        let mut s = verdict.score;
        while chance_of(data, s) < ACCEPT_CHANCE && s < 1000 {
            s += 1;
        }
        s
    };
    let gap = threshold - verdict.score;
    let objections: Vec<&ExplanationLine> =
        explanation.lines.iter().filter(|l| l.value < 0).collect();
    if let Some((index, value, (list, counter_chance, text))) = solutions.into_iter().next() {
        let article = &verdict.articles[index];
        explanation.blocker = Some(Blocker {
            text: article.blocked.clone().map_or_else(
                || article.label.clone(),
                |b| format!("{} ({b})", article.label),
            ),
            value,
            article: Some(index),
        });
        explanation.counter = Some(list);
        explanation.counter_chance = counter_chance;
        explanation.counter_text = text;
    } else if verdict.blocked.is_none() {
        // A general consideration that alone outweighs the gap.
        let single: Vec<&&ExplanationLine> = objections
            .iter()
            .filter(|l| l.article.is_none() && -l.value >= gap)
            .collect();
        if let Some(line) = single.first() {
            explanation.blocker = Some(Blocker {
                text: line.text.clone(),
                value: line.value,
                article: None,
            });
            if let Some(list) = counter_proposal(state, data, proposer, recipient, articles) {
                let counter = chance(state, data, proposer, recipient, &list);
                let added: Vec<String> = list
                    .iter()
                    .filter(|a| !articles.contains(a))
                    .map(|a| negotiation::article_label(state, data, proposer, recipient, a))
                    .collect();
                let removed: Vec<String> = articles
                    .iter()
                    .filter(|a| !list.contains(a))
                    .map(|a| negotiation::article_label(state, data, proposer, recipient, a))
                    .collect();
                let mut parts = Vec::new();
                if !added.is_empty() {
                    parts.push(format!("avec {}", added.join(", ")));
                }
                if !removed.is_empty() {
                    parts.push(format!("sans {}", removed.join(", ")));
                }
                explanation.counter = Some(list);
                explanation.counter_chance = counter.chance;
                explanation.counter_text = parts.join(" ; ");
            }
        }
    }
    let worst = objections.first();
    explanation.summary = match (&explanation.blocker, &verdict.blocked, worst) {
        (Some(blocker), _, _) => format!(
            "{name} refuse, sur un seul point : {} ({:+}).",
            blocker.text, blocker.value
        ),
        (None, Some(blocked), _) => format!("{name} ne peut accepter : {blocked}."),
        (None, None, Some(line)) => format!(
            "{name} refuse : {} ({:+}), entre autres ; il manque {} points.",
            line.text, line.value, gap
        ),
        (None, None, None) => format!("{name} refuse ; il manque {gap} points."),
    };
    explanation
}
