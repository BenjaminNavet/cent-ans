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
    // FE5: the feudal AI decides, whatever the order of the tests.
    ai::feudal::install();
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
        !orders
            .iter()
            .any(|o| matches!(o, Order::Attack { army, .. } if army == &english)),
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
    assert!(through_meaux(&strong), "a stronger army goes through Meaux");
}

#[test]
fn england_embarks_at_dover_for_the_continent() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 1).unwrap();
    // DC3: the planning horizon (10 steps of 70 km) reaches Edinburgh, which the Scots
    // threaten in 1337: only the French war is kept, so that the host looks to France.
    for faction in state.factions.values_mut() {
        faction.at_war_with.clear();
    }
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

/// What one 50-turn game measured (the C7a indicators).
struct Game {
    treasury: [i64; 2],
    provinces_delta: [i64; 2],
    bankruptcies: [u32; 2],
    siege_turns: u32,
    battles: u32,
    english_landings: u32,
    stuck_turns: u32,
    majors_alive: bool,
}

fn fifty_turns(data: &GameData, seed: u64) -> Game {
    const TURNS: u32 = 50;
    let majors = [fac("fac_france"), fac("fac_england")];
    let mut state = CampaignState::new_1337(data, majors[0].clone(), seed).unwrap();
    state.interactive_battles = false;
    let provinces_start = majors
        .clone()
        .map(|f| state.controlled_provinces(&f).len() as i64);
    let mut game = Game {
        treasury: [0; 2],
        provinces_delta: [0; 2],
        bankruptcies: [0; 2],
        siege_turns: 0,
        battles: 0,
        english_landings: 0,
        stuck_turns: 0,
        majors_alive: true,
    };
    // Armies standing still outside friendly places, not besieging.
    let mut still: std::collections::BTreeMap<ArmyId, ([f32; 2], u32)> = Default::default();
    for _ in 0..TURNS {
        for order in ai::plan_turn(&state, data, &majors[0]) {
            let _ = state.submit_order(data, order);
        }
        for event in state.end_turn_with(data, ai::plan_turn) {
            let faction = majors
                .iter()
                .position(|m| event.faction.as_ref() == Some(m));
            match event.kind {
                EventKind::Bankruptcy => {
                    if let Some(i) = faction {
                        game.bankruptcies[i] += 1;
                    }
                }
                EventKind::Battle if event.text_fr.contains("Vainqueur") => game.battles += 1,
                EventKind::Attrition
                    if event.text_fr.starts_with("Débarquement") && faction == Some(1) =>
                {
                    game.english_landings += 1
                }
                _ => {}
            }
        }
        game.siege_turns += state
            .settlements
            .values()
            .filter(|s| s.siege.is_some())
            .count() as u32;
        let mut now = std::collections::BTreeMap::new();
        for (id, army) in &state.armies {
            let point = state.army_point(data, army);
            let besieging = army
                .settlement()
                .and_then(|s| state.settlements.get(s))
                .is_some_and(|s| s.siege.is_some());
            let home = army
                .settlement()
                .is_some_and(|s| state.is_friendly_settlement(&army.faction, s));
            if besieging || home {
                continue;
            }
            let turns = match still.get(id) {
                Some((p, n)) if *p == point => n + 1,
                _ => 0,
            };
            if turns >= 4 {
                game.stuck_turns += 1;
            }
            now.insert(id.clone(), (point, turns));
        }
        still = now;
    }
    for (i, major) in majors.iter().enumerate() {
        let f = &state.factions[major];
        game.treasury[i] = f.treasury;
        game.provinces_delta[i] =
            state.controlled_provinces(major).len() as i64 - provinces_start[i];
        game.majors_alive &= f.alive;
    }
    game
}

/// Spec § 7: 50 turns of AI against AI on 8 seeds stay in the band measured
/// after C7a (`docs/wip/c7a-settlements-balance.md`, `docs/wip/m3-tour-ia.md`).
/// About a minute in release; run with
/// `cargo test --release -p ai --test m3_grid_ai -- --ignored`.
#[test]
#[ignore = "50 turns x 8 seeds: run in release with --ignored"]
fn fifty_turns_on_eight_seeds_stay_in_the_c7a_band() {
    let data = data();
    let games: Vec<Game> = (1..=8).map(|seed| fifty_turns(&data, seed)).collect();
    let n = games.len() as f64;
    let mean = |f: &dyn Fn(&Game) -> f64| games.iter().map(f).sum::<f64>() / n;
    // LR-04: every seed is printed, then every failure listed, so that one
    // run (an hour on a loaded machine) shows the whole picture.
    let mut failures = Vec::new();
    for (i, g) in games.iter().enumerate() {
        let seed = i + 1;
        println!(
            "seed {seed}: treasury {:?}, Δ provinces {:?}, bankruptcies {:?}, battles {}, sieges {}, landings {}",
            g.treasury, g.provinces_delta, g.bankruptcies, g.battles, g.siege_turns, g.english_landings
        );
        if !g.majors_alive {
            failures.push(format!("seed {seed}: France and England survive"));
        }
        if g.bankruptcies != [0, 0] {
            failures.push(format!("seed {seed}: bankruptcies {:?}", g.bankruptcies));
        }
        for (m, delta) in g.provinces_delta.iter().enumerate() {
            if delta.abs() > 5 {
                failures.push(format!(
                    "seed {seed}: major {m} Δ provinces {delta} (no collapse, no blitz)"
                ));
            }
        }
        if g.battles == 0 {
            failures.push(format!("seed {seed}: armies meet in the field"));
        }
    }
    let france = mean(&|g| g.treasury[0] as f64);
    let england = mean(&|g| g.treasury[1] as f64);
    let sieges = mean(&|g| f64::from(g.siege_turns) / 50.0);
    let stuck = mean(&|g| f64::from(g.stuck_turns) / 50.0);
    let landings = mean(&|g| f64::from(g.english_landings));
    // RS-B: the band is printed before it is checked, so that a failure shows every figure.
    println!(
        "France {france:.0}, England {england:.0}, sieges/turn {sieges:.3}, stuck/turn {stuck:.2}, English landings {landings:.1}, battles {:.1}",
        mean(&|g| f64::from(g.battles))
    );
    assert!(failures.is_empty(), "{failures:#?}");
    // RS-M (ADR 0113): 40 000 until TW2 (royal ransoms after battles) and FE
    // (the Empire's host at war with France from turn 8); 24 336 on main of
    // 2026-09-28, standard error of the 8-seed mean about 3 400. FE8 (ADR
    // 0114, effective host): 32 603, floor raised to 20 000 (the mean less
    // about three standard errors, rounded down to 5 000).
    assert!(
        (20_000.0..=160_000.0).contains(&france),
        "France's treasury {france}"
    );
    assert!(
        (5_000.0..=40_000.0).contains(&england),
        "England's treasury {england}"
    );
    // RS-B (ADR 0100): 1.5 until the AI weighed its places' buildings by
    // `province_effect_percent`; sieges went 1.65 -> 1.49 per turn on these
    // seeds, 1.40 after merging main of 2026-09-28 (margin for chaos: 1.3).
    assert!((1.3..=8.0).contains(&sieges), "sieges per turn {sieges}");
    assert!(stuck <= 1.0, "stuck armies per turn {stuck}");
    assert!(
        landings >= 1.0,
        "England lands on the continent ({landings} per game)"
    );
}

/// Review fix: a friendly army near the enemy counts once. Before, the
/// caller passed the power of every army of the faction at the same anchor
/// and the loop added the nearby ones again, overstating the odds.
#[test]
fn nearby_friendly_armies_are_counted_once() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 1).unwrap();
    at_war(&mut state, "fac_england", "fac_france");
    let english = main_army(&state, "fac_england");
    let french = main_army(&state, "fac_france");
    let meaux = data.settlement_point(&set("set_meaux")).unwrap();
    state.armies.get_mut(&english).unwrap().position = ArmyPosition::field(meaux);
    // A second English army next to the French one, within engagement range.
    let mut second = state.armies[&english].clone();
    second.position = ArmyPosition::field(east_of(&data, "set_meaux", 19.0));
    let second_id = ArmyId::from_index(9999);
    state.armies.insert(second_id.clone(), second);
    let f = state.armies.get_mut(&french).unwrap();
    f.position = ArmyPosition::field(east_of(&data, "set_meaux", 20.0));
    f.units.truncate(1);
    f.units[0].strength = 100;
    let mut ours = state.army_power(&data, &english) + state.army_power(&data, &second_id);
    // RC (ADR 0141): east of Meaux the Marne lies between the armies; the AI
    // weighs the crossing it would force, as the resolver does.
    if let Some(site) = sim_campaign::river_crossing::crossing_between(
        &data,
        meaux,
        east_of(&data, "set_meaux", 20.0),
    ) {
        ours *= site.effect().attacker_factor(&data.river_crossing_rules);
    }
    let per_man = state.army_power(&data, &french) / 100.0;
    let ratio = data.ai_grid.attack_ratio;
    let england = fac("fac_england");
    // Enemy just too strong for the true odds (but not for doubled ones).
    let too_strong = ((ours * 1.1 / ratio) / per_man).ceil() as u32;
    state.armies.get_mut(&french).unwrap().units[0].strength = too_strong;
    let planner = ai::grid::GridPlanner::new(&state, &data, &england);
    assert_eq!(planner.attack_order(&english), None, "odds overstated");
    // Enemy weak enough for the true odds.
    let weak = ((ours * 0.9 / ratio) / per_man).floor() as u32;
    state.armies.get_mut(&french).unwrap().units[0].strength = weak;
    let planner = ai::grid::GridPlanner::new(&state, &data, &england);
    assert!(
        matches!(planner.attack_order(&english), Some(Order::Attack { .. })),
        "the joint host attacks a weaker army"
    );
}

/// NT9: two armies of a host (N6 cap) do not both attack the same enemy in
/// one turn; the second leaves it to the first.
#[test]
fn one_attack_per_enemy_army_and_turn() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 1).unwrap();
    at_war(&mut state, "fac_england", "fac_france");
    let english = main_army(&state, "fac_england");
    let french = main_army(&state, "fac_france");
    state.armies.get_mut(&english).unwrap().position =
        ArmyPosition::field(east_of(&data, "set_meaux", 60.0));
    let mut second = state.armies[&english].clone();
    second.position = ArmyPosition::field(east_of(&data, "set_meaux", 62.0));
    let second_id = ArmyId::from_index(9999);
    state.armies.insert(second_id.clone(), second);
    let f = state.armies.get_mut(&french).unwrap();
    f.position = ArmyPosition::field(east_of(&data, "set_meaux", 40.0));
    f.units.truncate(1);
    let england = fac("fac_england");
    let planner = ai::grid::GridPlanner::new(&state, &data, &england);
    let target = |order: Option<Order>| match order {
        Some(Order::Attack { target_army, .. }) => Some(target_army),
        _ => None,
    };
    assert_eq!(
        target(planner.attack_order(&second_id)),
        Some(french.clone())
    );
    let mut engaged = Vec::new();
    assert_eq!(
        target(planner.attack_order_sparing(&english, &mut engaged)),
        Some(french.clone())
    );
    assert_ne!(
        target(planner.attack_order_sparing(&second_id, &mut engaged)),
        Some(french),
        "the second army leaves the engaged enemy alone"
    );
}
