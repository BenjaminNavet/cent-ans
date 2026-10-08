//! Lot DP1 (ADR 0025): multi-article treaties, their valuation and
//! acceptance chance, counter-proposals, war goals, war score and war
//! weariness.

use std::path::PathBuf;

use data_model::{FactionId, GameData, ProvinceId};
use sim_campaign::negotiation::{
    self, answer_roll, counter_proposal, evaluate_treaty, Article, Party, HOSTAGE_TURNS,
};
use sim_campaign::{CampaignState, Order, SettlementState};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _warnings) = GameData::load(&root).expect("game data loads");
    data
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn prov(id: &str) -> ProvinceId {
    ProvinceId::new(id).unwrap()
}

fn city_mut<'a>(state: &'a mut CampaignState, province: &str) -> &'a mut SettlementState {
    let id = state.province_city_id(&prov(province)).unwrap().clone();
    state.settlements.get_mut(&id).unwrap()
}

fn start(data: &GameData, player: &str, seed: u64) -> CampaignState {
    CampaignState::new_1337(data, fac(player), seed).expect("1337 start")
}

fn idle(_: &CampaignState, _: &GameData, _: &FactionId) -> Vec<Order> {
    Vec::new()
}

fn set_battle_score(state: &mut CampaignState, winner: &str, loser: &str, score: i32) {
    state
        .factions
        .get_mut(&fac(winner))
        .unwrap()
        .war_scores
        .insert(fac(loser), score);
    state
        .factions
        .get_mut(&fac(loser))
        .unwrap()
        .war_scores
        .insert(fac(winner), -score);
}

fn cede(giver: Party, province: &str) -> Article {
    Article::CedeProvince {
        giver,
        province: prov(province),
    }
}

// ----- valuation ---------------------------------------------------------------

#[test]
fn every_article_is_valued_with_reasons() {
    let data = data();
    let state = start(&data, "fac_france", 1);
    let (fr, en) = (fac("fac_france"), fac("fac_england"));
    let treaty = vec![
        Article::Peace,
        cede(Party::Recipient, "prov_guyenne"),
        Article::Gold {
            giver: Party::Proposer,
            amount: 2000,
        },
    ];
    let verdict = evaluate_treaty(&state, &data, &fr, &en, &treaty);
    assert_eq!(verdict.articles.len(), 3);
    assert!(verdict.articles[1].value < 0, "{verdict:?}");
    assert!(verdict.articles[2].value > 0, "{verdict:?}");
    assert!(verdict.context.iter().any(|(t, _)| t == "Attitude"));
    assert!(verdict.chance <= 100);
    assert!(verdict.blocked.is_none(), "{verdict:?}");
}

#[test]
fn demands_lower_and_gifts_raise_the_chance() {
    let data = data();
    let state = start(&data, "fac_france", 2);
    let (fr, en) = (fac("fac_france"), fac("fac_england"));
    let white = evaluate_treaty(&state, &data, &fr, &en, &[Article::Peace]).score;
    let greedy = evaluate_treaty(
        &state,
        &data,
        &fr,
        &en,
        &[Article::Peace, cede(Party::Recipient, "prov_guyenne")],
    )
    .score;
    let generous = evaluate_treaty(
        &state,
        &data,
        &fr,
        &en,
        &[
            Article::Peace,
            Article::Gold {
                giver: Party::Proposer,
                amount: 5000,
            },
        ],
    )
    .score;
    assert!(greedy < white, "{greedy} < {white}");
    assert!(generous > white, "{generous} > {white}");
}

#[test]
fn invalid_articles_are_blocked() {
    let data = data();
    let state = start(&data, "fac_france", 3);
    let (fr, en, ar) = (fac("fac_france"), fac("fac_england"), fac("fac_aragon"));
    // France does not own Kent.
    let bad = evaluate_treaty(
        &state,
        &data,
        &fr,
        &en,
        &[cede(Party::Proposer, "prov_kent")],
    );
    assert_eq!(bad.chance, 0);
    assert!(bad.blocked.is_some());
    // No peace without a war.
    assert!(negotiation::check_treaty(&state, &data, &fr, &ar, &[Article::Peace]).is_err());
    // Gold beyond the treasury.
    let treasury = state.factions[&fr].treasury;
    let rich = [Article::Gold {
        giver: Party::Proposer,
        amount: treasury + 1,
    }];
    assert!(negotiation::check_treaty(&state, &data, &fr, &ar, &rich).is_err());
    // No alliance while at war, unless the treaty makes peace.
    assert!(negotiation::check_treaty(&state, &data, &fr, &en, &[Article::Alliance]).is_err());
    assert!(negotiation::check_treaty(
        &state,
        &data,
        &fr,
        &en,
        &[Article::Peace, Article::Alliance]
    )
    .is_ok());
}

#[test]
fn a_crown_never_cedes_its_capital() {
    let data = data();
    let mut state = start(&data, "fac_france", 4);
    city_mut(&mut state, "prov_middlesex").controller = fac("fac_france");
    set_battle_score(&mut state, "fac_france", "fac_england", 100);
    let verdict = evaluate_treaty(
        &state,
        &data,
        &fac("fac_france"),
        &fac("fac_england"),
        &[Article::Peace, cede(Party::Recipient, "prov_middlesex")],
    );
    assert_eq!(verdict.chance, 0, "{verdict:?}");
}

#[test]
fn the_answer_is_a_stable_roll_per_season() {
    let data = data();
    let mut state = start(&data, "fac_france", 5);
    let (fr, en) = (fac("fac_france"), fac("fac_england"));
    let first = answer_roll(&state, &fr, &en);
    assert_eq!(first, answer_roll(&state, &fr, &en));
    assert!(first < 100);
    state.end_turn_with(&data, idle);
    // Another season, another roll (most likely different, always in range).
    assert!(answer_roll(&state, &fr, &en) < 100);
}

// ----- signing ------------------------------------------------------------------

#[test]
fn a_winning_peace_cedes_provinces_and_pays_gold() {
    let data = data();
    let mut state = start(&data, "fac_france", 6);
    let (fr, en) = (fac("fac_france"), fac("fac_england"));
    for p in ["prov_guyenne", "prov_gascogne", "prov_ponthieu"] {
        city_mut(&mut state, p).controller = fr.clone();
    }
    set_battle_score(&mut state, "fac_france", "fac_england", 100);
    state.factions.get_mut(&en).unwrap().ledger.weariness = 90;
    state.factions.get_mut(&en).unwrap().treasury = 10_000;
    let treaty = vec![
        Article::Peace,
        cede(Party::Recipient, "prov_guyenne"),
        Article::Gold {
            giver: Party::Recipient,
            amount: 500,
        },
    ];
    let verdict = evaluate_treaty(&state, &data, &fr, &en, &treaty);
    assert!(verdict.chance >= 95, "{verdict:?}");
    let before = state.factions[&fr].treasury;
    state
        .submit_order(
            &data,
            Order::ProposeTreaty {
                target: en.clone(),
                articles: treaty,
            },
        )
        .unwrap();
    assert!(!state.is_at_war(&fr, &en));
    assert!(state.has_truce(&fr, &en));
    assert_eq!(state.province_owner(&prov("prov_guyenne")), Some(&fr));
    assert_eq!(state.factions[&fr].treasury, before + 500);
    // Occupied provinces not ceded return to their owner.
    assert_eq!(state.province_controller(&prov("prov_gascogne")), Some(&en));
    let history = &state.factions[&fr].ledger.history;
    assert_eq!(history.len(), 1);
    assert!(history[0].accepted && history[0].proposed);
    assert!(state.factions[&en].ledger.history[0]
        .text_fr
        .contains("Guyenne"));
}

#[test]
fn a_refusal_is_reported_with_its_score() {
    let data = data();
    let mut state = start(&data, "fac_france", 7);
    let error = state
        .submit_order(
            &data,
            Order::ProposeTreaty {
                target: fac("fac_england"),
                articles: vec![
                    Article::Peace,
                    cede(Party::Recipient, "prov_guyenne"),
                    cede(Party::Recipient, "prov_gascogne"),
                    cede(Party::Recipient, "prov_kent"),
                ],
            },
        )
        .unwrap_err()
        .to_string();
    assert!(error.contains("score"), "{error}");
    assert!(state.is_at_war(&fac("fac_france"), &fac("fac_england")));
    // The player keeps a trace of the refused treaty.
    assert!(!state.factions[&fac("fac_france")].ledger.history[0].accepted);
}

#[test]
fn trade_agreements_are_stored_once_and_end_with_war() {
    let data = data();
    let mut state = start(&data, "fac_aragon", 8);
    let (ar, ca) = (fac("fac_aragon"), fac("fac_castile"));
    negotiation::apply_treaty(&mut state, &data, &ar, &ca, &[Article::TradeAgreement]).unwrap();
    assert!(state.factions[&ar].ledger.trade_agreements.contains(&ca));
    // C5 (ADR 0012): the treaty article is the trade agreement of the routes.
    assert!(state.has_trade_agreement(&ar, &ca) && state.has_trade_agreement(&ca, &ar));
    state.declare_war(&data, &ar, &ca).unwrap();
    state.end_turn_with(&data, idle);
    assert!(state.factions[&ar].ledger.trade_agreements.is_empty());
    assert!(state.factions[&ca].ledger.trade_agreements.is_empty());
}

#[test]
fn tributes_are_paid_each_season_then_end() {
    let data = data();
    let mut state = start(&data, "fac_aragon", 9);
    let (ar, ca) = (fac("fac_aragon"), fac("fac_castile"));
    let tribute = Article::Tribute {
        giver: Party::Proposer,
        per_season: 100,
        seasons: 2,
    };
    negotiation::apply_treaty(&mut state, &data, &ar, &ca, &[tribute]).unwrap();
    assert_eq!(state.factions[&ar].ledger.tributes.len(), 1);
    for _ in 0..3 {
        state.end_turn_with(&data, idle);
    }
    assert!(state.factions[&ar].ledger.tributes.is_empty());
}

#[test]
fn hostages_are_held_then_return_and_war_betrays_them() {
    let data = data();
    let mut state = start(&data, "fac_aragon", 10);
    let (ar, ca) = (fac("fac_aragon"), fac("fac_castile"));
    let ruler = state.factions[&ar].ruler.clone();
    let hostage = state
        .characters
        .iter()
        .find(|(id, c)| {
            c.alive
                && c.faction == ar
                && !c.captive
                && c.army.is_none()
                && Some(*id) != ruler.as_ref()
        })
        .map(|(id, _)| id.clone())
        .expect("an Aragonese courtier");
    let article = Article::Hostage {
        giver: Party::Proposer,
        character: hostage.clone(),
    };
    negotiation::apply_treaty(&mut state, &data, &ar, &ca, &[article]).unwrap();
    assert!(state.characters[&hostage].captive);
    assert_eq!(state.characters[&hostage].captor.as_ref(), Some(&ca));
    // Trust: Castile values Aragon's word more.
    let trust = |s: &CampaignState| {
        evaluate_treaty(s, &data, &ar, &ca, &[Article::TradeAgreement])
            .context
            .iter()
            .find(|(t, _)| t == "Confiance")
            .map_or(0, |(_, v)| *v)
    };
    assert!(trust(&state) > 0);
    // War: the pledge is betrayed.
    state.declare_war(&data, &ar, &ca).unwrap();
    state.end_turn_with(&data, idle);
    assert!(state.factions[&ca].ledger.hostages.is_empty());
    assert!(state.factions[&ca]
        .modifiers
        .iter()
        .any(|m| m.with == ar && m.reason_fr == negotiation::HOSTAGE_BETRAYAL_REASON));
    let _ = HOSTAGE_TURNS;
}

#[test]
fn releasing_a_captive_frees_him() {
    let data = data();
    let mut state = start(&data, "fac_france", 11);
    let (fr, en) = (fac("fac_france"), fac("fac_england"));
    let prisoner = state
        .characters
        .iter()
        .find(|(_, c)| c.alive && c.faction == en && c.army.is_none())
        .map(|(id, _)| id.clone())
        .unwrap();
    {
        let c = state.characters.get_mut(&prisoner).unwrap();
        c.captive = true;
        c.captor = Some(fr.clone());
    }
    let article = Article::ReleaseCaptive {
        giver: Party::Proposer,
        character: prisoner.clone(),
    };
    let verdict = evaluate_treaty(&state, &data, &fr, &en, &[Article::Peace, article.clone()]);
    assert!(verdict.articles[1].value > 0, "{verdict:?}");
    negotiation::apply_treaty(&mut state, &data, &fr, &en, &[Article::Peace, article]).unwrap();
    assert!(!state.characters[&prisoner].captive);
}

#[test]
fn counter_proposal_finds_what_it_would_take() {
    let data = data();
    let mut state = start(&data, "fac_france", 12);
    let (fr, en) = (fac("fac_france"), fac("fac_england"));
    state.factions.get_mut(&fr).unwrap().treasury = 50_000;
    state.factions.get_mut(&en).unwrap().ledger.weariness = 80;
    let greedy = vec![Article::Peace, cede(Party::Recipient, "prov_guyenne")];
    let before = evaluate_treaty(&state, &data, &fr, &en, &greedy).chance;
    let counter = counter_proposal(&state, &data, &fr, &en, &greedy).expect("a counter-offer");
    let after = evaluate_treaty(&state, &data, &fr, &en, &counter).chance;
    assert!(
        after >= 50 && after > before,
        "{before} -> {after}: {counter:?}"
    );
    assert!(counter.contains(&Article::Peace));
}

#[test]
fn the_player_receives_ai_treaties_as_offers() {
    let data = data();
    let mut state = start(&data, "fac_france", 13);
    let (fr, en) = (fac("fac_france"), fac("fac_england"));
    state
        .propose(
            &data,
            &en,
            &fr,
            sim_campaign::diplomacy::Proposal::Treaty {
                articles: vec![
                    Article::Peace,
                    Article::Gold {
                        giver: Party::Proposer,
                        amount: 100,
                    },
                ],
            },
        )
        .unwrap();
    let offer = state.factions[&fr].offers.last().cloned().expect("offer");
    assert!(offer.text_fr.contains("traité"), "{}", offer.text_fr);
    state.answer_offer(&data, &fr, offer.id, true).unwrap();
    assert!(!state.is_at_war(&fr, &en));
}

// ----- war goals, war score, weariness -------------------------------------------

#[test]
fn war_goals_fill_the_war_score() {
    let data = data();
    let mut state = start(&data, "fac_france", 14);
    let (fr, en) = (fac("fac_france"), fac("fac_england"));
    state.end_turn_with(&data, idle);
    let goals = state.factions[&fr]
        .ledger
        .war_goals
        .get(&en)
        .cloned()
        .unwrap_or_default();
    assert!(!goals.is_empty(), "France targets English lands");
    let before = state.war_score(&data, &fr, &en);
    let goal = goals[0].clone();
    let id = state.province_city_id(&goal).unwrap().clone();
    state.settlements.get_mut(&id).unwrap().controller = fr.clone();
    let after = state.war_score(&data, &fr, &en);
    let rules = &data.ai_diplomacy.negotiation;
    assert!(
        after >= before + rules.war_goal_score,
        "{before} -> {after} (goal {goal})"
    );
}

#[test]
fn weariness_grows_in_war_pushes_to_peace_and_stirs_unrest() {
    let data = data();
    let mut state = start(&data, "fac_france", 15);
    let (fr, en) = (fac("fac_france"), fac("fac_england"));
    let white_before = evaluate_treaty(&state, &data, &fr, &en, &[Article::Peace]).score;
    for _ in 0..6 {
        state.end_turn_with(&data, idle);
    }
    let weary = state.factions[&en].ledger.weariness;
    assert!(weary >= 6, "{weary}");
    state.factions.get_mut(&en).unwrap().ledger.weariness = 90;
    let white_after = evaluate_treaty(&state, &data, &fr, &en, &[Article::Peace]).score;
    assert!(
        white_after > white_before + 10,
        "{white_before} -> {white_after}"
    );
    state.end_turn_with(&data, idle);
    assert!(state.factions[&en].ledger.weariness_unrest > 0);
    // Peace: weariness recedes.
    negotiation::apply_treaty(&mut state, &data, &fr, &en, &[Article::Peace]).unwrap();
    let at_peace = state.factions[&en].ledger.weariness;
    state.end_turn_with(&data, idle);
    let still_at_war = !state.factions[&en]
        .at_war_with
        .iter()
        .any(|e| e.as_str() != "fac_rebels");
    if still_at_war {
        assert!(state.factions[&en].ledger.weariness < at_peace);
    }
}

#[test]
fn the_ai_winner_demands_what_it_holds() {
    let data = data();
    let mut state = start(&data, "fac_scotland", 16);
    let (fr, en) = (fac("fac_france"), fac("fac_england"));
    for p in ["prov_guyenne", "prov_gascogne"] {
        city_mut(&mut state, p).controller = fr.clone();
    }
    set_battle_score(&mut state, "fac_france", "fac_england", 60);
    state.factions.get_mut(&en).unwrap().ledger.weariness = 80;
    // Past the minimum length of a war.
    state.turn = 40;
    let order = negotiation::plan_peace(&state, &data, &fr).expect("France treats");
    let Order::ProposeTreaty { target, articles } = order else {
        panic!("a treaty");
    };
    assert_eq!(target, en);
    assert!(
        articles.iter().any(|a| matches!(
            a,
            Article::CedeProvince {
                giver: Party::Recipient,
                ..
            }
        )),
        "{articles:?}"
    );
}

#[test]
fn ledger_survives_a_save() {
    let data = data();
    let mut state = start(&data, "fac_aragon", 17);
    let (ar, ca) = (fac("fac_aragon"), fac("fac_castile"));
    negotiation::apply_treaty(
        &mut state,
        &data,
        &ar,
        &ca,
        &[
            Article::TradeAgreement,
            Article::MilitaryAccess {
                giver: Party::Recipient,
            },
        ],
    )
    .unwrap();
    let json = state.save_json();
    let back = CampaignState::load_json(&json).unwrap();
    assert_eq!(back.factions[&ar].ledger, state.factions[&ar].ledger);
    assert!(negotiation::has_military_access(&back, &ca, &ar));
}

#[test]
fn treaty_orders_parse_from_json() {
    let json = serde_json::json!({
        "type": "propose_treaty",
        "target": "fac_england",
        "articles": [
            {"kind": "peace"},
            {"kind": "cede_province", "giver": "recipient", "province": "prov_guyenne"},
            {"kind": "tribute", "giver": "proposer", "per_season": 200, "seasons": 8}
        ]
    });
    let order: Order = serde_json::from_value(json).unwrap();
    assert!(matches!(order, Order::ProposeTreaty { ref articles, .. } if articles.len() == 3));
}
