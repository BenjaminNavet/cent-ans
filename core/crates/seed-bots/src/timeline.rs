//! R1-0 (audit `docs/audits/2026-10-29-campagne.md`): the campaign chronicle probe.
//!
//! Plays a whole campaign headless from 1337 with the player's seat either left idle (`passive`)
//! or played by the strategic AI (`ai`), every other house played by the AI, and records one row
//! per year (provinces, treasury, prestige, men in the field and weariness of the three great
//! houses, France-England war and truce, war score, provinces changing hands, act, reign score
//! and renown, events of the year). The summary metrics measure the rhythm of the campaign; the
//! refits of the campaign loop (R1 to R5) are judged on them.
//!
//! CLI: `examples/campaign_timeline.rs`.

use std::collections::BTreeMap;

use data_model::{FactionId, GameData};
use sim_campaign::{CampaignState, Order};

/// Who plays the player's seat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    /// No order, no offer answered.
    Passive,
    /// The strategic AI plays the house and answers the offers as it would.
    Ai,
}

impl Profile {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "passive" => Some(Profile::Passive),
            "ai" | "active" => Some(Profile::Ai),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Profile::Passive => "passive",
            Profile::Ai => "ai",
        }
    }
}

/// Settings of one probe run.
#[derive(Debug, Clone)]
pub struct ProbeConfig {
    pub faction: String,
    pub seed: u64,
    pub profile: Profile,
    /// Turns at most (the campaign may end earlier).
    pub turns: u32,
    /// Ambitions chosen for the player's house at every act opening (R1-2), in order.
    pub ambitions: Vec<String>,
}

impl ProbeConfig {
    pub fn new(faction: &str, seed: u64, profile: Profile) -> Self {
        Self {
            faction: faction.to_owned(),
            seed,
            profile,
            turns: 470,
            ambitions: Vec::new(),
        }
    }
}

/// Figures of one great house in a yearly row.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HouseRow {
    pub alive: bool,
    pub provinces: u32,
    pub treasury: i64,
    pub prestige: i32,
    pub men: u32,
    pub weariness: i32,
}

/// One row per year (every fourth turn, and the last one).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct YearRow {
    pub year: i32,
    pub turn: u32,
    pub france: HouseRow,
    pub england: HouseRow,
    pub burgundy: HouseRow,
    pub fr_en_war: bool,
    pub fr_en_truce: bool,
    pub war_score: i32,
    /// Provinces whose controller changed during the year (all Europe).
    pub flips: u32,
    /// The same, for the provinces the player's house won or lost.
    pub player_flips: u32,
    /// 1-based act in progress (0: none).
    pub act: usize,
    pub act_score: u32,
    pub reign_score: u32,
    /// Renown of the player's house (R1-1); 0 before it exists.
    pub renown: i32,
    /// Men in the field of the player's house.
    pub player_men: u32,
    pub player_treasury: i64,
    /// Events of the year that touch the player's house, plus wars, peaces, plague and schism.
    pub events: BTreeMap<String, u32>,
}

/// A whole probe run.
#[derive(Debug, Clone, Default)]
pub struct Timeline {
    pub faction: String,
    pub seed: u64,
    pub profile: String,
    pub rows: Vec<YearRow>,
    /// Outcome kind (`Victory`, `Defeat`, `Ended`, `none`).
    pub outcome: String,
    pub end_year: i32,
    pub end_turn: u32,
    /// `act_id score ended_by end_year`.
    pub acts: Vec<String>,
    /// Events of the whole game (all Europe) by kind.
    pub events: BTreeMap<String, u32>,
    /// Turns sampled with France and England at war / in truce, and turns played.
    pub war_turns: u32,
    pub truce_turns: u32,
    pub turns: u32,
    pub offers_to_player: usize,
    pub player_orders: usize,
}

/// Names of the summary metrics (R1-0 spec), in print order.
pub const METRICS: [&str; 9] = [
    "flips_per_year",
    "player_flips_per_year",
    "fr_en_war_share",
    "truce_share",
    "score_frozen_years",
    "raids_per_game",
    "ransoms_per_game",
    "field_men_median",
    "years_under_300_men",
];

fn faction(id: &str) -> FactionId {
    FactionId::new(id).expect("faction id")
}

fn prestige(state: &CampaignState, id: &FactionId) -> i32 {
    state
        .factions
        .get(id)
        .and_then(|f| f.ruler.as_ref())
        .and_then(|r| state.characters.get(r))
        .map_or(0, |c| c.prestige)
}

fn men(state: &CampaignState, id: &FactionId) -> u32 {
    state
        .armies
        .values()
        .filter(|a| &a.faction == id)
        .map(|a| a.units.iter().map(|u| u.strength).sum::<u32>())
        .sum()
}

fn house(state: &CampaignState, id: &FactionId) -> HouseRow {
    match state.factions.get(id) {
        Some(f) if f.alive => HouseRow {
            alive: true,
            provinces: state.controlled_provinces(id).count() as u32,
            treasury: f.treasury,
            prestige: prestige(state, id),
            men: men(state, id),
            weariness: f.ledger.weariness as i32,
        },
        _ => HouseRow::default(),
    }
}

fn controllers(state: &CampaignState) -> BTreeMap<String, String> {
    state
        .provinces
        .keys()
        .map(|p| {
            (
                p.to_string(),
                state
                    .province_controller(p)
                    .map(|c| c.to_string())
                    .unwrap_or_default(),
            )
        })
        .collect()
}

/// Plays one campaign and records its timeline.
pub fn run(data: &GameData, config: &ProbeConfig) -> Timeline {
    let player = faction(&config.faction);
    let mut state =
        CampaignState::new_1337(data, player.clone(), config.seed).expect("campaign starts");
    state.chronicle.disabled = false;
    let (fr, en, bu) = (
        faction("fac_france"),
        faction("fac_england"),
        faction("fac_burgundy"),
    );
    let mut timeline = Timeline {
        faction: config.faction.clone(),
        seed: config.seed,
        profile: config.profile.name().to_owned(),
        ..Timeline::default()
    };
    let mut previous = controllers(&state);
    let (mut flips, mut player_flips) = (0u32, 0u32);
    let mut year_events: BTreeMap<String, u32> = BTreeMap::new();
    let mut ambitions_act = usize::MAX;
    while state.outcome.is_none() && timeline.turns < config.turns {
        // R1-2: the ambitions of the act, chosen once at its opening.
        if !config.ambitions.is_empty() && state.acts.current != ambitions_act {
            ambitions_act = state.acts.current;
            for ambition in &config.ambitions {
                let _ = state.choose_act_policy(data, ambition);
            }
        }
        let offers = state.factions[&player].offers.clone();
        timeline.offers_to_player += offers.len();
        if config.profile == Profile::Ai {
            for offer in offers {
                let accept = sim_campaign::negotiation::evaluate_treaty(
                    &state,
                    data,
                    &offer.from,
                    &player,
                    &offer.proposal.articles,
                )
                .accept;
                let _ = state.submit_order(
                    data,
                    Order::AnswerOffer {
                        offer: offer.id,
                        accept,
                    },
                );
            }
            for order in ai::plan_turn(&state, data, &player) {
                if state.submit_order(data, order).is_ok() {
                    timeline.player_orders += 1;
                }
            }
        }
        let events = state.end_turn_with(data, ai::plan_turn);
        timeline.turns += 1;
        for event in &events {
            let kind = format!("{:?}", event.kind);
            *timeline.events.entry(kind.clone()).or_default() += 1;
            if event.faction.as_ref() == Some(&player)
                || matches!(
                    kind.as_str(),
                    "WarDeclared" | "PeaceSigned" | "Plague" | "Schism"
                )
            {
                *year_events.entry(kind).or_default() += 1;
            }
        }
        if state.is_at_war(&fr, &en) {
            timeline.war_turns += 1;
        }
        if state.has_truce(&fr, &en) {
            timeline.truce_turns += 1;
        }
        let now = controllers(&state);
        for (province, controller) in &now {
            let old = previous.get(province).cloned().unwrap_or_default();
            if &old != controller {
                flips += 1;
                if controller == player.as_str() || old == player.as_str() {
                    player_flips += 1;
                }
            }
        }
        previous = now;
        if timeline.turns % 4 == 0 || state.outcome.is_some() {
            let (act, act_score) = state
                .act_status(data)
                .map_or((0, 0), |a| (a.index + 1, a.score));
            let player_row = house(&state, &player);
            timeline.rows.push(YearRow {
                year: state.year,
                turn: state.turn,
                france: house(&state, &fr),
                england: house(&state, &en),
                burgundy: house(&state, &bu),
                fr_en_war: state.is_at_war(&fr, &en),
                fr_en_truce: state.has_truce(&fr, &en),
                war_score: state.war_score(data, &fr, &en),
                flips,
                player_flips,
                act,
                act_score,
                reign_score: state.reign_score(data, true),
                renown: 0,
                player_men: player_row.men,
                player_treasury: player_row.treasury,
                events: std::mem::take(&mut year_events),
            });
            flips = 0;
            player_flips = 0;
        }
    }
    timeline.outcome = state
        .outcome
        .as_ref()
        .map_or("none".to_owned(), |o| format!("{:?}", o.kind));
    timeline.end_year = state.year;
    timeline.end_turn = state.turn;
    timeline.acts = state
        .acts
        .reports
        .iter()
        .map(|r| format!("{} {} {} {}", r.act_id, r.score, r.ended_by, r.end_year))
        .collect();
    timeline
}

fn median(mut values: Vec<f64>) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        (values[mid - 1] + values[mid]) / 2.0
    } else {
        values[mid]
    }
}

impl Timeline {
    /// Years covered by the rows.
    pub fn years(&self) -> f64 {
        (f64::from(self.turns) / 4.0).max(1.0)
    }

    /// The nine summary metrics of the R1-0 spec, plus the act openings and the final scores.
    pub fn summary(&self) -> BTreeMap<&'static str, f64> {
        let years = self.years();
        let mut out = BTreeMap::new();
        let flips: u32 = self.rows.iter().map(|r| r.flips).sum();
        let player_flips: u32 = self.rows.iter().map(|r| r.player_flips).sum();
        let turns = f64::from(self.turns.max(1));
        out.insert("flips_per_year", f64::from(flips) / years);
        out.insert("player_flips_per_year", f64::from(player_flips) / years);
        out.insert("fr_en_war_share", f64::from(self.war_turns) / turns);
        out.insert("truce_share", f64::from(self.truce_turns) / turns);
        // Years since the reign score last moved (0: it moved in the last year).
        let last = self.rows.last().map_or(0, |r| r.reign_score);
        let frozen_since = self
            .rows
            .iter()
            .rposition(|r| r.reign_score != last)
            .map_or(0, |i| i + 1);
        let frozen_rows = self.rows.len().saturating_sub(frozen_since + 1);
        out.insert("score_frozen_years", frozen_rows as f64);
        let count = |kind: &str| f64::from(self.events.get(kind).copied().unwrap_or(0));
        out.insert("raids_per_game", count("Raid"));
        out.insert("ransoms_per_game", count("Ransom"));
        out.insert(
            "field_men_median",
            median(self.rows.iter().map(|r| f64::from(r.player_men)).collect()),
        );
        out.insert(
            "years_under_300_men",
            self.rows.iter().filter(|r| r.player_men < 300).count() as f64,
        );
        // R1-1 acceptance: years the second and third acts opened (0: never).
        let opened = |act: usize| {
            self.rows
                .iter()
                .find(|r| r.act >= act)
                .map_or(0.0, |r| f64::from(r.year))
        };
        out.insert("act2_open_year", opened(2));
        out.insert("act3_open_year", opened(3));
        out.insert(
            "reign_score_end",
            f64::from(self.rows.last().map_or(0, |r| r.reign_score)),
        );
        out.insert(
            "renown_end",
            f64::from(self.rows.last().map_or(0, |r| r.renown)),
        );
        out.insert(
            "treasury_end",
            self.rows.last().map_or(0.0, |r| r.player_treasury as f64),
        );
        out.insert(
            "victory_year",
            if self.outcome == "Victory" {
                f64::from(self.end_year)
            } else {
                0.0
            },
        );
        out
    }

    /// Header of the yearly table.
    pub fn header() -> &'static str {
        "year turn | FR prov tre pres men wear | EN prov tre pres men wear | BU prov tre | FRxEN war truce ws | flips pflips | act score reign renown | events"
    }

    /// One printed line per year.
    pub fn row_line(row: &YearRow) -> String {
        let house = |h: &HouseRow| {
            if h.alive {
                format!(
                    "{:>3} {:>7} {:>4} {:>6} {:>3}",
                    h.provinces, h.treasury, h.prestige, h.men, h.weariness
                )
            } else {
                "  dead".to_owned()
            }
        };
        let events: Vec<String> = row.events.iter().map(|(k, v)| format!("{k}:{v}")).collect();
        format!(
            "{} {:>3} | {} | {} | {:>3} {:>7} | {:>5} {:>5} {:>4} | {:>3} {:>3} | A{} {:>3} {:>3} {:>5} | {}",
            row.year,
            row.turn,
            house(&row.france),
            house(&row.england),
            row.burgundy.provinces,
            row.burgundy.treasury,
            row.fr_en_war,
            row.fr_en_truce,
            row.war_score,
            row.flips,
            row.player_flips,
            row.act,
            row.act_score,
            row.reign_score,
            row.renown,
            events.join(" ")
        )
    }

    /// CSV of the yearly rows.
    pub fn csv(&self) -> String {
        let mut out = String::from(
            "year,turn,fr_prov,fr_treasury,fr_prestige,fr_men,fr_weariness,en_prov,en_treasury,en_prestige,en_men,en_weariness,bu_prov,bu_treasury,fr_en_war,fr_en_truce,war_score,flips,player_flips,act,act_score,reign_score,renown,player_men,player_treasury\n",
        );
        for r in &self.rows {
            out.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
                r.year,
                r.turn,
                r.france.provinces,
                r.france.treasury,
                r.france.prestige,
                r.france.men,
                r.france.weariness,
                r.england.provinces,
                r.england.treasury,
                r.england.prestige,
                r.england.men,
                r.england.weariness,
                r.burgundy.provinces,
                r.burgundy.treasury,
                r.fr_en_war,
                r.fr_en_truce,
                r.war_score,
                r.flips,
                r.player_flips,
                r.act,
                r.act_score,
                r.reign_score,
                r.renown,
                r.player_men,
                r.player_treasury
            ));
        }
        out
    }

    /// The full text report: rows, outcome, acts, totals, events, metrics.
    pub fn report(&self) -> String {
        let mut out = format!(
            "# {} seed {} profile {}\n{}\n",
            self.faction,
            self.seed,
            self.profile,
            Self::header()
        );
        for row in &self.rows {
            out.push_str(&Self::row_line(row));
            out.push('\n');
        }
        out.push_str(&format!(
            "OUTCOME {} year {} turn {}\n",
            self.outcome, self.end_year, self.end_turn
        ));
        for act in &self.acts {
            out.push_str(&format!("ACT {act}\n"));
        }
        out.push_str(&format!(
            "TOTAL offers_to_player {} player_orders {}\n",
            self.offers_to_player, self.player_orders
        ));
        let events: Vec<String> = self
            .events
            .iter()
            .map(|(k, v)| format!("{k}:{v}"))
            .collect();
        out.push_str(&format!("EVENTS {}\n", events.join(" ")));
        out.push_str(&self.metrics_line());
        out.push('\n');
        out
    }

    /// `METRICS name=value ...`.
    pub fn metrics_line(&self) -> String {
        let metrics: Vec<String> = self
            .summary()
            .iter()
            .map(|(k, v)| format!("{k}={v:.2}"))
            .collect();
        format!(
            "METRICS {} seed {} {} {}",
            self.faction,
            self.seed,
            self.profile,
            metrics.join(" ")
        )
    }
}
