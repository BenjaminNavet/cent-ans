//! Assemblies that consent to the extraordinary tax (ECO-R1, ADR « L'impôt consenti »),
//! mirroring `data/schemas/assemblies.schema.json` (`data/rules/assemblies.json`).
//!
//! A crown no longer *raises* an extraordinary tax, it *convenes* an assembly (États de langue
//! d'oïl or d'oc, Parlement, états de Bretagne, membres de Flandre) which grants a subsidy
//! proportioned to the necessity of the moment and asks for concessions in exchange. The whole
//! file is optional: without any assembly, the old extraordinary levies of `finance.json` stay.

use serde::{Deserialize, Serialize};

/// Why the crown asks (the core checks the situation: see `sim_campaign::assembly`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NecessityKind {
    /// At war against an enemy the realm did not set out to conquer.
    DefensiveWar,
    /// An enemy army stands on the realm's soil or holds one of its provinces.
    Invasion,
    /// The king is a captive or a ransom is owed.
    Ransom,
    /// At war against an enemy the realm has war goals against.
    OffensiveWar,
    /// The faction leads the crusade.
    Crusade,
}

impl NecessityKind {
    pub const ALL: [NecessityKind; 5] = [
        NecessityKind::DefensiveWar,
        NecessityKind::Invasion,
        NecessityKind::Ransom,
        NecessityKind::OffensiveWar,
        NecessityKind::Crusade,
    ];

    pub fn key(self) -> &'static str {
        match self {
            NecessityKind::DefensiveWar => "defensive_war",
            NecessityKind::Invasion => "invasion",
            NecessityKind::Ransom => "ransom",
            NecessityKind::OffensiveWar => "offensive_war",
            NecessityKind::Crusade => "crusade",
        }
    }
}

/// A necessity as the data sets it: how much the estates are moved by it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Necessity {
    pub id: NecessityKind,
    pub name: String,
    /// Multiplier of the subsidy.
    pub multiplier: f64,
    /// Points added to the estates' agreement.
    pub accord_bonus: f64,
}

/// A tier of demand (« palier de demande »).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AskTier {
    pub id: String,
    pub name: String,
    /// Subsidy per season, in multiples of the ordinary tax of the covered provinces.
    pub ordinary_multiple: f64,
    /// Number of seasons the subsidy is collected.
    pub term_seasons: u32,
    /// Points added to (or taken from) the estates' agreement.
    pub accord_modifier: f64,
}

/// The base of a subsidy (aide, gabelle, fouage...): who pays most.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubsidyForm {
    pub id: String,
    pub name: String,
    /// Assemblies that can vote it (empty: all).
    #[serde(default)]
    pub assemblies: Vec<String>,
    /// First year the form exists (0: always).
    #[serde(default)]
    pub from_year: i32,
    /// Weight of the burden on a wholly urban province ...
    pub urban_weight: f64,
    /// ... and on a wholly rural one (the unrest follows the burden).
    pub rural_weight: f64,
    /// Making the tax permanent (an ordinary tax, no more sessions): the year from which the
    /// estates can be asked, `None` when the form can never be permanent.
    #[serde(default)]
    pub permanent_from_year: Option<i32>,
    /// Permanent yield, per cent of the gross tax income, once granted.
    #[serde(default)]
    pub permanent_percent: f64,
    /// Constant unrest of the provinces under a permanent tax (urban weight applies).
    #[serde(default)]
    pub permanent_unrest: f64,
    /// R1-c: multiplier of the tier's multiple for this form (a poll tax yields more than a
    /// tenth and fifteenth, at the price of a heavier burden; 1 when absent).
    #[serde(default = "one")]
    pub yield_factor: f64,
    /// R1-c: the subsidy is levied on an export (the wool custom): only the covered provinces
    /// producing this resource pay, and the amount follows the share of the trade routes of the
    /// house that carry it to a foreign hub and are not cut by war or embargo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub export_good: Option<String>,
}

fn one() -> f64 {
    1.0
}

/// What a concession constrains or costs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConcessionKind {
    /// No debasement of the coinage during the term; debasing breaks the promise.
    StrongMoney,
    /// The household and administration are capped (`household_cap_percent`).
    ElectedOfficers,
    /// A councillor (`office`) is dismissed at once.
    DismissCounsellor,
    /// A truce must be signed within `truce_within_seasons` of the promise.
    EndWar,
    /// Privileges, remissions: only the costs apply.
    Grant,
}

/// One concession of the catalogue.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Concession {
    pub id: String,
    pub name: String,
    /// Sentence for the interface and the journal.
    pub text: String,
    pub kind: ConcessionKind,
    /// Cost for the crown, in points (ranks the demands, compared by the AI to its shortfall).
    pub cost: u32,
    /// Seasons the promise lasts.
    pub duration_seasons: u32,
    /// Prestige the sovereign loses when he grants it.
    #[serde(default)]
    pub prestige_cost: i32,
    /// Tenths of a point of unrest added (negative: calmed) to the covered provinces for the
    /// term.
    #[serde(default)]
    pub grievance_tenths: i32,
    /// Share of the ordinary tax of the covered provinces given up for the term, per cent.
    #[serde(default)]
    pub ordinary_percent: f64,
    /// `ElectedOfficers`: the household stays under this per cent of the gross income.
    #[serde(default)]
    pub household_cap_percent: Option<f64>,
    /// `DismissCounsellor`: the office (`data/council/offices.json`).
    #[serde(default)]
    pub office: Option<String>,
    /// `EndWar`: seasons to sign a truce.
    #[serde(default)]
    pub truce_within_seasons: Option<u32>,
}

/// An assembly: a house, the provinces it speaks for, its orders and its temper.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Assembly {
    pub id: String,
    pub name: String,
    /// The faction that convenes it.
    pub house: String,
    /// Provinces covered, listed ...
    #[serde(default)]
    pub provinces: Vec<String>,
    /// ... and every province of these regions.
    #[serde(default)]
    pub regions: Vec<String>,
    /// All the provinces of the house are covered (a national parliament).
    #[serde(default)]
    pub all_provinces: bool,
    /// Orders represented (clergy, nobility, towns, counties): flavour, shown by the interface.
    pub orders: Vec<String>,
    /// Mood (0-100) at the start of the campaign.
    pub start_mood: i32,
    /// Seasons between two convocations (at least 4).
    pub min_gap_seasons: u32,
    /// The house needs at least this many covered provinces to convene.
    pub min_covered_provinces: u32,
    /// Concessions it likes to ask for, in the order of preference.
    pub demands: Vec<String>,
}

/// The mood of the estates: how it moves between sessions.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct MoodRules {
    pub max: i32,
    /// The mood drifts toward this value ...
    pub rest: i32,
    /// ... by this many points a season.
    pub recovery_per_season: i32,
    /// Lost when the estates are refused terms they proposed or a session is refused.
    pub refusal_penalty: i32,
    /// Lost for each subsidy granted (fatigue; the running subsidy also weighs on the vote).
    pub grant_fatigue: i32,
    /// Won when a promise is kept.
    pub promise_kept_bonus: i32,
    /// Lost when a promise is broken.
    pub promise_broken_penalty: i32,
}

/// How the estates vote.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct VoteRules {
    /// Provisional legitimacy: points per point of the sovereign's prestige.
    pub prestige_weight: f64,
    /// Ceiling (and floor, negated) of the points legitimacy can add: renown has no limit, the
    /// estates' trust in it has.
    pub prestige_max_points: f64,
    /// Points lost per point of mean unrest (0-100) of the covered provinces.
    pub unrest_weight: f64,
    /// Points lost per season of ordinary tax still being paid as subsidy.
    pub subsidy_weight: f64,
    /// Points lost per broken promise remembered.
    pub broken_promise_weight: f64,
    /// Seasons a broken promise is remembered.
    pub broken_memory_seasons: u32,
    /// Agreement from which the estates grant without terms.
    pub grant_threshold: f64,
    /// Agreement from which they grant under terms; below, they refuse.
    pub conditional_threshold: f64,
    /// One more concession for each this many points under the grant threshold.
    pub points_per_concession: f64,
    pub max_concessions: u32,
    /// Points taken when the crown asks for a permanent tax.
    pub permanent_penalty: f64,
    /// Seasons a session answer lasts as long as the crown has not chosen.
    pub answer_window_seasons: u32,
    /// Seasons between the convocation and the answer.
    pub answer_delay_seasons: u32,
    /// Sessions kept in the memory of an assembly (the oldest leave first).
    pub history_sessions: u32,
}

/// Local unrest caused by the levy.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct GrogneRules {
    /// Unrest points per unit of burden (subsidy of the province / its ordinary tax).
    pub unrest_per_burden: f64,
    /// Ceiling of the unrest the levies add to one province.
    pub max_unrest: f64,
    /// Share of burghers in a province from which it counts as wholly urban (the form's
    /// urban weight applies in full).
    pub urban_full_share: f64,
    /// Unrest (points) spread on the covered provinces when a promise is broken ...
    pub broken_promise_unrest: f64,
    /// ... for this many seasons.
    pub broken_promise_unrest_turns: u32,
}

/// What the AI does with the assemblies (the first, minimal rules).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct AssemblyAiRules {
    /// Convene when the treasury falls under this many seasons of gross income.
    pub convene_below_seasons: f64,
    /// R1-c, war forecast: seasons looked ahead when the cost of the war (army upkeep, grown by
    /// `war_cost_factor`) is set against the treasury and the ordinary surplus.
    pub forecast_seasons: f64,
    /// R1-c: the war costs this many times the current army upkeep (levies, sieges, losses).
    pub war_cost_factor: f64,
    /// R1-c: a realm at war with a running campaign convenes as soon as its treasury is under this
    /// many seasons of gross income, whatever the forecast.
    pub campaign_below_seasons: f64,
    /// R1-c, honour: the AI debases its money in spite of a promise of strong money only when its
    /// treasury is below minus this many seasons of gross income.
    pub honour_debt_seasons: f64,
    /// R1-c, honour: a prince whose `ai_personality.diplomacy` (0-100, 50 when absent) is under
    /// this value does not hold himself to his concessions.
    pub honour_min_diplomacy: f64,
    /// R1-c, honour: the AI accepts an unkeepable condition (a truce it cannot sign, a strong
    /// money it cannot afford) only when its treasury is under this many seasons of gross income.
    pub need_seasons: f64,
    /// R1-c: the poll tax is asked only when the shortfall exceeds this many seasons of gross
    /// income and the covered provinces are calm (mean unrest under `poll_tax_max_unrest`).
    pub poll_tax_shortfall_seasons: f64,
    pub poll_tax_max_unrest: f64,
    /// Accept terms whose total cost is at most this many points.
    pub accept_cost_up_to: u32,
    /// The smallest tier whose total (multiple x term x necessity) reaches this many seasons of
    /// the ordinary tax of the covered provinces is asked (the largest if none does).
    pub target_seasons: f64,
}

/// Contents of `data/rules/assemblies.json`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct AssemblyRules {
    pub description: String,
    pub mood: MoodRules,
    pub vote: VoteRules,
    pub grogne: GrogneRules,
    pub ai: AssemblyAiRules,
    pub necessities: Vec<Necessity>,
    pub asks: Vec<AskTier>,
    pub forms: Vec<SubsidyForm>,
    pub concessions: Vec<Concession>,
    pub assemblies: Vec<Assembly>,
}

impl AssemblyRules {
    /// `true` when the file defines at least one assembly.
    pub fn is_active(&self) -> bool {
        !self.assemblies.is_empty()
    }

    pub fn assembly(&self, id: &str) -> Option<&Assembly> {
        self.assemblies.iter().find(|a| a.id == id)
    }

    pub fn necessity(&self, kind: NecessityKind) -> Option<&Necessity> {
        self.necessities.iter().find(|n| n.id == kind)
    }

    pub fn ask(&self, id: &str) -> Option<&AskTier> {
        self.asks.iter().find(|a| a.id == id)
    }

    pub fn form(&self, id: &str) -> Option<&SubsidyForm> {
        self.forms.iter().find(|f| f.id == id)
    }

    pub fn concession(&self, id: &str) -> Option<&Concession> {
        self.concessions.iter().find(|c| c.id == id)
    }

    /// Assemblies the house `faction` convenes.
    pub fn of_house<'a>(&'a self, faction: &str) -> impl Iterator<Item = &'a Assembly> + 'a {
        let faction = faction.to_owned();
        self.assemblies.iter().filter(move |a| a.house == faction)
    }
}
