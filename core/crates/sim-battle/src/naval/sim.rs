//! Real-time naval battle (lot NV1, ADR 0028).
//!
//! Ships move on an open sea centred on the origin, pushed by the wind
//! (sail) or their oars (galleys). Every [`DT`] seconds: AI orders, movement
//! and spacing, grapples, volleys from the castles, boarding melee, fire,
//! rams, sinking, surrender and flight. Same setup + seed + commands at the
//! same ticks = same battle.

use std::collections::BTreeSet;

use data_model::{NavalRules, Propulsion};
use serde::{Deserialize, Serialize};

use super::combat::{self, wrap, Wind};
use super::outcome::{
    NavalEvent, NavalEventKind, NavalOutcome, NavalSideResult, ShipFate, ShipResult,
};
use super::setup::NavalSetup;
use super::ship::{Crew, RangeRules, Ship, ShipOrder, ShipStatus};
use crate::rng::BattleRng;
use crate::setup::SideId;
use crate::shot::{ShotCover, ShotEvent, MAX_PENDING_SHOTS};

/// Simulation step, seconds (same as the land battle).
pub const NAVAL_DT: f64 = 0.1;
/// Period of the AI's decisions, seconds.
pub const NAVAL_AI_PERIOD: f64 = 2.0;
/// Distance between the two fleets at the start, metres.
pub const OPENING_GAP: f64 = 900.0;
/// Events kept for the renderer between two reads.
pub const MAX_PENDING_EVENTS: usize = 512;
/// Seconds the battle goes on once decided, to let runaways get clear.
pub const ESCAPE_GRACE: f64 = 90.0;

/// An order given to one ship.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NavalCommand {
    Move {
        ship: u32,
        x: f64,
        z: f64,
    },
    Board {
        ship: u32,
        target: u32,
    },
    Shoot {
        ship: u32,
        target: u32,
    },
    Ram {
        ship: u32,
        target: u32,
    },
    Hold {
        ship: u32,
    },
    Disengage {
        ship: u32,
    },
    /// Fire arrows on or off.
    FireArrows {
        ship: u32,
        enabled: bool,
    },
}

impl NavalCommand {
    pub fn ship(self) -> u32 {
        match self {
            NavalCommand::Move { ship, .. }
            | NavalCommand::Board { ship, .. }
            | NavalCommand::Shoot { ship, .. }
            | NavalCommand::Ram { ship, .. }
            | NavalCommand::Hold { ship }
            | NavalCommand::Disengage { ship }
            | NavalCommand::FireArrows { ship, .. } => ship,
        }
    }
}

/// Why a naval command was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NavalCommandError {
    #[error("navire inconnu n°{0}")]
    UnknownShip(u32),
    #[error("le navire n°{0} n'est plus au combat")]
    OutOfBattle(u32),
    #[error("le navire n°{0} n'obéit plus (équipage en fuite)")]
    Fleeing(u32),
    #[error("la cible n°{0} n'est pas un navire ennemi à flot")]
    BadTarget(u32),
    #[error("seule une galère peut éperonner")]
    NoRam,
    #[error("ce navire n'a pas de tireurs")]
    NoShooters,
    #[error("ce navire est enchaîné à la ligne")]
    Chained,
}

/// Why a naval setup is refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NavalSetupError {
    #[error("la flotte {0} n'a aucun navire")]
    NoShips(&'static str),
    #[error("le navire {ship} embarque un régiment inconnu n°{unit}")]
    UnknownUnit { ship: String, unit: usize },
}

/// A naval battle in progress.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NavalSim {
    pub setup: NavalSetup,
    pub ships: Vec<Ship>,
    pub wind: Wind,
    pub elapsed: f64,
    pub ticks: u64,
    rng: BattleRng,
    /// Sides driven by the AI.
    pub ai: [bool; 2],
    ai_clock: f64,
    finished: bool,
    winner: Option<SideId>,
    decided_at: Option<f64>,
    #[serde(skip)]
    shots: Vec<ShotEvent>,
    #[serde(skip)]
    events: Vec<NavalEvent>,
    /// Every event of the battle (battle log, tests).
    pub log: Vec<NavalEvent>,
}

impl NavalSim {
    /// Builds the battle: fleets in line facing each other, `OPENING_GAP`
    /// apart along x (attacker west), wind from the setup or the seed.
    pub fn new(setup: NavalSetup, seed: u64) -> Result<NavalSim, NavalSetupError> {
        if setup.attacker.ships.is_empty() {
            return Err(NavalSetupError::NoShips("assaillante"));
        }
        if setup.defender.ships.is_empty() {
            return Err(NavalSetupError::NoShips("en défense"));
        }
        let mut rng = BattleRng::from_seed(seed);
        let wind = draw_wind(&setup, &mut rng);
        let ranges = RangeRules {
            bow: setup.rules.bow_range_m,
            crossbow: setup.rules.crossbow_range_m,
        };
        let mut ships = Vec::new();
        for side in SideId::BOTH {
            let fleet = setup.side(side);
            let count = fleet.ships.len();
            for (index, spec) in fleet.ships.iter().enumerate() {
                let mut crew = Vec::new();
                for c in &spec.crew {
                    let unit =
                        fleet
                            .units
                            .get(c.unit)
                            .ok_or_else(|| NavalSetupError::UnknownUnit {
                                ship: spec.name.clone(),
                                unit: c.unit,
                            })?;
                    if c.men > 0 {
                        crew.push(Crew::from_unit(c.unit, unit, c.men, &ranges));
                    }
                }
                let (x, z) = spec.position.map_or_else(
                    || formation_slot(side, index, count, spec.chain.is_some()),
                    |p| (p[0], p[1]),
                );
                let facing = match side {
                    SideId::Attacker => 0.0,
                    SideId::Defender => std::f64::consts::PI,
                };
                let heading = spec.heading_deg.map_or(facing, f64::to_radians);
                let morale = fleet.units.iter().map(|u| f64::from(u.morale)).sum::<f64>()
                    / fleet.units.len().max(1) as f64;
                let class = spec.class.clone();
                let sailors = f64::from(class.sailors);
                ships.push(Ship {
                    id: ships.len() as u32,
                    side,
                    index,
                    name: spec.name.clone(),
                    hull: f64::from(class.hull),
                    stamina: setup.rules.rower_stamina_s,
                    class,
                    x,
                    z,
                    heading,
                    speed: 0.0,
                    fire: 0.0,
                    status: ShipStatus::Afloat,
                    swimmers: vec![0.0; crew.len()],
                    drowned: vec![0.0; crew.len()],
                    prisoners: vec![0.0; crew.len()],
                    crew,
                    sailors,
                    sailors_initial: sailors,
                    morale: morale.clamp(30.0, 100.0),
                    order: ShipOrder::Hold,
                    grappled: Vec::new(),
                    chain: spec.chain,
                    fireship: spec.fireship,
                    fire_arrows: spec.fire_arrows,
                    flagship: spec.flagship,
                    fleeing: false,
                    last_target: None,
                    melee_time: 0.0,
                });
            }
        }
        let ai = match setup.player_side {
            Some(SideId::Attacker) => [false, true],
            Some(SideId::Defender) => [true, false],
            None => [true, true],
        };
        Ok(NavalSim {
            setup,
            ships,
            wind,
            elapsed: 0.0,
            ticks: 0,
            rng,
            ai,
            ai_clock: 0.0,
            finished: false,
            winner: None,
            decided_at: None,
            shots: Vec::new(),
            events: Vec::new(),
            log: Vec::new(),
        })
    }

    pub fn rules(&self) -> &NavalRules {
        &self.setup.rules
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    pub fn winner(&self) -> Option<SideId> {
        self.winner
    }

    pub fn set_ai(&mut self, side: SideId, enabled: bool) {
        self.ai[side.index()] = enabled;
    }

    /// Volleys since the last call (renderer).
    pub fn take_shots(&mut self) -> Vec<ShotEvent> {
        std::mem::take(&mut self.shots)
    }

    /// Events since the last call (renderer).
    pub fn take_events(&mut self) -> Vec<NavalEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn ship(&self, id: u32) -> Option<&Ship> {
        self.ships.get(id as usize)
    }

    /// Soldiers and sailors still fighting on `side`.
    pub fn strength(&self, side: SideId) -> f64 {
        self.ships
            .iter()
            .filter(|s| s.side == side && s.is_active())
            .map(Ship::fighting_men)
            .sum()
    }

    /// Weather gauge: the side whose fleet lies upwind of the other.
    pub fn gauge(&self) -> Option<SideId> {
        let centre = |side: SideId| {
            let ships: Vec<&Ship> = self.ships.iter().filter(|s| s.side == side).collect();
            let n = ships.len().max(1) as f64;
            (
                ships.iter().map(|s| s.x).sum::<f64>() / n,
                ships.iter().map(|s| s.z).sum::<f64>() / n,
            )
        };
        let a = self
            .wind
            .alignment(centre(SideId::Attacker), centre(SideId::Defender));
        if a > 0.2 {
            Some(SideId::Attacker)
        } else if a < -0.2 {
            Some(SideId::Defender)
        } else {
            None
        }
    }

    // ----- commands ---------------------------------------------------------

    pub fn issue(&mut self, command: NavalCommand) -> Result<(), NavalCommandError> {
        let id = command.ship();
        let ship = self
            .ships
            .get(id as usize)
            .ok_or(NavalCommandError::UnknownShip(id))?;
        if !ship.is_afloat() {
            return Err(NavalCommandError::OutOfBattle(id));
        }
        if ship.fleeing {
            return Err(NavalCommandError::Fleeing(id));
        }
        let side = ship.side;
        let enemy_ok = |target: u32| {
            self.ships
                .get(target as usize)
                .is_some_and(|t| t.side != side && t.is_afloat())
        };
        let chained = ship.chain.is_some();
        let order = match command {
            NavalCommand::Move { x, z, .. } => {
                if chained {
                    return Err(NavalCommandError::Chained);
                }
                ShipOrder::MoveTo { x, z }
            }
            NavalCommand::Board { target, .. } => {
                if !enemy_ok(target) {
                    return Err(NavalCommandError::BadTarget(target));
                }
                if chained && !self.is_touching(id, target) {
                    return Err(NavalCommandError::Chained);
                }
                ShipOrder::Board { target }
            }
            NavalCommand::Shoot { target, .. } => {
                if !enemy_ok(target) {
                    return Err(NavalCommandError::BadTarget(target));
                }
                if !ship.crew.iter().any(|c| c.range > 0.0) {
                    return Err(NavalCommandError::NoShooters);
                }
                ShipOrder::Shoot { target }
            }
            NavalCommand::Ram { target, .. } => {
                if !enemy_ok(target) {
                    return Err(NavalCommandError::BadTarget(target));
                }
                if ship.class.ram <= 0.0 {
                    return Err(NavalCommandError::NoRam);
                }
                ShipOrder::Ram { target }
            }
            NavalCommand::Hold { .. } => ShipOrder::Hold,
            NavalCommand::Disengage { .. } => {
                if chained {
                    return Err(NavalCommandError::Chained);
                }
                ShipOrder::Disengage
            }
            NavalCommand::FireArrows { enabled, .. } => {
                self.ships[id as usize].fire_arrows = enabled;
                return Ok(());
            }
        };
        self.ships[id as usize].order = order;
        Ok(())
    }

    fn is_touching(&self, a: u32, b: u32) -> bool {
        let (sa, sb) = (&self.ships[a as usize], &self.ships[b as usize]);
        sa.grappled.contains(&b) || sa.gap_to(sb) <= self.setup.rules.grapple_gap_m
    }

    // ----- tick -------------------------------------------------------------

    /// Advances the battle by `seconds` (whole steps of [`NAVAL_DT`]).
    pub fn advance(&mut self, seconds: f64) {
        let steps = (seconds / NAVAL_DT).round().max(0.0) as u64;
        for _ in 0..steps {
            self.step();
        }
    }

    /// Runs to the end (AI on both sides if nobody commands).
    pub fn run_to_end(&mut self) -> NavalOutcome {
        while !self.finished {
            self.step();
        }
        self.outcome()
    }

    /// One step of [`NAVAL_DT`] seconds.
    pub fn step(&mut self) {
        if self.finished {
            return;
        }
        let dt = NAVAL_DT;
        self.elapsed += dt;
        self.ticks += 1;
        self.ai_clock -= dt;
        if self.ai_clock <= 0.0 {
            self.ai_clock += NAVAL_AI_PERIOD;
            for side in SideId::BOTH {
                if self.ai[side.index()] {
                    super::ai::plan(self, side);
                }
            }
            self.flee_and_hold_orders();
        }
        self.movement(dt);
        self.grapples(dt);
        self.shooting(dt);
        self.melee(dt);
        self.fires(dt);
        self.sinking();
        self.surrender();
        self.check_end();
    }

    fn push_event(&mut self, kind: NavalEventKind) {
        let event = NavalEvent {
            time: self.elapsed,
            kind,
        };
        self.log.push(event.clone());
        if self.events.len() >= MAX_PENDING_EVENTS {
            self.events.remove(0);
        }
        self.events.push(event);
    }

    /// Orders that no AI and no player can override: fleeing ships run
    /// downwind, away from the enemy.
    fn flee_and_hold_orders(&mut self) {
        for ship in &mut self.ships {
            if ship.fleeing && ship.is_afloat() {
                ship.order = ShipOrder::Disengage;
            }
        }
    }

    // ----- movement ---------------------------------------------------------

    /// Heading and top speed a ship wants this step.
    fn steering(&self, ship: &Ship) -> Option<(f64, f64)> {
        let rules = self.rules();
        let target_point = |id: u32| {
            self.ships
                .get(id as usize)
                .filter(|t| t.is_afloat())
                .map(|t| (t.x, t.z, t))
        };
        let (tx, tz, stop) = match ship.order {
            ShipOrder::Hold => return None,
            ShipOrder::MoveTo { x, z } => (x, z, 12.0),
            ShipOrder::Board { target } => {
                let (x, z, t) = target_point(target)?;
                (x, z, ship.radius() + t.radius() + rules.grapple_gap_m * 0.3)
            }
            ShipOrder::Ram { target } => {
                let (x, z, _) = target_point(target)?;
                (x, z, 0.0)
            }
            ShipOrder::Shoot { target } => {
                let (x, z, _) = target_point(target)?;
                let range = ship
                    .crew
                    .iter()
                    .filter(|c| c.shoots())
                    .map(|c| c.range)
                    .fold(0.0, f64::max);
                (x, z, (range * 0.6).max(40.0))
            }
            ShipOrder::Disengage => {
                // Away from the enemy's centre, leaning downwind.
                let (ex, ez) = self.enemy_centre(ship.side);
                let (mut dx, mut dz) = (ship.x - ex, ship.z - ez);
                let len = (dx * dx + dz * dz).sqrt().max(1.0);
                let (wx, wz) = self.wind.vector();
                dx = dx / len + wx * 0.8;
                dz = dz / len + wz * 0.8;
                (ship.x + dx * 500.0, ship.z + dz * 500.0, 0.0)
            }
        };
        let (dx, dz) = (tx - ship.x, tz - ship.z);
        let distance = (dx * dx + dz * dz).sqrt();
        if distance <= stop {
            return None;
        }
        let desired = dz.atan2(dx);
        let slow = ((distance - stop) / 40.0).clamp(0.25, 1.0);
        let ram = if matches!(ship.order, ShipOrder::Ram { .. }) {
            1.0 + rules.ram_speed
        } else {
            1.0
        };
        Some((desired, slow * ram))
    }

    fn enemy_centre(&self, side: SideId) -> (f64, f64) {
        let enemies: Vec<&Ship> = self
            .ships
            .iter()
            .filter(|s| s.side != side && s.is_afloat())
            .collect();
        if enemies.is_empty() {
            return (0.0, 0.0);
        }
        let n = enemies.len() as f64;
        (
            enemies.iter().map(|s| s.x).sum::<f64>() / n,
            enemies.iter().map(|s| s.z).sum::<f64>() / n,
        )
    }

    fn top_speed(&self, ship: &Ship, heading: f64) -> f64 {
        let rules = self.rules();
        let sail = ship.class.sail_speed * self.wind.sail_factor(heading, rules);
        let rowers = if ship.class.rowers > 0 {
            (ship.stamina / rules.rower_stamina_s)
                .clamp(0.2, 1.0)
                .sqrt()
        } else {
            1.0
        };
        let oars = ship.class.oar_speed * rowers;
        match ship.class.propulsion {
            Propulsion::Sail => sail,
            Propulsion::Oars => oars.max(sail * 0.5),
            Propulsion::Mixed => sail.max(oars * 0.8),
        }
    }

    fn movement(&mut self, dt: f64) {
        let rules = self.rules().clone();
        let (wx, wz) = self.wind.vector();
        let count = self.ships.len();
        for i in 0..count {
            let ship = &self.ships[i];
            if !ship.is_afloat() && !matches!(ship.status, ShipStatus::Abandoned) {
                continue;
            }
            // Chained and grappled ships drift with the wind (chained:
            // anchored, they hardly move).
            if ship.chain.is_some() || !ship.grappled.is_empty() || !ship.is_afloat() {
                let drift = if ship.chain.is_some() {
                    0.0
                } else {
                    rules.drift_speed * 0.3 * self.wind.strength
                };
                let s = &mut self.ships[i];
                s.speed = 0.0;
                s.x += wx * drift * dt;
                s.z += wz * drift * dt;
                continue;
            }
            let steering = self.steering(ship);
            let (heading, want) = match steering {
                Some((desired, factor)) => {
                    let heading = if ship.class.propulsion == Propulsion::Sail {
                        self.wind.best_heading(desired, &rules)
                    } else {
                        desired
                    };
                    (heading, factor)
                }
                None => (ship.heading, 0.0),
            };
            let turn = ship.class.turn_rate.to_radians() * dt;
            let delta = wrap(heading - ship.heading).clamp(-turn, turn);
            let new_heading = wrap(ship.heading + delta);
            let top = self.top_speed(ship, new_heading) * want;
            let rowing = ship.class.rowers > 0 && want > 0.0;
            let s = &mut self.ships[i];
            s.heading = new_heading;
            let accel = if top > s.speed { 0.25 } else { 0.6 };
            s.speed += (top - s.speed).clamp(-accel * dt, accel * dt);
            let (fx, fz) = s.forward();
            let drift = if s.speed < 0.2 {
                rules.drift_speed * self.wind.strength
            } else {
                0.0
            };
            s.x += (fx * s.speed + wx * drift) * dt;
            s.z += (fz * s.speed + wz * drift) * dt;
            if rowing {
                s.stamina = (s.stamina - dt).max(0.0);
            } else {
                s.stamina = (s.stamina + dt * 0.5).min(rules.rower_stamina_s);
            }
        }
        self.rams();
        self.spacing();
        self.escapes();
    }

    /// Pushes apart ships whose circles overlap (grappled and chained
    /// neighbours excepted); only free ships move.
    fn spacing(&mut self) {
        let count = self.ships.len();
        for i in 0..count {
            for j in (i + 1)..count {
                let (a, b) = (&self.ships[i], &self.ships[j]);
                if matches!(a.status, ShipStatus::Sunk | ShipStatus::Escaped)
                    || matches!(b.status, ShipStatus::Sunk | ShipStatus::Escaped)
                {
                    continue;
                }
                if a.grappled.contains(&b.id)
                    || (a.chain.is_some() && a.chain == b.chain && a.side == b.side)
                {
                    continue;
                }
                let min = (a.radius() + b.radius()) * 0.8;
                let (dx, dz) = (b.x - a.x, b.z - a.z);
                let d = (dx * dx + dz * dz).sqrt();
                if d >= min {
                    continue;
                }
                let (nx, nz) = if d < 1e-6 {
                    (1.0, 0.0)
                } else {
                    (dx / d, dz / d)
                };
                let push = min - d;
                let free_a = a.chain.is_none() && a.grappled.is_empty();
                let free_b = b.chain.is_none() && b.grappled.is_empty();
                let (wa, wb) = match (free_a, free_b) {
                    (true, true) => (0.5, 0.5),
                    (true, false) => (1.0, 0.0),
                    (false, true) => (0.0, 1.0),
                    (false, false) => (0.0, 0.0),
                };
                self.ships[i].x -= nx * push * wa;
                self.ships[i].z -= nz * push * wa;
                self.ships[j].x += nx * push * wb;
                self.ships[j].z += nz * push * wb;
            }
        }
    }

    fn escapes(&mut self) {
        let radius = self.rules().escape_radius_m;
        let mut escaped = Vec::new();
        for ship in &mut self.ships {
            if ship.is_afloat()
                && matches!(ship.order, ShipOrder::Disengage)
                && ship.grappled.is_empty()
                && (ship.x * ship.x + ship.z * ship.z).sqrt() > radius
            {
                ship.status = ShipStatus::Escaped;
                ship.speed = 0.0;
                escaped.push(ship.id);
            }
        }
        for id in escaped {
            self.push_event(NavalEventKind::Escaped { ship: id });
        }
    }

    /// Galleys driving their spur home.
    fn rams(&mut self) {
        let rules = self.rules().clone();
        let count = self.ships.len();
        for i in 0..count {
            let ShipOrder::Ram { target } = self.ships[i].order else {
                continue;
            };
            let t = target as usize;
            if t >= count || !self.ships[t].is_afloat() || !self.ships[i].is_afloat() {
                continue;
            }
            let (a, b) = (&self.ships[i], &self.ships[t]);
            if a.gap_to(b) > 1.5 || a.speed < 1.5 {
                continue;
            }
            let high = if b.class.freeboard_m >= 2.5 {
                rules.ram_high_factor
            } else {
                1.0
            };
            let damage = a.class.ram * a.speed * high;
            let id = a.id;
            self.ships[t].hull -= damage;
            self.ships[t].morale = (self.ships[t].morale - 8.0).max(0.0);
            self.ships[i].hull -= damage * 0.15;
            self.ships[i].speed = 0.0;
            // After the blow, the galley's men board.
            self.ships[i].order = ShipOrder::Board { target };
            self.push_event(NavalEventKind::Ram {
                ship: id,
                other: target,
                damage,
            });
        }
    }

    // ----- grapples -----------------------------------------------------------

    fn grapples(&mut self, dt: f64) {
        let rules = self.rules().clone();
        let count = self.ships.len();
        for i in 0..count {
            let ShipOrder::Board { target } = self.ships[i].order else {
                continue;
            };
            let t = target as usize;
            if t >= count || !self.ships[i].is_afloat() || !self.ships[t].is_afloat() {
                continue;
            }
            if self.ships[i].grappled.contains(&target) {
                continue;
            }
            if self.ships[i].gap_to(&self.ships[t]) > rules.grapple_gap_m {
                continue;
            }
            let sailors = if self.ships[i].sailors_initial > 0.0 {
                (self.ships[i].sailors / self.ships[i].sailors_initial).clamp(0.3, 1.0)
            } else {
                0.5
            };
            if !self.ships[i].fireship && self.rng.unit() >= rules.grapple_chance * sailors * dt {
                continue;
            }
            self.grapple(i, t);
        }
        // Ships trying to get away cut the lines.
        for i in 0..count {
            if !matches!(self.ships[i].order, ShipOrder::Disengage)
                || self.ships[i].grappled.is_empty()
            {
                continue;
            }
            if !self.ships[i].is_afloat() {
                continue;
            }
            let sailors = if self.ships[i].sailors_initial > 0.0 {
                (self.ships[i].sailors / self.ships[i].sailors_initial).clamp(0.0, 1.0)
            } else {
                0.0
            };
            if self.rng.unit() < rules.cut_chance * sailors * dt {
                let others = std::mem::take(&mut self.ships[i].grappled);
                let id = self.ships[i].id;
                for other in others {
                    self.ships[other as usize].grappled.retain(|&g| g != id);
                    self.push_event(NavalEventKind::Cut { ship: id, other });
                }
            }
        }
    }

    /// Lashes ship `i` alongside ship `t` (the boarder `i` moves).
    fn grapple(&mut self, i: usize, t: usize) {
        let (ai, at) = (self.ships[i].id, self.ships[t].id);
        let target = &self.ships[t];
        let (fx, fz) = target.forward();
        let (px, pz) = (-fz, fx);
        let side_gap = (target.class.beam_m + self.ships[i].class.beam_m) * 0.5 + 0.8;
        let along = (target.class.length_m + self.ships[i].class.length_m) * 0.5 + 1.0;
        let here = (self.ships[i].x, self.ships[i].z);
        let mut spots = [
            (
                target.x + px * side_gap,
                target.z + pz * side_gap,
                target.heading,
            ),
            (
                target.x - px * side_gap,
                target.z - pz * side_gap,
                target.heading,
            ),
            (target.x + fx * along, target.z + fz * along, target.heading),
            (target.x - fx * along, target.z - fz * along, target.heading),
        ];
        spots.sort_by(|a, b| {
            let da = (a.0 - here.0).powi(2) + (a.1 - here.1).powi(2);
            let db = (b.0 - here.0).powi(2) + (b.1 - here.1).powi(2);
            da.total_cmp(&db)
        });
        let occupied = |spot: &(f64, f64, f64)| {
            self.ships.iter().any(|s| {
                s.id != ai
                    && s.id != at
                    && !matches!(s.status, ShipStatus::Sunk | ShipStatus::Escaped)
                    && ((s.x - spot.0).powi(2) + (s.z - spot.1).powi(2)).sqrt() < side_gap * 0.8
            })
        };
        let spot = spots.iter().find(|s| !occupied(s)).copied();
        let fireship = self.ships[i].fireship;
        if let Some((x, z, heading)) = spot {
            let s = &mut self.ships[i];
            s.x = x;
            s.z = z;
            s.heading = heading;
        }
        self.ships[i].speed = 0.0;
        self.ships[i].grappled.push(at);
        self.ships[t].grappled.push(ai);
        self.push_event(NavalEventKind::Grapple {
            ship: ai,
            other: at,
        });
        if fireship {
            self.fireship_strike(i, t);
        } else {
            self.push_event(NavalEventKind::Board {
                ship: ai,
                other: at,
            });
        }
    }

    /// The fireship's crew lights it and rows off.
    fn fireship_strike(&mut self, i: usize, t: usize) {
        let rules = self.rules().clone();
        let (ai, at) = (self.ships[i].id, self.ships[t].id);
        let resist = self.ships[t].class.fire_resistance;
        let before = self.ships[t].fire;
        self.ships[t].fire = (before + rules.fireship_fire * (1.0 - resist)).min(1.0);
        self.ships[t].morale = (self.ships[t].morale - 15.0).max(0.0);
        let s = &mut self.ships[i];
        s.fire = 1.0;
        s.status = ShipStatus::Abandoned;
        // The sailors got away in the boat.
        for (k, crew) in s.crew.iter_mut().enumerate() {
            s.swimmers[k] += crew.men;
            crew.men = 0.0;
        }
        s.sailors = 0.0;
        self.push_event(NavalEventKind::Fireship {
            ship: ai,
            other: at,
        });
        if before <= 0.0 {
            self.push_event(NavalEventKind::Ignite { ship: at });
        }
    }

    // ----- shooting -----------------------------------------------------------

    /// Picks the shooting target of ship `i`: its ordered target in range,
    /// else the grappled enemy, else the nearest enemy in range.
    fn shooting_target(&self, i: usize) -> Option<usize> {
        let ship = &self.ships[i];
        let range = ship
            .crew
            .iter()
            .filter(|c| c.shoots())
            .map(|c| c.range * (1.0 + self.rules().wind_gauge))
            .fold(0.0, f64::max);
        if range <= 0.0 {
            return None;
        }
        let valid = |id: usize| {
            self.ships.get(id).is_some_and(|t| {
                t.side != ship.side && t.is_afloat() && ship.distance_to(t) <= range
            })
        };
        if let Some(target) = ship.order.target() {
            if valid(target as usize) {
                return Some(target as usize);
            }
        }
        if let Some(&g) = ship.grappled.iter().find(|&&g| valid(g as usize)) {
            return Some(g as usize);
        }
        self.ships
            .iter()
            .filter(|t| valid(t.id as usize))
            // Do not shoot into a melee where our own men fight.
            .filter(|t| {
                !t.grappled
                    .iter()
                    .any(|&g| self.ships[g as usize].side == ship.side && g != ship.id)
            })
            .min_by(|a, b| ship.distance_to(a).total_cmp(&ship.distance_to(b)))
            .map(|t| t.id as usize)
    }

    fn shooting(&mut self, dt: f64) {
        let rules = self.rules().clone();
        let rain = self.setup.rain;
        let count = self.ships.len();
        for i in 0..count {
            if !self.ships[i].is_afloat() || self.ships[i].fireship {
                continue;
            }
            // Reload.
            let mut ready = Vec::new();
            for (g, crew) in self.ships[i].crew.iter_mut().enumerate() {
                if crew.range <= 0.0 {
                    continue;
                }
                crew.reload -= dt;
                if crew.reload <= 0.0 && crew.shoots() {
                    ready.push(g);
                }
            }
            if ready.is_empty() {
                continue;
            }
            let Some(t) = self.shooting_target(i) else {
                continue;
            };
            let distance = self.ships[i].distance_to(&self.ships[t]);
            let alignment = self.wind.alignment(
                (self.ships[i].x, self.ships[i].z),
                (self.ships[t].x, self.ships[t].z),
            );
            // Grappled enemies: no fire arrows into a ship we are lashed to.
            let lashed = self.ships[i].grappled.contains(&(t as u32));
            for g in ready {
                let mut volley = combat::volley(
                    &self.ships[i],
                    g,
                    &self.ships[t],
                    distance,
                    self.wind,
                    alignment,
                    rain,
                    &rules,
                );
                if volley.missiles <= 0.0 {
                    continue;
                }
                if lashed {
                    volley.fire = 0.0;
                }
                let jitter = self.rng.range(0.8, 1.2);
                let before = self.ships[t].fighting_men();
                let killed = self.ships[t].take_losses(volley.hits * jitter, rules.armor_vs_ranged);
                if before > 0.0 {
                    let percent = killed / before * 100.0;
                    let m = &mut self.ships[t].morale;
                    *m = (*m - percent * rules.morale_per_loss_percent * 0.6).max(0.0);
                }
                let was_burning = self.ships[t].fire > 0.0;
                self.ships[t].fire = (self.ships[t].fire + volley.fire).min(1.0);
                if !was_burning && self.ships[t].fire > 0.0 {
                    self.push_event(NavalEventKind::Ignite { ship: t as u32 });
                }
                let reload = combat::reload_seconds(&self.ships[i].crew[g], &rules);
                let crew = &mut self.ships[i].crew[g];
                crew.ammo -= 1.0;
                crew.reload = reload * self.rng.range(0.9, 1.1);
                self.ships[i].last_target = Some(t as u32);
                let shot = ShotEvent {
                    time: self.elapsed,
                    shooter: self.ships[i].id,
                    target: Some(t as u32),
                    from: (self.ships[i].x, self.ships[i].z),
                    aim: (self.ships[t].x, self.ships[t].z),
                    missiles: volley.missiles.round() as u32,
                    kills: killed,
                    kind: volley.kind,
                    incendiary: volley.fire > 0.0,
                    cover: if self.ships[t].class.bulwark > 0.0 {
                        ShotCover::Pavise
                    } else {
                        ShotCover::None
                    },
                };
                if self.shots.len() >= MAX_PENDING_SHOTS {
                    self.shots.remove(0);
                }
                self.shots.push(shot);
            }
        }
    }

    // ----- melee --------------------------------------------------------------

    fn chain_support(&self, i: usize) -> f64 {
        let ship = &self.ships[i];
        let Some(chain) = ship.chain else {
            return 0.0;
        };
        let free = self
            .ships
            .iter()
            .filter(|s| {
                s.id != ship.id
                    && s.side == ship.side
                    && s.chain == Some(chain)
                    && s.is_afloat()
                    && s.grappled.is_empty()
                    && s.distance_to(ship) < ship.class.beam_m * 2.5 + 4.0
            })
            .count();
        free as f64 * self.rules().chain_support
    }

    fn melee(&mut self, dt: f64) {
        let rules = self.rules().clone();
        let count = self.ships.len();
        let mut pairs = BTreeSet::new();
        for ship in &self.ships {
            if !ship.is_afloat() {
                continue;
            }
            for &other in &ship.grappled {
                let o = &self.ships[other as usize];
                if o.side != ship.side && o.is_afloat() {
                    pairs.insert((ship.id.min(other), ship.id.max(other)));
                }
            }
        }
        let enemies_of = |ships: &[Ship], i: usize| {
            ships[i]
                .grappled
                .iter()
                .filter(|&&g| {
                    ships[g as usize].side != ships[i].side && ships[g as usize].is_afloat()
                })
                .count()
                .max(1) as f64
        };
        let mut losses = vec![0.0; count];
        let mut in_melee = vec![false; count];
        for &(a, b) in &pairs {
            let (a, b) = (a as usize, b as usize);
            let share_a = 1.0 / enemies_of(&self.ships, a);
            let share_b = 1.0 / enemies_of(&self.ships, b);
            let (on_b, on_a) = combat::melee_exchange(
                &self.ships[a],
                &self.ships[b],
                share_a,
                share_b,
                self.chain_support(a),
                self.chain_support(b),
                dt,
                &rules,
            );
            losses[b] += on_b;
            losses[a] += on_a;
            in_melee[a] = true;
            in_melee[b] = true;
        }
        for i in 0..count {
            if !in_melee[i] {
                continue;
            }
            let before = self.ships[i].fighting_men();
            let killed = self.ships[i].take_losses(losses[i], rules.armor_vs_melee);
            let ship = &mut self.ships[i];
            ship.melee_time += dt;
            if before > 0.0 {
                let percent = killed / before * 100.0;
                ship.morale = (ship.morale - percent * rules.morale_per_loss_percent).max(0.0);
            }
        }
    }

    // ----- fire ---------------------------------------------------------------

    fn fires(&mut self, dt: f64) {
        let rules = self.rules().clone();
        let count = self.ships.len();
        // Spread first (from the fires of the previous step).
        let mut spread = vec![0.0; count];
        for i in 0..count {
            let s = &self.ships[i];
            if s.fire < 0.2 || matches!(s.status, ShipStatus::Sunk | ShipStatus::Escaped) {
                continue;
            }
            for (j, slot) in spread.iter_mut().enumerate() {
                if i == j {
                    continue;
                }
                let o = &self.ships[j];
                if matches!(o.status, ShipStatus::Sunk | ShipStatus::Escaped) {
                    continue;
                }
                let touching = s.grappled.contains(&o.id) || s.gap_to(o) < 2.0;
                if touching {
                    *slot += s.fire * rules.fire_spread * (1.0 - o.class.fire_resistance) * dt;
                }
            }
        }
        for (j, add) in spread.into_iter().enumerate() {
            if add > 0.0 {
                let was = self.ships[j].fire;
                self.ships[j].fire = (was + add).min(1.0);
                if was <= 0.0 {
                    self.push_event(NavalEventKind::Ignite { ship: j as u32 });
                }
            }
        }
        let wind = self.wind;
        let mut abandoned = Vec::new();
        for ship in &mut self.ships {
            if matches!(
                ship.status,
                ShipStatus::Sunk | ShipStatus::Escaped | ShipStatus::Sinking { .. }
            ) {
                continue;
            }
            combat::burn(ship, wind, dt, &rules);
            if ship.is_afloat() && ship.fire >= rules.fire_abandon {
                abandoned.push(ship.id);
            }
        }
        for id in abandoned {
            self.abandon(id as usize);
        }
    }

    /// The crew leaps overboard (a burning ship): some drown, the rest swim
    /// to the nearest ship of their side.
    fn abandon(&mut self, i: usize) {
        let rules = self.rules().clone();
        let s = &mut self.ships[i];
        for (k, crew) in s.crew.iter_mut().enumerate() {
            let drown = (rules.drown_base + crew.armor / 100.0 * rules.drown_armor).min(1.0);
            s.drowned[k] += crew.men * drown;
            s.swimmers[k] += crew.men * (1.0 - drown);
            crew.men = 0.0;
        }
        s.sailors = 0.0;
        s.status = ShipStatus::Abandoned;
        s.order = ShipOrder::Hold;
        let id = s.id;
        self.release(i);
        self.push_event(NavalEventKind::Abandon { ship: id });
    }

    /// Removes every grapple of ship `i`.
    fn release(&mut self, i: usize) {
        let id = self.ships[i].id;
        let others = std::mem::take(&mut self.ships[i].grappled);
        for other in others {
            self.ships[other as usize].grappled.retain(|&g| g != id);
        }
    }

    // ----- sinking, surrender -------------------------------------------------

    fn sinking(&mut self) {
        let rules = self.rules().clone();
        let count = self.ships.len();
        for i in 0..count {
            match self.ships[i].status {
                ShipStatus::Sinking { since } => {
                    if self.elapsed - since >= rules.sink_seconds {
                        self.ships[i].status = ShipStatus::Sunk;
                        self.push_event(NavalEventKind::Sunk { ship: i as u32 });
                    }
                }
                ShipStatus::Sunk | ShipStatus::Escaped => {}
                _ => {
                    if self.ships[i].hull <= 0.0 {
                        self.ships[i].hull = 0.0;
                        let s = &mut self.ships[i];
                        for (k, crew) in s.crew.iter_mut().enumerate() {
                            let drown = (rules.drown_base + crew.armor / 100.0 * rules.drown_armor)
                                .min(1.0);
                            s.drowned[k] += crew.men * drown;
                            s.swimmers[k] += crew.men * (1.0 - drown);
                            crew.men = 0.0;
                        }
                        s.sailors = 0.0;
                        s.status = ShipStatus::Sinking {
                            since: self.elapsed,
                        };
                        self.release(i);
                        self.shake(i, 10.0);
                        self.push_event(NavalEventKind::Sinking { ship: i as u32 });
                    }
                }
            }
        }
    }

    /// The loss of a ship shakes its fleet (the flagship much more).
    fn shake(&mut self, i: usize, amount: f64) {
        let side = self.ships[i].side;
        let flagship = self.ships[i].flagship;
        let amount = if flagship { amount * 2.5 } else { amount };
        for ship in &mut self.ships {
            if ship.side == side && ship.is_afloat() {
                ship.morale = (ship.morale - amount).max(0.0);
            }
        }
    }

    fn surrender(&mut self) {
        let rules = self.rules().clone();
        let count = self.ships.len();
        for i in 0..count {
            if !self.ships[i].is_afloat() || self.ships[i].fireship {
                continue;
            }
            let ship = &self.ships[i];
            let crew_share = ship.fighting_men() / ship.fighting_initial().max(1.0);
            let broken =
                ship.morale < rules.surrender_morale || crew_share < rules.surrender_crew_share;
            if !broken {
                continue;
            }
            // Grappled to an enemy: the ship is taken.
            let captor = ship
                .grappled
                .iter()
                .map(|&g| &self.ships[g as usize])
                .filter(|o| o.side != ship.side && o.is_afloat())
                .max_by(|a, b| a.melee_power().total_cmp(&b.melee_power()))
                .map(|o| o.side);
            if let Some(by) = captor {
                let s = &mut self.ships[i];
                for (k, crew) in s.crew.iter_mut().enumerate() {
                    s.prisoners[k] += crew.men;
                    crew.men = 0.0;
                }
                s.sailors = 0.0;
                s.status = ShipStatus::Captured { by };
                s.fleeing = false;
                s.order = ShipOrder::Hold;
                self.release(i);
                self.shake(i, 8.0);
                self.push_event(NavalEventKind::Capture { ship: i as u32, by });
            } else if !self.ships[i].fleeing && self.ships[i].chain.is_none() {
                self.ships[i].fleeing = true;
                self.ships[i].order = ShipOrder::Disengage;
                self.shake(i, 4.0);
                self.push_event(NavalEventKind::Flee { ship: i as u32 });
            } else if self.ships[i].chain.is_some() && crew_share < rules.surrender_crew_share * 0.5
            {
                // Chained and emptied: the nearest enemy takes it.
                let by = self.ships[i].side.other();
                let s = &mut self.ships[i];
                for (k, crew) in s.crew.iter_mut().enumerate() {
                    s.prisoners[k] += crew.men;
                    crew.men = 0.0;
                }
                s.sailors = 0.0;
                s.status = ShipStatus::Captured { by };
                self.release(i);
                self.shake(i, 8.0);
                self.push_event(NavalEventKind::Capture { ship: i as u32, by });
            }
        }
    }

    fn check_end(&mut self) {
        let active = |side: SideId| {
            self.ships
                .iter()
                .any(|s| s.side == side && s.is_active() && !s.fireship && s.fighting_men() >= 1.0)
        };
        let (a, d) = (active(SideId::Attacker), active(SideId::Defender));
        let timeout = self.elapsed >= self.rules().max_duration_s;
        if a && d {
            if timeout {
                self.finished = true;
                let (sa, sd) = (
                    self.strength(SideId::Attacker),
                    self.strength(SideId::Defender),
                );
                let (ia, id) = (
                    self.initial_men(SideId::Attacker),
                    self.initial_men(SideId::Defender),
                );
                let (ra, rd) = (sa / ia.max(1.0), sd / id.max(1.0));
                self.winner = if ra > rd * 1.25 {
                    Some(SideId::Attacker)
                } else if rd > ra * 1.25 {
                    Some(SideId::Defender)
                } else {
                    None
                };
            }
            return;
        }
        self.winner = match (a, d) {
            (true, false) => Some(SideId::Attacker),
            (false, true) => Some(SideId::Defender),
            _ => None,
        };
        // The battle is decided; let the runaways get clear (or be caught)
        // for at most `ESCAPE_GRACE` seconds.
        let decided = *self.decided_at.get_or_insert(self.elapsed);
        let fleeing = self.ships.iter().any(|s| s.is_afloat() && s.fleeing);
        if !fleeing || timeout || self.elapsed - decided >= ESCAPE_GRACE {
            self.finished = true;
        }
    }

    fn initial_men(&self, side: SideId) -> f64 {
        self.ships
            .iter()
            .filter(|s| s.side == side)
            .map(Ship::fighting_initial)
            .sum()
    }

    // ----- outcome ------------------------------------------------------------

    /// Result so far (final once [`NavalSim::is_finished`]).
    pub fn outcome(&self) -> NavalOutcome {
        outcome_of(&self.setup, &self.ships, self.winner, self.elapsed, false)
    }
}

/// Builds the result of a battle from the ships' final state. Swimmers are
/// picked up by their side if it holds the sea, taken or drowned otherwise.
/// Afloat ships of the loser that did not get away and abandoned hulks go
/// to the winner as prizes.
pub(crate) fn outcome_of(
    setup: &NavalSetup,
    ships: &[Ship],
    winner: Option<SideId>,
    duration: f64,
    auto: bool,
) -> NavalOutcome {
    let side_result = |side: SideId| {
        let fleet = setup.side(side);
        let mut result = NavalSideResult {
            unit_losses: vec![0; fleet.units.len()],
            men_start: fleet.men(),
            ..Default::default()
        };
        let mut losses = vec![0.0; fleet.units.len()];
        let mut drowned = 0.0;
        let mut prisoners = 0.0;
        for ship in ships.iter().filter(|s| s.side == side) {
            let lost_side = winner.is_some_and(|w| w != side);
            let fate = match ship.status {
                ShipStatus::Sunk | ShipStatus::Sinking { .. } => ShipFate::Sunk,
                ShipStatus::Captured { .. } => ShipFate::Captured,
                ShipStatus::Escaped => ShipFate::Escaped,
                ShipStatus::Abandoned => {
                    if ship.fire > 0.3 || ship.hull <= 0.0 || winner.is_none() {
                        ShipFate::Sunk
                    } else if lost_side {
                        ShipFate::Captured
                    } else {
                        ShipFate::Kept
                    }
                }
                ShipStatus::Afloat => {
                    if lost_side && !ship.fleeing {
                        ShipFate::Captured
                    } else if ship.fleeing {
                        ShipFate::Escaped
                    } else {
                        ShipFate::Kept
                    }
                }
            };
            for (k, crew) in ship.crew.iter().enumerate() {
                let mut lost = crew.initial - crew.men - ship.swimmers[k] - ship.prisoners[k];
                lost = lost.max(0.0);
                // In the water: saved by the side that holds the sea.
                let rescued = winner == Some(side) || winner.is_none();
                if !rescued {
                    lost += ship.swimmers[k];
                    prisoners += ship.swimmers[k] * 0.5;
                    drowned += ship.swimmers[k] * 0.5;
                }
                lost += ship.prisoners[k];
                prisoners += ship.prisoners[k];
                drowned += ship.drowned[k];
                // Men still aboard a ship taken at the end.
                if fate == ShipFate::Captured && ship.is_afloat() {
                    lost += crew.men;
                    prisoners += crew.men;
                }
                if let Some(slot) = losses.get_mut(crew.unit) {
                    *slot += lost;
                }
            }
            result.ships.push(ShipResult {
                index: ship.index,
                name: ship.name.clone(),
                class: ship.class.id.to_string(),
                fate,
            });
        }
        for (slot, lost) in result.unit_losses.iter_mut().zip(&losses) {
            *slot = lost.round() as u32;
        }
        for (slot, unit) in result.unit_losses.iter_mut().zip(&fleet.units) {
            *slot = (*slot).min(unit.soldiers);
        }
        result.men_lost = result.unit_losses.iter().sum();
        result.drowned = drowned.round() as u32;
        result.prisoners = prisoners.round() as u32;
        result
    };
    let mut attacker = side_result(SideId::Attacker);
    let mut defender = side_result(SideId::Defender);
    // Prizes: captured ships go to the other side.
    for ship in &defender.ships {
        if ship.fate == ShipFate::Captured {
            attacker.prizes.push(ship.class.clone());
        }
    }
    for ship in &attacker.ships {
        if ship.fate == ShipFate::Captured {
            defender.prizes.push(ship.class.clone());
        }
    }
    NavalOutcome {
        winner,
        duration,
        attacker,
        defender,
        auto,
    }
}

/// Wind of the battle: from the setup, else the weather gauge goes to the
/// setup's `gauge` side or to a coin toss, the wind blowing from that fleet
/// towards the other, 20-40° off the axis.
fn draw_wind(setup: &NavalSetup, rng: &mut BattleRng) -> Wind {
    let strength = setup
        .wind_strength
        .unwrap_or_else(|| rng.range(0.3, 0.9))
        .clamp(0.0, 1.0);
    let to = match setup.wind_to_deg {
        Some(deg) => deg.to_radians(),
        None => {
            let gauge = setup.gauge.unwrap_or(if rng.unit() < 0.5 {
                SideId::Attacker
            } else {
                SideId::Defender
            });
            let base = match gauge {
                SideId::Attacker => 0.0,
                SideId::Defender => std::f64::consts::PI,
            };
            let off =
                rng.range(20.0, 40.0).to_radians() * if rng.unit() < 0.5 { 1.0 } else { -1.0 };
            base + off
        }
    };
    Wind {
        to: wrap(to),
        strength,
    }
}

/// Starting place of ship `index` of `count`: lines abreast, 8 ships a
/// line; chained ships lashed side by side.
pub fn formation_slot(side: SideId, index: usize, count: usize, chained: bool) -> (f64, f64) {
    let per_line = if chained { 10 } else { 8 };
    let line = index / per_line;
    let in_line = index % per_line;
    let in_this_line = (count - line * per_line).min(per_line);
    let spacing = if chained { 11.0 } else { 45.0 };
    let z = (in_line as f64 - (in_this_line as f64 - 1.0) * 0.5) * spacing;
    let depth = OPENING_GAP * 0.5 + line as f64 * 70.0;
    let x = match side {
        SideId::Attacker => -depth,
        SideId::Defender => depth,
    };
    (x, z)
}
