//! ECO2 (ADR 0777): the crown's charges, the bankers' credit, the royal default, the
//! extraordinary taxes, the dowry and the AI's finances.

use sim_campaign::causes::CauseKind;
use sim_campaign::finance;
use sim_campaign::plan_cache::PlanCache;
use sim_campaign::test_support::{idle, start_quiet};
use sim_campaign::{EventKind, Order};

use data_model::test_support::{fac, game_data};

#[test]
fn the_household_grows_with_rank_and_spares_small_realms() {
    let data = game_data();
    let state = start_quiet(data, "fac_france", 5);
    let france = fac("fac_france");
    let gross = state.faction_income(data, &france);
    let costs = finance::structural_costs(&state, data, &france, gross);
    assert!(costs.household > gross / 10, "France keeps a court");
    assert!(costs.household < gross / 2);
    // A one-province county pays no household.
    let small = state
        .factions
        .keys()
        .find(|id| !id.is_rebels() && state.controlled_provinces(id).count() == 1 && **id != france)
        .cloned()
        .expect("a small realm");
    let small_gross = state.faction_income(data, &small);
    assert_eq!(
        finance::structural_costs(&state, data, &small, small_gross).household,
        0
    );
    // The projected budget carries the charge.
    let economy = state.faction_economy(data, &france).unwrap();
    assert_eq!(economy.household, costs.household);
}

#[test]
fn borrowing_credits_the_treasury_and_costs_interest() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 5);
    let france = fac("fac_france");
    let before = state.factions[&france].treasury;
    let interest_before = state.faction_economy(data, &france).unwrap().interest;
    assert_eq!(interest_before, 0);
    state
        .submit_order(
            data,
            Order::Borrow {
                lender: "lombards".into(),
                amount: 10_000,
            },
        )
        .expect("within the limit");
    assert_eq!(state.factions[&france].treasury, before + 10_000);
    assert_eq!(state.factions[&france].finance.debt(), 10_000);
    let economy = state.faction_economy(data, &france).unwrap();
    assert_eq!(economy.interest, 450, "4.5 % a season");
    assert_eq!(economy.debt, 10_000);
    state.end_turn_with(data, idle);
    let budget = state.factions[&france].last_budget;
    assert_eq!(budget.interest, 450);
    assert_eq!(budget.borrowed, 10_000);
}

/// GAMEPLAY2 (ADR 0788), judge finding C2: the court's levy on a dormant hoard counts the
/// treasury net of the principal owed. Borrowing used to push the "administration" charge up
/// (the bankers' coin was taxed as the house's own savings).
#[test]
fn borrowing_does_not_raise_the_administration_charge() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 5);
    let france = fac("fac_france");
    let income = state.faction_income(data, &france);
    let before = state.administration_upkeep_for(data, &france, income);
    state
        .submit_order(
            data,
            Order::Borrow {
                lender: "lombards".into(),
                amount: 31_000,
            },
        )
        .expect("a loan");
    let after = state.administration_upkeep_for(data, &france, income);
    assert_eq!(after, before, "the borrowed coin is not dormant savings");
    // The same sum earned instead of borrowed is still taxed.
    let mut state = start_quiet(data, "fac_france", 5);
    state.factions.get_mut(&france).unwrap().treasury += 31_000;
    let earned = state.administration_upkeep_for(data, &france, income);
    assert!(earned > before, "a hoard of one's own is still taxed");
}

#[test]
fn credit_has_a_limit_a_minimum_and_a_known_banker() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 5);
    let over = state.submit_order(
        data,
        Order::Borrow {
            lender: "lombards".into(),
            amount: 10_000_000,
        },
    );
    assert!(over.is_err(), "beyond two seasons of income");
    assert!(state
        .submit_order(
            data,
            Order::Borrow {
                lender: "lombards".into(),
                amount: 10,
            },
        )
        .is_err());
    assert!(state
        .submit_order(
            data,
            Order::Borrow {
                lender: "nobody".into(),
                amount: 5_000,
            },
        )
        .is_err());
}

#[test]
fn repaying_pays_the_dearest_loan_first() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 5);
    let france = fac("fac_france");
    for (lender, amount) in [("lombards", 5_000), ("florentines", 6_000)] {
        state
            .submit_order(
                data,
                Order::Borrow {
                    lender: lender.into(),
                    amount,
                },
            )
            .unwrap();
    }
    state
        .submit_order(data, Order::Repay { amount: 6_000 })
        .unwrap();
    let loans = &state.factions[&france].finance.loans;
    assert_eq!(loans.len(), 1);
    assert_eq!(loans[0].lender, "lombards", "the florentine loan is gone");
    assert!(state
        .submit_order(
            data,
            Order::Repay {
                amount: 100_000_000
            }
        )
        .is_err());
}

#[test]
fn an_extraordinary_tax_pays_at_once_angers_the_provinces_and_has_a_cooldown() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 5);
    let france = fac("fac_france");
    let before = state.factions[&france].treasury;
    state
        .submit_order(
            data,
            Order::LevyExtraordinary {
                levy: "aide".into(),
            },
        )
        .unwrap();
    let gross = state.faction_income(data, &france);
    assert_eq!(state.factions[&france].treasury, before + (gross + 1) / 2);
    assert!(finance::finance_unrest(&state, &france) > 0.0);
    assert!(
        state
            .submit_order(
                data,
                Order::LevyExtraordinary {
                    levy: "aide".into(),
                },
            )
            .is_err(),
        "cooldown"
    );
    // The gabelle appears in 1341 only.
    assert!(state
        .submit_order(
            data,
            Order::LevyExtraordinary {
                levy: "gabelle".into(),
            },
        )
        .is_err());
    state.end_turn_with(data, idle);
    assert_eq!(
        state.factions[&france].last_budget.levy_income,
        (gross + 1) / 2
    );
    // The levy is in the journal of causes, with its cause.
    assert!(state
        .cause_journal()
        .records()
        .any(|r| r.cause.kind == CauseKind::ExtraordinaryTax));
}

#[test]
fn a_hopeless_debt_ends_in_a_royal_default() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 5);
    let france = fac("fac_france");
    {
        let f = state.factions.get_mut(&france).unwrap();
        for lender in ["lombards", "florentines"] {
            f.finance.loans.push(finance::Loan {
                lender: lender.into(),
                principal: 400_000,
                since: 0,
            });
        }
        f.treasury = -50_000;
    }
    let debt_before = state.factions[&france].finance.debt();
    state.end_turn_with(data, idle);
    let f = &state.factions[&france];
    assert_eq!(f.finance.defaults, 1);
    assert!(f.finance.debt() < debt_before, "the bankers lose a share");
    assert!(f.treasury >= 0, "the negative treasury is wiped out");
    assert!(f.finance.frozen_until > state.turn, "credit frozen");
    assert!(f.finance.rating.unwrap() < 100);
    assert!(f
        .finance
        .obligations
        .iter()
        .any(|o| o.kind == finance::ObligationKind::Seizure));
    assert!(state
        .submit_order(
            data,
            Order::Borrow {
                lender: "lombards".into(),
                amount: 1_000,
            },
        )
        .is_err());
    let event = state
        .events()
        .iter()
        .find(|e| e.kind == EventKind::Bankruptcy && e.text_fr.starts_with("Banqueroute royale"))
        .expect("default event");
    let record = state
        .cause_journal()
        .get(event.cause_id.expect("cause"))
        .unwrap();
    assert_eq!(record.cause.kind, CauseKind::RoyalDefault);
}

#[test]
fn debasing_the_money_is_explained_in_the_journal() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 5);
    state
        .submit_order(
            data,
            Order::SetCoinage {
                level: sim_campaign::coinage::CoinageLevel::Debased,
            },
        )
        .unwrap();
    state.end_turn_with(data, idle);
    assert!(state
        .cause_journal()
        .records()
        .any(|r| r.cause.kind == CauseKind::Debasement));
}

#[test]
fn a_dowry_goes_from_the_brides_house_to_the_grooms_realm() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 5);
    let (bride, groom) = (fac("fac_england"), fac("fac_france"));
    let gross = state.faction_income(data, &bride);
    let creditor_before = state.factions[&groom].treasury;
    finance_marriage(&mut state, data, &bride, &groom);
    let f = &state.factions[&bride];
    assert_eq!(f.finance.obligations.len(), 1);
    let per_season = f.finance.obligations[0].per_season;
    assert_eq!(per_season, gross / 4);
    state.end_turn_with(data, idle);
    // The groom's realm received an instalment (on top of its own budget).
    let net = state.factions[&groom].last_budget.income
        + state.factions[&groom].last_budget.trade_income
        - state.factions[&groom].last_budget.upkeep();
    let got = state.factions[&groom].treasury - creditor_before - net;
    assert!(
        got >= per_season,
        "instalment received ({got} vs {per_season})"
    );
    assert_eq!(
        state.factions[&bride].finance.obligations[0].seasons_left,
        3
    );
}

fn finance_marriage(
    state: &mut sim_campaign::CampaignState,
    data: &data_model::GameData,
    bride: &data_model::FactionId,
    groom: &data_model::FactionId,
) {
    // The marriage hook is crate-private; the same entry point is reached by an order
    // between two characters of different realms.
    let bride_char = state
        .characters
        .iter()
        .find(|(_, c)| {
            &c.faction == bride && c.alive && c.sex == data_model::Sex::Female && c.spouse.is_none()
        })
        .map(|(id, _)| id.clone());
    let groom_char = state
        .characters
        .iter()
        .find(|(_, c)| {
            &c.faction == groom && c.alive && c.sex == data_model::Sex::Male && c.spouse.is_none()
        })
        .map(|(id, _)| id.clone());
    if let (Some(b), Some(g)) = (bride_char, groom_char) {
        if sim_campaign::dynasty::propose_marriage(state, data, &g, &b).is_ok() {
            return;
        }
    }
    // No eligible pair in the data: book the dowry directly.
    sim_campaign::finance::book_dowry(state, data, bride, groom);
}

#[test]
fn a_poor_ai_realm_levies_or_borrows_and_a_rich_one_repays() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 5);
    let england = fac("fac_england");
    {
        let f = state.factions.get_mut(&england).unwrap();
        f.treasury = -5_000;
    }
    let cache = PlanCache::new(&state);
    let orders = finance::ai_finance_orders(&cache, data, &england);
    assert!(
        matches!(
            orders.as_slice(),
            [Order::LevyExtraordinary { .. }] | [Order::Borrow { .. }]
        ),
        "{orders:?}"
    );
    drop(cache);
    {
        let f = state.factions.get_mut(&england).unwrap();
        f.treasury = 500_000;
        f.finance.loans.push(finance::Loan {
            lender: "lombards".into(),
            principal: 10_000,
            since: 0,
        });
        f.last_budget.income = 20_000;
    }
    let cache = PlanCache::new(&state);
    let orders = finance::ai_finance_orders(&cache, data, &england);
    assert!(
        matches!(orders.as_slice(), [Order::Repay { amount: 10_000 }]),
        "{orders:?}"
    );
}

#[test]
fn old_saves_without_finance_state_load_with_an_empty_one() {
    let data = game_data();
    let state = start_quiet(data, "fac_france", 5);
    let json = serde_json::to_value(&state).unwrap();
    let france = json["factions"]["fac_france"].as_object().unwrap();
    assert!(!france.contains_key("finance"), "empty state is not saved");
    let back: sim_campaign::CampaignState = serde_json::from_value(json).unwrap();
    assert!(back.factions[&fac("fac_france")].finance.is_default());
}
