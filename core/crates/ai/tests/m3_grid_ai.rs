//! Lot M3: the AI on the navigation grid (spec
//! `docs/design/2026-09-24-mouvement-libre.md` § 4 and § 7) — attacks in
//! the bubble, avoidance of stronger armies, embarkations, determinism and
//! the time an AI faction takes to play its turn.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use data_model::{FactionId, GameData, SettlementId};
use sim_campaign::{ArmyId, ArmyPosition, CampaignState, EventKind, Order};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn set(id: &str) -> SettlementId {
    SettlementId::new(id).unwrap()
}

fn main_army(state: &CampaignState, faction: &str) -> ArmyId {
    let faction = fac(faction);
    state
        .armies
        .iter()
        .filter(|(_, a)| a.faction == faction)
        .max_by_key(|(id, a)| (a.units.len(), std::cmp::Reverse((*id).clone())))
        .map(|(id, _)| id.clone())
        .expect("faction has an army")
}

fn at_war(state: &mut CampaignState, a: &str, b: &str) {
    state
        .factions
        .get_mut(&fac(a))
        .unwrap()
        .at_war_with
        .insert(fac(b));
    state
        .factions
        .get_mut(&fac(b))
        .unwrap()
        .at_war_with
        .insert(fac(a));
}

/// A point `km` kilometres east of `settlement`.
fn east_of(data: &GameData, settlement: &str, km: f32) -> [f32; 2] {
    let p = data.settlement_point(&set(settlement)).unwrap();
    [p[0] + km * sim_campaign::march::px_per_km(data), p[1]]
}

#[test]
fn the_ai_attacks_a_weaker_army_in_its_bubble() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 1).unwrap();
    at_war(&mut state, "fac_england", "fac_france");
    let english = main_army(&state, "fac_england");
    let french = main_army(&state, "fac_france");
    let meaux = data.settlement_point(&set("set_meaux")).unwrap();
    state.armies.get_mut(&english).unwrap().position = ArmyPosition::field(meaux);
    let f = state.armies.get_mut(&french).unwrap();
    f.position = ArmyPosition::field(east_of(&data, "set_meaux", 20.0));
    f.units.truncate(1);
    let orders = ai::plan_turn(&state, &data, &fac("fac_england"));
    assert!(
        orders.iter().any(|o| matches!(
            o,
            Order::Attack { army, target_army } if army == &english && target_army == &french
        )),
        "the English army attacks: {orders:?}"
    );
    // Played at once: the battle is fought during the English turn.
    let mut events = Vec::new();
    state.play_ai_turn(&data, &fac("fac_england"), &ai::plan_turn, &mut events);
    let fought = state
        .pending_events
        .iter()
        .chain(events.iter())
        .any(|e| e.kind == EventKind::Battle && e.text_fr.contains("Vainqueur"));
    assert!(fought, "a battle was fought");
    assert!(state.pending_battles.is_empty(), "never left to the player");
}

#[test]
fn the_ai_does_not_attack_a_stronger_army() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 1).unwrap();
    at_war(&mut state, "fac_england", "fac_france");
    let english = main_army(&state, "fac_england");
    let french = main_army(&state, "fac_france");
    let meaux = data.settlement_point(&set("set_meaux")).unwrap();
    let e = state.armies.get_mut(&english).unwrap();
    e.position = ArmyPosition::field(meaux);
    e.units.truncate(1);
    state.armies.get_mut(&french).unwrap().position =
        ArmyPosition::field(east_of(&data, "set_meaux", 20.0));
    let orders = ai::plan_turn(&state, &data, &fac("fac_england"));
    assert!(
        !orders.iter().any(|o| matches!(o, Order::Attack { army, .. } if army == &english)),
        "a single company does not charge the French host: {orders:?}"
    );
}

#[test]
fn routes_avoid_the_zone_of_control_of_stronger_armies() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 1).unwrap();
    at_war(&mut state, "fac_england", "fac_france");
    let english = main_army(&state, "fac_england");
    // The English host camps just outside Meaux, a French town.
    state.armies.get_mut(&english).unwrap().position =
        ArmyPosition::field(east_of(&data, "set_meaux", 2.0));
    let power = state.army_power(&data, &english);
    let france = fac("fac_france");
    let planner = ai::grid::GridPlanner::new(&state, &data, &france);
    let start = set("set_paris");
    let through_meaux = |table: &ai::grid::Table| {
        table
            .values()
            .any(|r| r.previous.as_ref() == Some(&set("set_meaux")))
    };
    let weak = planner.table(&start, 5000, 1000, power / 4.0);
    assert!(
        !through_meaux(&weak),
        "a weaker army never marches through Meaux"
    );
    let strong = planner.table(&start, 5000, 1000, power * 4.0);
    assert!(
        through_meaux(&strong),
        "a stronger army goes through Meaux"
    );
}

#[test]
fn england_embarks_at_dover_for_the_continent() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 1).unwrap();
    at_war(&mut state, "fac_england", "fac_france");
    let english = main_army(&state, "fac_england");
    let dover = set("set_dover");
    state.armies.get_mut(&english).unwrap().position = ArmyPosition::Settlement(dover.clone());
    let full = state.army_grid_allowance(&data, &state.armies[&english]);
    state.armies.get_mut(&english).unwrap().movement_left = full;
    let orders = ai::plan_turn(&state, &data, &fac("fac_england"));
    assert!(
        orders.iter().any(|o| matches!(
            o,
            Order::Embark { army, to_port } if army == &english && to_port == &set("set_wissant")
        )),
        "the English host sails from Dover: {orders:?}"
    );
    let mut events = Vec::new();
    state.play_ai_turn(&data, &fac("fac_england"), &ai::plan_turn, &mut events);
    let army = &state.armies[&english];
    let wissant = data.settlement_point(&set("set_wissant")).unwrap();
    let landed = state.army_point(&data, army);
    let km = ((landed[0] - wissant[0]).hypot(landed[1] - wissant[1]))
        / sim_campaign::march::px_per_km(&data);
    assert!(km < 10.0, "the army landed at Wissant ({km} km away)");
}

/// State after `turns` AI-driven turns on `seed`.
fn play(data: &GameData, seed: u64, turns: u32) -> CampaignState {
    let france = fac("fac_france");
    let mut state = CampaignState::new_1337(data, france.clone(), seed).unwrap();
    state.interactive_battles = false;
    for _ in 0..turns {
        for order in ai::plan_turn(&state, data, &france) {
            let _ = state.submit_order(data, order);
        }
        state.end_turn_with(data, ai::plan_turn);
    }
    state
}

#[test]
fn ai_turns_are_deterministic_on_a_seed() {
    let data = data();
    let (a, b) = (play(&data, 3, 4), play(&data, 3, 4));
    assert!(a == b, "two games on the same seed differ");
    assert_eq!(a.events, b.events);
}

/// Best of three plays of `faction`'s turn on copies of `state`.
fn best_turn_time(state: &CampaignState, data: &GameData, faction: &FactionId) -> Duration {
    (0..3)
        .map(|_| {
            let mut copy = state.clone();
            let mut events = Vec::new();
            let started = Instant::now();
            copy.play_ai_turn(data, faction, &ai::plan_turn, &mut events);
            started.elapsed()
        })
        .min()
        .unwrap()
}

/// Spec § 4: under 50 ms per faction and turn in release (the debug build
/// is only held to a looser bound). Timed on the 1337 start and after a few
/// turns of war.
#[test]
fn an_ai_faction_plays_its_turn_quickly() {
    let data = data();
    let _ = data.navgrid().component(0, 0);
    let limit = if cfg!(debug_assertions) {
        Duration::from_millis(1500)
    } else {
        Duration::from_millis(50)
    };
    let france = fac("fac_france");
    let mut state = CampaignState::new_1337(&data, france.clone(), 1).unwrap();
    state.interactive_battles = false;
    for turn in 0..3 {
        let factions: Vec<FactionId> = state
            .factions
            .iter()
            .filter(|(id, f)| f.alive && **id != france)
            .map(|(id, _)| id.clone())
            .collect();
        let mut events = Vec::new();
        for faction in factions {
            let time = best_turn_time(&state, &data, &faction);
            assert!(
                time < limit,
                "{faction} took {time:?} on turn {turn} (limit {limit:?})"
            );
            state.play_ai_turn(&data, &faction, &ai::plan_turn, &mut events);
        }
        state.resolve_end_of_turn(&data, &mut events);
    }
}
