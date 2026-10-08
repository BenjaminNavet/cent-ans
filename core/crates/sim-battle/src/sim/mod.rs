//! The battle simulation proper: deployment, fixed ticks, commands, rules.

// Index loops read better than iterators here: each step mutates one unit
// while reading the others.
#![allow(clippy::needless_range_loop)]

mod camp;
mod capture;
mod commands;
mod decision;
mod deployment;
mod fire;
mod footing;
mod hold;
mod indirect;
mod melee;
mod modes;
mod morale;
mod movement;
mod neighbours;
mod obstacles;
mod opening;
mod pathing;
mod pipeline;
mod push;
mod queue;
mod reinforcements;
mod scenario;
mod separation;
mod setup;
mod shooting;
mod siege_assault;
mod siege_extra;
mod standards;
mod time_of_day;
mod water;
mod width;

pub use camp::CampState;
pub use deployment::{DeploymentZone, SIEGE_STANDOFF, ZONE_DEPTH};
pub use opening::AmbushLayout;
pub use reinforcements::MAX_ON_FIELD;
pub use separation::FRIEND_GAP;
pub use siege_assault::{Ladder, SiegeEngineKind, SiegeEngineView};
pub use width::{MoveShape, AUTO_GROUP_TAG};

use std::sync::{Arc, OnceLock};

use data_model::{Ability, UnitCategory, UnitStats};

use crate::ai;
use crate::command::{Command, CommandError};
use crate::decision::BattleEnd;
use crate::field::{Battlefield, Weather};
use crate::impact::{self, ImpactEvent, ImpactKind, LossCause, MAX_PENDING_IMPACTS};
use crate::orders::OrderUses;
use crate::outcome::{BattleEvent, BattleOutcome, SideResult};
use crate::queue::QueuedOrder;
use crate::rng::BattleRng;
use crate::scale::BattleScale;
use crate::setup::{BattleSetup, SideId, UnitSetup};
use crate::shot::{MissileKind, ShotCover, ShotEvent, MAX_PENDING_SHOTS};
use crate::siege::{self, PieceKind, SiegeWorks};
use crate::unit::{Unit, UnitFate, UnitState};
use width::move_group_tag;

/// Fixed simulation step, in seconds.
pub const DT: f64 = 0.1;
/// A battle lasts at most one simulated hour; the defender then wins.
pub const MAX_DURATION: f64 = 3600.0;
/// Seconds between two battle-AI decisions (M9: every 2 simulated seconds).
pub const AI_PERIOD: f64 = 2.0;
/// Upper bound of fixed steps run by a single [`BattleSim::tick`] call.
const MAX_STEPS_PER_CALL: u32 = 600;

/// Gap below which two enemy regiments are in contact (metres).
pub const CONTACT_GAP: f64 = 2.5;
/// Duration of the charge impact bonus (seconds).
pub const CHARGE_IMPACT: f64 = 4.0;
/// Seconds an archer unit must stand still before its stakes are planted.
pub const STAKES_DELAY: f64 = 15.0;
/// Speed ceiling of knights dismounted for a siege assault when the order
/// catalogue has no `dismount` order.
const ASSAULT_DISMOUNT_SPEED: u8 = 35;

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

/// Data derived from the field, computed once and shared by the forks of a
/// step (reset to a fresh cell by [`BattleSim::field_mut`]).
type Derived<T> = Arc<OnceLock<T>>;

/// A battle in progress.
#[derive(Debug, Clone)]
pub struct BattleSim {
    /// Immutable during a battle: shared by the forks of [`BattleSim::fork_for_step`].
    setup: Arc<BattleSetup>,
    field: Arc<Battlefield>,
    weather: Weather,
    units: Vec<Unit>,
    pub(crate) rng: BattleRng,
    elapsed: f64,
    ticks: u64,
    accumulator: f64,
    ai_enabled: [bool; 2],
    /// NT11: "hold ground" per side (see `sim/hold.rs`).
    hold: [bool; 2],
    end_conditions: bool,
    finished: bool,
    winner: Option<SideId>,
    general_alive: [bool; 2],
    general_killed: [bool; 2],
    general_captured: [bool; 2],
    events: Vec<BattleEvent>,
    events_read: usize,
    /// CB5: typed alerts emitted alongside the journal. Output only: not
    /// part of [`crate::replay::state_digest`], see `alerts.rs`.
    alerts: Vec<crate::alerts::BattleAlert>,
    alerts_read: usize,
    /// CB5: whether a `Flanked` alert already fired for this unit's current
    /// spell of being flanked (`Unit::flanked` itself is cleared every tick
    /// by [`BattleSim::step`], so the rising edge needs its own memory).
    flanked_alerted: Vec<bool>,
    /// Volleys resolved since the renderer last read them (BV1).
    shots: std::collections::VecDeque<ShotEvent>,
    charge_announced: Vec<bool>,
    /// Charge impacts since the renderer last read them (BV2).
    impacts: std::collections::VecDeque<ImpactEvent>,
    /// Siege battles: the town walls (M8 § 2).
    siege: Option<SiegeWorks>,
    square_announced: bool,
    /// T4: the "square threatened" alert fired (re-armed when the
    /// square's progress falls back to 0). Output only.
    square_threatened: bool,
    /// Leader's orders: uses and cooldowns per side (F10b).
    pub(crate) order_uses: OrderUses,
    /// The side gave the "no quarter" order.
    pub(crate) no_quarter: [bool; 2],
    /// Deployment phase (F5a): time frozen until `start_battle`.
    deploying: bool,
    /// Siege pathing cache, one slot per regiment (F5a; derived data).
    path_cache: std::cell::RefCell<Vec<Option<pathing::CachedPath>>>,
    /// Siege pathing: obstacle cells per side (review 2026-09-26; derived
    /// data, reset with the walls, the houses or [`BattleSim::siege_mut`]).
    obstacle_cache: std::cell::RefCell<pathing::ObstacleCache>,
    /// Tactical reading of the relief for the AI (R2b; derived data, read
    /// once per battle, reset by [`BattleSim::field_mut`]).
    relief_map: Derived<crate::relief_ai::ReliefMap>,
    /// BR3: props of the battle village (derived data, reset by
    /// [`BattleSim::field_mut`]).
    village_props: Derived<Vec<crate::town::Prop>>,
    /// Siege fires (S2): rules and their own random stream.
    fire: fire::FireSystem,
    /// SG1: renderer events of the assault, ram and oil timers.
    assault: siege_assault::AssaultState,
    /// Scale of the battle (EP1): field size and regiments per side.
    scale: BattleScale,
    /// EP5: rules of the standards, their own random stream, the routs
    /// already seen and the standards taken so far.
    /// Shared: cloned cheaply by each step of the standards.
    standard_rules: std::sync::Arc<data_model::BattleStandardRules>,
    /// EP11: continuous push of the lines (`data/rules/battle_push.json`).
    push_rules: crate::push::PushRules,
    standard_rng: BattleRng,
    standard_rout_seen: Vec<bool>,
    trophies: Vec<crate::outcome::StandardTrophy>,
    /// EP3: crossings of the river (derived data, reset by `field_mut`).
    crossings: Derived<Vec<crate::hydro::Crossing>>,
    /// EP3: regiments whose drowning was announced.
    drown_announced: Vec<u32>,
    /// EP6: looting of each side's camp.
    camp_states: [camp::CampState; 2],
    /// EP6: solid footprints of the decor by cell (derived data, reset by
    /// `field_mut`).
    decor_grid: Derived<obstacles::DecorGrid>,
    /// EP8: hour of the day when the battle began (the day moves on with
    /// `elapsed`) and the phase last announced in the journal.
    start_hour: f64,
    day_phase: Option<String>,
    /// EP9: rules of the end of a field battle, the engagement clock and
    /// how the battle ended.
    decision: crate::decision::DecisionRules,
    /// EP9b: the attacker's archery duel and the second echelon.
    duel: crate::duel::DuelRules,
    /// EP10: direction of the rout and contagion of morale.
    rout: crate::rout::RoutRules,
    /// A6-L13: lethality of the melee and of the missiles.
    pace: crate::pace::PaceRules,
    /// Nearest distance between the two armies, refreshed every step while the
    /// approach pace applies (`Pace::approach_range_m`).
    approach_gap: f64,
    clock: crate::decision::EngagementClock,
    end: Option<crate::decision::BattleEnd>,
    /// EP7: scenario of a historical battle (waves, posts, weather).
    scenario: Option<Box<scenario::Scenario>>,
    /// CV3-2: layout of an ambush opening (derived from setup and field).
    ambush: Option<opening::AmbushLayout>,
}

/// The battering ram every besieging army brings to a siege battle
/// (battle-only regiment: its crew is not a campaign unit).
fn ram_setup() -> UnitSetup {
    UnitSetup {
        unit_type: "battle_ram".to_owned(),
        name: "Bélier".to_owned(),
        category: UnitCategory::Siege,
        mounted: false,
        soldiers: 12,
        max_soldiers: 12,
        morale: 60,
        experience: 0,
        stats: UnitStats {
            melee: 5,
            ranged: 0,
            range: 0,
            armor: 75,
            morale: 60,
            speed: 22,
            ammo: 0,
            charge: None,
            siege_attack: None,
        },
        abilities: Vec::new(),
        missile: None,
    }
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

/// A new order interrupts a climb (the ladders stay behind).
fn stop_climbing(unit: &mut Unit) {
    if unit.climbing.take().is_some() {
        unit.climb_progress = 0.0;
        if unit.state == UnitState::Climbing {
            unit.state = UnitState::Idle;
        }
    }
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
/// Multiplier of a horseman's blows at foot in a square or with pikes
/// (1 otherwise).
pub(crate) fn horse_against_foot(attacker: &Unit, defender: &Unit) -> f64 {
    if !attacker.is_cavalry() {
        return 1.0;
    }
    if defender.braced() {
        if defender.has(Ability::PikeSquare) {
            0.25
        } else {
            defender.formation.def().modifiers.horse_blows
        }
    } else if defender.has(Ability::PikeSquare) {
        0.7
    } else {
        1.0
    }
}

/// Multiplier of pikemen's blows at horsemen (1 otherwise).
pub(crate) fn pikes_against_horse(attacker: &Unit, defender: &Unit) -> f64 {
    if defender.is_cavalry() && attacker.has(Ability::PikeSquare) {
        1.8
    } else {
        1.0
    }
}

pub(crate) fn attack_angle(defender: &Unit, attacker_x: f64, attacker_z: f64) -> u8 {
    if defender.all_round() {
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
    // ----- accessors ------------------------------------------------------

    pub fn setup(&self) -> &BattleSetup {
        &self.setup
    }

    pub fn field(&self) -> &Battlefield {
        &self.field
    }

    /// Scale of the battle (EP1).
    pub fn scale(&self) -> &BattleScale {
        &self.scale
    }

    /// Mutable field (tests and laboratory set-ups: hedges, villages).
    pub fn field_mut(&mut self) -> &mut Battlefield {
        self.relief_map = Default::default();
        self.crossings = Default::default();
        self.village_props = Default::default();
        self.decor_grid = Default::default();
        Arc::make_mut(&mut self.field)
    }

    /// Tactical reading of the relief (R2b), computed on first use.
    pub fn relief_map(&self) -> &crate::relief_ai::ReliefMap {
        self.relief_map
            .get_or_init(|| crate::relief_ai::ReliefMap::new(&self.field))
    }

    pub fn weather(&self) -> Weather {
        self.weather
    }

    /// The town walls of a siege battle.
    pub fn siege(&self) -> Option<&SiegeWorks> {
        self.siege.as_ref()
    }

    /// Mutable walls, for tests and scripted scenarios.
    pub fn siege_mut(&mut self) -> Option<&mut SiegeWorks> {
        self.obstacle_cache = Default::default();
        self.siege.as_mut()
    }

    /// Height at which `unit`'s soldiers stand at (x, z): the ground, raised
    /// to the wall walk on the walls and part-way up while climbing.
    pub fn standing_height(&self, unit: &Unit, x: f64, z: f64) -> f64 {
        let ground = self.field.walk_height(x, z);
        let Some(works) = &self.siege else {
            return ground;
        };
        if unit.on_wall {
            ground + works.wall_height
        } else if unit.climbing.is_some() {
            ground + works.wall_height * unit.climb_progress.clamp(0.0, 1.0)
        } else {
            ground
        }
    }

    /// `true` while `unit` scales a wall with ladders (no docked tower).
    pub fn on_ladders(&self, unit: &Unit) -> bool {
        match (unit.climbing, &self.siege) {
            (Some(p), Some(works)) => works.pieces[p].docked_tower.is_none(),
            _ => false,
        }
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

    /// Replaces the fighting rates (tuning probes; the game uses the data file).
    pub fn set_pace(&mut self, rules: crate::pace::PaceRules) {
        self.pace = rules;
    }

    /// Replaces the rout rules (tuning probes).
    pub fn set_rout_rules(&mut self, rules: crate::rout::RoutRules) {
        self.rout = rules;
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

    /// Volleys resolved since the previous call (BV1: arrows, stuck arrows
    /// and blood are drawn from them; at most [`MAX_PENDING_SHOTS`] kept).
    pub fn take_shots(&mut self) -> Vec<ShotEvent> {
        self.shots.drain(..).collect()
    }

    fn record_shot(&mut self, shot: ShotEvent) {
        if self.shots.len() >= MAX_PENDING_SHOTS {
            self.shots.pop_front();
        }
        self.shots.push_back(shot);
    }

    /// Missile kind of a shooting regiment (engines: bombard ball or stone).
    ///
    /// UR2: reads `unit_types/*.missile` (data-driven) when the type sets
    /// it; otherwise falls back to the old id/ability heuristic so units
    /// without the field (older fixtures, `ram_setup`) keep working.
    pub fn missile_kind(unit: &Unit) -> MissileKind {
        if let Some(missile) = unit.missile {
            return match missile {
                data_model::Missile::Arrow => MissileKind::Arrow,
                data_model::Missile::Bolt => MissileKind::Bolt,
                data_model::Missile::Bullet => MissileKind::Bullet,
                data_model::Missile::Javelin => MissileKind::Javelin,
                data_model::Missile::Stone => MissileKind::Stone,
            };
        }
        if unit.category == UnitCategory::Siege {
            if &*unit.unit_type == "unit_bombard" {
                MissileKind::Ball
            } else {
                MissileKind::Stone
            }
        } else if unit.unit_type.contains("crossbow") || unit.has(Ability::Pavise) {
            MissileKind::Bolt
        } else {
            MissileKind::Arrow
        }
    }

    /// Missiles one volley of `unit` looses (one per man, one per engine).
    fn missiles(unit: &Unit) -> u32 {
        if unit.category == UnitCategory::Siege {
            unit.soldiers().clamp(1, 4)
        } else {
            unit.soldiers()
        }
    }

    /// Charge impacts resolved since the previous call (BV2: men knocked
    /// down, horses slowed, riders unhorsed; at most [`MAX_PENDING_IMPACTS`]).
    pub fn take_impacts(&mut self) -> Vec<ImpactEvent> {
        self.impacts.drain(..).collect()
    }

    fn record_impact(&mut self, event: ImpactEvent) {
        if self.impacts.len() >= MAX_PENDING_IMPACTS {
            self.impacts.pop_front();
        }
        self.impacts.push_back(event);
    }

    /// Cause of the casualties a volley of `unit` inflicts (BV2). Mirrors
    /// [`Self::missile_kind`] (same data field, same fallback) so a bullet or
    /// a javelin kill draws the matching corpse decoration.
    fn missile_cause(unit: &Unit) -> LossCause {
        match Self::missile_kind(unit) {
            MissileKind::Arrow => LossCause::Arrow,
            MissileKind::Bolt => LossCause::Bolt,
            MissileKind::Ball => LossCause::Ball,
            MissileKind::Stone => LossCause::Stone,
            MissileKind::Bullet => LossCause::Bullet,
            MissileKind::Javelin => LossCause::Javelin,
        }
    }

    /// Journal entries added since the previous call.
    pub fn take_new_events(&mut self) -> Vec<BattleEvent> {
        let new = self.events[self.events_read..].to_vec();
        self.events_read = self.events.len();
        new
    }

    /// CB5: typed alerts added since the previous call. Output only: see
    /// `alerts.rs`.
    pub fn take_new_alerts(&mut self) -> Vec<crate::alerts::BattleAlert> {
        let new = self.alerts[self.alerts_read..].to_vec();
        self.alerts_read = self.alerts.len();
        new
    }

    /// Records a CB5 alert at the current simulated time. Called at the
    /// same points as the matching `log()`; never reads or changes
    /// simulated state.
    pub(crate) fn alert(
        &mut self,
        kind: crate::alerts::AlertKind,
        x: f64,
        z: f64,
        side: Option<SideId>,
        unit: Option<u32>,
    ) {
        self.alerts.push(crate::alerts::BattleAlert {
            kind,
            time: self.elapsed,
            x,
            z,
            side,
            unit,
        });
    }

    /// Effective shooting range of `unit` (weather, time of day, height
    /// advantage).
    pub fn effective_range(&self, unit: &Unit, target_x: f64, target_z: f64) -> f64 {
        let height_gain = (self.standing_height(unit, unit.x, unit.z)
            - self.field.height(target_x, target_z))
        .max(0.0);
        // CB4: the aimed shot shortens the range.
        let ability = self.ability_effects(unit).map_or(1.0, |e| e.range_factor);
        f64::from(unit.stats.range) * self.range_factor() * (1.0 + height_gain / 100.0) * ability
    }

    pub(crate) fn log(&mut self, text_fr: String, side: Option<SideId>) {
        self.events.push(BattleEvent {
            time: self.elapsed,
            text_fr,
            side,
        });
    }

    /// Logs a line about regiment `index`, to its own side; `text` receives
    /// the regiment's label ("Archers anglais").
    pub(crate) fn log_unit(&mut self, index: usize, text: impl FnOnce(&str) -> String) {
        let side = self.units[index].side;
        let line = text(&self.unit_label(index));
        self.log(line, Some(side));
    }

    pub(crate) fn unit_label(&self, index: usize) -> String {
        let unit = &self.units[index];
        let faction = &self.setup.side(unit.side).faction_name;
        format!("{} {}", unit.name, of_faction(faction))
    }

    // ----- time -----------------------------------------------------------

    /// Advances the battle by `dt` seconds, running as many fixed steps as
    /// needed (at most a few hundred per call).
    pub fn tick(&mut self, dt: f64) {
        self.tick_with(dt, BattleSim::step);
    }

    /// Runs one fixed step of [`DT`] seconds.
    pub fn step(&mut self) {
        if self.finished || self.deploying {
            return;
        }
        for unit in &mut self.units {
            unit.tick_losses = 0.0;
            unit.flanked = 0;
        }
        self.advance_day();
        // A6-L13b: the approach lasts until the two armies are within range.
        self.approach_gap = if self.pace().move_speed_factor == 1.0 {
            0.0
        } else {
            self.army_gap().unwrap_or(0.0)
        };
        let ai_ticks = (AI_PERIOD / DT).round() as u64;
        if self.ticks.is_multiple_of(ai_ticks) {
            self.check_sortie();
            self.tick_scenario();
            for side in SideId::BOTH {
                if self.ai_enabled[side.index()] {
                    for command in ai::plan(self, side) {
                        // EP7: held waves and posted regiments (historical maps).
                        // NT11: a side holding its ground keeps its place.
                        if let Some(command) = self
                            .scenario_filter(command)
                            .and_then(|command| self.hold_filter(side, command))
                        {
                            let _ = self.apply_command(command, Some(side));
                        }
                    }
                }
            }
        }
        let contacts = self.contacts();
        self.resolve_skirmish(&contacts);
        self.resolve_movement(&contacts);
        self.separate_friends();
        self.resolve_water();
        self.resolve_siege_works();
        self.resolve_capture_points();
        self.relieve_rams();
        let contacts = self.contacts();
        self.resolve_shooting(&contacts);
        self.tower_fire();
        self.boiling_oil();
        self.resolve_fire();
        self.resolve_push(&contacts);
        self.resolve_melee(&contacts);
        self.resolve_standards(&contacts);
        self.resolve_camps();
        self.resolve_morale_and_fatigue(&contacts);
        self.tick_orders(DT);
        for unit in &mut self.units {
            unit.tick_reform(DT);
        }
        self.tick_abilities();
        self.elapsed += DT;
        self.ticks += 1;
        self.release_reserves();
        self.record_siege_transitions();
        self.track_engagement();
        self.check_end();
    }

    /// Slowing of the AI patience clocks of this battle (`Pace::ai_patience_factor`).
    pub(crate) fn ai_patience_factor(&self) -> f64 {
        self.pace().ai_patience_factor
    }

    /// Fighting rates of this battle: sieges keep their own pace.
    pub(crate) fn pace(&self) -> &crate::pace::Pace {
        if self.siege.is_some() {
            &self.pace.siege
        } else if self.scenario.is_some() {
            // Historical maps keep the pace their order of battle was tuned for.
            &self.pace.historical
        } else {
            &self.pace.field
        }
    }

    // ----- results --------------------------------------------------------

    /// The result, once the battle is finished.
    pub fn outcome(&self) -> Option<BattleOutcome> {
        let winner = self.winner?;
        let end = self.end.unwrap_or_default();
        let refused = end == BattleEnd::Refused;
        let side_result = |side: SideId| -> SideResult {
            let setup = self.setup.side(side);
            let mut losses = vec![0u32; setup.units.len()];
            for unit in self.units.iter().filter(|u| u.side == side && !u.synthetic) {
                losses[unit.setup_index] = unit.initial_soldiers.saturating_sub(unit.soldiers());
            }
            let won = side == winner;
            let fates: Vec<UnitFate> = self
                .units
                .iter()
                .filter(|u| u.side == side && !u.synthetic)
                .map(Unit::fate)
                .collect();
            let withdrew =
                !won && fates.contains(&UnitFate::Withdrawn) && !fates.contains(&UnitFate::Routed);
            SideResult {
                total_losses: losses.iter().sum(),
                losses,
                morale_delta: match (refused, won) {
                    (true, true) => self.decision.refused_morale.defender,
                    (true, false) => self.decision.refused_morale.attacker,
                    (false, true) => 5,
                    (false, false) => -20,
                },
                // EP9: an army that gave up the field unbroken was not routed.
                routed: !won && !matches!(end, BattleEnd::Refused | BattleEnd::Lull),
                general_killed: self.general_killed[side.index()],
                general_captured: self.general_captured[side.index()],
                no_quarter: self.no_quarter[side.index()],
                withdrew,
                standards_taken: self.trophies_of(side),
                standards_lost: self.trophies.iter().filter(|t| t.taken_by != side).count() as u32,
                baggage_lost: self.camp_states[side.index()].looted,
            }
        };
        Some(BattleOutcome {
            winner,
            attacker: side_result(SideId::Attacker),
            defender: side_result(SideId::Defender),
            duration: self.elapsed,
            end,
        })
    }

    /// Head count still able to fight on each side (HUD balance bar).
    pub fn strength(&self, side: SideId) -> u32 {
        self.units
            .iter()
            .filter(|u| u.side == side && (u.able() || u.reserve) && !u.synthetic)
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
            result.extend(self.soldier_poses(unit, 1.0));
        }
        result
    }
}
