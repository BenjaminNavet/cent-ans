//! SG3: assault durations at the landmark towns with both sides under AI. One assault test
//! (Avignon: the gate falls to the ram within minutes) and the wooden-gate / stone-wall invariant
//! (a wall holds clearly longer than the gate at every fortification level).

use data_model::{FactionId, GameData};
use sim_battle::{BattleSim, SideId, SiegeFxKind};
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

/// One assault: seconds to the first ram blow and to the gate falling.
#[derive(Debug, Default, Clone)]
struct Assault {
    first_blow: Option<f64>,
    gate_at: Option<f64>,
}

fn assault(data: &GameData, landmark: &str, attacker: &str, seed: u64, limit_s: f64) -> Assault {
    let mut state =
        CampaignState::new_1337(&data.clone(), FactionId::new("fac_france").unwrap(), 5)
            .expect("1337 start");
    state.chronicle.disabled = true;
    let army = largest_army(&state, attacker);
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
    while !sim.is_finished() && sim.elapsed() < limit_s {
        sim.step();
    }
    let mut out = Assault::default();
    for fx in sim.siege_fx() {
        match fx.kind {
            SiegeFxKind::RamStrike { .. } => {
                out.first_blow.get_or_insert(fx.time);
            }
            SiegeFxKind::GateBroken { .. } => {
                out.gate_at.get_or_insert(fx.time);
            }
            _ => {}
        }
    }
    out
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
