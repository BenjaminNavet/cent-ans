//! Leader's orders (F10b, spec `docs/design/battle-orders.md`) on the real
//! catalogue of `data/battle_orders/` and the real unit types.

mod common;

use common::*;
use data_model::{BattleOrderKind, GameData, UnitCategory};
use sim_battle::{BattleSetup, BattleSim, Command, CommandError, GeneralSetup, SideId, UnitState};

fn general(unit_index: usize, command: u8) -> GeneralSetup {
    GeneralSetup {
        character: "chr_test".to_owned(),
        name: "Le connétable".to_owned(),
        command,
        unit_index,
        morale_bonus: 0.0,
        charge_percent: 0.0,
        ranged_percent: 0.0,
        defense_percent: 0.0,
        sovereign: false,
    }
}

/// France (attacker, general on regiment 0) against England, with the order
/// catalogue.
fn order_setup(data: &GameData, attacker: &[&str], defender: &[&str]) -> BattleSetup {
    let mut s = setup(units(data, attacker), units(data, defender), None);
    s.defender.faction = "fac_england".to_owned();
    s.attacker.general = Some(general(0, 6));
    s.defender.general = Some(general(0, 6));
    s.orders = data.battle_orders.values().cloned().collect();
    s
}

fn lab_sim(setup: BattleSetup, seed: u64) -> BattleSim {
    let mut sim = BattleSim::new(setup, seed).unwrap();
    lab(&mut sim);
    sim
}

fn order(sim: &mut BattleSim, side: SideId, id: &str, units: Vec<u32>) -> Result<(), CommandError> {
    sim.apply_command(
        Command::LeaderOrder {
            side: None,
            order: id.to_owned(),
            units,
        },
        Some(side),
    )
}

fn reason(result: Result<(), CommandError>) -> String {
    match result {
        Err(CommandError::OrderUnavailable { reason, .. }) => reason,
        other => panic!("expected an unavailable order, got {other:?}"),
    }
}

#[test]
fn catalogue_loads_the_five_orders() {
    let data = data();
    let kinds: Vec<BattleOrderKind> = data.battle_orders.values().map(|o| o.kind).collect();
    assert_eq!(data.battle_orders.len(), 5);
    for kind in [
        BattleOrderKind::WarCry,
        BattleOrderKind::NoQuarter,
        BattleOrderKind::Dismount,
        BattleOrderKind::Pavise,
        BattleOrderKind::Rally,
    ] {
        assert!(kinds.contains(&kind), "{kind:?} missing");
    }
    let cry = &data.battle_orders["order_war_cry"];
    assert_eq!(cry.label_for("fac_france"), "Montjoie ! Saint-Denis !");
    assert_eq!(cry.label_for("fac_england"), "Saint George !");
    assert_eq!(cry.label_for("fac_burgundy"), "Montjoie Saint-Andrieu !");
    assert_eq!(cry.label_for("fac_scotland"), "Saint Andrew !");
    assert_eq!(cry.label_for("fac_castile"), "¡Santiago!");
    assert_eq!(cry.label_for("fac_navarre"), "Cri de guerre");
    let banner = &data.battle_orders["order_no_quarter"];
    assert_eq!(banner.label_for("fac_france"), "Déployer l'oriflamme");
    assert_eq!(
        banner.label_for("fac_england"),
        "Lever la bannière au dragon"
    );
    assert_eq!(
        banner.label_for("fac_navarre"),
        "Déployer la bannière rouge"
    );
}

#[test]
fn leader_order_command_serialises() {
    let command = Command::LeaderOrder {
        side: Some(SideId::Attacker),
        order: "order_war_cry".to_owned(),
        units: vec![1, 2],
    };
    let json = serde_json::to_value(&command).unwrap();
    assert_eq!(json["type"], "leader_order");
    assert_eq!(json["side"], "attacker");
    assert_eq!(json["order"], "order_war_cry");
    let back: Command = serde_json::from_value(json).unwrap();
    assert_eq!(back, command);
    // `side` and `units` are optional.
    let short: Command =
        serde_json::from_str(r#"{"type": "leader_order", "order": "order_rally"}"#).unwrap();
    assert_eq!(
        short,
        Command::LeaderOrder {
            side: None,
            order: "order_rally".to_owned(),
            units: Vec::new(),
        }
    );
}

#[test]
fn war_cry_lifts_morale_around_the_general_then_fades_and_recharges() {
    let data = data();
    let s = order_setup(
        &data,
        &[
            "unit_men_at_arms_foot",
            "unit_urban_militia",
            "unit_urban_militia",
        ],
        &["unit_urban_militia"],
    );
    let mut sim = lab_sim(s, 3);
    place(&mut sim, 0, 600.0, 200.0, 0.0);
    place(&mut sim, 1, 650.0, 200.0, 0.0); // within 150 m
    place(&mut sim, 2, 1000.0, 200.0, 0.0); // far away
    place(&mut sim, 3, 600.0, 700.0, std::f64::consts::PI);
    for unit in sim.units_mut() {
        unit.morale = 50.0;
    }
    let cap_near = sim.units()[1].morale_cap;
    order(&mut sim, SideId::Attacker, "order_war_cry", vec![]).unwrap();
    assert!((sim.units()[1].morale - 62.0).abs() < 1e-9);
    assert!((sim.units()[0].morale - 62.0).abs() < 1e-9);
    assert!((sim.units()[2].morale - 50.0).abs() < 1e-9, "out of reach");
    assert!(
        (sim.units()[3].morale - 50.0).abs() < 1e-9,
        "enemy untouched"
    );
    assert!(has_event(&sim, "« Montjoie ! Saint-Denis ! »"));
    // Cooldown.
    let again = reason(order(&mut sim, SideId::Attacker, "order_war_cry", vec![]));
    assert!(again.contains("recharge"), "{again}");
    // The ceiling returns to normal once the cry has faded (30 s).
    assert!(sim.units()[1].morale_cap > cap_near);
    run(&mut sim, 31.0);
    assert!((sim.units()[1].morale_cap - cap_near).abs() < 1e-9);
    // Ready again after 120 s.
    run(&mut sim, 90.0);
    order(&mut sim, SideId::Attacker, "order_war_cry", vec![]).unwrap();
}

#[test]
fn orders_need_a_commanding_general_and_a_known_id() {
    let data = data();
    let mut s = order_setup(&data, &["unit_men_at_arms_foot"], &["unit_urban_militia"]);
    s.attacker.general = None;
    let mut sim = lab_sim(s, 3);
    let why = reason(order(&mut sim, SideId::Attacker, "order_war_cry", vec![]));
    assert_eq!(why, "aucun chef à la tête de l'armée");
    assert_eq!(
        order(&mut sim, SideId::Attacker, "order_fireball", vec![]),
        Err(CommandError::UnknownOrder("order_fireball".to_owned()))
    );
    // A leader's order for the other side is refused.
    let wrong = sim.apply_command(
        Command::LeaderOrder {
            side: Some(SideId::Defender),
            order: "order_war_cry".to_owned(),
            units: vec![],
        },
        Some(SideId::Attacker),
    );
    assert_eq!(wrong, Err(CommandError::WrongSide));
    assert_eq!(
        CommandError::WrongSide.to_string(),
        "cet ordre ne concerne pas votre armée"
    );
    // The routed general no longer commands.
    let s = order_setup(&data, &["unit_men_at_arms_foot"], &["unit_urban_militia"]);
    let mut sim = lab_sim(s, 3);
    sim.units_mut()[0].state = UnitState::Routing;
    let why = reason(order(&mut sim, SideId::Attacker, "order_war_cry", vec![]));
    assert_eq!(why, "le chef ne commande plus");
}

#[test]
fn no_quarter_emboldens_the_whole_army_once_and_is_reported() {
    let data = data();
    let s = order_setup(
        &data,
        &["unit_men_at_arms_foot", "unit_urban_militia"],
        &["unit_urban_militia"],
    );
    let mut sim = lab_sim(s, 4);
    place(&mut sim, 1, 1100.0, 50.0, 0.0); // far from the general: still reached
    for unit in sim.units_mut() {
        unit.morale = 40.0;
    }
    order(&mut sim, SideId::Attacker, "order_no_quarter", vec![]).unwrap();
    assert!((sim.units()[0].morale - 60.0).abs() < 1e-9);
    assert!((sim.units()[1].morale - 60.0).abs() < 1e-9);
    assert!((sim.units()[2].morale - 40.0).abs() < 1e-9);
    assert!(has_event(&sim, "Déployer l'oriflamme"));
    assert!(sim.no_quarter(SideId::Attacker));
    let again = reason(order(
        &mut sim,
        SideId::Attacker,
        "order_no_quarter",
        vec![],
    ));
    assert_eq!(again, "déjà donné dans cette bataille");
    // The flag reaches the campaign outcome.
    sim.set_end_conditions(true);
    sim.units_mut()[2].state = UnitState::Routing;
    sim.units_mut()[2].morale = 5.0;
    sim.step();
    let outcome = sim.outcome().expect("finished");
    assert!(outcome.attacker.no_quarter);
    assert!(!outcome.defender.no_quarter);
    let json = serde_json::to_value(&outcome).unwrap();
    assert_eq!(json["attacker"]["no_quarter"], true);
}

#[test]
fn dismount_turns_knights_into_heavy_foot_for_good() {
    let data = data();
    let s = order_setup(
        &data,
        &["unit_men_at_arms_foot", "unit_knights", "unit_knights"],
        &["unit_urban_militia"],
    );
    let mut sim = lab_sim(s, 5);
    let before = sim.units()[1].clone();
    // Only the selected regiment dismounts; foot soldiers in the selection
    // are ignored.
    order(&mut sim, SideId::Attacker, "order_dismount", vec![0, 1]).unwrap();
    let after = &sim.units()[1];
    assert!(!after.mounted && after.dismounted);
    assert_eq!(after.category, UnitCategory::Infantry);
    assert!(after.stats.speed <= 35 && after.stats.speed < before.stats.speed);
    assert_eq!(after.stats.armor, before.stats.armor + 8);
    assert_eq!(after.stats.charge, None);
    assert!(sim.units()[2].mounted, "not selected");
    assert!(has_event(
        &sim,
        "Chevaliers de France mettent pied à terre."
    ));
    // Foot soldiers alone cannot obey.
    run(&mut sim, 6.0);
    let why = reason(order(&mut sim, SideId::Attacker, "order_dismount", vec![0]));
    assert_eq!(why, "aucun des régiments désignés ne peut l'exécuter");
    // No selection: every remaining knight dismounts, then none is left.
    order(&mut sim, SideId::Attacker, "order_dismount", vec![]).unwrap();
    assert!(!sim.units()[2].mounted);
    run(&mut sim, 6.0);
    let why = reason(order(&mut sim, SideId::Attacker, "order_dismount", vec![]));
    assert_eq!(why, "aucune cavalerie lourde à démonter");
    // Irreversible: formations of foot only.
    assert!(sim
        .apply_command(
            Command::Formation {
                units: vec![1],
                kind: sim_battle::Formation::Wedge,
            },
            None,
        )
        .is_err());
}

#[test]
fn siege_assault_dismount_uses_the_order_wording() {
    let data = data();
    let mut s = order_setup(
        &data,
        &["unit_men_at_arms_foot", "unit_knights"],
        &["unit_urban_militia"],
    );
    s.siege = Some(sim_battle::SiegeSetup {
        fortification: 1,
        breach: 0,
    });
    let sim = BattleSim::new(s, 2).unwrap();
    assert!(has_event(
        &sim,
        "Les chevaliers de France mettent pied à terre pour l'assaut."
    ));
    let knights = &sim.units()[1];
    assert!(knights.dismounted && !knights.mounted);
    assert!(knights.stats.speed <= 35);
}

/// Casualties of Genoese crossbowmen under a longbow volley, pavises raised
/// or not.
fn crossbow_losses(data: &GameData, raise: bool) -> f64 {
    let s = order_setup(
        data,
        &["unit_men_at_arms_foot", "unit_genoese_crossbowmen"],
        &["unit_men_at_arms_foot", "unit_longbowmen"],
    );
    let mut sim = lab_sim(s, 9);
    place(&mut sim, 0, 300.0, 100.0, 0.0);
    place(&mut sim, 1, 600.0, 300.0, 0.0);
    place(&mut sim, 2, 900.0, 700.0, std::f64::consts::PI);
    place(&mut sim, 3, 600.0, 480.0, std::f64::consts::PI);
    hold_fire(&mut sim, SideId::Attacker);
    if raise {
        order(&mut sim, SideId::Attacker, "order_pavise", vec![]).unwrap();
        assert!(sim.units()[1].pavise.is_some());
    }
    let before = sim.units()[1].hp;
    run(&mut sim, 40.0);
    before - sim.units()[1].hp
}

#[test]
fn pavises_cut_missile_casualties_until_the_next_move() {
    let data = data();
    let open = crossbow_losses(&data, false);
    let covered = crossbow_losses(&data, true);
    assert!(open > 0.0);
    assert!(
        covered < open * 0.7,
        "pavises should shelter the crossbowmen: {covered} vs {open}"
    );
    // Raised pavises: the regiment stands still; a move order lifts them.
    let s = order_setup(
        &data,
        &["unit_men_at_arms_foot", "unit_genoese_crossbowmen"],
        &["unit_urban_militia"],
    );
    let mut sim = lab_sim(s, 9);
    order(&mut sim, SideId::Attacker, "order_pavise", vec![1]).unwrap();
    assert!(has_event(&sim, "dressent leurs pavois"));
    let why = reason(order(&mut sim, SideId::Attacker, "order_pavise", vec![1]));
    assert!(why.contains("recharge"), "{why}");
    let (x, z) = (sim.units()[1].x, sim.units()[1].z);
    sim.apply_command(
        Command::Move {
            units: vec![1],
            x: x + 50.0,
            z,
            run: false,
            facing: None,
            queue: false,
        },
        None,
    )
    .unwrap();
    assert_eq!(sim.units()[1].pavise, None);
    // Only crossbowmen with pavises can obey.
    run(&mut sim, 11.0);
    let why = reason(order(&mut sim, SideId::Attacker, "order_pavise", vec![0]));
    assert_eq!(why, "aucun des régiments désignés ne peut l'exécuter");
}

#[test]
fn rally_depends_on_the_chance_and_is_logged() {
    let data = data();
    let run_rally = |chance: f64| {
        let mut s = order_setup(
            &data,
            &[
                "unit_men_at_arms_foot",
                "unit_urban_militia",
                "unit_urban_militia",
            ],
            &["unit_urban_militia"],
        );
        for o in &mut s.orders {
            if o.kind == BattleOrderKind::Rally {
                o.effects.rally_chance = chance;
                o.effects.rally_chance_per_command = 0.0;
            }
        }
        let mut sim = lab_sim(s, 11);
        place(&mut sim, 0, 600.0, 200.0, 0.0);
        place(&mut sim, 1, 640.0, 180.0, 0.0);
        place(&mut sim, 2, 560.0, 180.0, 0.0);
        for id in [1, 2] {
            let unit = &mut sim.units_mut()[id];
            unit.state = UnitState::Routing;
            unit.morale = 10.0;
        }
        order(&mut sim, SideId::Attacker, "order_rally", vec![]).unwrap();
        sim
    };
    let sure = run_rally(1.0);
    for id in [1, 2] {
        assert_eq!(sure.units()[id].state, UnitState::Rallied);
        assert!(sure.units()[id].morale >= 45.0);
    }
    assert!(has_event(&sure, "Le connétable rallie les Milice urbaine"));
    let hopeless = run_rally(0.0);
    for id in [1, 2] {
        assert_eq!(hopeless.units()[id].state, UnitState::Routing);
    }
    assert!(has_event(
        &hopeless,
        "n'entendent pas l'appel de Le connétable"
    ));
    // Nobody fleeing near the general: refused.
    let s = order_setup(&data, &["unit_men_at_arms_foot"], &["unit_urban_militia"]);
    let mut sim = lab_sim(s, 11);
    let why = reason(order(&mut sim, SideId::Attacker, "order_rally", vec![]));
    assert_eq!(why, "aucun régiment en déroute près du chef");
}

#[test]
fn rally_chance_grows_with_command() {
    let data = data();
    let rallied = |command: u8| -> usize {
        let mut count = 0;
        for seed in 0..40 {
            let mut s = order_setup(
                &data,
                &["unit_men_at_arms_foot", "unit_urban_militia"],
                &["unit_urban_militia"],
            );
            s.attacker.general = Some(general(0, command));
            let mut sim = lab_sim(s, seed);
            let (gx, gz) = (sim.units()[0].x, sim.units()[0].z);
            place(&mut sim, 1, gx + 30.0, gz, 0.0);
            sim.units_mut()[1].state = UnitState::Routing;
            order(&mut sim, SideId::Attacker, "order_rally", vec![]).unwrap();
            if sim.units()[1].state == UnitState::Rallied {
                count += 1;
            }
        }
        count
    };
    let weak = rallied(0);
    let strong = rallied(10);
    assert!(strong > weak, "command 10: {strong}, command 0: {weak}");
}

/// Snapshot of a battle for determinism checks.
fn fingerprint(sim: &BattleSim) -> Vec<(u64, u64, u64, String)> {
    sim.units()
        .iter()
        .map(|u| {
            (
                u.x.to_bits(),
                u.hp.to_bits(),
                u.morale.to_bits(),
                format!("{:?}", u.state),
            )
        })
        .collect()
}

#[test]
fn orders_are_deterministic() {
    let data = data();
    let play = || {
        let s = order_setup(
            &data,
            &[
                "unit_men_at_arms_foot",
                "unit_knights",
                "unit_genoese_crossbowmen",
                "unit_urban_militia",
            ],
            &["unit_men_at_arms_foot", "unit_longbowmen", "unit_knights"],
        );
        let mut sim = BattleSim::new(s, 21).unwrap();
        sim.set_ai(SideId::Attacker, false);
        for tick in 0..6000 {
            if tick == 100 {
                let _ = order(&mut sim, SideId::Attacker, "order_dismount", vec![1]);
                let _ = order(&mut sim, SideId::Attacker, "order_pavise", vec![]);
            }
            if tick == 900 {
                let _ = order(&mut sim, SideId::Attacker, "order_war_cry", vec![]);
                let _ = order(&mut sim, SideId::Attacker, "order_rally", vec![]);
            }
            if sim.is_finished() {
                break;
            }
            sim.step();
        }
        (fingerprint(&sim), sim.events().to_vec())
    };
    assert_eq!(play(), play());
}

#[test]
fn leader_orders_view_lists_availability() {
    let data = data();
    let s = order_setup(
        &data,
        &["unit_men_at_arms_foot", "unit_knights"],
        &["unit_urban_militia"],
    );
    let mut sim = lab_sim(s, 1);
    let views = sim.leader_orders(SideId::Attacker);
    let ids: Vec<&str> = views.iter().map(|v| v.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "order_war_cry",
            "order_rally",
            "order_dismount",
            "order_pavise",
            "order_no_quarter"
        ]
    );
    let cry = &views[0];
    assert_eq!(cry.label, "Montjoie ! Saint-Denis !");
    assert!(cry.available && cry.reason.is_empty());
    assert!(!views[1].available, "nobody routs");
    assert!(views[2].available, "knights can dismount");
    assert!(!views[3].available, "no crossbowmen");
    order(&mut sim, SideId::Attacker, "order_war_cry", vec![]).unwrap();
    let cry = &sim.leader_orders(SideId::Attacker)[0];
    assert!(!cry.available);
    assert!((cry.cooldown_remaining - 120.0).abs() < 1e-9);
    let english = sim.leader_orders(SideId::Defender);
    assert_eq!(english[0].label, "Saint George !");
    assert_eq!(english[4].label, "Lever la bannière au dragon");
}

// ----- battle AI ---------------------------------------------------------------

fn ai_lab(setup: BattleSetup, side: SideId, seed: u64) -> BattleSim {
    let mut sim = lab_sim(setup, seed);
    sim.set_ai(side, true);
    sim
}

#[test]
fn ai_cries_on_engagement() {
    let data = data();
    let s = order_setup(
        &data,
        &["unit_men_at_arms_foot", "unit_urban_militia"],
        &["unit_men_at_arms_foot"],
    );
    let mut sim = ai_lab(s, SideId::Attacker, 2);
    place(&mut sim, 0, 600.0, 300.0, 0.0);
    place(&mut sim, 1, 660.0, 300.0, 0.0);
    place(&mut sim, 2, 600.0, 800.0, std::f64::consts::PI);
    run(&mut sim, 4.0);
    assert!(
        !has_event(&sim, "Montjoie"),
        "no cry while the enemy is far"
    );
    place(&mut sim, 2, 600.0, 370.0, std::f64::consts::PI);
    run(&mut sim, 4.0);
    assert!(has_event(&sim, "« Montjoie ! Saint-Denis ! »"));
}

#[test]
fn ai_rallies_fleeing_regiments() {
    let data = data();
    let s = order_setup(
        &data,
        &["unit_men_at_arms_foot", "unit_urban_militia"],
        &["unit_men_at_arms_foot"],
    );
    let mut sim = ai_lab(s, SideId::Attacker, 2);
    place(&mut sim, 0, 600.0, 300.0, 0.0);
    place(&mut sim, 1, 640.0, 280.0, 0.0);
    place(&mut sim, 2, 600.0, 900.0, std::f64::consts::PI);
    let unit = &mut sim.units_mut()[1];
    unit.state = UnitState::Routing;
    unit.morale = 5.0;
    run(&mut sim, 2.5);
    assert_eq!(sim.order_use(SideId::Attacker, "order_rally").uses, 1);
}

#[test]
fn ai_raises_pavises_under_fire() {
    let data = data();
    let s = order_setup(
        &data,
        &["unit_men_at_arms_foot", "unit_genoese_crossbowmen"],
        &["unit_men_at_arms_foot", "unit_longbowmen"],
    );
    let mut sim = ai_lab(s, SideId::Attacker, 9);
    place(&mut sim, 0, 300.0, 100.0, 0.0);
    place(&mut sim, 1, 600.0, 300.0, 0.0);
    place(&mut sim, 2, 1100.0, 750.0, std::f64::consts::PI);
    place(&mut sim, 3, 600.0, 490.0, std::f64::consts::PI);
    let mut raised = false;
    for _ in 0..400 {
        sim.step();
        if sim.units()[1].pavise.is_some() {
            raised = true;
            break;
        }
    }
    assert!(
        raised,
        "the Genoese should raise their pavises under the arrows"
    );
}

#[test]
fn ai_dismounts_on_the_defensive() {
    let data = data();
    // A weak English host with knights waits for a much stronger French one.
    let s = order_setup(
        &data,
        &[
            "unit_men_at_arms_foot",
            "unit_men_at_arms_foot",
            "unit_men_at_arms_foot",
            "unit_knights",
            "unit_knights",
        ],
        &["unit_urban_militia", "unit_knights"],
    );
    let mut sim = ai_lab(s, SideId::Defender, 4);
    run(&mut sim, 3.0);
    let knights = sim
        .units()
        .iter()
        .find(|u| u.side == SideId::Defender && u.unit_type == "unit_knights")
        .unwrap();
    assert!(knights.dismounted, "the defensive AI fights on foot");
    // The strong side keeps its horses.
    let mut sim = ai_lab(
        order_setup(
            &data,
            &[
                "unit_men_at_arms_foot",
                "unit_men_at_arms_foot",
                "unit_men_at_arms_foot",
                "unit_knights",
            ],
            &["unit_urban_militia"],
        ),
        SideId::Attacker,
        4,
    );
    run(&mut sim, 3.0);
    assert!(sim.units()[3].mounted);
}

#[test]
fn ai_gives_no_quarter_only_outnumbered_against_its_hereditary_enemy() {
    let data = data();
    let weak_french = |defender_faction: &str| {
        let mut s = order_setup(
            &data,
            &["unit_men_at_arms_foot", "unit_urban_militia"],
            &[
                "unit_men_at_arms_foot",
                "unit_men_at_arms_foot",
                "unit_men_at_arms_foot",
                "unit_longbowmen",
            ],
        );
        s.defender.faction = defender_faction.to_owned();
        let mut sim = ai_lab(s, SideId::Attacker, 6);
        place(&mut sim, 0, 600.0, 300.0, 0.0);
        place(&mut sim, 1, 660.0, 300.0, 0.0);
        for (k, id) in [2u32, 3, 4, 5].into_iter().enumerate() {
            place(
                &mut sim,
                id,
                450.0 + 90.0 * k as f64,
                420.0,
                std::f64::consts::PI,
            );
        }
        run(&mut sim, 2.5);
        sim
    };
    let against_england = weak_french("fac_england");
    assert!(against_england.no_quarter(SideId::Attacker));
    let against_navarre = weak_french("fac_navarre");
    assert!(!against_navarre.no_quarter(SideId::Attacker));
    // An army at least as strong never gives it.
    let s = order_setup(
        &data,
        &["unit_men_at_arms_foot", "unit_men_at_arms_foot"],
        &["unit_urban_militia"],
    );
    let mut sim = ai_lab(s, SideId::Attacker, 6);
    let gz = sim.units()[0].z;
    place(&mut sim, 2, 600.0, gz + 100.0, std::f64::consts::PI);
    run(&mut sim, 10.0);
    assert!(!sim.no_quarter(SideId::Attacker));
}

#[test]
fn ai_battle_with_orders_is_deterministic_and_uses_them() {
    let data = data();
    let army = [
        "unit_men_at_arms_foot",
        "unit_men_at_arms_foot",
        "unit_longbowmen",
        "unit_genoese_crossbowmen",
        "unit_knights",
        "unit_urban_militia",
    ];
    let play = || {
        let s = order_setup(&data, &army, &army);
        let mut sim = BattleSim::new(s, 17).unwrap();
        run_to_end(&mut sim);
        (fingerprint(&sim), sim.events().to_vec())
    };
    let (a, events) = play();
    let (b, _) = play();
    assert_eq!(a, b);
    assert!(
        events.iter().any(|e| e.text_fr.contains("cri de guerre")),
        "the AI should cry on engagement"
    );
}
