//! Naval battles on the GDExtension side (lot NV1, ADR 0028): the
//! `NavalBattleSim` class and the naval methods of `CampaignSim` (a
//! secondary `#[godot_api]` block).
//!
//! Setups, commands and outcomes cross as `Dictionary`s converted through
//! JSON with the serde types of `sim_battle::naval`; ships go out as one
//! dictionary each (a battle has a few dozen ships).

use std::path::PathBuf;

use data_model::load::load_entities;
use data_model::{NavalData, UnitType};
use godot::classes::RefCounted;
use godot::prelude::*;
use sim_battle::naval::{
    auto_resolve, NavalCommand, NavalEventKind, NavalOutcome, NavalScenario, NavalSetup, NavalSim,
    Ship, ShipStatus, NAVAL_DT,
};
use sim_battle::SideId;

use crate::battle_sim::{from_dict, result_dict, to_dict};
use crate::campaign_sim::{events_array, CampaignSim};

/// Godot-facing handle on one naval battle.
#[derive(GodotClass)]
#[class(base = RefCounted)]
pub struct NavalBattleSim {
    sim: Option<NavalSim>,
    seed: u64,
    /// Time not yet simulated (less than one step).
    pending: f64,
    /// Scenario shown in the UI (`name`, `date`, `description`).
    scenario: Option<NavalScenario>,
    base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for NavalBattleSim {
    fn init(base: Base<RefCounted>) -> Self {
        NavalBattleSim {
            sim: None,
            seed: 1,
            pending: 0.0,
            scenario: None,
            base,
        }
    }
}

fn side_key(side: SideId) -> &'static str {
    side.key()
}

fn ship_dict(ship: &Ship) -> VarDictionary {
    let crew: VarArray = ship
        .crew
        .iter()
        .map(|c| {
            vdict! {
                "unit" => c.unit as i64,
                "unit_type" => c.unit_type.as_str(),
                "men" => c.men,
                "initial" => c.initial,
                "shoots" => c.shoots(),
                "ranged" => c.range > 0.0,
                "missile" => c.missile.key(),
                "ammo" => c.ammo,
            }
            .to_variant()
        })
        .collect();
    let grappled: PackedInt32Array = ship.grappled.iter().map(|&g| g as i32).collect();
    let captor = match ship.status {
        ShipStatus::Captured { by } => side_key(by),
        _ => "",
    };
    let sinking = match ship.status {
        ShipStatus::Sinking { since } => since,
        _ => -1.0,
    };
    let class = &ship.class;
    vdict! {
        "id" => i64::from(ship.id),
        "side" => side_key(ship.side),
        "index" => ship.index as i64,
        "name" => ship.name.as_str(),
        "class" => class.id.as_str(),
        "class_name" => class.name.display.as_str(),
        "model" => class.model.as_str(),
        "x" => ship.x,
        "z" => ship.z,
        "heading" => ship.heading,
        "speed" => ship.speed,
        "hull" => ship.hull,
        "hull_max" => f64::from(class.hull),
        "fire" => ship.fire,
        "status" => ship.status.key(),
        "captor" => captor,
        "sinking_since" => sinking,
        "crew" => &crew,
        "soldiers" => ship.soldiers(),
        "soldiers_initial" => ship.soldiers_initial(),
        "sailors" => ship.sailors,
        "sailors_initial" => ship.sailors_initial,
        "rowers" => i64::from(class.rowers),
        "stamina" => ship.stamina,
        "morale" => ship.morale,
        "order" => ship.order.key(),
        "target" => ship.order.target().map_or(-1, i64::from),
        "last_target" => ship.last_target.map_or(-1, i64::from),
        "grappled" => &grappled,
        "chain" => ship.chain.map_or(-1, i64::from),
        "fireship" => ship.fireship,
        "fire_arrows" => ship.fire_arrows,
        "flagship" => ship.flagship,
        "fleeing" => ship.fleeing,
        "galley" => ship.is_galley(),
        "can_ram" => class.ram > 0.0,
        "length" => class.length_m,
        "beam" => class.beam_m,
        "freeboard" => class.freeboard_m,
        "forecastle" => class.forecastle_m,
        "aftcastle" => class.aftcastle_m,
        "melee_time" => ship.melee_time,
        "melee_power" => ship.melee_power(),
        "ranged_power" => ship.ranged_power(),
    }
}

fn event_dict(time: f64, kind: &NavalEventKind) -> VarDictionary {
    let (key, ship, other, extra) = match *kind {
        NavalEventKind::Grapple { ship, other } => ("grapple", ship, Some(other), 0.0),
        NavalEventKind::Cut { ship, other } => ("cut", ship, Some(other), 0.0),
        NavalEventKind::Board { ship, other } => ("board", ship, Some(other), 0.0),
        NavalEventKind::Capture { ship, by } => (
            "capture",
            ship,
            None,
            if by == SideId::Attacker { 0.0 } else { 1.0 },
        ),
        NavalEventKind::Ignite { ship } => ("ignite", ship, None, 0.0),
        NavalEventKind::Fireship { ship, other } => ("fireship", ship, Some(other), 0.0),
        NavalEventKind::Abandon { ship } => ("abandon", ship, None, 0.0),
        NavalEventKind::Sinking { ship } => ("sinking", ship, None, 0.0),
        NavalEventKind::Sunk { ship } => ("sunk", ship, None, 0.0),
        NavalEventKind::Ram {
            ship,
            other,
            damage,
        } => ("ram", ship, Some(other), damage),
        NavalEventKind::Flee { ship } => ("flee", ship, None, 0.0),
        NavalEventKind::Escaped { ship } => ("escaped", ship, None, 0.0),
        NavalEventKind::Assault { ship, side } => (
            "assault",
            ship,
            None,
            if side == SideId::Attacker { 0.0 } else { 1.0 },
        ),
    };
    vdict! {
        "time" => time,
        "kind" => key,
        "ship" => i64::from(ship),
        "other" => other.map_or(-1, i64::from),
        "value" => extra,
    }
}

impl NavalBattleSim {
    fn start(&mut self, setup: NavalSetup, seed: u64) -> bool {
        match NavalSim::new(setup, seed) {
            Ok(sim) => {
                self.sim = Some(sim);
                self.seed = seed;
                self.pending = 0.0;
                true
            }
            Err(error) => {
                godot_error!("NavalBattleSim: {error}");
                self.sim = None;
                false
            }
        }
    }
}

#[godot_api]
impl NavalBattleSim {
    /// Builds the battle from a `CampaignSim.get_naval_battle_setup` setup.
    #[func]
    fn setup(&mut self, setup: VarDictionary, seed: i64) -> bool {
        match from_dict::<NavalSetup>(&setup) {
            Ok(setup) => {
                self.scenario = None;
                self.start(setup, seed as u64)
            }
            Err(error) => {
                godot_error!("NavalBattleSim.setup: {error}");
                false
            }
        }
    }

    /// Loads the historical scenario `data_dir/naval/scenarios/<id>.json`
    /// (`sluys`, `la_rochelle`) with the ship classes and unit types of
    /// `data_dir`.
    #[func]
    fn setup_scenario(&mut self, data_dir: GString, id: GString, seed: i64) -> bool {
        let root = PathBuf::from(data_dir.to_string());
        let loaded = (|| -> Result<(NavalScenario, NavalSetup), String> {
            let naval = NavalData::load(&root).map_err(|e| e.to_string())?;
            let units = load_entities(&root.join("unit_types"), |u: &UnitType| &u.id)
                .map_err(|e| e.to_string())?;
            let path = root.join("naval/scenarios").join(format!("{}.json", id));
            let text =
                std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let scenario: NavalScenario = serde_json::from_str(&text).map_err(|e| e.to_string())?;
            let setup = scenario.to_setup(&naval.ship_classes, &units, &naval.rules)?;
            Ok((scenario, setup))
        })();
        match loaded {
            Ok((scenario, setup)) => {
                self.scenario = Some(scenario);
                self.start(setup, seed as u64)
            }
            Err(error) => {
                godot_error!("NavalBattleSim.setup_scenario({id}): {error}");
                false
            }
        }
    }

    /// `{id, name, date, place_name, description}` of the loaded scenario
    /// (empty for a campaign battle).
    #[func]
    fn get_scenario(&self) -> VarDictionary {
        let Some(s) = &self.scenario else {
            return VarDictionary::new();
        };
        vdict! {
            "id" => s.id.as_str(),
            "name" => s.name.as_str(),
            "date" => s.date.as_str(),
            "place_name" => s.place_name.as_str(),
            "description" => s.description.as_deref().unwrap_or(""),
        }
    }

    /// Advances the battle by `dt` seconds (fixed steps inside).
    #[func]
    fn tick(&mut self, dt: f64) {
        let Some(sim) = &mut self.sim else {
            return;
        };
        self.pending += dt.max(0.0);
        while self.pending >= NAVAL_DT {
            self.pending -= NAVAL_DT;
            sim.step();
        }
    }

    /// `{type: move|board|shoot|ram|hold|disengage|fire_arrows, ship, target,
    /// x, z, enabled}` → `{ok, error}`.
    #[func]
    fn issue_command(&mut self, command: VarDictionary) -> VarDictionary {
        let Some(sim) = &mut self.sim else {
            return result_dict(Err("aucune bataille".to_owned()));
        };
        let result = from_dict::<NavalCommand>(&command)
            .and_then(|c| sim.issue(c).map_err(|e| e.to_string()));
        result_dict(result)
    }

    /// Hands a side to the AI (or back to the player).
    #[func]
    fn set_ai(&mut self, side: GString, enabled: bool) {
        if let (Some(sim), Some(side)) = (&mut self.sim, SideId::parse(&side.to_string())) {
            sim.set_ai(side, enabled);
        }
    }

    /// One dictionary per ship (see `ship_dict`).
    #[func]
    fn get_ships(&self) -> VarArray {
        let Some(sim) = &self.sim else {
            return VarArray::new();
        };
        sim.ships
            .iter()
            .map(|s| ship_dict(s).to_variant())
            .collect()
    }

    /// Volleys since the previous call: `[{time, shooter, target, from:
    /// Vector2, aim: Vector2, missiles, kills, kind, incendiary}]`.
    #[func]
    fn get_shots(&mut self) -> VarArray {
        let Some(sim) = &mut self.sim else {
            return VarArray::new();
        };
        sim.take_shots()
            .iter()
            .map(|shot| {
                vdict! {
                    "time" => shot.time,
                    "shooter" => i64::from(shot.shooter),
                    "target" => shot.target.map_or(-1, i64::from),
                    "from" => Vector2::new(shot.from.0 as f32, shot.from.1 as f32),
                    "aim" => Vector2::new(shot.aim.0 as f32, shot.aim.1 as f32),
                    "missiles" => i64::from(shot.missiles),
                    "kills" => shot.kills,
                    "kind" => shot.kind.key(),
                    "incendiary" => shot.incendiary,
                }
                .to_variant()
            })
            .collect()
    }

    /// Events since the previous call: `[{time, kind, ship, other, value}]`
    /// (`kind`: grapple, cut, board, capture, ignite, fireship, abandon,
    /// sinking, sunk, ram, flee, escaped; `value`: ram damage, or the
    /// captor side of a capture, 0 attacker / 1 defender).
    #[func]
    fn get_events(&mut self) -> VarArray {
        let Some(sim) = &mut self.sim else {
            return VarArray::new();
        };
        sim.take_events()
            .iter()
            .map(|e| event_dict(e.time, &e.kind).to_variant())
            .collect()
    }

    /// `{to (radians, direction the wind blows towards), strength, gauge}`.
    #[func]
    fn get_wind(&self) -> VarDictionary {
        let Some(sim) = &self.sim else {
            return VarDictionary::new();
        };
        vdict! {
            "to" => sim.wind.to,
            "strength" => sim.wind.strength,
            "gauge" => sim.gauge().map_or("", side_key),
        }
    }

    #[func]
    fn get_setup(&self) -> VarDictionary {
        self.sim
            .as_ref()
            .map_or_else(VarDictionary::new, |sim| to_dict(&sim.setup))
    }

    /// Soldiers and sailors still fighting on `side`.
    #[func]
    fn get_strength(&self, side: GString) -> f64 {
        match (&self.sim, SideId::parse(&side.to_string())) {
            (Some(sim), Some(side)) => sim.strength(side),
            _ => 0.0,
        }
    }

    #[func]
    fn get_elapsed(&self) -> f64 {
        self.sim.as_ref().map_or(0.0, |s| s.elapsed)
    }

    #[func]
    fn is_finished(&self) -> bool {
        self.sim.as_ref().is_some_and(NavalSim::is_finished)
    }

    /// `NavalOutcome` as a dictionary (for `CampaignSim.resolve_naval_battle`).
    #[func]
    fn get_outcome(&self) -> VarDictionary {
        self.sim
            .as_ref()
            .map_or_else(VarDictionary::new, |sim| to_dict(&sim.outcome()))
    }

    /// Auto-resolves the loaded battle (without touching the running one).
    #[func]
    fn auto_resolve(&self) -> VarDictionary {
        self.sim.as_ref().map_or_else(VarDictionary::new, |sim| {
            to_dict(&auto_resolve(&sim.setup, self.seed))
        })
    }
}

#[godot_api(secondary)]
impl CampaignSim {
    /// Naval battles waiting for the player (lot NV1): `[{index, army, from,
    /// to, sea, sea_name, interceptor, interceptor_name, faction,
    /// faction_name, interceptor_ships, transport_ships, interceptor_men,
    /// army_men, win_chance, player_side, seed}]`.
    #[func]
    fn get_pending_naval_battles(&self) -> VarArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarArray::new();
        };
        state
            .pending_naval_views(data)
            .iter()
            .map(|view| {
                let mut dict = to_dict(view);
                let seed = state.naval_battle_seed(view.index).unwrap_or(1);
                dict.set("seed", (seed & 0x7FFF_FFFF_FFFF) as i64);
                dict.set("player_side", view.player_side.key());
                dict.to_variant()
            })
            .collect()
    }

    /// Setup of pending naval battle `index` for `NavalBattleSim.setup`.
    #[func]
    fn get_naval_battle_setup(&self, index: i64) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarDictionary::new();
        };
        match state.naval_battle_setup(data, index.max(0) as usize) {
            Ok(setup) => to_dict(&setup),
            Err(error) => {
                godot_warn!("CampaignSim.get_naval_battle_setup({index}): {error}");
                VarDictionary::new()
            }
        }
    }

    /// Applies a `NavalBattleSim.get_outcome()` → `{ok, error, events}`.
    #[func]
    fn resolve_naval_battle(&mut self, index: i64, outcome: VarDictionary) -> VarDictionary {
        let (Some(state), Some(data)) = (&mut self.state, &self.data) else {
            return result_dict(Err("aucune campagne en cours".to_owned()));
        };
        let result = from_dict::<NavalOutcome>(&outcome)
            .map_err(|e| format!("résultat invalide : {e}"))
            .and_then(|outcome| {
                state
                    .resolve_naval_battle(data, index.max(0) as usize, &outcome)
                    .map_err(|e| e.to_string())
            });
        match result {
            Ok(events) => {
                let mut dict = result_dict(Ok(()));
                dict.set("events", &events_array(&events));
                dict
            }
            Err(error) => result_dict(Err(error)),
        }
    }

    /// Auto-resolves pending naval battle `index` → `{ok, error, events}`.
    #[func]
    fn auto_resolve_naval_battle(&mut self, index: i64) -> VarDictionary {
        let (Some(state), Some(data)) = (&mut self.state, &self.data) else {
            return result_dict(Err("aucune campagne en cours".to_owned()));
        };
        match state.auto_resolve_naval_battle(data, index.max(0) as usize) {
            Ok(events) => {
                let mut dict = result_dict(Ok(()));
                dict.set("events", &events_array(&events));
                dict
            }
            Err(error) => result_dict(Err(error.to_string())),
        }
    }

    /// The intercepted fleet puts back into port → `{ok, error, events}`.
    #[func]
    fn withdraw_naval_battle(&mut self, index: i64) -> VarDictionary {
        let (Some(state), Some(data)) = (&mut self.state, &self.data) else {
            return result_dict(Err("aucune campagne en cours".to_owned()));
        };
        match state.withdraw_naval_battle(data, index.max(0) as usize) {
            Ok(events) => {
                let mut dict = result_dict(Ok(()));
                dict.set("events", &events_array(&events));
                dict
            }
            Err(error) => result_dict(Err(error.to_string())),
        }
    }

    /// `{fleets: {faction: {class: count}}, control: {sea: {faction,
    /// level, name}}, blockaded: [settlement]}`.
    #[func]
    fn get_naval_state(&self) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarDictionary::new();
        };
        let mut naval = state.naval.clone();
        naval.ensure(data);
        let mut control = VarDictionary::new();
        for (sea, c) in &naval.control {
            control.set(
                sea.as_str(),
                &vdict! {
                    "faction" => c.faction.as_str(),
                    "level" => i64::from(c.level),
                    "name" => data.naval.sea_name(sea).as_str(),
                },
            );
        }
        let mut dict = to_dict(&naval);
        dict.set("control", &control);
        dict
    }

    /// Debug (smoke test, screenshots): `interceptor` bars `army`'s crossing
    /// to `to_port`; returns the index of the pending naval battle or -1.
    #[func]
    fn debug_stage_naval(&mut self, army: GString, to_port: GString, interceptor: GString) -> i64 {
        let (Some(state), Some(data)) = (&mut self.state, &self.data) else {
            return -1;
        };
        let (Some(army), Ok(to), Ok(by)) = (
            sim_campaign::ArmyId::parse(&army.to_string()),
            data_model::SettlementId::new(to_port.to_string().as_str()),
            data_model::FactionId::new(interceptor.to_string().as_str()),
        ) else {
            return -1;
        };
        state
            .debug_stage_naval(data, &army, &to, &by)
            .map_or(-1, |i| i as i64)
    }
}
