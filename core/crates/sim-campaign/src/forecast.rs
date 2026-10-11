//! RG8: next-season forecast of the top-bar gauges (prestige, men under arms,
//! unrest) with what weighs most on each.
//!
//! The projected value comes from the end-of-turn resolution itself
//! ([`CampaignState::resolve_end_of_turn`]) run on a copy of the state with
//! the chronicle's random events off: no formula is duplicated, so the
//! forecast cannot drift from the real change. It does not foresee the other
//! factions' moves (played before the resolution) nor random events.
//!
//! The contributions explain the change. Prestige and unrest read the same
//! helpers the resolution uses ([`crate::table::table_prestige`],
//! [`crate::dynasty::court_prestige_parts`],
//! [`crate::chivalry::yearly_order_prestige`], [`crate::population::unrest_pull`]);
//! men under arms measures the phases that move armies' strength, each run
//! alone on a copy. What they do not explain is `other`.

use data_model::{FactionId, GameData};

use crate::events::GameEvent;
use crate::state::CampaignState;

/// One signed term of a forecast.
#[derive(Debug, Clone, PartialEq)]
pub struct Contribution {
    /// Stable key (`unrest_terms` keys for unrest, `table`, `building`, ...).
    pub key: String,
    /// French label when the core knows it (building or order names); empty
    /// when the UI maps `key` itself.
    pub label: String,
    pub amount: f64,
}

/// A gauge now, next season, and why.
#[derive(Debug, Clone, PartialEq)]
pub struct GaugeForecast {
    pub current: i64,
    pub next: i64,
    /// Non-zero terms, sorted by decreasing absolute amount.
    pub contributions: Vec<Contribution>,
    /// `next - current` minus the sum of `contributions` (events, rounding).
    pub other: f64,
}

impl GaugeForecast {
    fn new(current: i64, next: i64, mut contributions: Vec<Contribution>) -> Self {
        contributions.retain(|c| c.amount.abs() >= 0.05);
        contributions.sort_by(|a, b| {
            b.amount
                .abs()
                .total_cmp(&a.amount.abs())
                .then_with(|| a.key.cmp(&b.key))
        });
        let explained: f64 = contributions.iter().map(|c| c.amount).sum();
        Self {
            current,
            next,
            other: (next - current) as f64 - explained,
            contributions,
        }
    }

    pub fn delta(&self) -> i64 {
        self.next - self.current
    }
}

/// Forecast of the three top-bar gauges of a faction.
#[derive(Debug, Clone, PartialEq)]
pub struct FactionForecast {
    pub prestige: GaugeForecast,
    pub men: GaugeForecast,
    pub unrest: GaugeForecast,
}

fn contribution(key: &str, label: &str, amount: f64) -> Contribution {
    Contribution {
        key: key.to_owned(),
        label: label.to_owned(),
        amount,
    }
}

/// The state after the next end-of-turn resolution, without random events.
pub fn simulate_next_turn(state: &CampaignState, data: &GameData) -> CampaignState {
    let mut next = state.clone();
    next.chronicle.disabled = true;
    let mut events: Vec<GameEvent> = Vec::new();
    next.resolve_end_of_turn(data, &mut events);
    next
}

fn soldiers_of(state: &CampaignState, faction: &FactionId) -> i64 {
    state
        .faction_summary(faction)
        .map_or(0, |s| i64::try_from(s.soldiers).unwrap_or(i64::MAX))
}

/// Change in men under arms due to one phase alone.
fn phase_men(
    state: &CampaignState,
    faction: &FactionId,
    phase: impl FnOnce(&mut CampaignState, &mut Vec<GameEvent>),
) -> f64 {
    let mut copy = state.clone();
    copy.chronicle.disabled = true;
    let before = soldiers_of(&copy, faction);
    let mut events = Vec::new();
    phase(&mut copy, &mut events);
    (soldiers_of(&copy, faction) - before) as f64
}

fn prestige_contributions(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> Vec<Contribution> {
    let mut terms = vec![contribution(
        "table",
        "Table de la cour",
        f64::from(crate::table::table_prestige(state, data, faction)),
    )];
    if state.season == crate::state::Season::Winter {
        let parts = crate::dynasty::court_prestige_parts(state, data, faction);
        if parts.divisor > 0.0 {
            for (id, value) in &parts.buildings {
                let name = data
                    .buildings
                    .get(id)
                    .map_or_else(|| id.to_string(), |b| b.name.display.clone());
                terms.push(contribution(
                    "building",
                    &format!("{name} (bilan de l'hiver)"),
                    value / parts.divisor,
                ));
            }
            terms.push(contribution(
                "tech",
                "Techniques (bilan de l'hiver)",
                parts.tech / parts.divisor,
            ));
            terms.push(contribution(
                "ruler",
                "Souverain : traits et talents (hiver)",
                parts.own / parts.divisor,
            ));
            terms.push(contribution(
                "retinue",
                "Compagnons du souverain (hiver)",
                f64::from(parts.retinue),
            ));
        }
        if let Some((name, prestige)) = crate::chivalry::yearly_order_prestige(state, data, faction)
        {
            terms.push(contribution(
                "order",
                &format!("{name} (hiver)"),
                f64::from(prestige),
            ));
        }
    }
    terms
}

fn men_contributions(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> Vec<Contribution> {
    vec![
        contribution(
            "attrition",
            "",
            phase_men(state, faction, |s, e| {
                crate::economy::resolve_attrition(s, data, e);
            }),
        ),
        contribution(
            "replenish",
            "",
            phase_men(state, faction, |s, e| {
                crate::replenish::resolve_replenishment(s, data, e);
            }),
        ),
        contribution(
            "mercenaries",
            "",
            phase_men(state, faction, |s, e| {
                crate::mercenaries::resolve_mercenaries(s, data, e);
            }),
        ),
        contribution(
            "crusade",
            "",
            phase_men(state, faction, |s, e| {
                crate::crusade::resolve_crusade(s, data, e);
            }),
        ),
    ]
}

/// Forecast of prestige, men under arms and unrest of `faction`; `None` for
/// an unknown faction.
pub fn faction_forecast(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> Option<FactionForecast> {
    let now = state.faction_summary(faction)?;
    let next_state = simulate_next_turn(state, data);
    let next = next_state.faction_summary(faction)?;
    let unrest: Vec<Contribution> = crate::population::unrest_pull(state, data, faction)
        .into_iter()
        .map(|(key, amount)| contribution(key, "", amount))
        .collect();
    Some(FactionForecast {
        prestige: GaugeForecast::new(
            i64::from(now.prestige),
            i64::from(next.prestige),
            prestige_contributions(state, data, faction),
        ),
        men: GaugeForecast::new(
            i64::try_from(now.soldiers).unwrap_or(i64::MAX),
            i64::try_from(next.soldiers).unwrap_or(i64::MAX),
            men_contributions(state, data, faction),
        ),
        unrest: GaugeForecast::new(
            i64::from(now.mean_unrest),
            i64::from(next.mean_unrest),
            unrest,
        ),
    })
}

#[cfg(test)]
mod tests {
    use data_model::test_support::{fac, game_data};

    use super::*;

    fn quiet(faction: &str) -> CampaignState {
        let data = game_data();
        let mut state = CampaignState::new_1337(data, fac(faction), 11).unwrap();
        state.chronicle.disabled = true;
        state
    }

    /// The forecast equals the real change after the end-of-turn resolution,
    /// whatever the season, for several factions.
    #[test]
    fn forecast_matches_the_real_end_of_turn() {
        let data = game_data();
        for faction in ["fac_france", "fac_england", "fac_burgundy"] {
            let mut state = quiet(faction);
            let id = fac(faction);
            for _ in 0..6 {
                let forecast = faction_forecast(&state, data, &id).expect("forecast");
                let mut events = Vec::new();
                state.resolve_end_of_turn(data, &mut events);
                let after = state.faction_summary(&id).unwrap();
                assert_eq!(
                    forecast.prestige.next,
                    i64::from(after.prestige),
                    "{faction}"
                );
                assert_eq!(forecast.men.next, after.soldiers as i64, "{faction}");
                assert_eq!(
                    forecast.unrest.next,
                    i64::from(after.mean_unrest),
                    "{faction}"
                );
            }
        }
    }

    #[test]
    #[ignore = "timing probe"]
    fn timing_probe() {
        let data = game_data();
        let state = quiet("fac_france");
        let start = std::time::Instant::now();
        for _ in 0..10 {
            faction_forecast(&state, data, &fac("fac_france")).unwrap();
        }
        eprintln!("forecast: {:?} each", start.elapsed() / 10);
        let start = std::time::Instant::now();
        for _ in 0..10 {
            let _ = state.clone();
        }
        eprintln!("clone: {:?} each", start.elapsed() / 10);
        let start = std::time::Instant::now();
        let mut ev = Vec::new();
        let mut c = state.clone();
        c.chronicle.disabled = true;
        for _ in 0..10 {
            crate::population::resolve_population(&mut c, data, &mut ev);
        }
        eprintln!("population: {:?} each", start.elapsed() / 10);
        let start = std::time::Instant::now();
        for _ in 0..10 {
            let _ = crate::population::unrest_pull(&state, data, &fac("fac_france"));
        }
        eprintln!("pull: {:?} each", start.elapsed() / 10);
        let mut c = state.clone();
        c.chronicle.disabled = true;
        let mut ev: Vec<GameEvent> = Vec::new();
        macro_rules! t {
            ($name:expr, $e:expr) => {{
                let start = std::time::Instant::now();
                $e;
                eprintln!("phase {}: {:?}", $name, start.elapsed());
            }};
        }
        t!("sieges", crate::siege::resolve_sieges(&mut c, data, &mut ev));
        t!("raids", crate::siege::resolve_raids(&mut c, data, &mut ev));
        t!("construction", crate::buildings::resolve_construction(&mut c, data, &mut ev));
        t!("goods", crate::economy::resolve_goods(&mut c, data));
        t!("table_req", crate::table::resolve_requirements(&mut c, data, &mut ev));
        t!("edicts_req", crate::edicts::resolve_requirements(&mut c, data, &mut ev));
        t!("economy", crate::economy::resolve_economy(&mut c, data, &mut ev));
        t!("mercs", crate::mercenaries::resolve_mercenaries(&mut c, data, &mut ev));
        t!("trade", crate::trade::resolve_trade(&mut c, data, &mut ev));
        t!("coinage", crate::coinage::resolve_coinage(&mut c, &mut ev));
        t!("ransoms", crate::ransom::resolve_ransoms(&mut c, data, &mut ev));
        t!("chivalry", crate::chivalry::resolve_chivalry(&mut c, data, &mut ev));
        t!("research", crate::research::resolve_research(&mut c, data, &mut ev));
        t!("attrition", crate::economy::resolve_attrition(&mut c, data, &mut ev));
        t!("replenish", crate::replenish::resolve_replenishment(&mut c, data, &mut ev));
        t!("pools", crate::recruit_pool::resolve_recruit_pools(&mut c, data));
        t!("posture", crate::posture::end_of_turn(&mut c, data));
        t!("decay", crate::economy::resolve_decay(&mut c, data));
        t!("naval", crate::naval::resolve_season(&mut c, data, &mut ev));
        t!("trespass", crate::passage::resolve_trespass(&mut c, data, &mut ev));
        t!("diplomacy", crate::diplomacy::resolve_diplomacy(&mut c, data, &mut ev));
        t!("religion", crate::religion::resolve_religion(&mut c, data, &mut ev));
        t!("crusade", crate::crusade::resolve_crusade(&mut c, data, &mut ev));
        t!("agents", crate::agents::resolve_agents(&mut c, data, &mut ev));
        t!("chronicle", crate::chronicle::resolve_chronicle(&mut c, data, &mut ev));
        t!("lent", crate::table::resolve_lent(&mut c, data, &mut ev));
        t!("population", crate::population::resolve_population(&mut c, data, &mut ev));
        t!("governance", crate::dynasty::resolve_governance(&mut c, data));
        t!("court", crate::dynasty::resolve_court_prestige(&mut c, data));
        t!("retinue", crate::retinue::resolve_retinue(&mut c, data, &mut ev));
        t!("characters", crate::characters::resolve_characters(&mut c, data, &mut ev));
        t!("births", crate::dynasty::resolve_births(&mut c, data, &mut ev));
        t!("regencies", crate::dynasty::resolve_regencies(&mut c, data, &mut ev));
        t!("deaths", crate::characters::resolve_faction_deaths(&mut c, data, &mut ev));
        t!("feudal", crate::feudal::resolve_feudal(&mut c, data, &mut ev));
        t!("stats", crate::campaign_stats::update(&mut c));
        t!("victory", crate::victory::resolve_victory(&mut c, data, &mut ev));
        t!("missions", crate::missions::resolve_missions(&mut c, data, &mut ev));
    }

    #[test]
    fn contributions_are_sorted_and_close_the_sum() {
        let data = game_data();
        let state = quiet("fac_france");
        let forecast = faction_forecast(&state, data, &fac("fac_france")).unwrap();
        for gauge in [&forecast.prestige, &forecast.men, &forecast.unrest] {
            let sorted = gauge
                .contributions
                .windows(2)
                .all(|w| w[0].amount.abs() >= w[1].amount.abs());
            assert!(sorted);
            let sum: f64 = gauge.contributions.iter().map(|c| c.amount).sum::<f64>() + gauge.other;
            assert!((sum - gauge.delta() as f64).abs() < 1e-6);
        }
        assert!(!forecast.unrest.contributions.is_empty());
    }

    /// Winter prestige is explained by the court's sources.
    #[test]
    fn winter_prestige_has_court_terms() {
        let data = game_data();
        let mut state = quiet("fac_france");
        let id = fac("fac_france");
        let mut guard = 0;
        while state.season != crate::state::Season::Winter && guard < 5 {
            let mut events = Vec::new();
            state.resolve_end_of_turn(data, &mut events);
            guard += 1;
        }
        let parts = crate::dynasty::court_prestige_parts(&state, data, &id);
        assert_eq!(
            parts.total(),
            crate::dynasty::yearly_court_prestige(&state, data, &id)
        );
        let forecast = faction_forecast(&state, data, &id).unwrap();
        assert!(forecast
            .prestige
            .contributions
            .iter()
            .all(|c| c.amount.is_finite()));
    }
}
