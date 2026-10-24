//! The counsellor's season advice and the announcement of the postures of the AI houses
//! (GAMEPLAY2, ADR 0789).
//!
//! At every end of season the player's counsellor looks at the realm and may say one thing
//! (`data/rules/advisor.json`): a danger (the treasury will not last), an opportunity (colonies
//! without works, savings asleep), or the balance of the year. Each advice has its own
//! cooldown and the number of advices per season is bounded, so he never chatters. The notes
//! of the season are kept in the state ([`AdvisorState`]) and served as alerts by `game-api`.
//!
//! The same pass watches the doctrines (ADR 0782) of the houses at war with the player or
//! bordering his realm and writes one chronicle line when one of them changes posture.

use std::collections::BTreeMap;

use data_model::{Advice, AdviceTrigger, FactionId, GameData};
use serde::{Deserialize, Serialize};

use crate::events::{EventKind, GameEvent};
use crate::state::{CampaignState, Season};

/// One thing the counsellor says this season.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdvisorNote {
    pub id: String,
    /// `opportunity`, `danger` or `season_end`.
    pub kind: String,
    pub title: String,
    pub text: String,
}

/// Bookkeeping of the counsellor, saved with the campaign.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AdvisorState {
    /// Turn each advice was last said.
    #[serde(default)]
    pub last_said: BTreeMap<String, u32>,
    /// What he says this season.
    #[serde(default)]
    pub notes: Vec<AdvisorNote>,
    /// Doctrine each house was last seen following (id of the catalogue).
    #[serde(default)]
    pub doctrines: BTreeMap<FactionId, String>,
}

/// French number with a narrow no-break space between thousands.
pub fn fr_number(value: i64) -> String {
    let digits = value.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push('\u{202f}');
        }
        out.push(c);
    }
    if value < 0 {
        format!("-{out}")
    } else {
        out
    }
}

fn fill(template: &str, values: &[(&str, String)]) -> String {
    values.iter().fold(template.to_owned(), |text, (k, v)| {
        text.replace(&format!("{{{k}}}"), v)
    })
}

/// Records the doctrines at the creation of the campaign, so that only changes are announced.
pub(crate) fn init(state: &mut CampaignState) {
    let ids: Vec<FactionId> = state
        .factions
        .iter()
        .filter(|(id, f)| f.alive && !id.is_rebels())
        .map(|(id, _)| id.clone())
        .collect();
    for id in ids {
        let doctrine = crate::doctrines::active(state, &id).id.to_owned();
        state.advisor.doctrines.insert(id, doctrine);
    }
}

/// Idle settlements with an option the treasury can pay: their number, the first province.
fn idle_sites(state: &CampaignState, data: &GameData, player: &FactionId) -> (usize, String) {
    let overview = crate::holdings::holdings_overview(state, data, player);
    let treasury = state.factions.get(player).map_or(0, |f| f.treasury);
    let mut sites = 0;
    let mut first = String::new();
    for province in &overview.provinces {
        for place in &province.settlements {
            let cheapest = place
                .options_available
                .iter()
                .map(|o| i64::from(o.cost))
                .min();
            if place.idle && cheapest.is_some_and(|c| c <= treasury) {
                sites += 1;
                if first.is_empty() {
                    first = province.name.clone();
                }
            }
        }
    }
    (sites, first)
}

/// The values of an advice whose trigger holds, `None` when it does not.
fn evaluate(
    state: &CampaignState,
    data: &GameData,
    player: &FactionId,
    trigger: &AdviceTrigger,
) -> Option<Vec<(&'static str, String)>> {
    let faction = state.factions.get(player)?;
    match trigger {
        AdviceTrigger::IdleSites { min_sites } => {
            let (sites, province) = idle_sites(state, data, player);
            (sites >= *min_sites as usize).then(|| {
                vec![
                    ("sites", sites.to_string()),
                    ("treasury", fr_number(faction.treasury)),
                    ("province", province),
                ]
            })
        }
        AdviceTrigger::IdleHoard { seasons, min } => {
            let income = state.faction_income(data, player).max(0);
            let net = faction.treasury - faction.finance.debt();
            (net >= *min && net >= income * i64::from(*seasons)).then(|| {
                vec![
                    ("treasury", fr_number(net)),
                    ("levy", fr_number(data.economy_rules.opulence(net, income))),
                ]
            })
        }
        AdviceTrigger::RunwayShort { max_seasons } => {
            let balance = state.faction_economy(data, player)?.net_income();
            if balance >= 0 || faction.treasury < 0 {
                return None;
            }
            let left = (faction.treasury + (-balance) - 1) / (-balance);
            (left <= i64::from(*max_seasons)).then(|| {
                vec![
                    ("treasury", fr_number(faction.treasury)),
                    ("balance", fr_number(balance)),
                    ("seasons", left.max(1).to_string()),
                ]
            })
        }
        AdviceTrigger::YearEnd => (state.season == Season::Winter)
            .then(|| act_values(state, data))
            .flatten(),
        AdviceTrigger::ActScoreLow { below } => {
            if state.season != Season::Winter {
                return None;
            }
            let values = act_values(state, data)?;
            let score = values
                .iter()
                .find(|(k, _)| *k == "score")?
                .1
                .parse::<u32>()
                .ok()?;
            (score < *below).then_some(values)
        }
    }
}

/// `{act}`, `{score}`, `{objective}` of the act in progress: the objective to earn that is the
/// most worth (weight not yet earned).
fn act_values(state: &CampaignState, data: &GameData) -> Option<Vec<(&'static str, String)>> {
    let status = state.act_status(data)?;
    if state.acts.finished {
        return None;
    }
    let act = &data.acts_rules.acts[status.index];
    let target = status
        .objectives
        .iter()
        .filter(|o| !o.hold && !o.met)
        .max_by_key(|o| o.objective.weight * (100 - o.gain_percent))?;
    Some(vec![
        ("act", act.name.clone()),
        ("score", status.score.to_string()),
        ("objective", target.objective.title.clone()),
    ])
}

fn rank(kind: &str) -> u8 {
    match kind {
        "danger" => 0,
        "opportunity" => 1,
        _ => 2,
    }
}

/// End of season: the counsellor's notes of the new season and the postures that changed.
pub(crate) fn update(state: &mut CampaignState, data: &GameData, events: &mut Vec<GameEvent>) {
    let rules = &data.advisor_rules;
    if !rules.enabled() {
        return;
    }
    let player = state.player_faction.clone();
    state.advisor.notes.clear();
    if state.turn >= rules.quiet_seasons {
        let mut ready: Vec<(&Advice, Vec<(&'static str, String)>)> = Vec::new();
        for advice in &rules.advices {
            let waited = state
                .advisor
                .last_said
                .get(&advice.id)
                .is_none_or(|t| state.turn.saturating_sub(*t) >= advice.cooldown_seasons);
            if !waited {
                continue;
            }
            if let Some(values) = evaluate(state, data, &player, &advice.trigger) {
                ready.push((advice, values));
            }
        }
        ready.sort_by_key(|(advice, _)| rank(&advice.kind));
        for (advice, values) in ready.into_iter().take(rules.max_per_season.max(1) as usize) {
            state
                .advisor
                .last_said
                .insert(advice.id.clone(), state.turn);
            state.advisor.notes.push(AdvisorNote {
                id: advice.id.clone(),
                kind: advice.kind.clone(),
                title: advice.title.clone(),
                text: fill(&advice.text, &values),
            });
        }
    }
    announce_postures(state, data, events);
}

/// A change of doctrine of a house at war with the player or bordering his realm is news.
fn announce_postures(state: &mut CampaignState, data: &GameData, events: &mut Vec<GameEvent>) {
    let news = &data.advisor_rules.posture;
    if news.text.is_empty() {
        return;
    }
    let player = state.player_faction.clone();
    let ids: Vec<FactionId> = state
        .factions
        .iter()
        .filter(|(id, f)| f.alive && !id.is_rebels() && **id != player)
        .map(|(id, _)| id.clone())
        .collect();
    for id in ids {
        let current = crate::doctrines::active(state, &id);
        let seen = state.advisor.doctrines.get(&id).map(String::as_str);
        if seen == Some(current.id) {
            continue;
        }
        let first_sight = seen.is_none();
        state
            .advisor
            .doctrines
            .insert(id.clone(), current.id.to_owned());
        if first_sight || !known_to_player(state, data, &player, &id) {
            continue;
        }
        let intent = news
            .intents
            .get(current.id)
            .unwrap_or(&news.default_intent)
            .clone();
        let text = fill(
            &news.text,
            &[
                ("faction", data.faction_name(&id)),
                ("doctrine", current.doctrine.label.clone()),
                ("intent", intent),
            ],
        );
        events.push(GameEvent::new(EventKind::Diplomacy, text).faction(&id));
    }
}

/// At war with the player, or holding a province next to one he controls.
fn known_to_player(
    state: &CampaignState,
    data: &GameData,
    player: &FactionId,
    other: &FactionId,
) -> bool {
    if state
        .factions
        .get(player)
        .is_some_and(|f| f.at_war_with.contains(other))
    {
        return true;
    }
    let mine: std::collections::BTreeSet<_> = state.controlled_provinces(player).collect();
    state.owned_provinces(other).iter().any(|province| {
        data.provinces
            .get(province)
            .is_some_and(|p| p.neighbors.iter().any(|n| mine.contains(n)))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn french_numbers_group_thousands() {
        assert_eq!(fr_number(0), "0");
        assert_eq!(fr_number(999), "999");
        assert_eq!(fr_number(31_150), "31\u{202f}150");
        assert_eq!(fr_number(-4_677), "-4\u{202f}677");
    }

    #[test]
    fn placeholders_are_filled() {
        let text = fill(
            "{a} et {b}",
            &[("a", "x".to_owned()), ("b", "y".to_owned())],
        );
        assert_eq!(text, "x et y");
    }
}
