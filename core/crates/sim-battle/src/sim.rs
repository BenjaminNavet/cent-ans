//! The battle simulation proper: deployment, fixed ticks, commands, rules.

// Index loops read better than iterators here: each step mutates one unit
// while reading the others.
#![allow(clippy::needless_range_loop)]

use data_model::{Ability, UnitCategory};

use crate::ai;
use crate::command::{Command, CommandError};
use crate::field::{Battlefield, Weather, ATTACKER_LINE_Z, DEFENDER_LINE_Z};
use crate::outcome::{BattleEvent, BattleOutcome, SideResult};
use crate::rng::BattleRng;
use crate::setup::{BattleSetup, SideId};
use crate::unit::{Formation, Unit, UnitState};

/// Fixed simulation step, in seconds.
pub const DT: f64 = 0.1;
/// A battle lasts at most one simulated hour; the defender then wins.
pub const MAX_DURATION: f64 = 3600.0;
/// Upper bound of fixed steps run by a single [`BattleSim::tick`] call.
const MAX_STEPS_PER_CALL: u32 = 600;

/// Gap below which two enemy regiments are in contact (metres).
pub const CONTACT_GAP: f64 = 2.5;
/// Radius of the general's morale aura (metres).
pub const GENERAL_AURA: f64 = 150.0;
/// Morale below which a regiment routs.
pub const ROUT_MORALE: f64 = 20.0;
/// Morale above which a routing regiment may rally.
pub const RALLY_MORALE: f64 = 40.0;
/// Enemies closer than this prevent rallying (metres).
pub const RALLY_SAFE_DISTANCE: f64 = 150.0;
/// Duration of the charge impact bonus (seconds).
pub const CHARGE_IMPACT: f64 = 4.0;
/// Seconds an archer unit must stand still before its stakes are planted.
pub const STAKES_DELAY: f64 = 15.0;

const MELEE_RATE: f64 = 0.05;
const RANGED_RATE: f64 = 0.4;
const LOSS_MORALE_FACTOR: f64 = 120.0;

/// Why a setup cannot start a battle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupError {
    EmptySide(SideId),
}

impl std::fmt::Display for SetupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SetupError::EmptySide(side) => write!(f, "le camp {} n'a aucune unité", side.key()),
        }
    }
}

impl std::error::Error for SetupError {}

/// A battle in progress.
#[derive(Debug, Clone)]
pub struct BattleSim {
    setup: BattleSetup,
    field: Battlefield,
    weather: Weather,
    units: Vec<Unit>,
    rng: BattleRng,
    elapsed: f64,
    ticks: u64,
    accumulator: f64,
    ai_enabled: [bool; 2],
    end_conditions: bool,
    finished: bool,
    winner: Option<SideId>,
    general_alive: [bool; 2],
    general_killed: [bool; 2],
    general_captured: [bool; 2],
    events: Vec<BattleEvent>,
    events_read: usize,
    charge_announced: Vec<bool>,
}

fn angle_to(dx: f64, dz: f64) -> f64 {
    dx.atan2(dz)
}

fn wrap_angle(a: f64) -> f64 {
    let tau = std::f64::consts::TAU;
    let mut a = a % tau;
    if a > std::f64::consts::PI {
        a -= tau;
    } else if a < -std::f64::consts::PI {
        a += tau;
    }
    a
}

/// Rotates `from` towards `to` by at most `max_step` radians.
fn turn_towards(from: f64, to: f64, max_step: f64) -> f64 {
    let diff = wrap_angle(to - from);
    wrap_angle(from + diff.clamp(-max_step, max_step))
}

/// Armour reduction factor (armour 0-100, 130 = invulnerable).
fn armor_factor(armor: f64) -> f64 {
    (1.0 - armor.clamp(0.0, 120.0) / 130.0).max(0.05)
}

/// "de France", "d'Angleterre".
pub fn of_faction(name: &str) -> String {
    let first = name.chars().next().unwrap_or('x').to_lowercase().next();
    if matches!(
        first,
        Some('a' | 'e' | 'i' | 'o' | 'u' | 'é' | 'è' | 'ê' | 'â' | 'î' | 'ô' | 'û')
    ) {
        format!("d'{name}")
    } else {
        format!("de {name}")
    }
}

/// Relative position of an attacker around a defender: 0 front, 1 flank, 2 rear.
fn attack_angle(defender: &Unit, attacker_x: f64, attacker_z: f64) -> u8 {
    if defender.formation == Formation::Square {
        return 0;
    }
    let (dx, dz) = (attacker_x - defender.x, attacker_z - defender.z);
    let len = (dx * dx + dz * dz).sqrt().max(1e-6);
    let (fx, fz) = defender.forward();
    let cos = (dx * fx + dz * fz) / len;
    if cos > 0.5 {
        0
    } else if cos > -0.5 {
        1
    } else {
        2
    }
}

impl BattleSim {
    /// Deploys both armies and draws the weather and the field from `seed`.
    pub fn new(setup: BattleSetup, seed: u64) -> Result<Self, SetupError> {
        for side in SideId::BOTH {
            if setup.side(side).units.iter().all(|u| u.soldiers == 0) {
                return Err(SetupError::EmptySide(side));
            }
        }
        let mut rng = BattleRng::from_seed(seed);
        let weather = Weather::draw(setup.season, &mut rng);
        let field = Battlefield::generate(setup.terrain, setup.river, weather, &mut rng);
        let mut units = Vec::new();
        for side in SideId::BOTH {
            let side_setup = setup.side(side);
            let general = side_setup.general.as_ref();
            for (index, unit_setup) in side_setup.units.iter().enumerate() {
                if unit_setup.soldiers == 0 {
                    continue;
                }
                let mut unit = Unit::from_setup(units.len() as u32, side, index, unit_setup);
                if let Some(general) = general {
                    unit.morale_cap = (unit.morale_cap + general.morale_bonus).clamp(0.0, 100.0);
                    unit.morale = (unit.morale + general.morale_bonus).clamp(0.0, 100.0);
                    unit.is_general = general.unit_index == index;
                }
                units.push(unit);
            }
        }
        let ai_enabled = match setup.player_side {
            Some(SideId::Attacker) => [false, true],
            Some(SideId::Defender) => [true, false],
            None => [true, true],
        };
        let general_alive = [
            units
                .iter()
                .any(|u| u.side == SideId::Attacker && u.is_general),
            units
                .iter()
                .any(|u| u.side == SideId::Defender && u.is_general),
        ];
        let count = units.len();
        let mut sim = BattleSim {
            setup,
            field,
            weather,
            units,
            rng,
            elapsed: 0.0,
            ticks: 0,
            accumulator: 0.0,
            ai_enabled,
            end_conditions: true,
            finished: false,
            winner: None,
            general_alive,
            general_killed: [false; 2],
            general_captured: [false; 2],
            events: Vec::new(),
            events_read: 0,
            charge_announced: vec![false; count],
        };
        sim.deploy();
        let text = match sim.weather {
            Weather::Clear => "Le ciel est dégagé sur le champ de bataille.".to_owned(),
            Weather::Rain => "Il pleut : les cordes des arcs se détendent.".to_owned(),
            Weather::Fog => "Un épais brouillard couvre le champ de bataille.".to_owned(),
            Weather::Snow => "La neige tombe sur le champ de bataille.".to_owned(),
        };
        sim.log(text, None);
        Ok(sim)
    }

    fn deploy(&mut self) {
        for side in SideId::BOTH {
            let (line_z, facing, back) = match side {
                SideId::Attacker => (ATTACKER_LINE_Z, 0.0, -1.0),
                SideId::Defender => (DEFENDER_LINE_Z, std::f64::consts::PI, 1.0),
            };
            let ids: Vec<usize> = (0..self.units.len())
                .filter(|&i| self.units[i].side == side)
                .collect();
            let of = |cat: &dyn Fn(&Unit) -> bool| -> Vec<usize> {
                ids.iter()
                    .copied()
                    .filter(|&i| cat(&self.units[i]))
                    .collect()
            };
            let infantry = of(&|u| u.category == UnitCategory::Infantry);
            let foot_ranged = of(&|u| u.category == UnitCategory::Ranged && !u.mounted);
            let cavalry = of(&|u| {
                u.category == UnitCategory::Cavalry
                    || (u.mounted && u.category == UnitCategory::Ranged)
            });
            let siege = of(&|u| u.category == UnitCategory::Siege);
            for &i in &ids {
                self.units[i].facing = facing;
            }
            let front_row = if infantry.is_empty() {
                &foot_ranged
            } else {
                &infantry
            };
            let front_width = self.place_row(front_row, line_z, back);
            if !infantry.is_empty() {
                self.place_row(&foot_ranged, line_z + back * 45.0, back);
            }
            // Cavalry on the wings, alternating left and right.
            let mut left = 600.0 - front_width * 0.5 - 20.0;
            let mut right = 600.0 + front_width * 0.5 + 20.0;
            for (k, &i) in cavalry.iter().enumerate() {
                let (w, _) = self.units[i].extent();
                let x = if k % 2 == 0 {
                    right += w * 0.5;
                    let x = right;
                    right += w * 0.5 + 12.0;
                    x
                } else {
                    left -= w * 0.5;
                    let x = left;
                    left -= w * 0.5 + 12.0;
                    x
                };
                let unit = &mut self.units[i];
                unit.x = x.clamp(30.0, self.field.width - 30.0);
                unit.z = line_z + back * 15.0;
            }
            self.place_row(&siege, line_z + back * 95.0, back);
        }
    }

    /// Places `row` side by side, centred on x = 600, wrapping into extra rows
    /// behind when wider than the field. Returns the width of the first row.
    fn place_row(&mut self, row: &[usize], z: f64, back: f64) -> f64 {
        let gap = 12.0;
        let max_width = self.field.width - 100.0;
        let mut lines: Vec<Vec<usize>> = vec![Vec::new()];
        let mut width = 0.0;
        for &i in row {
            let (w, _) = self.units[i].extent();
            if width + w > max_width && !lines.last().is_some_and(Vec::is_empty) {
                lines.push(Vec::new());
                width = 0.0;
            }
            width += w + gap;
            lines.last_mut().expect("non-empty").push(i);
        }
        let mut first_width = 0.0;
        for (line_index, line) in lines.iter().enumerate() {
            let total: f64 = line
                .iter()
                .map(|&i| self.units[i].extent().0 + gap)
                .sum::<f64>()
                - gap;
            if line_index == 0 {
                first_width = total.max(0.0);
            }
            let mut x = 600.0 - total * 0.5;
            for &i in line {
                let (w, _) = self.units[i].extent();
                let unit = &mut self.units[i];
                unit.x = x + w * 0.5;
                unit.z = z + back * 30.0 * line_index as f64;
                x += w + gap;
            }
        }
        first_width
    }

    // ----- accessors ------------------------------------------------------

    pub fn setup(&self) -> &BattleSetup {
        &self.setup
    }

    pub fn field(&self) -> &Battlefield {
        &self.field
    }

    pub fn weather(&self) -> Weather {
        self.weather
    }

    pub fn units(&self) -> &[Unit] {
        &self.units
    }

    /// Mutable access for tests and scripted scenarios (placement, morale).
    pub fn units_mut(&mut self) -> &mut [Unit] {
        &mut self.units
    }

    pub fn unit(&self, id: u32) -> Option<&Unit> {
        self.units.get(id as usize)
    }

    pub fn elapsed(&self) -> f64 {
        self.elapsed
    }

    pub fn ticks(&self) -> u64 {
        self.ticks
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    pub fn winner(&self) -> Option<SideId> {
        self.winner
    }

    pub fn general_alive(&self, side: SideId) -> bool {
        self.general_alive[side.index()]
    }

    /// Forces the weather (scripted scenarios and tests).
    pub fn set_weather(&mut self, weather: Weather) {
        self.weather = weather;
    }

    /// Enables or disables the end-of-battle checks (lab scenarios where a
    /// single regiment routs without ending the battle).
    pub fn set_end_conditions(&mut self, enabled: bool) {
        self.end_conditions = enabled;
    }

    /// Enables or disables the battle AI of `side`.
    pub fn set_ai(&mut self, side: SideId, enabled: bool) {
        self.ai_enabled[side.index()] = enabled;
    }

    /// Every journal entry since the start.
    pub fn events(&self) -> &[BattleEvent] {
        &self.events
    }

    /// Journal entries added since the previous call.
    pub fn take_new_events(&mut self) -> Vec<BattleEvent> {
        let new = self.events[self.events_read..].to_vec();
        self.events_read = self.events.len();
        new
    }

    /// Effective shooting range of `unit` (weather, height advantage).
    pub fn effective_range(&self, unit: &Unit, target_x: f64, target_z: f64) -> f64 {
        let height_gain =
            (self.field.height(unit.x, unit.z) - self.field.height(target_x, target_z)).max(0.0);
        f64::from(unit.stats.range) * self.weather.range_factor() * (1.0 + height_gain / 100.0)
    }

    fn log(&mut self, text_fr: String, side: Option<SideId>) {
        self.events.push(BattleEvent {
            time: self.elapsed,
            text_fr,
            side,
        });
    }

    fn unit_label(&self, index: usize) -> String {
        let unit = &self.units[index];
        let faction = &self.setup.side(unit.side).faction_name;
        format!("{} {}", unit.name, of_faction(faction))
    }

    // ----- commands -------------------------------------------------------

    /// Applies a player command. Units of the side the player does not
    /// command are refused when the setup names a player side.
    pub fn issue_command(&mut self, command: Command) -> Result<(), CommandError> {
        self.apply_command(command, self.setup.player_side)
    }

    /// Applies a command on behalf of `side` (`None` = no ownership check).
    pub fn apply_command(
        &mut self,
        command: Command,
        side: Option<SideId>,
    ) -> Result<(), CommandError> {
        if self.finished {
            return Err(CommandError::Finished);
        }
        let ids = command.units();
        if ids.is_empty() {
            return Err(CommandError::NoUnits);
        }
        for &id in ids {
            let unit = self
                .units
                .get(id as usize)
                .ok_or(CommandError::UnknownUnit(id))?;
            if side.is_some_and(|s| s != unit.side) {
                return Err(CommandError::NotYours(id));
            }
            if !unit.present() || unit.state == UnitState::Routing {
                return Err(CommandError::Unavailable(id));
            }
        }
        match command {
            Command::Move {
                units,
                x,
                z,
                run,
                facing,
            } => {
                if !self.field.inside(x, z) {
                    return Err(CommandError::OutsideField);
                }
                let destinations = self.group_destinations(&units, x, z, facing);
                for (id, (dx, dz)) in units.iter().zip(destinations) {
                    let unit = &mut self.units[*id as usize];
                    unit.destination = Some((
                        dx.clamp(5.0, self.field.width - 5.0),
                        dz.clamp(5.0, self.field.depth - 5.0),
                    ));
                    unit.destination_facing = facing;
                    unit.target = None;
                    unit.running = run;
                    unit.withdrawing = false;
                    unit.disengaging = unit.state == UnitState::Melee;
                    if unit.state != UnitState::Melee {
                        unit.state = UnitState::Marching;
                    }
                }
            }
            Command::Attack { units, target, run } => {
                let target_unit = self
                    .units
                    .get(target as usize)
                    .ok_or(CommandError::UnknownTarget(target))?;
                if !target_unit.present() {
                    return Err(CommandError::UnknownTarget(target));
                }
                let target_side = target_unit.side;
                for &id in &units {
                    if self.units[id as usize].side == target_side {
                        return Err(CommandError::FriendlyTarget(target));
                    }
                }
                for &id in &units {
                    let unit = &mut self.units[id as usize];
                    unit.target = Some(target);
                    unit.destination = None;
                    unit.destination_facing = None;
                    unit.running = run;
                    unit.withdrawing = false;
                    unit.disengaging = false;
                }
            }
            Command::Halt { units } => {
                for &id in &units {
                    let unit = &mut self.units[id as usize];
                    unit.target = None;
                    unit.destination = None;
                    unit.destination_facing = None;
                    unit.running = false;
                    unit.disengaging = false;
                    if matches!(unit.state, UnitState::Marching | UnitState::Charging) {
                        unit.state = UnitState::Idle;
                    }
                }
            }
            Command::Formation { units, kind } => {
                for &id in &units {
                    let unit = &self.units[id as usize];
                    let allowed = match kind {
                        Formation::Line | Formation::Column => {
                            unit.category != UnitCategory::Siege || kind == Formation::Line
                        }
                        Formation::Square => {
                            unit.category == UnitCategory::Infantry && !unit.mounted
                        }
                        Formation::Wedge => unit.category == UnitCategory::Cavalry,
                    };
                    if !allowed {
                        return Err(CommandError::InvalidFormation {
                            unit: id,
                            formation: kind,
                        });
                    }
                }
                for &id in &units {
                    self.units[id as usize].formation = kind;
                }
            }
            Command::FireAtWill { units, enabled } => {
                for &id in &units {
                    if !self.units[id as usize].can_shoot() {
                        return Err(CommandError::NoMissile(id));
                    }
                }
                for &id in &units {
                    self.units[id as usize].fire_at_will = enabled;
                }
            }
            Command::Withdraw { units } => {
                for &id in &units {
                    let depth = self.field.depth;
                    let unit = &mut self.units[id as usize];
                    let edge_z = match unit.side {
                        SideId::Attacker => -50.0,
                        SideId::Defender => depth + 50.0,
                    };
                    unit.withdrawing = true;
                    unit.target = None;
                    unit.destination = Some((unit.x, edge_z));
                    unit.destination_facing = None;
                    unit.running = true;
                    unit.state = UnitState::Marching;
                }
                let label = self.unit_label(units[0] as usize);
                let text = if units.len() == 1 {
                    format!("Les {label} se retirent du champ de bataille.")
                } else {
                    format!(
                        "{} régiments {} se retirent du champ de bataille.",
                        units.len(),
                        of_faction(
                            &self
                                .setup
                                .side(self.units[units[0] as usize].side)
                                .faction_name
                        )
                    )
                };
                let side = self.units[units[0] as usize].side;
                self.log(text, Some(side));
            }
        }
        Ok(())
    }

    /// Destinations of a group move: along a line perpendicular to `facing`
    /// when given (ordered by current lateral position), else keeping the
    /// offsets to the group's centroid.
    fn group_destinations(
        &self,
        ids: &[u32],
        x: f64,
        z: f64,
        facing: Option<f64>,
    ) -> Vec<(f64, f64)> {
        let n = ids.len() as f64;
        let (cx, cz) = ids.iter().fold((0.0, 0.0), |(sx, sz), id| {
            let u = &self.units[*id as usize];
            (sx + u.x / n, sz + u.z / n)
        });
        match facing {
            None => ids
                .iter()
                .map(|id| {
                    let u = &self.units[*id as usize];
                    (x + u.x - cx, z + u.z - cz)
                })
                .collect(),
            Some(angle) => {
                let right = (angle.cos(), -angle.sin());
                let mut order: Vec<(usize, f64)> = ids
                    .iter()
                    .enumerate()
                    .map(|(k, id)| {
                        let u = &self.units[*id as usize];
                        (k, (u.x - cx) * right.0 + (u.z - cz) * right.1)
                    })
                    .collect();
                order.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
                let widths: Vec<f64> = ids
                    .iter()
                    .map(|id| self.units[*id as usize].extent().0 + 10.0)
                    .collect();
                let total: f64 = widths.iter().sum();
                let mut result = vec![(x, z); ids.len()];
                let mut offset = -total * 0.5;
                for (k, _) in order {
                    let lateral = offset + widths[k] * 0.5;
                    result[k] = (x + right.0 * lateral, z + right.1 * lateral);
                    offset += widths[k];
                }
                result
            }
        }
    }

    // ----- time -----------------------------------------------------------

    /// Advances the battle by `dt` seconds, running as many fixed steps as
    /// needed (at most a few hundred per call).
    pub fn tick(&mut self, dt: f64) {
        if self.finished || !dt.is_finite() || dt <= 0.0 {
            return;
        }
        self.accumulator += dt;
        let mut steps = 0;
        while self.accumulator >= DT - 1e-9 && steps < MAX_STEPS_PER_CALL && !self.finished {
            self.accumulator -= DT;
            self.step();
            steps += 1;
        }
        if steps == MAX_STEPS_PER_CALL {
            self.accumulator = 0.0;
        }
    }

    /// Runs one fixed step of [`DT`] seconds.
    pub fn step(&mut self) {
        if self.finished {
            return;
        }
        for unit in &mut self.units {
            unit.tick_losses = 0.0;
            unit.flanked = 0;
        }
        if self.ticks.is_multiple_of(10) {
            for side in SideId::BOTH {
                if self.ai_enabled[side.index()] {
                    for command in ai::plan(self, side) {
                        let _ = self.apply_command(command, Some(side));
                    }
                }
            }
        }
        let contacts = self.contacts();
        self.resolve_movement(&contacts);
        let contacts = self.contacts();
        self.resolve_shooting(&contacts);
        self.resolve_melee(&contacts);
        self.resolve_morale_and_fatigue(&contacts);
        self.elapsed += DT;
        self.ticks += 1;
        self.check_end();
    }

    /// For each unit, the enemy units in contact with it (in id order).
    fn contacts(&self) -> Vec<Vec<usize>> {
        let n = self.units.len();
        let mut result = vec![Vec::new(); n];
        for i in 0..n {
            if !self.units[i].present() {
                continue;
            }
            for j in (i + 1)..n {
                let (a, b) = (&self.units[i], &self.units[j]);
                if a.side == b.side || !b.present() {
                    continue;
                }
                let (dx, dz) = (b.x - a.x, b.z - a.z);
                let dist = (dx * dx + dz * dz).sqrt();
                if dist > 250.0 {
                    continue;
                }
                let dir = if dist > 1e-6 {
                    (dx / dist, dz / dist)
                } else {
                    (0.0, 1.0)
                };
                let gap_ab = a.distance_to_rect(b.x, b.z) - b.support(dir);
                let gap_ba = b.distance_to_rect(a.x, a.z) - a.support(dir);
                if gap_ab.max(gap_ba) < CONTACT_GAP {
                    result[i].push(j);
                    result[j].push(i);
                }
            }
        }
        result
    }

    fn nearest_enemy(&self, index: usize, able_only: bool) -> Option<(usize, f64)> {
        let unit = &self.units[index];
        self.units
            .iter()
            .enumerate()
            .filter(|(_, e)| e.side != unit.side && e.present() && (!able_only || e.able()))
            .map(|(j, e)| (j, ((e.x - unit.x).powi(2) + (e.z - unit.z).powi(2)).sqrt()))
            .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
    }

    /// Movement speed of `unit` heading along `dir`, in m/s.
    fn speed(&self, unit: &Unit, dir: (f64, f64)) -> f64 {
        let mut speed = f64::from(unit.stats.speed) * 0.04;
        if unit.running || unit.state == UnitState::Routing {
            speed *= if unit.is_cavalry() && unit.state == UnitState::Charging {
                2.5
            } else {
                2.0
            };
        }
        speed *= match unit.formation {
            Formation::Column => 1.15,
            Formation::Square => 0.3,
            Formation::Line | Formation::Wedge => 1.0,
        };
        if self.field.in_forest(unit.x, unit.z) {
            speed *= if unit.mounted { 0.4 } else { 0.65 };
        }
        if self.field.in_mud(unit.x, unit.z) {
            speed *= if self.weather == Weather::Rain {
                0.45
            } else {
                0.55
            };
        }
        match self.field.water_at(unit.x, unit.z) {
            Some(true) => speed *= 0.5,
            Some(false) => speed *= 0.25,
            None => {}
        }
        if self.weather == Weather::Snow {
            speed *= 0.8;
        }
        let here = self.field.height(unit.x, unit.z);
        let ahead = self
            .field
            .height(unit.x + dir.0 * 3.0, unit.z + dir.1 * 3.0);
        let grade = (ahead - here) / 3.0;
        speed *= if grade > 0.0 {
            1.0 / (1.0 + grade * 6.0)
        } else {
            1.0 + (-grade).min(0.1)
        };
        speed * (1.0 - unit.fatigue / 200.0)
    }

    fn turn_rate(unit: &Unit) -> f64 {
        let degrees: f64 = if unit.mounted { 40.0 } else { 15.0 };
        degrees.to_radians() * DT
    }

    /// Moves `index` towards `(tx, tz)`; returns the remaining distance.
    fn advance(&mut self, index: usize, tx: f64, tz: f64, may_leave: bool) -> f64 {
        let unit = &self.units[index];
        let (dx, dz) = (tx - unit.x, tz - unit.z);
        let dist = (dx * dx + dz * dz).sqrt();
        if dist < 1e-6 {
            return 0.0;
        }
        let dir = (dx / dist, dz / dist);
        let step = (self.speed(unit, dir) * DT).min(dist);
        let heading = angle_to(dx, dz);
        let (width, depth) = (self.field.width, self.field.depth);
        let unit = &mut self.units[index];
        unit.x += dir.0 * step;
        unit.z += dir.1 * step;
        if !may_leave {
            unit.x = unit.x.clamp(1.0, width - 1.0);
            unit.z = unit.z.clamp(1.0, depth - 1.0);
        }
        let rate = if unit.state == UnitState::Routing {
            std::f64::consts::PI
        } else {
            Self::turn_rate(unit) * 4.0
        };
        unit.facing = turn_towards(unit.facing, heading, rate);
        unit.still_time = 0.0;
        unit.stakes_planted = false;
        dist - step
    }

    fn resolve_movement(&mut self, contacts: &[Vec<usize>]) {
        for i in 0..self.units.len() {
            if !self.units[i].present() {
                continue;
            }
            let state = self.units[i].state;
            // Routing: flee away from the nearest enemy and towards the own edge.
            if state == UnitState::Routing {
                let edge = match self.units[i].side {
                    SideId::Attacker => -1.0,
                    SideId::Defender => 1.0,
                };
                let (mut fx, mut fz) = (0.0, edge);
                if let Some((j, d)) = self.nearest_enemy(i, false) {
                    if d > 1e-6 {
                        let e = &self.units[j];
                        let u = &self.units[i];
                        fx += (u.x - e.x) / d;
                        fz += (u.z - e.z) / d;
                    }
                }
                let (x, z) = (self.units[i].x, self.units[i].z);
                self.advance(i, x + fx * 50.0, z + fz * 50.0, true);
                self.check_left_field(i);
                continue;
            }
            if self.units[i].withdrawing {
                if let Some((tx, tz)) = self.units[i].destination {
                    self.advance(i, tx, tz, true);
                }
                self.check_left_field(i);
                continue;
            }
            if self.units[i].rally_timer > 0.0 {
                self.units[i].rally_timer -= DT;
                if self.units[i].rally_timer <= 0.0 && self.units[i].state == UnitState::Rallied {
                    self.units[i].state = UnitState::Idle;
                }
            }
            let in_contact = !contacts[i].is_empty();
            if in_contact && !self.units[i].disengaging {
                self.enter_melee(i, &contacts[i]);
                continue;
            }
            if !in_contact {
                self.units[i].disengaging = false;
            }
            if let Some(target) = self.units[i].target {
                let t = target as usize;
                if !self.units[t].present() {
                    self.units[i].target = None;
                    self.units[i].state = UnitState::Idle;
                    continue;
                }
                let (tx, tz) = (self.units[t].x, self.units[t].z);
                let unit = &self.units[i];
                let dist = ((tx - unit.x).powi(2) + (tz - unit.z).powi(2)).sqrt();
                if unit.can_shoot() && unit.ammo > 0 && dist <= self.effective_range(unit, tx, tz) {
                    let unit = &mut self.units[i];
                    unit.state = UnitState::Shooting;
                    unit.facing = turn_towards(
                        unit.facing,
                        angle_to(tx - unit.x, tz - unit.z),
                        Self::turn_rate(unit) * 2.0,
                    );
                    continue;
                }
                let charge_distance = if unit.is_cavalry() { 120.0 } else { 40.0 };
                let charging = unit.running && dist < charge_distance && !unit.can_shoot();
                if charging && self.units[i].state != UnitState::Charging {
                    self.units[i].state = UnitState::Charging;
                    if self.units[i].is_cavalry() && !self.charge_announced[i] {
                        self.charge_announced[i] = true;
                        let text = format!("Les {} chargent !", self.unit_label(i));
                        let side = self.units[i].side;
                        self.log(text, Some(side));
                    }
                } else if !charging {
                    self.units[i].state = UnitState::Marching;
                }
                self.advance(i, tx, tz, false);
                continue;
            }
            if let Some((tx, tz)) = self.units[i].destination {
                let remaining = self.advance(i, tx, tz, false);
                if remaining < 1.5 {
                    let unit = &mut self.units[i];
                    unit.destination = None;
                    unit.running = false;
                    unit.state = UnitState::Idle;
                    if let Some(facing) = unit.destination_facing.take() {
                        unit.facing = facing;
                    }
                } else if self.units[i].state != UnitState::Melee {
                    self.units[i].state = UnitState::Marching;
                }
                continue;
            }
            // Standing still: idle (shooting is decided later).
            let unit = &mut self.units[i];
            if matches!(
                unit.state,
                UnitState::Marching | UnitState::Charging | UnitState::Melee
            ) {
                unit.state = UnitState::Idle;
            }
            unit.still_time += DT;
            if unit.has(Ability::Stakes) && !unit.stakes_planted && unit.still_time >= STAKES_DELAY
            {
                unit.stakes_planted = true;
                let text = format!("Les {} plantent leurs pieux.", self.unit_label(i));
                let side = self.units[i].side;
                self.log(text, Some(side));
            }
        }
    }

    fn check_left_field(&mut self, i: usize) {
        let unit = &self.units[i];
        let margin = 5.0;
        if unit.x < -margin
            || unit.z < -margin
            || unit.x > self.field.width + margin
            || unit.z > self.field.depth + margin
        {
            self.units[i].left_field = true;
            let text = format!("Les {} quittent le champ de bataille.", self.unit_label(i));
            let side = self.units[i].side;
            self.log(text, Some(side));
        }
    }

    /// A unit touching enemies fights; a charging unit delivers its impact.
    fn enter_melee(&mut self, i: usize, contacts: &[usize]) {
        let primary = self.primary_opponent(i, contacts);
        let was = self.units[i].state;
        let charging = was == UnitState::Charging
            || (self.units[i].running && self.units[i].target.is_some() && was != UnitState::Melee);
        self.units[i].state = UnitState::Melee;
        self.units[i].destination = None;
        self.units[i].still_time = 0.0;
        if let Some(p) = primary {
            // Reforming under attack is slow (3°/s on foot): a flank attack
            // keeps its bonus for a good while. With several opponents the
            // unit only turns towards its own target.
            if contacts.len() == 1 || self.units[i].target == Some(p as u32) {
                let (px, pz) = (self.units[p].x, self.units[p].z);
                let unit = &mut self.units[i];
                unit.facing = turn_towards(
                    unit.facing,
                    angle_to(px - unit.x, pz - unit.z),
                    Self::turn_rate(unit) * 0.2,
                );
            }
            if charging && was != UnitState::Melee {
                self.charge_impact(i, p);
            }
        }
    }

    fn charge_impact(&mut self, i: usize, p: usize) {
        let angle = attack_angle(&self.units[p], self.units[i].x, self.units[i].z);
        let cavalry = self.units[i].is_cavalry();
        if cavalry && angle == 0 && self.units[p].stakes_planted {
            let unit = &mut self.units[i];
            let loss = unit.hp * 0.12;
            unit.hp -= loss;
            unit.tick_losses += loss;
            unit.morale -= 15.0;
            unit.charge_timer = 0.0;
            let text = format!("Les {} s'empalent sur les pieux !", self.unit_label(i));
            let side = self.units[i].side;
            self.log(text, Some(side));
            return;
        }
        self.units[i].charge_timer = CHARGE_IMPACT;
        if cavalry && self.units[p].formation != Formation::Square {
            let shock = if angle == 0 { 8.0 } else { 15.0 };
            self.units[p].morale -= shock;
        }
    }

    fn primary_opponent(&self, i: usize, contacts: &[usize]) -> Option<usize> {
        if let Some(t) = self.units[i].target {
            if contacts.contains(&(t as usize)) {
                return Some(t as usize);
            }
        }
        let unit = &self.units[i];
        contacts.iter().copied().min_by(|&a, &b| {
            let da = (self.units[a].x - unit.x).powi(2) + (self.units[a].z - unit.z).powi(2);
            let db = (self.units[b].x - unit.x).powi(2) + (self.units[b].z - unit.z).powi(2);
            da.total_cmp(&db).then(a.cmp(&b))
        })
    }

    fn general_bonus(&self, side: SideId) -> Option<&crate::setup::GeneralSetup> {
        if self.general_alive[side.index()] {
            self.setup.side(side).general.as_ref()
        } else {
            None
        }
    }

    fn defense_points(&self, unit: &Unit) -> f64 {
        f64::from(unit.stats.armor)
            + self
                .general_bonus(unit.side)
                .map_or(0.0, |g| g.defense_percent)
    }

    fn resolve_shooting(&mut self, contacts: &[Vec<usize>]) {
        for i in 0..self.units.len() {
            let unit = &self.units[i];
            if !unit.present()
                || !unit.can_shoot()
                || unit.ammo == 0
                || !contacts[i].is_empty()
                || matches!(
                    unit.state,
                    UnitState::Routing | UnitState::Melee | UnitState::Charging
                )
                || unit.withdrawing
            {
                continue;
            }
            let moving = unit.state == UnitState::Marching;
            if moving && !unit.has(Ability::Skirmish) {
                continue;
            }
            if self.units[i].reload > 0.0 {
                self.units[i].reload -= DT;
                continue;
            }
            let Some(target) = self.pick_shooting_target(i) else {
                if self.units[i].state == UnitState::Shooting && self.units[i].target.is_none() {
                    self.units[i].state = UnitState::Idle;
                }
                continue;
            };
            self.fire(i, target);
        }
    }

    fn visible(&self, shooter: &Unit, target: &Unit, dist: f64) -> bool {
        if self.field.in_forest(target.x, target.z) && dist > 60.0 {
            return false;
        }
        shooter.has(Ability::Volley)
            || !self
                .field
                .blocks_sight((shooter.x, shooter.z), (target.x, target.z))
    }

    fn pick_shooting_target(&self, i: usize) -> Option<usize> {
        let unit = &self.units[i];
        let in_range = |j: usize| -> Option<f64> {
            let t = &self.units[j];
            if t.side == unit.side || !t.present() || t.state == UnitState::Melee {
                return None;
            }
            let dist = ((t.x - unit.x).powi(2) + (t.z - unit.z).powi(2)).sqrt();
            (dist <= self.effective_range(unit, t.x, t.z) && self.visible(unit, t, dist))
                .then_some(dist)
        };
        if let Some(t) = unit.target {
            if in_range(t as usize).is_some() {
                return Some(t as usize);
            }
            return None;
        }
        if !unit.fire_at_will {
            return None;
        }
        (0..self.units.len())
            .filter_map(|j| in_range(j).map(|d| (j, d)))
            .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
            .map(|(j, _)| j)
    }

    fn fire(&mut self, i: usize, t: usize) {
        let shooter = &self.units[i];
        let target = &self.units[t];
        let dist = ((target.x - shooter.x).powi(2) + (target.z - shooter.z).powi(2)).sqrt();
        let range = self.effective_range(shooter, target.x, target.z).max(1.0);
        let shots = if shooter.category == UnitCategory::Siege {
            shooter.hp * 3.0
        } else {
            shooter.hp
        };
        let mut accuracy = 0.3 * (1.0 - 0.5 * dist / range);
        if shooter.has(Ability::RainPenalty) {
            accuracy *= self.weather.bow_factor();
        }
        accuracy *= 1.0 + f64::from(shooter.experience) / 20.0;
        if let Some(general) = self.general_bonus(shooter.side) {
            accuracy *= 1.0 + general.ranged_percent / 100.0;
        }
        let mut kills = shots * accuracy * f64::from(shooter.stats.ranged) / 100.0
            * armor_factor(self.defense_points(target))
            * RANGED_RATE;
        if self.field.in_forest(target.x, target.z) {
            kills *= 0.5;
        }
        if target.has(Ability::Pavise) && target.state != UnitState::Marching {
            kills *= 0.6;
        }
        if target.formation == Formation::Square {
            kills *= 1.2;
        }
        if attack_angle(target, shooter.x, shooter.z) == 2 {
            kills *= 1.3;
        }
        let reload = if shooter.category == UnitCategory::Siege {
            12.0
        } else if shooter.has(Ability::Pavise) {
            9.0
        } else {
            6.0
        };
        let heading = angle_to(target.x - shooter.x, target.z - shooter.z);
        let kills = kills.min(self.units[t].hp);
        self.units[t].hp -= kills;
        self.units[t].tick_losses += kills;
        let shooter = &mut self.units[i];
        shooter.reload = reload;
        shooter.ammo -= 1;
        shooter.facing = turn_towards(shooter.facing, heading, 0.5);
        if shooter.state != UnitState::Marching {
            shooter.state = UnitState::Shooting;
        }
        if shooter.ammo == 0 {
            let missiles = if shooter.category == UnitCategory::Siege {
                "projectiles"
            } else if shooter.has(Ability::Pavise) {
                "carreaux"
            } else {
                "flèches"
            };
            let text = format!("Les {} sont à court de {missiles}.", self.unit_label(i));
            let side = self.units[i].side;
            self.log(text, Some(side));
            if self.units[i].state == UnitState::Shooting {
                self.units[i].state = UnitState::Idle;
            }
        }
        if self.units[t].hp <= 0.0 {
            self.unit_destroyed(t);
        }
    }

    fn melee_damage(&self, attacker: &Unit, defender: &Unit) -> f64 {
        let mut damage = attacker.fighting_soldiers() * f64::from(attacker.stats.melee) / 100.0
            * armor_factor(self.defense_points(defender))
            * MELEE_RATE
            * DT;
        if attacker.charge_timer > 0.0 {
            let charge = f64::from(attacker.stats.charge.unwrap_or(20));
            let lance = if attacker.has(Ability::ChargeLance) {
                1.5
            } else {
                1.0
            };
            let wedge = if attacker.formation == Formation::Wedge {
                1.2
            } else {
                1.0
            };
            let general = self
                .general_bonus(attacker.side)
                .map_or(0.0, |g| g.charge_percent);
            damage *= 1.0 + charge / 100.0 * lance * wedge * (1.0 + general / 100.0);
        }
        damage *= match attack_angle(defender, attacker.x, attacker.z) {
            0 => 1.0,
            1 => 1.5,
            _ => 2.0,
        };
        if defender.state == UnitState::Routing {
            damage *= 1.5;
        }
        if attacker.is_cavalry() {
            if defender.formation == Formation::Square {
                damage *= if defender.has(Ability::PikeSquare) {
                    0.25
                } else {
                    0.4
                };
            } else if defender.has(Ability::PikeSquare) {
                damage *= 0.7;
            }
        }
        if defender.is_cavalry() && attacker.has(Ability::PikeSquare) {
            damage *= 1.8;
        }
        damage *= 1.0 - attacker.fatigue / 250.0;
        damage *= 1.0 + f64::from(attacker.experience) / 20.0;
        damage *= 0.6 + attacker.morale.max(0.0) / 250.0;
        damage
    }

    fn resolve_melee(&mut self, contacts: &[Vec<usize>]) {
        let n = self.units.len();
        let mut damage = vec![0.0; n];
        let mut flanked = vec![0u8; n];
        for i in 0..n {
            let unit = &self.units[i];
            if !unit.present() || contacts[i].is_empty() || unit.state == UnitState::Routing {
                continue;
            }
            if unit.withdrawing || unit.disengaging {
                continue;
            }
            let Some(p) = self.primary_opponent(i, &contacts[i]) else {
                continue;
            };
            let defender = &self.units[p];
            damage[p] += self.melee_damage(unit, defender);
            match attack_angle(defender, unit.x, unit.z) {
                1 => flanked[p] |= 1,
                2 => flanked[p] |= 2,
                _ => {}
            }
        }
        for i in 0..n {
            let unit = &mut self.units[i];
            if unit.charge_timer > 0.0 {
                unit.charge_timer -= DT;
            }
            unit.flanked = flanked[i];
            if damage[i] <= 0.0 || !unit.present() {
                continue;
            }
            let before = unit.hp;
            let dealt = damage[i].min(unit.hp);
            unit.hp -= dealt;
            unit.tick_losses += dealt;
            if unit.is_general && self.general_alive[unit.side.index()] {
                let chance = dealt / before.max(1.0) * 0.4;
                if self.rng.unit() < chance || self.units[i].hp <= 0.0 {
                    self.kill_general(self.units[i].side);
                }
            }
            if self.units[i].hp <= 0.0 {
                self.unit_destroyed(i);
            }
        }
    }

    fn unit_destroyed(&mut self, i: usize) {
        self.units[i].hp = 0.0;
        if self.units[i].is_general && self.general_alive[self.units[i].side.index()] {
            self.kill_general(self.units[i].side);
        }
        let text = format!("Les {} sont anéantis.", self.unit_label(i));
        let side = self.units[i].side;
        self.log(text, Some(side));
    }

    fn kill_general(&mut self, side: SideId) {
        self.general_alive[side.index()] = false;
        self.general_killed[side.index()] = true;
        let name = self
            .setup
            .side(side)
            .general
            .as_ref()
            .map_or_else(|| "Le général".to_owned(), |g| g.name.clone());
        self.log(format!("{name} est tombé au combat !"), Some(side));
        for unit in self.units.iter_mut().filter(|u| u.side == side) {
            unit.morale -= 25.0;
        }
    }

    fn resolve_morale_and_fatigue(&mut self, contacts: &[Vec<usize>]) {
        let n = self.units.len();
        let general_pos: [Option<(f64, f64, f64)>; 2] = SideId::BOTH.map(|side| {
            let command = self.general_bonus(side).map(|g| f64::from(g.command))?;
            self.units
                .iter()
                .find(|u| u.side == side && u.is_general && u.able())
                .map(|u| (u.x, u.z, command))
        });
        let snapshot: Vec<(SideId, f64, f64, bool, bool)> = self
            .units
            .iter()
            .map(|u| {
                (
                    u.side,
                    u.x,
                    u.z,
                    u.present() && u.state == UnitState::Routing,
                    u.able(),
                )
            })
            .collect();
        let mut new_events: Vec<(String, SideId)> = Vec::new();
        for i in 0..n {
            if !self.units[i].present() {
                continue;
            }
            let label = self.unit_label(i);
            let unit = &mut self.units[i];
            let mut morale = unit.morale;
            morale -= unit.tick_losses / f64::from(unit.max_soldiers) * LOSS_MORALE_FACTOR;
            if unit.flanked & 1 != 0 {
                morale -= 1.5 * DT;
            }
            if unit.flanked & 2 != 0 {
                morale -= 3.0 * DT;
            }
            if unit.fatigue > 60.0 {
                morale -= (unit.fatigue - 60.0) * 0.02 * DT;
            }
            if unit.state == UnitState::Melee && unit.hp < f64::from(unit.max_soldiers) * 0.5 {
                morale -= 0.3 * DT;
            }
            let mut routing_friends = 0;
            let mut nearest_enemy = f64::INFINITY;
            for (j, &(side, x, z, routing, able)) in snapshot.iter().enumerate() {
                if j == i {
                    continue;
                }
                let d2 = (x - unit.x).powi(2) + (z - unit.z).powi(2);
                if side == unit.side {
                    if routing && d2 < 120.0 * 120.0 {
                        routing_friends += 1;
                    }
                } else if able {
                    nearest_enemy = nearest_enemy.min(d2.sqrt());
                }
            }
            morale -= f64::from(routing_friends.min(3)) * 1.0 * DT;
            let mut aura = 0.0;
            if let Some((gx, gz, command)) = general_pos[unit.side.index()] {
                if (gx - unit.x).powi(2) + (gz - unit.z).powi(2) < GENERAL_AURA * GENERAL_AURA {
                    aura = (0.1 + command * 0.05) * DT;
                }
            }
            let engaged = !contacts[i].is_empty();
            if unit.state == UnitState::Routing {
                if nearest_enemy > RALLY_SAFE_DISTANCE {
                    morale += 0.8 * DT + aura;
                }
            } else if !engaged && nearest_enemy > 100.0 {
                if morale < unit.morale_cap {
                    morale = (morale + 0.3 * DT + aura).min(unit.morale_cap);
                }
            } else if morale < unit.morale_cap + 10.0 {
                morale += aura;
            }
            unit.morale = morale.clamp(0.0, 100.0);

            // Fatigue.
            let mut rate: f64 = match unit.state {
                UnitState::Idle | UnitState::Rallied => -0.8,
                UnitState::Shooting => 0.02,
                UnitState::Marching if unit.running || unit.withdrawing => 0.25,
                UnitState::Marching => 0.05,
                UnitState::Charging => 0.4,
                UnitState::Melee => 0.3,
                UnitState::Routing => 0.3,
            };
            if rate > 0.0 {
                if self.weather == Weather::Snow {
                    rate *= 1.3;
                }
                if unit.mounted {
                    rate *= 0.8;
                }
            }
            unit.fatigue = (unit.fatigue + rate * DT).clamp(0.0, 100.0);

            // Rout and rally.
            if unit.state != UnitState::Routing && unit.morale < ROUT_MORALE && !unit.withdrawing {
                unit.state = UnitState::Routing;
                unit.target = None;
                unit.destination = None;
                unit.stakes_planted = false;
                unit.charge_timer = 0.0;
                new_events.push((format!("Les {label} sont en déroute !"), unit.side));
            } else if unit.state == UnitState::Routing
                && unit.morale > RALLY_MORALE
                && nearest_enemy > RALLY_SAFE_DISTANCE
                && unit.hp >= f64::from(unit.max_soldiers) * 0.2
            {
                unit.state = UnitState::Rallied;
                unit.rally_timer = 5.0;
                new_events.push((format!("Les {label} se rallient."), unit.side));
            }
        }
        for (text, side) in new_events {
            self.log(text, Some(side));
        }
    }

    fn check_end(&mut self) {
        if !self.end_conditions {
            return;
        }
        let able = SideId::BOTH.map(|side| {
            self.units
                .iter()
                .filter(|u| u.side == side && u.able())
                .count()
        });
        let timeout = self.elapsed >= MAX_DURATION - 1e-9;
        if able[0] > 0 && able[1] > 0 && !timeout {
            return;
        }
        let winner = if able[1] == 0 && able[0] > 0 {
            SideId::Attacker
        } else {
            SideId::Defender
        };
        self.finished = true;
        self.winner = Some(winner);
        let loser = winner.other();
        if self.general_alive[loser.index()] {
            if let Some(unit) = self.units.iter().find(|u| u.side == loser && u.is_general) {
                let caught = if unit.left_field {
                    unit.state == UnitState::Routing && !unit.withdrawing
                } else {
                    unit.state == UnitState::Routing
                };
                let chance = if unit.left_field { 0.2 } else { 0.35 };
                if caught && self.rng.unit() < chance {
                    self.general_captured[loser.index()] = true;
                    let name = self
                        .setup
                        .side(loser)
                        .general
                        .as_ref()
                        .map_or_else(|| "Le général".to_owned(), |g| g.name.clone());
                    self.log(format!("{name} est fait prisonnier."), Some(loser));
                }
            }
        }
        let text = if timeout && able[0] > 0 && able[1] > 0 {
            format!(
                "La nuit tombe : {} tient le terrain.",
                self.setup.side(winner).faction_name
            )
        } else {
            format!(
                "Victoire {} !",
                of_faction(&self.setup.side(winner).faction_name)
            )
        };
        self.log(text, Some(winner));
    }

    // ----- results --------------------------------------------------------

    /// The result, once the battle is finished.
    pub fn outcome(&self) -> Option<BattleOutcome> {
        let winner = self.winner?;
        let side_result = |side: SideId| -> SideResult {
            let setup = self.setup.side(side);
            let mut losses = vec![0u32; setup.units.len()];
            for unit in self.units.iter().filter(|u| u.side == side) {
                losses[unit.setup_index] = unit.initial_soldiers.saturating_sub(unit.soldiers());
            }
            let won = side == winner;
            SideResult {
                total_losses: losses.iter().sum(),
                losses,
                morale_delta: if won { 5 } else { -20 },
                routed: !won,
                general_killed: self.general_killed[side.index()],
                general_captured: self.general_captured[side.index()],
            }
        };
        Some(BattleOutcome {
            winner,
            attacker: side_result(SideId::Attacker),
            defender: side_result(SideId::Defender),
            duration: self.elapsed,
        })
    }

    /// Head count still able to fight on each side (HUD balance bar).
    pub fn strength(&self, side: SideId) -> u32 {
        self.units
            .iter()
            .filter(|u| u.side == side && u.able())
            .map(Unit::soldiers)
            .sum()
    }

    /// `(x, y, z, angle)` of every living soldier of `side`, filtered by
    /// `category` when given.
    pub fn soldier_transforms(
        &self,
        side: SideId,
        category: Option<UnitCategory>,
    ) -> Vec<[f64; 4]> {
        let mut result = Vec::new();
        for unit in self
            .units
            .iter()
            .filter(|u| u.side == side && category.is_none_or(|c| c == u.category))
        {
            for (x, z, angle) in unit.soldier_positions() {
                result.push([x, self.field.height(x, z), z, angle]);
            }
        }
        result
    }
}
