//! Lot DP2 (ADR 0075): readable refusals (every weighted reason), the
//! single blocking point and its counter-offer, and the trade agreement
//! with a rival (low value, on purpose).

use std::path::PathBuf;

use data_model::{FactionId, GameData};
use sim_campaign::diplomacy::RelationKind;
use sim_campaign::negotiation::{evaluate_treaty, Article, Party};
use sim_campaign::religion::faction_religion;
use sim_campaign::treaty_explain::{explain_treaty, ACCEPT_CHANCE};
use sim_campaign::CampaignState;

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _warnings) = GameData::load(&root).expect("game data loads");
    data
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn start(data: &GameData) -> CampaignState {
    CampaignState::new_1337(data, fac("fac_france"), 1).expect("1337 start")
}

/// A faction at plain peace with France and of its faith, first by id (OM:
/// the first peaceful faction by id is now a distant power of another faith,
/// whose extra objections hide the single blocking point under test).
fn neutral(state: &CampaignState, data: &GameData) -> FactionId {
    let fr = fac("fac_france");
    let faith = faction_religion(state, data, &fr);
    state
        .factions
        .iter()
        .find(|(id, f)| {
            f.alive
                && **id != fr
                && state.relation(&fr, id) == RelationKind::Peace
                && faction_religion(state, data, id) == faith
        })
        .map(|(id, _)| id.clone())
        .expect("a neutral")
}

#[test]
fn every_weighted_reason_is_a_line_objections_first() {
    let data = data();
    let state = start(&data);
    let (fr, other) = (fac("fac_france"), neutral(&state, &data));
    let treaty = vec![
        Article::TradeAgreement,
        Article::Gold {
            giver: Party::Recipient,
            amount: 1000,
        },
    ];
    let verdict = evaluate_treaty(&state, &data, &fr, &other, &treaty);
    let explanation = explain_treaty(&state, &data, &fr, &other, &treaty);
    assert_eq!(explanation.chance, verdict.chance);
    // Every reason, with its weight: the lines add up to the score.
    let total: i32 = explanation.lines.iter().map(|l| l.value).sum();
    assert_eq!(total, verdict.score, "{explanation:#?}");
    let reasons = verdict.context.len()
        + verdict
            .articles
            .iter()
            .map(|a| a.reasons.len())
            .sum::<usize>();
    assert_eq!(explanation.lines.len(), reasons);
    // Objections first, heaviest first; then arguments, strongest first.
    let values: Vec<i32> = explanation.lines.iter().map(|l| l.value).collect();
    let split = values.iter().position(|v| *v > 0).unwrap_or(values.len());
    assert!(
        values[..split].windows(2).all(|w| w[0] <= w[1]),
        "{values:?}"
    );
    assert!(
        values[split..].windows(2).all(|w| w[0] >= w[1]),
        "{values:?}"
    );
    // Article lines name their article, general ones speak for them.
    assert!(explanation
        .lines
        .iter()
        .any(|l| l.article == Some(1) && l.text.contains(" — ")));
    assert!(explanation
        .lines
        .iter()
        .any(|l| l.article.is_none() && l.text == "Prudence devant tout engagement"));
    assert!(!explanation.summary.is_empty());
}

#[test]
fn a_single_excessive_demand_is_named_and_lowered_in_a_counter_offer() {
    let data = data();
    let mut state = start(&data);
    let (fr, other) = (fac("fac_france"), neutral(&state, &data));
    state.factions.get_mut(&other).unwrap().treasury = 50_000;
    // A generous offer spoiled by one greedy demand.
    let treaty = vec![
        Article::Gold {
            giver: Party::Proposer,
            amount: 3000,
        },
        Article::Gold {
            giver: Party::Recipient,
            amount: 20_000,
        },
    ];
    state.factions.get_mut(&fr).unwrap().treasury = 50_000;
    let explanation = explain_treaty(&state, &data, &fr, &other, &treaty);
    assert!(!explanation.accept, "{explanation:#?}");
    let blocker = explanation.blocker.as_ref().expect("one point blocks");
    assert_eq!(blocker.article, Some(1));
    assert!(blocker.value < 0);
    assert!(
        explanation.summary.contains("un seul point"),
        "{}",
        explanation.summary
    );
    let counter = explanation.counter.as_ref().expect("a counter-offer");
    assert!(explanation.counter_chance >= ACCEPT_CHANCE);
    assert_eq!(
        evaluate_treaty(&state, &data, &fr, &other, counter).chance,
        explanation.counter_chance
    );
    // The demand is lowered (or dropped), the offer kept.
    assert_eq!(counter[0], treaty[0]);
    match counter.get(1) {
        Some(Article::Gold {
            giver: Party::Recipient,
            amount,
        }) => assert!(*amount < 20_000),
        None => {}
        other => panic!("unexpected counter {other:?}"),
    }
    assert!(!explanation.counter_text.is_empty());
}

#[test]
fn a_general_consideration_can_be_the_single_blocking_point() {
    let data = data();
    let mut state = start(&data);
    let (fr, other) = (fac("fac_france"), neutral(&state, &data));
    // They loathe us: a gift alone does not suffice.
    let turn = state.turn;
    state.factions.get_mut(&other).unwrap().modifiers.push(
        sim_campaign::diplomacy::OpinionModifier {
            with: fr.clone(),
            value: -200,
            reason_fr: "Test".to_owned(),
            expires_turn: turn + 10,
        },
    );
    state.factions.get_mut(&fr).unwrap().treasury = 50_000;
    let treaty = vec![Article::Gold {
        giver: Party::Proposer,
        amount: 3000,
    }];
    let explanation = explain_treaty(&state, &data, &fr, &other, &treaty);
    assert!(!explanation.accept, "{explanation:#?}");
    let blocker = explanation.blocker.as_ref().expect("the attitude blocks");
    assert_eq!(blocker.article, None);
    assert_eq!(blocker.text, "Ils ne vous aiment guère");
}

#[test]
fn several_objections_give_no_single_point() {
    let data = data();
    let mut state = start(&data);
    let (fr, other) = (fac("fac_france"), neutral(&state, &data));
    state.factions.get_mut(&other).unwrap().treasury = 100_000;
    let turn = state.turn;
    state.factions.get_mut(&other).unwrap().modifiers.push(
        sim_campaign::diplomacy::OpinionModifier {
            with: fr.clone(),
            value: -200,
            reason_fr: "Test".to_owned(),
            expires_turn: turn + 10,
        },
    );
    let treaty = vec![
        Article::Gold {
            giver: Party::Recipient,
            amount: 30_000,
        },
        Article::MilitaryAccess {
            giver: Party::Recipient,
        },
    ];
    let explanation = explain_treaty(&state, &data, &fr, &other, &treaty);
    assert!(!explanation.accept);
    assert!(explanation.blocker.is_none(), "{explanation:#?}");
    assert!(explanation.counter.is_none());
    assert!(
        explanation.summary.contains("entre autres"),
        "{}",
        explanation.summary
    );
}

#[test]
fn an_acceptable_treaty_says_why() {
    let data = data();
    let mut state = start(&data);
    let (fr, other) = (fac("fac_france"), neutral(&state, &data));
    state.factions.get_mut(&fr).unwrap().treasury = 50_000;
    let treaty = vec![Article::Gold {
        giver: Party::Proposer,
        amount: 10_000,
    }];
    let explanation = explain_treaty(&state, &data, &fr, &other, &treaty);
    assert!(explanation.accept, "{explanation:#?}");
    assert!(explanation.blocker.is_none());
    assert!(explanation.summary.contains("accepterait"));
}

/// C5R noted a low value of a trade agreement with England: the rival
/// malus (« Enrichir un rival », −12) is intended (ADR 0075 § 4). The
/// agreement stays possible once the attitude is good or with a sweetener.
#[test]
fn a_trade_agreement_with_a_rival_is_worth_little_on_purpose() {
    let data = data();
    let state = start(&data);
    let (fr, en) = (fac("fac_france"), fac("fac_england"));
    assert!(sim_campaign::diplomacy::rivals(&state, &en).contains(&fr));
    let explanation = explain_treaty(&state, &data, &fr, &en, &[Article::TradeAgreement]);
    let rival = explanation
        .lines
        .iter()
        .find(|l| l.text.ends_with("Enrichir un rival"))
        .expect("the rival malus is shown");
    assert_eq!(rival.value, -12);
    // Between two factions that are not rivals, the same article is worth
    // at least the rival malus more.
    let other = neutral(&state, &data);
    let with_rival = evaluate_treaty(&state, &data, &fr, &en, &[Article::TradeAgreement]);
    let plain = evaluate_treaty(&state, &data, &fr, &other, &[Article::TradeAgreement]);
    let rival_article = with_rival.articles[0].value;
    let plain_article = plain.articles[0].value;
    let common_routes = |v: &sim_campaign::negotiation::TreatyEvaluation| {
        v.articles[0]
            .reasons
            .iter()
            .filter(|(t, _)| t.starts_with("Routes") || t.starts_with("Richesse"))
            .map(|(_, x)| x)
            .sum::<i32>()
    };
    assert_eq!(
        rival_article - common_routes(&with_rival) + 12,
        plain_article - common_routes(&plain),
        "{with_rival:#?} {plain:#?}"
    );
}
