//! Helpers shared by the siege and AI tests.
#![allow(dead_code)]

use std::path::PathBuf;

use data_model::{GameData, Terrain, UnitTypeId};
use sim_battle::{
    BattleSeason, BattleSetup, BattleSim, SideId, SideSetup, SiegeSetup, UnitSetup, Weather, DT,
};

pub fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

pub fn unit(data: &GameData, id: &str) -> UnitSetup {
    let unit_type = &data.unit_types[&UnitTypeId::new(id).unwrap()];
    UnitSetup::from_unit_type(unit_type, unit_type.soldiers, unit_type.stats.morale, 0)
}

pub fn units(data: &GameData, ids: &[&str]) -> Vec<UnitSetup> {
    ids.iter().map(|id| unit(data, id)).collect()
}

pub fn side(name: &str, units: Vec<UnitSetup>) -> SideSetup {
    SideSetup {
        faction: format!("fac_{}", name.to_lowercase()),
        faction_name: name.to_owned(),
        army: String::new(),
        units,
        general: None,
        forced_march: false,
        entrenched: false,
        start_fatigue: 0.0,
    }
}

pub fn setup(
    attacker: Vec<UnitSetup>,
    defender: Vec<UnitSetup>,
    siege: Option<SiegeSetup>,
) -> BattleSetup {
    BattleSetup {
        province: "prov_test".to_owned(),
        province_name: "Test".to_owned(),
        terrain: Terrain::Plains,
        river: false,
        season: BattleSeason::Summer,
        coastal: false,
        village: None,
        attacker: side("France", attacker),
        defender: side("Angleterre", defender),
        player_side: None,
        siege,
        siege_layout: None,
        orders: Vec::new(),
        standards: None,
        decor_plan: None,
        opening: Default::default(),
    }
}

/// Both AIs off, clear weather, no end checks.
pub fn lab(sim: &mut BattleSim) {
    sim.set_ai(SideId::Attacker, false);
    sim.set_ai(SideId::Defender, false);
    sim.set_weather(Weather::Clear);
    sim.set_end_conditions(false);
}

pub fn place(sim: &mut BattleSim, id: u32, x: f64, z: f64, facing: f64) {
    let unit = &mut sim.units_mut()[id as usize];
    unit.x = x;
    unit.z = z;
    unit.facing = facing;
}

pub fn run(sim: &mut BattleSim, seconds: f64) {
    let steps = (seconds / DT).round() as u64;
    for _ in 0..steps {
        if sim.is_finished() {
            return;
        }
        sim.step();
    }
}

pub fn run_to_end(sim: &mut BattleSim) {
    while !sim.is_finished() {
        sim.step();
    }
}

/// Shooters of `side` hold their fire (lab scenarios about something else).
pub fn hold_fire(sim: &mut BattleSim, side: SideId) {
    let shooters: Vec<u32> = sim
        .units()
        .iter()
        .filter(|u| u.side == side && u.can_shoot())
        .map(|u| u.id)
        .collect();
    if !shooters.is_empty() {
        sim.apply_command(
            sim_battle::Command::FireAtWill {
                units: shooters,
                enabled: false,
            },
            None,
        )
        .unwrap();
    }
}

pub fn has_event(sim: &BattleSim, needle: &str) -> bool {
    sim.events().iter().any(|e| e.text_fr.contains(needle))
}
