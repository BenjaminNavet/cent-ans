//! CB4: active abilities of the regiments (plan
//! `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md` § CB4,
//! `docs/research/cb4-capacites.md`): catalogue, cooldown, conditions, end
//! with its reason, each effect, replay determinism, AI.

mod common;

use common::*;
use data_model::{AbilityKind, BattleAbility, GameData};
use sim_battle::{BattleSim, SideId};

fn game_data() -> &'static GameData {
    static DATA: std::sync::OnceLock<GameData> = std::sync::OnceLock::new();
    DATA.get_or_init(data)
}

#[test]
fn the_catalogue_has_the_five_abilities_of_the_historian() {
    let data = game_data();
    let mut kinds: Vec<AbilityKind> = data.battle_abilities.values().map(|a| a.kind).collect();
    kinds.sort_by_key(|k| format!("{k:?}"));
    assert_eq!(
        kinds,
        vec![
            AbilityKind::AimedShot,
            AbilityKind::BannerRally,
            AbilityKind::CloseRanks,
            AbilityKind::Pavise,
            AbilityKind::PlantedPikes,
        ]
    );
    assert!(!data.battle_orders.contains_key("order_pavise"));
}

// ----- reference battles --------------------------------------------------

/// The ability catalogue, keeping the AI rule of the kinds in `ai_kinds`
/// only (every kind when `None`).
fn catalogue(ai_kinds: Option<&[AbilityKind]>) -> Vec<BattleAbility> {
    game_data()
        .battle_abilities
        .values()
        .cloned()
        .map(|mut a| {
            if ai_kinds.is_some_and(|kinds| !kinds.contains(&a.kind)) {
                a.ai = None;
            }
            a
        })
        .collect()
}

fn historical(id: &str, seed: u64, abilities: Vec<BattleAbility>) -> BattleSim {
    let data = game_data();
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../data/battle_maps")
        .join(format!("{id}.json"));
    let map =
        sim_battle::HistoricalMap::from_json(&std::fs::read_to_string(path).unwrap()).unwrap();
    let mut setup = map
        .battle_setup(
            &data.unit_types,
            data.battle_orders.values().cloned().collect(),
            Some(data.battle_standard_rules.clone()),
            None,
        )
        .unwrap();
    setup.abilities = abilities;
    map.start(setup, seed).unwrap()
}

fn mixed(seed: u64, abilities: Vec<BattleAbility>) -> BattleSim {
    let data = game_data();
    let mut battle = setup(
        units(
            data,
            &[
                "unit_knights",
                "unit_men_at_arms_foot",
                "unit_men_at_arms_foot",
                "unit_crossbowmen",
                "unit_crossbowmen",
                "unit_knights",
            ],
        ),
        units(
            data,
            &[
                "unit_men_at_arms_foot",
                "unit_longbowmen",
                "unit_longbowmen",
                "unit_knights",
            ],
        ),
        None,
    );
    battle.village = Some(false);
    battle.abilities = abilities;
    let mut sim = BattleSim::new(battle, seed).unwrap();
    sim.set_ai(SideId::Attacker, true);
    sim.set_ai(SideId::Defender, true);
    sim
}

/// English victories out of 20 on each historical map, French out of 16 in
/// the EQ7 mixed battle (seeds of `ep7_historical` and `eq7_cavalry`).
fn margins(ai_kinds: Option<&[AbilityKind]>) -> [usize; 4] {
    let mut out = [0; 4];
    for (k, id) in ["crecy", "azincourt", "poitiers"].iter().enumerate() {
        out[k] = (1..21)
            .filter(|&seed| {
                let mut sim = historical(id, seed, catalogue(ai_kinds));
                run(&mut sim, 2400.0);
                sim.winner() == Some(SideId::Defender)
            })
            .count();
    }
    out[3] = (0..16)
        .filter(|&seed| {
            let mut sim = mixed(seed, catalogue(ai_kinds));
            run_to_end(&mut sim);
            sim.winner() == Some(SideId::Attacker)
        })
        .count();
    out
}

/// Probe (ignored): pavises raised at Crécy by the legacy order and by the
/// ability.
#[test]
#[ignore = "probe"]
fn probe_crecy_pavises() {
    let legacy: data_model::BattleOrder = serde_json::from_str(
        &std::fs::read_to_string(std::env::var("CB4_LEGACY_PAVISE").unwrap()).unwrap(),
    )
    .unwrap();
    for seed in 1..4 {
        for with_order in [true, false] {
            let mut sim = if with_order {
                let mut sim = historical("crecy", seed, Vec::new());
                let _ = &mut sim;
                sim
            } else {
                historical("crecy", seed, catalogue(Some(&[AbilityKind::Pavise])))
            };
            if with_order {
                // Rebuild with the order in the catalogue.
                let data = game_data();
                let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../../data/battle_maps/crecy.json");
                let map =
                    sim_battle::HistoricalMap::from_json(&std::fs::read_to_string(path).unwrap())
                        .unwrap();
                let mut orders: Vec<data_model::BattleOrder> =
                    data.battle_orders.values().cloned().collect();
                orders.push(legacy.clone());
                let setup = map
                    .battle_setup(
                        &data.unit_types,
                        orders,
                        Some(data.battle_standard_rules.clone()),
                        None,
                    )
                    .unwrap();
                sim = map.start(setup, seed).unwrap();
            }
            run(&mut sim, 2400.0);
            let raised = sim
                .events()
                .iter()
                .filter(|e| e.text_fr.contains("pavois"))
                .count();
            println!(
                "seed {seed} order={with_order}: {raised} pavise lines, winner {:?} at {:.0}",
                sim.winner(),
                sim.elapsed()
            );
        }
    }
}

/// Probe (ignored): the margins with the AI of each ability alone.
/// `cargo test --release -p sim-battle --test cb4_abilities -- --ignored --nocapture probe_margins`
#[test]
#[ignore = "probe"]
fn probe_margins() {
    use AbilityKind as K;
    let variants: [(&str, Option<&[AbilityKind]>); 7] = [
        ("no AI", Some(&[])),
        ("pavise", Some(&[K::Pavise])),
        ("aimed shot", Some(&[K::AimedShot])),
        ("banner rally", Some(&[K::BannerRally])),
        ("close ranks", Some(&[K::CloseRanks])),
        ("planted pikes", Some(&[K::PlantedPikes])),
        ("all", None),
    ];
    for (name, kinds) in variants {
        let [c, a, p, e] = margins(kinds);
        println!("{name}: Crécy {c}/20, Azincourt {a}/20, Poitiers {p}/20, EQ7 {e}/16");
    }
}
