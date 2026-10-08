//! SG3: assault durations of the landmark towns (Paris, Avignon, Bruges,
//! Calais, Rouen) with both sides under AI, over several seeds. The probe
//! (`cargo test -p sim-campaign --test sg3_assault_probe -- --ignored
//! --nocapture`) prints one line per town; the plain test checks that a
//! wooden gate falls to the ram within a few minutes while stone walls hold
//! much longer.

use data_model::{FactionId, GameData};
use sim_battle::{BattleSim, PieceKind, SideId, SiegeFxKind};
use sim_campaign::{ArmyId, CampaignState};

use data_model::test_support::game_data;

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
    /// Soldiers of each side at the start (attacker, garrison).
    men: (u32, u32),
    /// Share of its soldiers each side lost.
    lost: (f64, f64),
    withdrew: bool,
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
                    experience_residue: 0,
                });
        }
    }
    let index = state
        .debug_stage_landmark_siege(data, &army, landmark)
        .unwrap_or_else(|e| panic!("{landmark}: {e}"));
    let mut setup = state.battle_setup(data, index).expect("setup");
    // `ATTACKER_SHARE=0.5`: the besiegers at half strength (a lost assault).
    if let Some(share) = std::env::var("ATTACKER_SHARE")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
    {
        for unit in setup.attacker.units.iter_mut() {
            unit.soldiers = ((f64::from(unit.soldiers) * share).round() as u32).max(1);
        }
    }
    let mut sim = BattleSim::new(setup, seed).expect("battle");
    sim.set_ai(SideId::Attacker, true);
    sim.set_ai(SideId::Defender, true);
    if sim.is_deploying() {
        sim.start_battle().expect("start");
    }
    let mut out = Assault::default();
    let men = |sim: &BattleSim, side: SideId| -> (f64, f64) {
        sim.units()
            .iter()
            .filter(|u| u.side == side && !u.synthetic)
            .fold((0.0, 0.0), |(l, f), u| {
                (l + u.hp.max(0.0), f + f64::from(u.initial_soldiers))
            })
    };
    out.men = (
        men(&sim, SideId::Attacker).1 as u32,
        men(&sim, SideId::Defender).1 as u32,
    );
    if let Some(works) = sim.siege() {
        out.gate_hp = works.pieces[works.gate].max_hp;
        out.wall_hp = works
            .pieces
            .iter()
            .filter(|p| p.kind == PieceKind::Wall)
            .map(|p| p.max_hp)
            .fold(0.0, f64::max);
    }
    let dump = std::env::var("DUMP").is_ok_and(|d| d == format!("{landmark}:{seed}"));
    let mut next_dump = 0.0;
    while !sim.is_finished() && sim.elapsed() < limit_s {
        sim.step();
        if dump && sim.elapsed() >= next_dump {
            next_dump += 60.0;
            dump_units(&sim);
        }
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
    let (a, d) = (men(&sim, SideId::Attacker), men(&sim, SideId::Defender));
    out.lost = (1.0 - a.0 / a.1.max(1.0), 1.0 - d.0 / d.1.max(1.0));
    out.withdrew = sim
        .units()
        .iter()
        .any(|u| u.side == SideId::Attacker && u.withdrawing);
    out.attacker_won = sim.winner() == Some(SideId::Attacker);
    out
}

/// `DUMP=<town>:<seed>`: the state of every regiment once a minute.
fn dump_units(sim: &BattleSim) {
    let works = sim.siege().unwrap();
    eprintln!(
        "--- t {:.0} s, gate {:.0} hp, openings {:?}",
        sim.elapsed(),
        works.pieces[works.gate].hp,
        works.openings()
    );
    for u in sim.units().iter().filter(|u| u.present()) {
        eprintln!(
            "  {:?} {:>3} {:<28} hp {:>5.0} mor {:>3.0} {:?} ({:>4.0},{:>4.0}) in {} wall {} climb {:?} dest {:?} tgt {:?} wd {}",
            u.side,
            u.id,
            u.unit_type,
            u.hp,
            u.morale,
            u.state,
            u.x,
            u.z,
            works.inside(u.x, u.z),
            u.on_wall,
            u.climbing,
            u.destination.map(|(x, z)| (x.round(), z.round())),
            u.target,
            u.withdrawing
        );
    }
}

/// The reported case (SG2 probe, Avignon, seed 11): the gate now falls to the
/// ram within ten minutes, a few minutes after the first blow.
#[test]
fn avignon_gate_falls_to_the_ram_within_minutes() {
    let data = game_data();
    let run = assault(data, "avignon", "fac_england", 11, 600.0);
    let first = run.first_blow.expect("the ram reaches the gate");
    let fell = run.gate_at.expect("the gate falls within 600 s");
    assert!(fell - first < 300.0, "battered for {:.0} s", fell - first);
}

/// Wooden gate against stone wall (`data/rules/siege_works.json`): at every
/// fortification level a full ram crew breaks the gate in a few minutes, a
/// trebuchet alone needs clearly longer for a wall piece (SB, ADR 0107: at
/// least 1.5 times as long; the gate falls in 40-60 s at level 3, a wall in
/// 6-10 trebuchet shots).
#[test]
fn stone_walls_hold_far_longer_than_the_gate() {
    let data = game_data();
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
            wall_s >= 1.5 * ram_s,
            "fort {fort}: wall {wall_s:.0} s, gate {ram_s:.0} s"
        );
    }
}
