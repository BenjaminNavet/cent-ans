//! GAMEPLAY2 (ADR 0788): measures the reign score of three play profiles of the player's house.
//!
//! `cargo run --profile seed-bots -p seed-bots --example reign_probe -- [--seeds 1,2] [--faction fac_france]
//!   [--profiles passive,moderate,active] [--turns 520]`
//!
//! - `passive`: no order, no offer answered (the player lets the realm run).
//! - `active`: the strategic AI plays the house (`ai::plan_turn`, offers answered as the AI
//!   would): it builds, recruits, allies and fights, but does not pursue the act objectives.
//! - `decisive`: `active`, and the player carries out one decisive campaign per act on the
//!   territorial objectives (control of the provinces they name, expulsion of the foreign
//!   occupiers) four years into the act: the conquests a good player makes, simulated by a change of
//!   controller of the settlements involved.
//!
//! Prints, per run: the outcome, the final reign score, the act scores and the ending reasons.

use std::path::Path;

use data_model::{ActCondition, FactionId, GameData};
use sim_campaign::{CampaignState, Order};

#[derive(Clone, Copy, PartialEq)]
enum Profile {
    Passive,
    Active,
    Decisive,
}

/// One decisive campaign per act, four years into the act: the settlements of the territorial objectives
/// of the act pass under the player's control.
fn conquer(state: &mut CampaignState, data: &GameData, player: &FactionId) {
    let Some(status) = state.act_status(data) else {
        return;
    };
    // Four years into the act: before any early end (24 seasons at the least).
    let elapsed = state.turn.saturating_sub(state.acts.act_start_turn);
    if elapsed != 16 {
        return;
    }
    for line in status.objectives.iter().filter(|o| !o.met) {
        match &line.objective.condition {
            ActCondition::ControlAll { provinces }
            | ActCondition::ControlCount { provinces, .. } => {
                for settlement in state.settlements.values_mut() {
                    if provinces.contains(&settlement.province) {
                        settlement.controller = player.clone();
                    }
                }
            }
            ActCondition::NoForeignControl { faction, provinces } => {
                for settlement in state.settlements.values_mut() {
                    if provinces.contains(&settlement.province) && &settlement.controller == faction
                    {
                        settlement.controller = player.clone();
                    }
                }
            }
            _ => {}
        }
    }
}

fn play(data: &GameData, faction: &str, seed: u64, profile: Profile, turns: u32) -> String {
    let id = FactionId::new(faction).expect("faction id");
    let mut state = CampaignState::new_1337(data, id.clone(), seed).expect("campaign");
    state.chronicle.disabled = false;
    let mut played = 0;
    while state.outcome.is_none() && played < turns {
        if profile != Profile::Passive {
            for offer in state.factions[&id].offers.clone() {
                let accept = sim_campaign::negotiation::evaluate_treaty(
                    &state,
                    data,
                    &offer.from,
                    &id,
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
            for order in ai::plan_turn(&state, data, &id) {
                let _ = state.submit_order(data, order);
            }
        }
        if profile == Profile::Decisive {
            conquer(&mut state, data, &id);
        }
        state.end_turn_with(data, ai::plan_turn);
        played += 1;
    }
    let reports: Vec<String> = state
        .acts
        .reports
        .iter()
        .map(|r| format!("{}={}({})", r.act_id, r.score, r.ended_by))
        .collect();
    let outcome = state
        .outcome
        .as_ref()
        .map_or("none".to_owned(), |o| format!("{:?}", o.kind));
    let detail: Vec<String> = state
        .acts
        .reports
        .iter()
        .map(|r| {
            let lines: Vec<String> = r
                .objectives
                .iter()
                .map(|o| format!("{}:{}{}", o.id, o.percent, if o.met { "*" } else { "" }))
                .collect();
            format!("    {} {}", r.act_id, lines.join(" "))
        })
        .collect();
    let head = format!(
        "{faction} seed {seed} {:<8} -> {outcome:<8} year {} turn {} reign {:>3} acts [{}]",
        match profile {
            Profile::Passive => "passive",
            Profile::Active => "active",
            Profile::Decisive => "decisive",
        },
        state.year,
        state.turn,
        state.reign_score(data, true),
        reports.join(" ")
    );
    if std::env::var("REIGN_DETAIL").is_ok() {
        format!("{head}\n{}", detail.join("\n"))
    } else {
        head
    }
}

fn main() {
    let mut seeds = vec![1u64];
    let mut faction = "fac_france".to_owned();
    let mut profiles = vec![Profile::Passive, Profile::Active, Profile::Decisive];
    let mut turns = 520;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--seeds" => {
                seeds = args
                    .next()
                    .expect("--seeds a,b")
                    .split(',')
                    .map(|s| s.parse().expect("seed"))
                    .collect();
            }
            "--faction" => faction = args.next().expect("--faction id"),
            "--turns" => turns = args.next().and_then(|s| s.parse().ok()).expect("--turns N"),
            "--profiles" => {
                profiles = args
                    .next()
                    .expect("--profiles a,b")
                    .split(',')
                    .map(|p| match p {
                        "passive" => Profile::Passive,
                        "decisive" => Profile::Decisive,
                        _ => Profile::Active,
                    })
                    .collect();
            }
            other => panic!("unknown argument {other}"),
        }
    }
    let root = std::env::var("CENT_ANS_DATA_DIR").map_or_else(
        |_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data"),
        std::path::PathBuf::from,
    );
    let (data, _) = GameData::load(&root).expect("game data");
    for seed in seeds {
        for &profile in &profiles {
            println!("{}", play(&data, &faction, seed, profile, turns));
        }
    }
}
