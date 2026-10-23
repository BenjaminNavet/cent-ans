//! View of the royal finances (ECO2, ADR 0777): the bankers and what they would lend, the
//! loans and their rate, the extraordinary taxes, the standing obligations and the risk of a
//! royal default. The budget lines themselves are in [`crate::hud::EconomyView`]. Everything is
//! read from `core`; the client only draws it and sends `Borrow`, `Repay` and
//! `LevyExtraordinary` orders.

use data_model::{FactionId, GameData};
use serde::Serialize;
use sim_campaign::finance;
use sim_campaign::CampaignState;

/// A banking house and what it would lend now.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LenderView {
    pub id: String,
    pub name: String,
    /// Interest per season, thousandths of the principal, at the faction's rating.
    pub rate_permille: f64,
    /// Livres still available to borrow (0 when the credit is frozen).
    pub room: i64,
    pub min_loan: i64,
}

/// A loan in force.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LoanView {
    pub lender: String,
    pub lender_name: String,
    pub principal: i64,
    pub rate_permille: f64,
    /// Livres of interest per season.
    pub interest: i64,
    pub since: u32,
}

/// An extraordinary tax.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LevyView {
    pub id: String,
    pub name: String,
    /// Livres raised at once if levied now.
    pub yield_livres: i64,
    pub unrest: f64,
    pub unrest_turns: u32,
    pub cooldown_turns: u32,
    /// Turns before it can be raised again (0: available).
    pub ready_in: u32,
}

/// A standing payment.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ObligationView {
    /// `dowry`, `seizure`, `tribute`.
    pub kind: String,
    pub per_season: i64,
    pub seasons_left: u32,
    pub creditor: Option<String>,
}

/// The finances of one faction.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FinanceView {
    pub debt: i64,
    /// Credit rating, 0-100.
    pub rating: u32,
    /// No loan before this turn (0 when the credit is open).
    pub frozen_until: u32,
    pub defaults: u32,
    /// Interest paid per season.
    pub interest: i64,
    /// Debt as a share of the level at which an empty treasury ends in default (100 = default).
    pub default_risk_percent: i64,
    /// Livres still owed in ransoms.
    pub ransom_owed: i64,
    pub lenders: Vec<LenderView>,
    pub loans: Vec<LoanView>,
    pub levies: Vec<LevyView>,
    pub obligations: Vec<ObligationView>,
    /// Unrest points the extraordinary taxes and the default add to each province now.
    pub grievance_unrest: f64,
}

/// Finances of `faction`; `None` for an unknown faction.
pub fn finance_view(
    data: &GameData,
    state: &CampaignState,
    faction: &FactionId,
) -> Option<FinanceView> {
    let f = state.factions.get(faction)?;
    let rules = &data.finance_rules;
    let gross = state.faction_income(data, faction);
    let rating = finance::rating(state, data, faction);
    let rate = |lender: &data_model::Lender| finance::loan_rate_permille(rules, lender, rating);
    let loans: Vec<LoanView> = f
        .finance
        .loans
        .iter()
        .filter_map(|loan| {
            let lender = rules.lender(&loan.lender)?;
            let rate_permille = rate(lender);
            Some(LoanView {
                lender: loan.lender.clone(),
                lender_name: lender.name.clone(),
                principal: loan.principal,
                rate_permille,
                interest: (loan.principal as f64 * rate_permille / 1000.0).round() as i64,
                since: loan.since,
            })
        })
        .collect();
    let threshold = rules.credit.default.debt_seasons * gross.max(1) as f64;
    Some(FinanceView {
        debt: f.finance.debt(),
        rating,
        frozen_until: if f.finance.frozen_until > state.turn {
            f.finance.frozen_until
        } else {
            0
        },
        defaults: f.finance.defaults,
        interest: loans.iter().map(|l| l.interest).sum(),
        default_risk_percent: if threshold > 0.0 {
            (f.finance.debt() as f64 * 100.0 / threshold).round() as i64
        } else {
            0
        },
        ransom_owed: sim_campaign::ransom::ransom_debt_total(state, faction),
        lenders: rules
            .credit
            .lenders
            .iter()
            .map(|lender| LenderView {
                id: lender.id.clone(),
                name: lender.name.clone(),
                rate_permille: rate(lender),
                room: finance::credit_room(state, data, faction, &lender.id, gross),
                min_loan: lender.min_loan,
            })
            .collect(),
        loans,
        levies: finance::levy_status(state, data, faction)
            .into_iter()
            .map(|(levy, ready)| LevyView {
                id: levy.id.clone(),
                name: levy.name.clone(),
                yield_livres: finance::levy_yield(levy, gross),
                unrest: levy.unrest,
                unrest_turns: levy.unrest_turns,
                cooldown_turns: levy.cooldown_turns,
                ready_in: ready.saturating_sub(state.turn),
            })
            .collect(),
        obligations: f
            .finance
            .obligations
            .iter()
            .map(|o| ObligationView {
                kind: o.kind.key().to_owned(),
                per_season: o.per_season,
                seasons_left: o.seasons_left,
                creditor: o.creditor.as_ref().map(|c| c.as_str().to_owned()),
            })
            .collect(),
        grievance_unrest: finance::finance_unrest(state, faction),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_finance_view_lists_bankers_levies_and_the_loans_in_force() {
        let data = data_model::test_support::game_data();
        let mut state = sim_campaign::test_support::start_quiet(data, "fac_france", 5);
        let france = FactionId::new("fac_france").unwrap();
        let view = finance_view(data, &state, &france).expect("france");
        assert_eq!(view.debt, 0);
        assert_eq!(view.lenders.len(), 2);
        assert!(view.lenders.iter().all(|l| l.room > 0));
        assert!(view
            .levies
            .iter()
            .any(|l| l.id == "aide" && l.ready_in == 0));
        // The gabelle only exists from 1341.
        assert!(view.levies.iter().all(|l| l.id != "gabelle"));
        state
            .submit_order(
                data,
                sim_campaign::Order::Borrow {
                    lender: "lombards".into(),
                    amount: 10_000,
                },
            )
            .unwrap();
        let view = finance_view(data, &state, &france).unwrap();
        assert_eq!(view.debt, 10_000);
        assert_eq!(view.loans.len(), 1);
        assert_eq!(view.interest, 450);
        assert!(view.default_risk_percent > 0);
        let economy = crate::hud::economy(data, &state, &france).unwrap();
        assert_eq!(economy.debt, 10_000);
        assert!(economy.household > 0);
        assert!(economy
            .budget_lines
            .iter()
            .any(|l| l.key == "household" && l.charge));
        assert!(finance_view(data, &state, &FactionId::new("fac_nobody").unwrap()).is_none());
    }
}
