//! Lot WH `diplob` (ADR 0282, 0283): league against a hegemon, « Rejoindre la
//! guerre contre X », an ally's call to the player, non-aggression pact and
//! defensive alliance, AI ultimatums.

use data_model::test_support::{fac, game_data};
use data_model::{FactionId, GameData};
use sim_campaign::diplomacy::{plan_diplomacy, LEAGUE_REASON};
use sim_campaign::negotiation::{
    evaluate_treaty, Article, DiplomaticLedger, Party, Rupture, Treaty,
};
use sim_campaign::plan_cache::PlanCache;
use sim_campaign::test_support::idle;
use sim_campaign::{CampaignState, EventKind, Order};

fn start(seed: u64) -> CampaignState {
    let mut state = CampaignState::new_1337(game_data(), fac("fac_france"), seed).expect("start");
    state.chronicle.disabled = true;
    state
}

fn ally(state: &mut CampaignState, a: &str, b: &str) {
    for (x, y) in [(a, b), (b, a)] {
        let f = state.factions.get_mut(&fac(x)).unwrap();
        f.at_war_with.remove(&fac(y));
        f.allies.insert(fac(y));
    }
}

/// Peace between every pair of the listed factions, no truce.
fn calm(state: &mut CampaignState, names: &[&str]) {
    for a in names {
        for b in names {
            if a != b {
                let f = state.factions.get_mut(&fac(a)).unwrap();
                f.at_war_with.remove(&fac(b));
                f.truces.remove(&fac(b));
                f.allies.remove(&fac(b));
            }
        }
    }
}

fn goodwill(state: &mut CampaignState, holder: &str, with: &str, value: i32) {
    let expires_turn = state.turn + 40;
    state
        .factions
        .get_mut(&fac(holder))
        .unwrap()
        .modifiers
        .push(sim_campaign::OpinionModifier {
            with: fac(with),
            value,
            reason_fr: "Bonne volonté".to_owned(),
            expires_turn,
        });
}

/// Hands provinces to `faction` (taken from the others, the player's last)
/// until it holds the league's share, and strips every other army.
fn make_hegemon(state: &mut CampaignState, data: &GameData, faction: &str) {
    let rules = &data.ai_diplomacy.league;
    let total = state
        .provinces
        .keys()
        .filter(|p| state.province_owner(p).is_some())
        .count();
    let wanted = (rules.province_share * total as f64).ceil() as usize + 1;
    let hegemon = fac(faction);
    let mut held = state.owned_provinces(&hegemon).len();
    let ids: Vec<_> = state.provinces.keys().cloned().collect();
    for province in ids {
        if held >= wanted {
            break;
        }
        let Some(owner) = state.province_owner(&province).cloned() else {
            continue;
        };
        if owner == hegemon || owner == state.player_faction || owner.is_rebels() {
            continue;
        }
        let places: Vec<_> = state
            .settlements_of(&province)
            .map(|(id, _)| id.clone())
            .collect();
        for id in places {
            let s = state.settlements.get_mut(&id).unwrap();
            s.owner = hegemon.clone();
            s.controller = hegemon.clone();
        }
        held += 1;
    }
    state.armies.retain(|_, a| a.faction == hegemon);
    for s in state.settlements.values_mut() {
        if s.controller != hegemon {
            s.garrison.clear();
        }
    }
}

fn turn_with_diplomacy(state: &mut CampaignState, data: &GameData) -> Vec<sim_campaign::GameEvent> {
    state.end_turn_with(data, |s, d, f| {
        if f.is_rebels() {
            return Vec::new();
        }
        let cache = PlanCache::new(s);
        plan_diplomacy(&cache, d, f)
    })
}

// ----- league --------------------------------------------------------------

#[test]
fn league_forms_against_a_hegemon_and_weighs_on_attitude() {
    let data = game_data();
    let mut state = start(11);
    assert_eq!(state.hegemon(data), None, "no hegemon in 1337");
    make_hegemon(&mut state, data, "fac_england");
    assert_eq!(state.hegemon(data), Some(fac("fac_england")));
    let events = state.end_turn_with(data, idle);
    let league = state.league.clone().expect("league formed");
    assert_eq!(league.target, fac("fac_england"));
    assert!(events
        .iter()
        .any(|e| e.kind == EventKind::Diplomacy && e.text_fr.contains("se liguent")));
    // Everyone not bound to England likes it less; its ally does not.
    ally(&mut state, "fac_england", "fac_portugal");
    let third = state.attitude(data, &fac("fac_aragon"), &fac("fac_england"));
    assert!(third
        .1
        .iter()
        .any(|(t, v)| t == LEAGUE_REASON && *v == data.ai_diplomacy.league.attitude));
    let friend = state.attitude(data, &fac("fac_portugal"), &fac("fac_england"));
    assert!(!friend.1.iter().any(|(t, _)| t == LEAGUE_REASON));
    // War on the hegemon needs no other casus belli.
    assert!(state
        .casus_belli(data, &fac("fac_aragon"), &fac("fac_england"))
        .is_some());
    // The league is announced once.
    let again = state.end_turn_with(data, idle);
    assert!(!again.iter().any(|e| e.text_fr.contains("se liguent")));
}

#[test]
fn league_dissolves_once_the_hegemon_is_cut_down() {
    let data = game_data();
    let mut state = start(12);
    make_hegemon(&mut state, data, "fac_england");
    state.end_turn_with(data, idle);
    assert!(state.league.is_some());
    // The hegemon loses its lands; the league outlives them until it expires.
    for s in state.settlements.values_mut() {
        if s.owner == fac("fac_england") {
            s.owner = fac("fac_france");
            s.controller = fac("fac_france");
        }
    }
    state.end_turn_with(data, idle);
    assert!(state.league.is_some(), "a league lasts its minimum term");
    state.turn += data.ai_diplomacy.league.duration_turns;
    let events = state.end_turn_with(data, idle);
    assert!(state.league.is_none());
    assert!(events.iter().any(|e| e.text_fr.contains("se dissout")));
}

#[test]
fn enemies_of_the_hegemon_ally_with_each_other() {
    let data = game_data();
    let mut state = start(13);
    make_hegemon(&mut state, data, "fac_england");
    let neighbours = ["fac_aragon", "fac_castile", "fac_navarre", "fac_portugal"];
    let mut everyone: Vec<&str> = neighbours.to_vec();
    everyone.push("fac_england");
    calm(&mut state, &everyone);
    for a in neighbours {
        for b in neighbours {
            goodwill(&mut state, a, b, 15);
        }
    }
    let bonds = |s: &CampaignState| {
        neighbours
            .iter()
            .flat_map(|a| neighbours.iter().map(move |b| (a, b)))
            .filter(|(a, b)| a < b && s.is_allied(&fac(a), &fac(b)))
            .count()
    };
    let before = bonds(&state);
    for _ in 0..12 {
        turn_with_diplomacy(&mut state, data);
        if bonds(&state) > before {
            break;
        }
    }
    assert!(
        bonds(&state) > before,
        "the neighbours of the hegemon band together"
    );
}

// ----- « Rejoindre la guerre contre X » ----------------------------------------

#[test]
fn join_war_is_checked_valued_and_applied() {
    let data = game_data();
    let mut state = start(21);
    let (fr, pt, ca) = (fac("fac_france"), fac("fac_portugal"), fac("fac_castile"));
    calm(&mut state, &["fac_france", "fac_portugal", "fac_castile"]);
    ally(&mut state, "fac_france", "fac_portugal");
    let join = vec![Article::JoinWar {
        giver: Party::Recipient,
        target: ca.clone(),
    }];
    // France is not at war with Castile: nothing to join.
    assert!(sim_campaign::negotiation::check_treaty(&state, data, &fr, &pt, &join).is_err());
    state
        .submit_order(data, Order::DeclareWar { target: ca.clone() })
        .unwrap();
    assert!(sim_campaign::negotiation::check_treaty(&state, data, &fr, &pt, &join).is_ok());
    // Portugal allied with Castile cannot be asked.
    let mut bound = state.clone();
    ally(&mut bound, "fac_portugal", "fac_castile");
    assert!(sim_campaign::negotiation::check_treaty(&bound, data, &fr, &pt, &join).is_err());
    // Rich, eager, rested Portugal accepts; the war is declared by it.
    goodwill(&mut state, "fac_portugal", "fac_france", 60);
    goodwill(&mut state, "fac_portugal", "fac_castile", -60);
    state.factions.get_mut(&pt).unwrap().treasury = 50_000;
    state.factions.get_mut(&pt).unwrap().ledger.weariness = 0;
    let verdict = evaluate_treaty(&state, data, &fr, &pt, &join);
    assert!(verdict.accept, "{:?}", verdict.reasons());
    state
        .submit_order(
            data,
            Order::ProposeTreaty {
                target: pt.clone(),
                articles: join.clone(),
            },
        )
        .unwrap();
    assert!(state.is_at_war(&pt, &ca));
    assert!(state.is_at_war(&fr, &ca));
}

#[test]
fn join_war_is_refused_by_a_weary_or_broke_ally() {
    let data = game_data();
    let mut state = start(22);
    let (fr, pt, ca) = (fac("fac_france"), fac("fac_portugal"), fac("fac_castile"));
    calm(&mut state, &["fac_france", "fac_portugal", "fac_castile"]);
    ally(&mut state, "fac_france", "fac_portugal");
    state
        .submit_order(data, Order::DeclareWar { target: ca.clone() })
        .unwrap();
    let join = vec![Article::JoinWar {
        giver: Party::Recipient,
        target: ca.clone(),
    }];
    goodwill(&mut state, "fac_portugal", "fac_france", 10);
    state.factions.get_mut(&pt).unwrap().treasury = 50_000;
    state.factions.get_mut(&pt).unwrap().ledger.weariness = 0;
    let rested = evaluate_treaty(&state, data, &fr, &pt, &join).score;
    state.factions.get_mut(&pt).unwrap().ledger.weariness = 100;
    let tired = evaluate_treaty(&state, data, &fr, &pt, &join);
    assert!(tired.score < rested);
    assert!(tired
        .detailed_reasons()
        .iter()
        .any(|(t, v)| t.contains("Épuisement") && *v < 0));
    // Broke: the article cannot be executed, with the reason.
    state.factions.get_mut(&pt).unwrap().treasury = -5;
    let broke = evaluate_treaty(&state, data, &fr, &pt, &join);
    assert!(!broke.accept);
    assert!(broke.blocked.unwrap().contains("trésor vide"));
    // The explanation names the objections.
    let explanation = sim_campaign::treaty_explain::explain_treaty(&state, data, &fr, &pt, &join);
    assert!(!explanation.accept);
}

#[test]
fn promising_to_join_their_war_is_worth_points() {
    let data = game_data();
    let mut state = start(23);
    let (fr, pt, ca) = (fac("fac_france"), fac("fac_portugal"), fac("fac_castile"));
    calm(&mut state, &["fac_france", "fac_portugal", "fac_castile"]);
    state
        .factions
        .get_mut(&pt)
        .unwrap()
        .at_war_with
        .insert(ca.clone());
    state
        .factions
        .get_mut(&ca)
        .unwrap()
        .at_war_with
        .insert(pt.clone());
    let promise = vec![Article::JoinWar {
        giver: Party::Proposer,
        target: ca.clone(),
    }];
    let verdict = evaluate_treaty(&state, data, &fr, &pt, &promise);
    assert!(verdict.articles[0].value > 0, "{:?}", verdict.reasons());
}

// ----- an ally's call to the player ------------------------------------------

fn attacked_ally() -> (CampaignState, FactionId, FactionId) {
    let data = game_data();
    let mut state = start(31);
    let (fr, pt, ca) = (fac("fac_france"), fac("fac_portugal"), fac("fac_castile"));
    calm(&mut state, &["fac_france", "fac_portugal", "fac_castile"]);
    ally(&mut state, "fac_france", "fac_portugal");
    state.declare_war(data, &ca, &pt).unwrap();
    assert!(state.is_at_war(&ca, &pt));
    let _ = fr;
    (state, pt, ca)
}

#[test]
fn attacked_ally_calls_the_player_by_an_offer() {
    let data = game_data();
    let (state, pt, ca) = attacked_ally();
    let fr = fac("fac_france");
    // No war by default: the player is asked.
    assert!(!state.is_at_war(&fr, &ca));
    let offers = &state.factions[&fr].offers;
    let call = offers
        .iter()
        .find(|o| o.proposal.is_ally_call())
        .expect("ally call offer");
    assert_eq!(call.from, pt);
    assert!(call.text_fr.contains("réclame votre aide"));
    let _ = data;
}

#[test]
fn accepting_the_call_means_war() {
    let data = game_data();
    let (mut state, _, ca) = attacked_ally();
    let fr = fac("fac_france");
    let id = state.factions[&fr].offers[0].id;
    state
        .submit_order(
            data,
            Order::AnswerOffer {
                offer: id,
                accept: true,
            },
        )
        .unwrap();
    assert!(state.is_at_war(&fr, &ca));
    assert!(state.is_allied(&fr, &fac("fac_portugal")));
}

#[test]
fn refusing_the_call_costs_the_alliance_and_a_grudge() {
    let data = game_data();
    let (mut state, pt, ca) = attacked_ally();
    let fr = fac("fac_france");
    let id = state.factions[&fr].offers[0].id;
    state
        .submit_order(
            data,
            Order::AnswerOffer {
                offer: id,
                accept: false,
            },
        )
        .unwrap();
    assert!(!state.is_at_war(&fr, &ca));
    assert!(!state.is_allied(&fr, &pt));
    let (_, reasons) = state.attitude(data, &pt, &fr);
    assert!(reasons
        .iter()
        .any(|(t, v)| t == "A refusé l'appel aux armes"
            && *v == data.ai_diplomacy.ally_call.refuse_attitude));
    assert!(state.factions[&fr]
        .ledger
        .history
        .iter()
        .any(|r| r.rupture == Some(Rupture::RefusedCall) && r.with == pt));
}

#[test]
fn an_unanswered_call_expires_as_a_refusal() {
    let data = game_data();
    let (mut state, pt, ca) = attacked_ally();
    let fr = fac("fac_france");
    for _ in 0..4 {
        state.end_turn_with(data, idle);
    }
    assert!(state.factions[&fr]
        .offers
        .iter()
        .all(|o| !o.proposal.is_ally_call()));
    assert!(!state.is_at_war(&fr, &ca));
    assert!(!state.is_allied(&fr, &pt));
}

// ----- non-aggression pact and defensive alliance ------------------------------

#[test]
fn non_aggression_pact_binds_like_a_truce() {
    let data = game_data();
    let mut state = start(41);
    let (fr, ar) = (fac("fac_france"), fac("fac_aragon"));
    calm(&mut state, &["fac_france", "fac_aragon"]);
    goodwill(&mut state, "fac_aragon", "fac_france", 40);
    let pact = vec![Article::NonAggression { turns: 12 }];
    assert!(evaluate_treaty(&state, data, &fr, &ar, &pact).accept);
    state
        .submit_order(
            data,
            Order::ProposeTreaty {
                target: ar.clone(),
                articles: pact.clone(),
            },
        )
        .unwrap();
    assert!(state.has_non_aggression(&fr, &ar) && state.has_non_aggression(&ar, &fr));
    assert!(sim_campaign::negotiation::check_treaty(&state, data, &fr, &ar, &pact).is_err());
    // The AI no longer picks the signatory as a target.
    // Breaking it is perjury.
    let cost = state.declaration_prestige_cost(data, &fr, &ar);
    assert_eq!(cost, sim_campaign::diplomacy::PERJURY_PRESTIGE);
    state
        .submit_order(data, Order::DeclareWar { target: ar.clone() })
        .unwrap();
    assert!(state.is_at_war(&fr, &ar));
    assert!(!state.has_non_aggression(&fr, &ar));
    let (_, reasons) = state.attitude(data, &fac("fac_castile"), &fr);
    assert!(reasons.iter().any(|(t, _)| t.contains("Parjure")));
    assert!(state.factions[&fr].ledger.history.iter().any(|r| {
        r.rupture == Some(Rupture::Perjury) && r.articles == vec!["non_aggression".to_owned()]
    }));
}

#[test]
fn non_aggression_pact_expires() {
    let data = game_data();
    let mut state = start(42);
    let (fr, ar) = (fac("fac_france"), fac("fac_aragon"));
    calm(&mut state, &["fac_france", "fac_aragon"]);
    goodwill(&mut state, "fac_aragon", "fac_france", 40);
    state
        .submit_order(
            data,
            Order::ProposeTreaty {
                target: ar.clone(),
                articles: vec![Article::NonAggression { turns: 4 }],
            },
        )
        .unwrap();
    assert!(state.has_non_aggression(&fr, &ar));
    for _ in 0..6 {
        state.end_turn_with(data, idle);
    }
    assert!(!state.has_non_aggression(&fr, &ar));
    assert!(state.factions[&fr].ledger.non_aggression.is_empty());
}

#[test]
fn defensive_alliance_is_lighter_and_upgrades() {
    let data = game_data();
    let mut state = start(43);
    let (fr, pt) = (fac("fac_france"), fac("fac_portugal"));
    calm(&mut state, &["fac_france", "fac_portugal"]);
    state.factions.get_mut(&pt).unwrap().allies.clear();
    state.factions.get_mut(&fr).unwrap().allies.clear();
    let military = evaluate_treaty(&state, data, &fr, &pt, &[Article::Alliance]).score;
    let defensive = evaluate_treaty(&state, data, &fr, &pt, &[Article::DefensiveAlliance]);
    eprintln!("{:?}", defensive.detailed_reasons());
    let defensive = defensive.score;
    assert!(defensive > military, "{defensive} > {military}");
    goodwill(&mut state, "fac_portugal", "fac_france", 80);
    state
        .submit_order(
            data,
            Order::ProposeTreaty {
                target: pt.clone(),
                articles: vec![Article::DefensiveAlliance],
            },
        )
        .unwrap();
    assert!(state.is_allied(&fr, &pt));
    assert!(state.is_defensive_alliance(&fr, &pt) && state.is_defensive_alliance(&pt, &fr));
    // A military alliance replaces it.
    state
        .submit_order(
            data,
            Order::ProposeTreaty {
                target: pt.clone(),
                articles: vec![Article::Alliance],
            },
        )
        .unwrap();
    assert!(state.is_allied(&fr, &pt) && !state.is_defensive_alliance(&fr, &pt));
}

/// Does `faction` declare war on `enemy` in an odd season of 4..16 (those
/// where it only looks for an ally's war to join, never a war of its own)?
fn declares_on(state: &mut CampaignState, data: &GameData, faction: &str, enemy: &str) -> bool {
    let slot = faction
        .bytes()
        .fold(0u32, |a, b| a.wrapping_add(u32::from(b)));
    let (faction, enemy) = (fac(faction), fac(enemy));
    (4..16).filter(|turn| (turn + slot) % 2 == 1).any(|turn| {
        state.turn = turn;
        let cache = PlanCache::new(state);
        plan_diplomacy(&cache, data, &faction)
            .iter()
            .any(|o| matches!(o, Order::DeclareWar { target } if *target == enemy))
    })
}

#[test]
fn only_military_allies_follow_offensive_wars() {
    let data = game_data();
    let mut state = start(45);
    let names = ["fac_france", "fac_scotland", "fac_england"];
    calm(&mut state, &names);
    state
        .factions
        .get_mut(&fac("fac_scotland"))
        .unwrap()
        .treasury = 50_000;
    state
        .factions
        .get_mut(&fac("fac_scotland"))
        .unwrap()
        .ledger
        .weariness = 0;
    goodwill(&mut state, "fac_scotland", "fac_france", 60);
    goodwill(&mut state, "fac_scotland", "fac_england", -60);
    ally(&mut state, "fac_france", "fac_scotland");
    for (a, b) in [("fac_france", "fac_england"), ("fac_england", "fac_france")] {
        state
            .factions
            .get_mut(&fac(a))
            .unwrap()
            .at_war_with
            .insert(fac(b));
    }
    let mut defensive = state.clone();
    for (a, b) in [
        ("fac_france", "fac_scotland"),
        ("fac_scotland", "fac_france"),
    ] {
        defensive
            .factions
            .get_mut(&fac(a))
            .unwrap()
            .ledger
            .defensive_allies
            .insert(fac(b));
    }
    assert!(
        declares_on(&mut state, data, "fac_scotland", "fac_england"),
        "a military ally follows France into war"
    );
    assert!(
        !declares_on(&mut defensive, data, "fac_scotland", "fac_england"),
        "a defensive ally stays out of an offensive war"
    );
}

#[test]
fn an_old_save_reads_as_military_alliances() {
    let ledger: DiplomaticLedger = serde_json::from_str(r#"{"weariness": 3}"#).unwrap();
    assert!(ledger.defensive_allies.is_empty());
    assert!(ledger.non_aggression.is_empty());
    let article: Article = serde_json::from_str(r#"{"kind":"alliance"}"#).unwrap();
    assert_eq!(article, Article::Alliance);
    let state = start(44);
    let json = serde_json::to_string(&state).unwrap();
    assert!(!json.contains("defensive_allies"));
    let back: CampaignState = serde_json::from_str(&json).unwrap();
    assert!(back.league.is_none());
}

// ----- AI ultimatums ----------------------------------------------------------

fn ultimatum_scene() -> (CampaignState, FactionId, FactionId) {
    let data = game_data();
    let mut state = start(51);
    let (fr, en) = (fac("fac_france"), fac("fac_england"));
    calm(&mut state, &["fac_france", "fac_england"]);
    // England dwarfs a disarmed France.
    state.armies.retain(|_, a| a.faction != fr);
    for s in state.settlements.values_mut() {
        if s.controller == fr {
            s.garrison.clear();
        }
    }
    let kin: Vec<_> = state.factions[&fr].allies.iter().cloned().collect();
    for k in kin {
        state.factions.get_mut(&k).unwrap().allies.remove(&fr);
    }
    state.factions.get_mut(&fr).unwrap().allies.clear();
    state.factions.get_mut(&en).unwrap().treasury = 100_000;
    let _ = data;
    (state, en, fr)
}

#[test]
fn an_ultimatum_is_an_offer_with_a_threat() {
    let data = game_data();
    let (mut state, en, fr) = ultimatum_scene();
    let demand = Treaty::single(Article::Tribute {
        giver: Party::Recipient,
        per_season: 500,
        seasons: 8,
    });
    assert!(demand.is_ultimatum());
    assert!(!Treaty::single(Article::Peace).is_ultimatum());
    state.propose(data, &en, &fr, demand).unwrap();
    let offer = state.factions[&fr].offers.last().unwrap().clone();
    assert!(offer.text_fr.contains("ultimatum"));
    assert!(state.factions[&en].ledger.ultimatum_sent.contains_key(&fr));
    // Refused: offence, casus belli, war next season.
    state
        .submit_order(
            data,
            Order::AnswerOffer {
                offer: offer.id,
                accept: false,
            },
        )
        .unwrap();
    assert!(state.factions[&en]
        .ledger
        .ultimatum_refused
        .contains_key(&fr));
    assert_eq!(
        state.casus_belli(data, &en, &fr).as_deref(),
        Some("ultimatum refusé").or(state.casus_belli(data, &en, &fr).as_deref())
    );
    state.turn += 1;
    let cache = PlanCache::new(&state);
    let orders = plan_diplomacy(&cache, data, &en);
    assert!(orders
        .iter()
        .any(|o| matches!(o, Order::DeclareWar { target } if *target == fr)));
}

#[test]
fn an_accepted_ultimatum_pays_and_spares_the_war() {
    let data = game_data();
    let (mut state, en, fr) = ultimatum_scene();
    state
        .propose(
            data,
            &en,
            &fr,
            Treaty::single(Article::Tribute {
                giver: Party::Recipient,
                per_season: 500,
                seasons: 8,
            }),
        )
        .unwrap();
    let id = state.factions[&fr].offers.last().unwrap().id;
    state
        .submit_order(
            data,
            Order::AnswerOffer {
                offer: id,
                accept: true,
            },
        )
        .unwrap();
    assert!(!state.is_at_war(&en, &fr));
    assert_eq!(state.factions[&fr].ledger.tributes.len(), 1);
    assert!(state.factions[&en].ledger.ultimatum_refused.is_empty());
}

#[test]
fn an_expired_ultimatum_counts_as_refused() {
    let data = game_data();
    let (mut state, en, fr) = ultimatum_scene();
    state
        .propose(
            data,
            &en,
            &fr,
            Treaty::single(Article::Tribute {
                giver: Party::Recipient,
                per_season: 500,
                seasons: 8,
            }),
        )
        .unwrap();
    for _ in 0..4 {
        state.end_turn_with(data, idle);
    }
    assert!(state.factions[&en]
        .ledger
        .ultimatum_refused
        .contains_key(&fr));
}

#[test]
fn a_strong_ai_sends_an_ultimatum_before_war_then_waits() {
    let data = game_data();
    let (mut state, en, fr) = ultimatum_scene();
    let mut sent = None;
    for _ in 0..16 {
        let cache = PlanCache::new(&state);
        let orders = plan_diplomacy(&cache, data, &en);
        let ultimatum = orders.iter().find(|o| {
            matches!(o, Order::ProposeTreaty { target, articles }
                if *target == fr && Treaty::new(articles.clone()).is_ultimatum())
        });
        assert!(
            !orders
                .iter()
                .any(|o| matches!(o, Order::DeclareWar { target } if *target == fr)),
            "no war before the ultimatum: {orders:?} (power {} vs {})",
            cache.coalition_power(&en),
            cache.coalition_power(&fr)
        );
        if let Some(order) = ultimatum {
            sent = Some(order.clone());
            break;
        }
        state.turn += 1;
    }
    let order = sent.expect("England sends an ultimatum");
    let (target, treaty) = order.proposal(&state, &en).unwrap();
    state.propose(data, &en, &target, treaty).unwrap();
    // Cooldown: the same AI does not send another one, nor declare war.
    for _ in 0..6 {
        state.turn += 1;
        let cache = PlanCache::new(&state);
        let orders = plan_diplomacy(&cache, data, &en);
        assert!(!orders
            .iter()
            .any(|o| matches!(o, Order::ProposeTreaty { target, .. } if *target == fr)));
        assert!(!orders
            .iter()
            .any(|o| matches!(o, Order::DeclareWar { target } if *target == fr)));
    }
}
