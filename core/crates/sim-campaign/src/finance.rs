//! Royal finances (ECO2, ADR 0777): the structural charges of a crown (household,
//! officers' wages, sieges, dowries, tribute), the bankers' credit with its interest,
//! the royal default, and the extraordinary taxes. Every value is in
//! `data/rules/finance.json` ([`data_model::FinanceRules`]); this module is the mechanism.
//!
//! Money is meant to stay scarce: a faction that is rich in a season is asked to
//! choose, not to hoard. The projected season is [`StructuralCosts`], added to
//! [`crate::economy::TurnBudget`]; the loans and levies are orders.

use std::collections::BTreeMap;

use data_model::{FactionId, FinanceRules, GameData, Sex};
use serde::{Deserialize, Serialize};

use crate::cause_builders;
use crate::events::{EventKind, GameEvent};
use crate::plan_cache::PlanCache;
use crate::state::CampaignState;
use crate::Order;

/// A loan from one banking house.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Loan {
    /// Id of the lender in `finance.json`.
    pub lender: String,
    pub principal: i64,
    /// Turn the money was lent.
    pub since: u32,
}

/// What a recurring payment is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObligationKind {
    /// Marriage portion paid to the groom's realm.
    Dowry,
    /// Share of the income the bankers seize after a royal default.
    Seizure,
    /// Tribute or subsidy owed to another faction.
    Tribute,
}

impl ObligationKind {
    pub fn key(self) -> &'static str {
        match self {
            ObligationKind::Dowry => "dowry",
            ObligationKind::Seizure => "seizure",
            ObligationKind::Tribute => "tribute",
        }
    }
}

/// A payment due every season for a number of seasons.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Obligation {
    pub kind: ObligationKind,
    pub per_season: i64,
    pub seasons_left: u32,
    /// Who receives the money (`None`: the bankers, or nobody).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creditor: Option<FactionId>,
}

/// Unrest added to every province of the realm until `until_turn`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Grievance {
    /// Levy id, or `default`.
    pub source: String,
    /// Tenths of a point of unrest.
    pub unrest_tenths: i32,
    pub until_turn: u32,
}

/// Money moved by orders since the last resolved season (the budget's ledger).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Ledger {
    pub levied: i64,
    pub borrowed: i64,
    pub repaid: i64,
}

/// Finance state of one faction.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FinanceState {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub loans: Vec<Loan>,
    /// Credit rating (0-100); `None` until it first moves (`start_rating`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rating: Option<u32>,
    /// No banker lends before this turn.
    #[serde(default)]
    pub frozen_until: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub obligations: Vec<Obligation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub grievances: Vec<Grievance>,
    /// Levy id -> first turn it can be raised again.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub levy_ready: BTreeMap<String, u32>,
    #[serde(default)]
    pub ledger: Ledger,
    /// Royal defaults so far.
    #[serde(default)]
    pub defaults: u32,
    /// Levies raised so far.
    #[serde(default)]
    pub levies_raised: u32,
}

impl FinanceState {
    pub fn is_default(&self) -> bool {
        *self == FinanceState::default()
    }

    /// Principal owed to all the bankers.
    pub fn debt(&self) -> i64 {
        self.loans.iter().map(|l| l.principal).sum()
    }

    fn debt_to(&self, lender: &str) -> i64 {
        self.loans
            .iter()
            .filter(|l| l.lender == lender)
            .map(|l| l.principal)
            .sum()
    }
}

/// The recurring charges of a crown for one season, in livres.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StructuralCosts {
    pub household: i64,
    pub wages: i64,
    pub siege: i64,
    pub obligations: i64,
    pub interest: i64,
}

impl StructuralCosts {
    pub fn total(&self) -> i64 {
        self.household + self.wages + self.siege + self.obligations + self.interest
    }
}

/// Why a finance order was refused (French messages for the UI).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FinanceError {
    #[error("banquier inconnu : {0}")]
    UnknownLender(String),
    #[error("impôt inconnu : {0}")]
    UnknownLevy(String),
    #[error("le prêt minimal est de {0} livres")]
    BelowMinimum(i64),
    #[error("les banquiers ne prêtent plus à la couronne avant le tour {0}")]
    CreditFrozen(u32),
    #[error("{lender} ne prêtera pas plus de {room} livres à la couronne")]
    OverLimit { lender: String, room: i64 },
    #[error("rien à rembourser")]
    NoDebt,
    #[error("le trésor ne couvre pas {0} livres")]
    CannotAfford(i64),
    #[error("{0} ne peut pas encore être levé (disponible au tour {1})")]
    LevyNotReady(String, u32),
    #[error("{0} n'existe pas encore à cette date")]
    LevyTooEarly(String),
    #[error("une couronne sans province ne peut pas lever d'impôt")]
    NoProvince,
}

fn rules(data: &GameData) -> &FinanceRules {
    &data.finance_rules
}

/// Share of the income a rule is capped at, in livres.
fn capped(amount: f64, percent_cap: f64, gross: i64) -> i64 {
    let ceiling = gross.max(0) as f64 * percent_cap / 100.0;
    amount.min(ceiling).round().max(0.0) as i64
}

/// Percent of the gross tax income the household costs (rank and prestige).
pub fn household_percent(state: &CampaignState, data: &GameData, faction: &FactionId) -> f64 {
    let rules = &rules(data).household;
    let provinces = state.controlled_provinces(faction).count() as u32;
    let rank = rules
        .tiers
        .iter()
        .filter(|t| provinces >= t.min_provinces)
        .map(|t| t.percent)
        .fold(0.0, f64::max);
    if rank <= 0.0 {
        return 0.0;
    }
    let prestige = state
        .factions
        .get(faction)
        .and_then(|f| f.ruler.as_ref())
        .and_then(|r| state.characters.get(r))
        .map_or(0, |c| c.prestige)
        .max(0);
    (rank + f64::from(prestige) / 10.0 * rules.percent_per_10_prestige).min(rules.max_percent)
}

fn household_cost(state: &CampaignState, data: &GameData, faction: &FactionId, gross: i64) -> i64 {
    (gross.max(0) as f64 * household_percent(state, data, faction) / 100.0).round() as i64
}

fn wages_cost(state: &CampaignState, data: &GameData, faction: &FactionId, gross: i64) -> i64 {
    let rules = &rules(data).officers;
    let governors = state
        .characters
        .values()
        .filter(|c| c.alive && &c.faction == faction && c.governor_of.is_some())
        .count() as i64;
    let commanders = state
        .armies
        .values()
        .filter(|a| &a.faction == faction && a.general.is_some())
        .count() as i64;
    let total = governors * rules.governor_wage + commanders * rules.commander_wage;
    capped(total as f64, rules.max_percent, gross)
}

fn siege_cost(state: &CampaignState, data: &GameData, faction: &FactionId, gross: i64) -> i64 {
    let rules = &rules(data).siege;
    if rules.per_siege <= 0 {
        return 0;
    }
    let total: f64 = state
        .settlements
        .values()
        .filter_map(|s| s.siege.as_ref())
        .filter(|s| &s.attacker == faction)
        .map(|s| {
            let growth = (i64::from(s.turns_elapsed) * rules.growth_percent_per_season)
                .min(rules.growth_max_percent);
            rules.per_siege as f64 * (100 + growth) as f64 / 100.0
        })
        .sum();
    capped(total, rules.max_percent, gross)
}

/// Interest rate of a loan, in thousandths of the principal per season.
pub fn loan_rate_permille(rules: &FinanceRules, lender: &data_model::Lender, rating: u32) -> f64 {
    f64::from(lender.rate_permille)
        + f64::from(100u32.saturating_sub(rating)) * rules.credit.rate_penalty_permille_per_point
}

/// Credit rating of `faction` (0-100).
pub fn rating(state: &CampaignState, data: &GameData, faction: &FactionId) -> u32 {
    state
        .factions
        .get(faction)
        .and_then(|f| f.finance.rating)
        .unwrap_or(rules(data).credit.start_rating)
}

fn interest_cost(state: &CampaignState, data: &GameData, faction: &FactionId) -> i64 {
    let rules = rules(data);
    let Some(f) = state.factions.get(faction) else {
        return 0;
    };
    let rating = rating(state, data, faction);
    f.finance
        .loans
        .iter()
        .filter_map(|loan| {
            let lender = rules.lender(&loan.lender)?;
            Some(loan.principal as f64 * loan_rate_permille(rules, lender, rating) / 1000.0)
        })
        .sum::<f64>()
        .round() as i64
}

/// The recurring charges of `faction` for the coming season; `gross` is its gross tax income
/// (`CampaignState::faction_income`).
pub fn structural_costs(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    gross: i64,
) -> StructuralCosts {
    let Some(f) = state.factions.get(faction) else {
        return StructuralCosts::default();
    };
    if faction.is_rebels() {
        return StructuralCosts::default();
    }
    StructuralCosts {
        household: household_cost(state, data, faction, gross),
        wages: wages_cost(state, data, faction, gross),
        siege: siege_cost(state, data, faction, gross),
        obligations: f.finance.obligations.iter().map(|o| o.per_season).sum(),
        interest: interest_cost(state, data, faction),
    }
}

/// Flat unrest the faction's grievances add to each of its provinces (extraordinary taxes,
/// royal default).
pub fn finance_unrest(state: &CampaignState, faction: &FactionId) -> f64 {
    state.factions.get(faction).map_or(0.0, |f| {
        f.finance
            .grievances
            .iter()
            .filter(|g| g.until_turn > state.turn)
            .map(|g| f64::from(g.unrest_tenths) / 10.0)
            .sum()
    })
}

/// How much `lender` would still lend to `faction` right now (0 when credit is frozen).
pub fn credit_room(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    lender_id: &str,
    gross: i64,
) -> i64 {
    let rules = rules(data);
    let (Some(lender), Some(f)) = (rules.lender(lender_id), state.factions.get(faction)) else {
        return 0;
    };
    if f.finance.frozen_until > state.turn {
        return 0;
    }
    let scale = if rules.credit.rating_scales_limit {
        f64::from(rating(state, data, faction)) / 100.0
    } else {
        1.0
    };
    let limit = (lender.limit_seasons * gross.max(0) as f64 * scale).floor() as i64;
    (limit - f.finance.debt_to(lender_id)).max(0)
}

fn player_note(state: &mut CampaignState, faction: &FactionId, event: GameEvent) {
    if faction == &state.player_faction {
        state.pending_events.push(event);
    }
}

/// `Borrow` order: lends `amount` livres from `lender_id`.
pub fn borrow(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    lender_id: &str,
    amount: i64,
) -> Result<(), FinanceError> {
    let rules = rules(data);
    let lender = rules
        .lender(lender_id)
        .ok_or_else(|| FinanceError::UnknownLender(lender_id.to_owned()))?;
    if amount < lender.min_loan {
        return Err(FinanceError::BelowMinimum(lender.min_loan));
    }
    let frozen_until = state.factions[faction].finance.frozen_until;
    if frozen_until > state.turn {
        return Err(FinanceError::CreditFrozen(frozen_until));
    }
    let gross = state.faction_income(data, faction);
    let room = credit_room(state, data, faction, lender_id, gross);
    if amount > room {
        return Err(FinanceError::OverLimit {
            lender: lender.name.clone(),
            room,
        });
    }
    let turn = state.turn;
    let name = lender.name.clone();
    let f = state.factions.get_mut(faction).expect("checked by caller");
    f.treasury += amount;
    f.finance.ledger.borrowed += amount;
    f.finance.loans.push(Loan {
        lender: lender_id.to_owned(),
        principal: amount,
        since: turn,
    });
    let label = data.faction_label(faction);
    player_note(
        state,
        faction,
        GameEvent::new(
            EventKind::Income,
            format!("{label} emprunte {amount} livres à {name}."),
        )
        .faction(faction),
    );
    Ok(())
}

/// `Repay` order: pays back up to `amount` livres, the dearest loans first.
pub fn repay(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    amount: i64,
) -> Result<i64, FinanceError> {
    let rules = rules(data);
    let rating = rating(state, data, faction);
    let f = state.factions.get_mut(faction).expect("checked by caller");
    if f.finance.loans.is_empty() {
        return Err(FinanceError::NoDebt);
    }
    let to_pay = amount.min(f.finance.debt()).min(f.treasury.max(0));
    if to_pay <= 0 || amount > f.treasury {
        return Err(FinanceError::CannotAfford(amount));
    }
    let rate = |loan: &Loan| {
        rules
            .lender(&loan.lender)
            .map_or(0.0, |l| loan_rate_permille(rules, l, rating))
    };
    f.finance.loans.sort_by(|a, b| {
        rate(b)
            .partial_cmp(&rate(a))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.since.cmp(&b.since))
    });
    let mut left = to_pay;
    for loan in &mut f.finance.loans {
        let paid = left.min(loan.principal);
        loan.principal -= paid;
        left -= paid;
        if left == 0 {
            break;
        }
    }
    f.finance.loans.retain(|l| l.principal > 0);
    f.treasury -= to_pay;
    f.finance.ledger.repaid += to_pay;
    Ok(to_pay)
}

/// Yield of `levy` for `gross` income.
pub fn levy_yield(levy: &data_model::Levy, gross: i64) -> i64 {
    (levy.income_seasons * gross.max(0) as f64).round() as i64
}

/// Levies available to `faction` at the moment: `(levy, ready_turn)` pairs, `ready_turn`
/// being 0 when the tax can be raised now.
pub fn levy_status<'a>(
    state: &CampaignState,
    data: &'a GameData,
    faction: &FactionId,
) -> Vec<(&'a data_model::Levy, u32)> {
    let Some(f) = state.factions.get(faction) else {
        return Vec::new();
    };
    rules(data)
        .levies
        .iter()
        .filter(|l| state.year >= l.from_year)
        .map(|l| {
            let ready = f.finance.levy_ready.get(&l.id).copied().unwrap_or(0);
            (l, if ready > state.turn { ready } else { 0 })
        })
        .collect()
}

/// `LevyExtraordinary` order: raises an extraordinary tax now, at the price of unrest.
pub fn levy(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    levy_id: &str,
) -> Result<i64, FinanceError> {
    let rules = rules(data);
    let levy = rules
        .levy(levy_id)
        .ok_or_else(|| FinanceError::UnknownLevy(levy_id.to_owned()))?;
    if state.year < levy.from_year {
        return Err(FinanceError::LevyTooEarly(levy.name.clone()));
    }
    let ready = state.factions[faction]
        .finance
        .levy_ready
        .get(levy_id)
        .copied()
        .unwrap_or(0);
    if ready > state.turn {
        return Err(FinanceError::LevyNotReady(levy.name.clone(), ready));
    }
    if state.controlled_provinces(faction).next().is_none() {
        return Err(FinanceError::NoProvince);
    }
    let gross = state.faction_income(data, faction);
    let amount = levy_yield(levy, gross);
    let before = state.factions[faction].treasury;
    let (turn, unrest_turns) = (state.turn, levy.unrest_turns);
    let f = state.factions.get_mut(faction).expect("checked by caller");
    f.treasury += amount;
    f.finance.ledger.levied += amount;
    f.finance.levies_raised += 1;
    f.finance
        .levy_ready
        .insert(levy_id.to_owned(), turn + levy.cooldown_turns);
    f.finance.grievances.push(Grievance {
        source: levy_id.to_owned(),
        unrest_tenths: (levy.unrest * 10.0).round() as i32,
        until_turn: turn + unrest_turns,
    });
    let name = levy.name.clone();
    let label = data.faction_label(faction);
    let cause = cause_builders::levy_cause(&cause_builders::LevyFacts {
        faction,
        yield_livres: amount,
        unrest: levy.unrest.round() as i64,
        treasury_before: before,
    });
    player_note(
        state,
        faction,
        GameEvent::new(
            EventKind::Income,
            format!("{label} lève {name} : {amount} livres, mais les provinces grondent."),
        )
        .faction(faction)
        .cause(cause),
    );
    Ok(amount)
}

/// Dowry of a marriage between `bride_faction` and `groom_faction` (no-op within one realm).
pub fn book_dowry(
    state: &mut CampaignState,
    data: &GameData,
    bride_faction: &FactionId,
    groom_faction: &FactionId,
) {
    let rules = &rules(data).dowry;
    if rules.installments == 0 || rules.income_seasons <= 0.0 || bride_faction == groom_faction {
        return;
    }
    if !state.factions.contains_key(bride_faction) || !state.factions.contains_key(groom_faction) {
        return;
    }
    let gross = state.faction_income(data, bride_faction);
    let total = (rules.income_seasons * gross.max(0) as f64).round() as i64;
    let per_season = total / i64::from(rules.installments);
    if per_season <= 0 {
        return;
    }
    state
        .factions
        .get_mut(bride_faction)
        .expect("checked")
        .finance
        .obligations
        .push(Obligation {
            kind: ObligationKind::Dowry,
            per_season,
            seasons_left: rules.installments,
            creditor: Some(groom_faction.clone()),
        });
}

/// The faction of the bride and of the groom of a marriage between two characters.
pub(crate) fn dowry_for_marriage(
    state: &mut CampaignState,
    data: &GameData,
    a: &data_model::CharacterId,
    b: &data_model::CharacterId,
) {
    let (Some(ca), Some(cb)) = (state.characters.get(a), state.characters.get(b)) else {
        return;
    };
    let (bride, groom) = if ca.sex == Sex::Female {
        (ca.faction.clone(), cb.faction.clone())
    } else {
        (cb.faction.clone(), ca.faction.clone())
    };
    book_dowry(state, data, &bride, &groom);
}

/// End of season, after the economy has booked the budget: obligations tick (the creditors
/// are paid), grievances lapse, the rating recovers, a hopeless debt ends in default.
pub(crate) fn resolve_finance(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let rules = rules(data);
    let ids: Vec<FactionId> = state.factions.keys().cloned().collect();
    let turn = state.turn;
    for id in ids {
        if id.is_rebels() || !state.factions[&id].alive {
            continue;
        }
        // Obligations: the budget charged the payer; the creditor receives now.
        let obligations = std::mem::take(
            &mut state
                .factions
                .get_mut(&id)
                .expect("listed")
                .finance
                .obligations,
        );
        let mut kept = Vec::new();
        for mut ob in obligations {
            if let Some(creditor) = ob.creditor.as_ref().and_then(|c| state.factions.get_mut(c)) {
                creditor.treasury += ob.per_season;
            }
            ob.seasons_left = ob.seasons_left.saturating_sub(1);
            if ob.seasons_left > 0 {
                kept.push(ob);
            }
        }
        let f = state.factions.get_mut(&id).expect("listed");
        f.finance.obligations = kept;
        f.finance.grievances.retain(|g| g.until_turn > turn);
        if f.finance.frozen_until <= turn {
            let current = f.finance.rating.unwrap_or(rules.credit.start_rating);
            if current < rules.credit.start_rating {
                f.finance.rating =
                    Some((current + rules.credit.rating_recovery).min(rules.credit.start_rating));
            }
        }
        maybe_default(state, data, &id, events);
    }
}

/// The royal default of a crown whose treasury is empty and whose debt cannot be carried.
fn maybe_default(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    events: &mut Vec<GameEvent>,
) {
    let rules = rules(data);
    let rule = &rules.credit.default;
    let f = &state.factions[faction];
    let debt = f.finance.debt();
    let gross = f.last_budget.income.max(1);
    if f.treasury >= 0 || debt <= 0 || (debt as f64) <= rule.debt_seasons * gross as f64 {
        return;
    }
    let turn = state.turn;
    let seasons_of_debt = debt * 100 / gross;
    let written_off = debt * rule.write_off_percent / 100;
    let treasury_before = f.treasury;
    let f = state.factions.get_mut(faction).expect("checked");
    // The bankers lose their share of every loan.
    for loan in &mut f.finance.loans {
        loan.principal -= loan.principal * rule.write_off_percent / 100;
    }
    f.finance.loans.retain(|l| l.principal > 0);
    if rule.clear_treasury {
        f.treasury = 0;
    }
    let current = f.finance.rating.unwrap_or(rules.credit.start_rating);
    f.finance.rating = Some(current.saturating_sub(rule.rating_loss));
    f.finance.frozen_until = turn + rule.freeze_turns;
    f.finance.defaults += 1;
    let seizure = (gross as f64 * rule.seizure_percent / 100.0).round() as i64;
    if seizure > 0 && rule.seizure_turns > 0 {
        f.finance.obligations.push(Obligation {
            kind: ObligationKind::Seizure,
            per_season: seizure,
            seasons_left: rule.seizure_turns,
            creditor: None,
        });
    }
    f.finance.grievances.push(Grievance {
        source: "default".to_owned(),
        unrest_tenths: (rule.unrest * 10.0).round() as i32,
        until_turn: turn + rule.unrest_turns,
    });
    if let Some(ruler) = f.ruler.clone() {
        if let Some(c) = state.characters.get_mut(&ruler) {
            c.prestige -= rule.prestige_loss;
        }
    }
    let label = data.faction_label(faction);
    let mut event = GameEvent::new(
        EventKind::Bankruptcy,
        format!(
            "Banqueroute royale : {label} cesse de payer ses banquiers ({written_off} livres effacés, crédit gelé jusqu'au tour {}).",
            turn + rule.freeze_turns
        ),
    )
    .faction(faction);
    if faction == &state.player_faction {
        event = event.cause(cause_builders::default_cause(&cause_builders::DefaultFacts {
            faction,
            debt_seasons_percent: seasons_of_debt,
            written_off_percent: rule.write_off_percent,
            freeze_turns: i64::from(rule.freeze_turns),
            seizure,
            treasury: treasury_before,
        }));
    }
    events.push(event);
}

// ----- AI -------------------------------------------------------------------------------

/// The finance orders of an AI faction: repay when rich, otherwise raise an extraordinary tax
/// if the realm is calm, otherwise borrow from the cheapest banker. At most one inflow per
/// season.
pub fn ai_finance_orders(cache: &PlanCache, data: &GameData, faction: &FactionId) -> Vec<Order> {
    let state = cache.state();
    let rules = rules(data);
    let Some(f) = state.factions.get(faction).filter(|f| f.alive) else {
        return Vec::new();
    };
    if faction.is_rebels() || rules.credit.lenders.is_empty() {
        return Vec::new();
    }
    let gross = cache.faction_income(data, faction).max(1);
    let ai = &rules.ai;
    let last = &f.last_budget;
    let net_last = last.receipts() - last.upkeep();
    let expected = f.treasury + net_last.min(0);
    let reserve = (ai.repay_above_seasons * gross as f64) as i64;
    if f.finance.debt() > 0 && f.treasury > reserve && net_last >= 0 {
        let amount = (f.treasury - reserve).min(f.finance.debt());
        return vec![Order::Repay { amount }];
    }
    let need_below = (ai.borrow_below_seasons * gross as f64) as i64;
    if expected >= need_below {
        return Vec::new();
    }
    // A calm realm raises a tax first.
    if state.mean_unrest(faction) < ai.levy_max_unrest {
        let best = levy_status(state, data, faction)
            .into_iter()
            .filter(|(_, ready)| *ready == 0)
            .max_by(|(a, _), (b, _)| {
                let key = |l: &data_model::Levy| l.income_seasons / (1.0 + l.unrest);
                key(a)
                    .partial_cmp(&key(b))
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(b.id.cmp(&a.id))
            });
        if let Some((levy, _)) = best {
            return vec![Order::LevyExtraordinary {
                levy: levy.id.clone(),
            }];
        }
    }
    let wanted = (ai.borrow_seasons * gross as f64) as i64;
    let mut lenders: Vec<&data_model::Lender> = rules.credit.lenders.iter().collect();
    lenders.sort_by_key(|l| (l.rate_permille, l.id.clone()));
    for lender in lenders {
        let room = credit_room(state, data, faction, &lender.id, gross);
        let amount = wanted.min(room);
        if amount >= lender.min_loan {
            return vec![Order::Borrow {
                lender: lender.id.clone(),
                amount,
            }];
        }
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_empty_and_skipped() {
        let s = FinanceState::default();
        assert!(s.is_default());
        assert_eq!(s.debt(), 0);
    }

    #[test]
    fn debt_sums_the_principals() {
        let mut s = FinanceState::default();
        for (lender, principal) in [("a", 100), ("b", 50), ("a", 25)] {
            s.loans.push(Loan {
                lender: lender.into(),
                principal,
                since: 0,
            });
        }
        assert_eq!(s.debt(), 175);
        assert_eq!(s.debt_to("a"), 125);
    }
}
