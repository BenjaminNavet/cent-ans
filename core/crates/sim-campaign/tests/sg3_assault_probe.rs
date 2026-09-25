//! SG3: assault durations of the landmark towns (Paris, Avignon, Bruges,
//! Calais, Rouen) with both sides under AI, over several seeds. The probe
//! (`cargo test -p sim-campaign --test sg3_assault_probe -- --ignored
//! --nocapture`) prints one line per town; the plain test checks that a
//! wooden gate falls to the ram within a few minutes while stone walls hold
//! much longer.

use std::path::PathBuf;

use data_model::{FactionId, GameData};
use sim_battle::{BattleSim, PieceKind, SideId, SiegeFxKind};
use sim_campaign::{ArmyId, CampaignState};

const TOWNS: [(&str, &str); 5] = [
    ("paris", "fac_england"),
    ("avignon", "fac_england"),
    ("bruges", "fac_france"),
    ("calais", "fac_england"),
    ("rouen", "fac_england"),
];

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _warnings) = GameData::load(&root).expect("game data loads");
    data
}

fn largest_army(state: &CampaignState, faction: &str) -> ArmyId {
    let faction = FactionId::new(faction).unwrap();
    state
        .armies
        .iter()
        .filter(|(_, a)| a.faction == faction)
        .max_by_key(|(_, a)| a.units.len())
        .map(|(id, _)| id.clone())
        .unwrap()
}

/// One assault: seconds to the first ram blow, to the gate falling, to the
/// first wall breach; gate and wall HP; battle length and winner.
#[derive(Debug, Default, Clone)]
struct Assault {
    gate_hp: f64,
    gate_left: f64,
    wall_hp: f64,
    first_blow: Option<f64>,
    gate_at: Option<f64>,
    breach_at: Option<f64>,
    blows: usize,
    ended: f64,
    attacker_won: bool,
}

fn assault(data: &GameData, landmark: &str, attacker: &str, seed: u64, limit_s: f64) -> Assault {
    let mut state =
        CampaignState::new_1337(&data.clone(), FactionId::new("fac_france").unwrap(), 5)
            .expect("1337 start");
    state.chronicle.disabled = true;
    let army = largest_army(&state, attacker);
    if std::env::var("ENGINES").is_ok() {
        // Engines added to the besiegers: how long the stone walls hold.
        for id in ["unit_trebuchet", "unit_bombard"] {
            let t = data
                .unit_types
                .values()
                .find(|t| t.id.as_str() == id)
                .unwrap();
            state
                .armies
                .get_mut(&army)
                .unwrap()
                .units
                .push(sim_campaign::state::Unit {
                    unit_type: t.id.clone(),
                    strength: t.soldiers,
                    max_strength: t.soldiers,
                    morale: t.stats.morale,
                    experience: 0,
                    levy_armor: 0,
                    levy_ranged: 0,
                });
        }
    }
    let index = state
        .debug_stage_landmark_siege(data, &army, landmark)
        .unwrap_or_else(|e| panic!("{landmark}: {e}"));
    let setup = state.battle_setup(data, index).expect("setup");
    let mut sim = BattleSim::new(setup, seed).expect("battle");
    sim.set_ai(SideId::Attacker, true);
    sim.set_ai(SideId::Defender, true);
    if sim.is_deploying() {
        sim.start_battle().expect("start");
    }
    let mut out = Assault::default();
    if let Some(works) = sim.siege() {
        out.gate_hp = works.pieces[works.gate].max_hp;
        out.wall_hp = works
            .pieces
            .iter()
            .filter(|p| p.kind == PieceKind::Wall)
            .map(|p| p.max_hp)
            .fold(0.0, f64::max);
    }
    while !sim.is_finished() && sim.elapsed() < limit_s {
        sim.step();
    }
    for fx in sim.siege_fx() {
        match fx.kind {
            SiegeFxKind::RamStrike { .. } => {
                out.blows += 1;
                out.first_blow.get_or_insert(fx.time);
            }
            SiegeFxKind::GateBroken { .. } => {
                out.gate_at.get_or_insert(fx.time);
            }
            SiegeFxKind::WallBreached { .. } => {
                out.breach_at.get_or_insert(fx.time);
            }
            _ => {}
        }
    }
    out.gate_left = sim
        .siege()
        .map_or(0.0, |w| w.pieces[w.gate].hp.max(0.0) / out.gate_hp.max(1.0));
    out.ended = sim.elapsed();
    out.attacker_won = sim.winner() == Some(SideId::Attacker);
    out
}

fn fmt(v: Option<f64>) -> String {
    v.map_or("—".to_owned(), |t| format!("{t:.0}"))
}

fn median(mut v: Vec<f64>) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(f64::total_cmp);
    Some(v[v.len() / 2])
}

#[test]
#[ignore = "probe: prints the assault durations of the landmark towns"]
fn probe_landmark_assaults() {
    let data = data();
    let seeds: u64 = std::env::var("SEEDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);
    let limit: f64 = std::env::var("LIMIT_S")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1800.0);
    println!("| Ville | PV porte | PV mur | 1er coup (s) | porte tombée | porte (médiane s) | brèche (médiane s) | victoires assaillant | durée médiane (s) |");
    println!("|---|---|---|---|---|---|---|---|---|");
    for (landmark, attacker) in TOWNS {
        let runs: Vec<Assault> = (0..seeds)
            .map(|k| assault(&data, landmark, attacker, 11 + k, limit))
            .collect();
        for (k, r) in runs.iter().enumerate() {
            eprintln!(
                "{landmark} seed {}: blow {} gate {} (left {:.0} %) breach {} blows {} end {:.0} won {}",
                11 + k as u64,
                fmt(r.first_blow),
                fmt(r.gate_at),
                r.gate_left * 100.0,
                fmt(r.breach_at),
                r.blows,
                r.ended,
                r.attacker_won
            );
        }
        let gates: Vec<f64> = runs.iter().filter_map(|r| r.gate_at).collect();
        let breaches: Vec<f64> = runs.iter().filter_map(|r| r.breach_at).collect();
        println!(
            "| {landmark} | {:.0} | {:.0} | {} | {}/{} | {} | {} | {}/{} | {} |",
            runs[0].gate_hp,
            runs[0].wall_hp,
            fmt(median(runs.iter().filter_map(|r| r.first_blow).collect())),
            gates.len(),
            runs.len(),
            fmt(median(gates)),
            fmt(median(breaches)),
            runs.iter().filter(|r| r.attacker_won).count(),
            runs.len(),
            fmt(median(runs.iter().map(|r| r.ended).collect())),
        );
    }
}

/// The reported case (SG2 probe, Avignon, seed 11): the gate now falls to the
/// ram within ten minutes, a few minutes after the first blow.
#[test]
fn avignon_gate_falls_to_the_ram_within_minutes() {
    let data = data();
    let run = assault(&data, "avignon", "fac_england", 11, 600.0);
    let first = run.first_blow.expect("the ram reaches the gate");
    let fell = run.gate_at.expect("the gate falls within 600 s");
    assert!(fell - first < 300.0, "battered for {:.0} s", fell - first);
}

/// Wooden gate against stone wall (`data/rules/siege_works.json`): at every
/// fortification level a full ram crew breaks the gate in a few minutes, a
/// trebuchet alone needs at least three times as long for a wall piece.
#[test]
fn stone_walls_hold_far_longer_than_the_gate() {
    let data = data();
    let rules = sim_battle::SiegeWorkRules::bundled();
    let trebuchet = data
        .unit_types
        .values()
        .find(|t| t.id.as_str() == "unit_trebuchet")
        .unwrap();
    let per_shot = f64::from(trebuchet.stats.siege_attack.unwrap_or(0))
        * rules.engine.wall_damage_per_siege_attack;
    for fort in 0..=5 {
        let (wall, gate) = rules.hp(fort);
        let ram_s = gate / rules.ram.damage_per_s;
        let wall_s = (wall / per_shot).ceil() * sim_battle::shot::ENGINE_RELOAD;
        assert!(ram_s <= 180.0, "fort {fort}: gate {ram_s:.0} s");
        assert!(
            wall_s >= 3.0 * ram_s,
            "fort {fort}: wall {wall_s:.0} s, gate {ram_s:.0} s"
        );
    }
}
