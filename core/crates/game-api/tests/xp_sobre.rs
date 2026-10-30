//! XP-SOBRE (lot D): a reminder learned is not repeated; a reign milestone at seasons 4, 8, 12;
//! no empty text (« : . »).

use std::collections::BTreeMap;

use game_api::{agenda, Agenda};

const HOUSES: [&str; 8] = [
    "fac_france",
    "fac_england",
    "fac_navarre",
    "fac_burgundy",
    "fac_brittany",
    "fac_castile",
    "fac_scotland",
    "fac_flanders",
];
/// The judge counts over 10 seasons; the milestone test goes one past season 12.
const SEASONS: u32 = 10;
const MILESTONE_SEASONS: u32 = 13;

/// Plays `SEASONS` seasons of `faction` with an idle player; `learn` records what the
/// Audience showed (as the client does at the end of each season).
fn play(faction: &str, learn: bool, seasons: u32) -> Vec<Agenda> {
    let data = data_model::test_support::game_data();
    let mut state = sim_campaign::test_support::start(data, faction, 1337);
    let player = state.player_faction().clone();
    let mut events = Vec::new();
    let mut out = Vec::new();
    for _ in 0..seasons {
        out.push(agenda(data, &state, &player, &events));
        if learn {
            game_api::agenda::note_season_end(data, &mut state, &player);
        }
        events = state.end_turn_with(data, sim_campaign::test_support::idle);
    }
    out
}

/// Repeats of a reminder: for each kind, the seasons it is shown beyond the first (the figures in
/// its sentence change every season, so the kind is what repeats).
fn repeats(agendas: &[Agenda]) -> BTreeMap<String, usize> {
    let mut shown: BTreeMap<String, usize> = BTreeMap::new();
    for a in agendas {
        let kinds: std::collections::BTreeSet<&String> = a
            .affairs
            .iter()
            .map(|x| &x.kind)
            .chain(a.watch.iter().map(|x| &x.kind))
            .collect();
        for kind in kinds {
            *shown.entry(kind.clone()).or_default() += 1;
        }
    }
    shown
        .into_iter()
        .map(|(kind, count)| (kind, count.saturating_sub(1)))
        .collect()
}

#[test]
fn reminders_are_learned_over_12_seasons() {
    let mut before: BTreeMap<String, usize> = BTreeMap::new();
    let mut after: BTreeMap<String, usize> = BTreeMap::new();
    let mut worst: BTreeMap<String, usize> = BTreeMap::new();
    for house in HOUSES {
        for (kind, n) in repeats(&play(house, false, SEASONS)) {
            *before.entry(kind).or_default() += n;
        }
        for (kind, n) in repeats(&play(house, true, SEASONS)) {
            *after.entry(kind.clone()).or_default() += n;
            let w = worst.entry(kind).or_default();
            *w = (*w).max(n);
        }
    }
    eprintln!("repeats before (8 houses summed): {before:?}");
    eprintln!("repeats after  (8 houses summed): {after:?}");
    eprintln!("repeats after  (worst house):     {worst:?}");
    for kind in ["free_slot", "advice", "idle_army", "ambition_choice"] {
        assert!(
            worst.get(kind).copied().unwrap_or(0) <= 3,
            "{kind} repeated {:?}",
            worst.get(kind)
        );
    }
}

#[test]
fn no_empty_sentence_in_the_audience() {
    for house in HOUSES {
        for a in play(house, true, SEASONS) {
            for text in a
                .affairs
                .iter()
                .map(|x| &x.text)
                .chain(a.watch.iter().map(|x| &x.text))
            {
                assert!(!text.contains(": ."), "{house}: {text}");
                assert!(!text.trim().is_empty(), "{house}: empty line");
            }
        }
    }
}

/// The reasons the treasurer gives are the projected budget lines: they sum to the net balance.
#[test]
fn treasury_reasons_sum_to_the_net_balance() {
    let data = data_model::test_support::game_data();
    for house in HOUSES {
        let mut state = sim_campaign::test_support::start(data, house, 1337);
        let player = state.player_faction().clone();
        for _ in 0..4 {
            let economy = state.faction_economy(data, &player).unwrap();
            let sum: i64 = economy.budget_lines(None).iter().map(|l| l.projected).sum();
            assert_eq!(sum, economy.net_income(), "{house}");
            state.end_turn_with(data, sim_campaign::test_support::idle);
        }
    }
}

#[test]
fn reign_milestone_at_seasons_4_8_12() {
    for house in HOUSES {
        for a in play(house, true, MILESTONE_SEASONS) {
            let milestone = a.affairs.iter().find(|x| x.kind == "reign_milestone");
            if matches!(a.turn, 4 | 8 | 12) {
                let m = milestone.unwrap_or_else(|| panic!("{house} turn {}", a.turn));
                eprintln!("MILESTONE {house} t{}: {}", a.turn, m.text);
                assert!(!m.text.contains('{'), "{}", m.text);
            } else {
                assert!(milestone.is_none(), "{house} turn {}", a.turn);
            }
        }
    }
}
