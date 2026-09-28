//! The battle simulation proper: deployment, fixed ticks, commands, rules.

// Index loops read better than iterators here: each step mutates one unit
// while reading the others.
#![allow(clippy::needless_range_loop)]

mod camp;
mod capture;
mod decision;
mod deployment;
mod fire;
mod indirect;
mod modes;
mod obstacles;
mod opening;
mod pathing;
mod pipeline;
mod push;
mod queue;
mod reinforcements;
mod scenario;
mod separation;
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
use crate::unit::{Formation, Unit, UnitFate, UnitState};
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
/// Seconds a rallied regiment stays in the `Rallied` state.
pub const RALLY_PAUSE: f64 = 5.0;
/// Speed ceiling of knights dismounted for a siege assault when the order
/// catalogue has no `dismount` order.
const ASSAULT_DISMOUNT_SPEED: u8 = 35;

const MELEE_RATE: f64 = 0.035;
const RANGED_RATE: f64 = 0.3;
const LOSS_MORALE_FACTOR: f64 = 60.0;
/// Fatigue above which a unit loses morale over time (SV4: named so the
/// battle markers show "exhausted" from the same threshold).
pub const EXHAUSTED_FATIGUE: f64 = 60.0;

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
    pub(crate) rng: BattleRng,
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
    relief_map: std::cell::OnceCell<crate::relief_ai::ReliefMap>,
    /// BR3: props of the battle village (derived data, reset by
    /// [`BattleSim::field_mut`]).
    village_props: std::cell::OnceCell<Vec<crate::town::Prop>>,
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
    crossings: std::cell::OnceCell<Vec<crate::hydro::Crossing>>,
    /// EP3: regiments whose drowning was announced.
    drown_announced: Vec<u32>,
    /// EP6: looting of each side's camp.
    camp_states: [camp::CampState; 2],
    /// EP6: solid footprints of the decor by cell (derived data, reset by
    /// `field_mut`).
    decor_grid: std::cell::OnceCell<obstacles::DecorGrid>,
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
    if defender.formation == Formation::Square {
        if defender.has(Ability::PikeSquare) {
            0.25
        } else {
            0.4
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
    /// Deploys both armies and draws the weather and the field from `seed`,
    /// at the scale of the setup's head count ([`BattleScale::for_setup`]).
    pub fn new(setup: BattleSetup, seed: u64) -> Result<Self, SetupError> {
        let scale = BattleScale::for_setup(&setup);
        Self::new_scaled(setup, seed, scale)
    }

    /// [`Self::new`] at a given scale (EP1: forced tier; sieges always use
    /// the standard field, whatever `scale` says about the field).
    pub fn new_scaled(
        setup: BattleSetup,
        seed: u64,
        mut scale: BattleScale,
    ) -> Result<Self, SetupError> {
        if setup.siege.is_some() {
            scale.field = crate::scale::FieldSize::STANDARD;
        }
        for side in SideId::BOTH {
            if setup.side(side).units.iter().all(|u| u.soldiers == 0) {
                return Err(SetupError::EmptySide(side));
            }
        }
        let mut rng = BattleRng::from_seed(seed);
        let weather = Weather::draw(setup.season, &mut rng);
        let is_siege = setup.siege.is_some();
        let mut field =
            Battlefield::generate_site_sized(&setup.field_site(), scale.field, weather, &mut rng);
        if !is_siege {
            // EP6: countryside and camps (derived stream), then the hand-made
            // decor of a historical map.
            // A bare field (`village: Some(false)`: labs, tests) keeps its
            // camps only.
            if setup.village == Some(false) {
                field.lay_camps(&rng);
            } else {
                field.lay_decor(&setup.province, &rng);
            }
            if let Some(plan) = &setup.decor_plan {
                field.apply_decor_plan(plan);
            }
        }
        let mut siege = setup.siege.as_ref().map(|s| {
            SiegeWorks::for_battle(
                s.fortification,
                s.breach,
                setup.siege_layout.as_ref(),
                &mut rng,
            )
        });
        let mut fire = fire::FireSystem::new(seed, is_siege);
        if let Some(works) = siege.as_mut() {
            fire.prepare(works);
            // BR3: props of the suburbs too (deterministic, no draw).
            works.lay_props();
        }
        if let Some(works) = siege.as_ref() {
            field.prepare_for_siege_around(if works.landmark.is_some() {
                works.outer_radius()
            } else {
                crate::siege::RING_RADIUS
            });
        }
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
        let dismount_order = setup
            .orders
            .iter()
            .find(|o| o.kind == data_model::BattleOrderKind::Dismount)
            .cloned();
        if is_siege {
            // Men-at-arms who can fight on foot (`dismount`) leave their
            // horses for the assault, as at every medieval escalade (same
            // rule as the "pied à terre" order, without its armour bonus).
            let speed = dismount_order
                .as_ref()
                .and_then(|o| o.effects.speed_max)
                .unwrap_or(ASSAULT_DISMOUNT_SPEED);
            for unit in units
                .iter_mut()
                .filter(|u| u.side == SideId::Attacker && u.mounted && u.has(Ability::Dismount))
            {
                unit.dismount(speed, 0);
            }
            let mut ram = Unit::from_setup(
                units.len() as u32,
                SideId::Attacker,
                usize::MAX,
                &ram_setup(),
            );
            ram.synthetic = true;
            ram.ram = true;
            units.push(ram);
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
        let standard_rng = rng.derive(standards::STANDARD_SALT);
        let standard_rules = std::sync::Arc::new(setup.standards.clone().unwrap_or_default());
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
            alerts: Vec::new(),
            alerts_read: 0,
            flanked_alerted: vec![false; count],
            shots: std::collections::VecDeque::new(),
            charge_announced: vec![false; count],
            impacts: std::collections::VecDeque::new(),
            siege,
            square_announced: false,
            square_threatened: false,
            order_uses: Default::default(),
            no_quarter: [false; 2],
            deploying: false,
            path_cache: Default::default(),
            obstacle_cache: Default::default(),
            relief_map: Default::default(),
            village_props: Default::default(),
            fire,
            assault: Default::default(),
            scale,
            standard_rules,
            push_rules: crate::push::PushRules::bundled().clone(),
            standard_rng,
            standard_rout_seen: vec![false; count],
            trophies: Vec::new(),
            crossings: Default::default(),
            drown_announced: Vec::new(),
            camp_states: Default::default(),
            decor_grid: Default::default(),
            start_hour: crate::time_of_day::TimeOfDayRules::bundled().default_hour,
            day_phase: None,
            decision: crate::decision::DecisionRules::bundled().clone(),
            duel: crate::duel::DuelRules::bundled().clone(),
            rout: crate::rout::RoutRules::bundled().clone(),
            clock: Default::default(),
            end: None,
            scenario: None,
            ambush: None,
        };
        sim.hold_reserves();
        if sim.siege.is_some() {
            sim.deploy_siege();
        } else {
            sim.deploy();
            sim.apply_opening();
        }
        let text = match sim.weather {
            Weather::Clear => "Le ciel est dégagé sur le champ de bataille.".to_owned(),
            Weather::Rain => "Il pleut : les cordes des arcs se détendent.".to_owned(),
            Weather::Fog => "Un épais brouillard couvre le champ de bataille.".to_owned(),
            Weather::Snow => "La neige tombe sur le champ de bataille.".to_owned(),
        };
        sim.log(text, None);
        sim.log_opening();
        if let Some(works) = &sim.siege {
            let open = works.openings().len();
            let text = if open > 0 {
                format!("Siège : {open} brèche(s) déjà ouverte(s) dans l'enceinte.")
            } else {
                "Siège : les murailles sont intactes ; échelles, tours et bélier sont prêts."
                    .to_owned()
            };
            sim.log(text, None);
            let dismounted = sim
                .units
                .iter()
                .any(|u| u.side == SideId::Attacker && u.dismounted);
            if dismounted {
                let of = of_faction(&sim.setup.attacker.faction_name);
                let text = match dismount_order.and_then(|o| o.journal_assault) {
                    Some(template) => template.replace("{of_faction}", &of),
                    None => format!("Les chevaliers {of} mettent pied à terre pour l'assaut."),
                };
                sim.log(text, Some(SideId::Attacker));
            }
        }
        Ok(sim)
    }

    /// Initial field deployment: each side in « Ligne de bataille » (CB6,
    /// `data/rules/group_formations.json`), rows centred on the field with
    /// +x as the lateral axis on both sides (the placement before CB6).
    fn deploy(&mut self) {
        let rules = crate::group_formation::GroupFormationRules::bundled();
        let preset = rules.battle_line();
        for side in SideId::BOTH {
            let (line_z, facing, back) = match side {
                SideId::Attacker => (self.field.attacker_line_z(), 0.0, -1.0),
                SideId::Defender => (self.field.defender_line_z(), std::f64::consts::PI, 1.0),
            };
            let ids: Vec<usize> = (0..self.units.len())
                .filter(|&i| self.units[i].side == side && !self.units[i].reserve)
                .collect();
            let frame = crate::group_formation::Frame::with_axes(
                self.field.size.center_x(),
                line_z,
                (1.0, 0.0),
                (0.0, -back),
                facing,
            );
            let placed = crate::group_formation::layout(
                rules,
                preset,
                &self.units,
                &ids,
                frame,
                crate::group_formation::LayoutOptions {
                    field_width: self.field.width,
                    separate_general: false,
                    wings: crate::group_formation::WingFill::Alternate,
                },
            );
            for p in placed {
                let unit = &mut self.units[p.index];
                unit.x = p.x;
                unit.z = p.z;
                unit.facing = p.facing;
            }
        }
    }

    /// Cavalry on the wings of a front `front_width` wide, alternating right
    /// and left.
    fn place_wings(&mut self, cavalry: &[usize], front_width: f64, z: f64) {
        let center = self.field.size.center_x();
        let mut left = center - front_width * 0.5 - 20.0;
        let mut right = center + front_width * 0.5 + 20.0;
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
            unit.z = z;
        }
    }

    fn side_units(&self, side: SideId, keep: impl Fn(&Unit) -> bool) -> Vec<usize> {
        (0..self.units.len())
            .filter(|&i| {
                self.units[i].side == side && !self.units[i].reserve && keep(&self.units[i])
            })
            .collect()
    }

    /// Siege deployment (M8 § 2): the besiegers south of the walls (shooters
    /// ahead of the infantry, towers and ram in front, engines behind); the
    /// garrison on the wall walk facing out (shooters first), a guard behind
    /// the gate and a reserve in the central square.
    fn deploy_siege(&mut self) {
        let works = self.siege.clone().expect("siege battle");
        let front_z = works
            .vertices
            .iter()
            .map(|v| v.1)
            .fold(f64::INFINITY, f64::min);
        // Besiegers.
        let a = SideId::Attacker;
        for i in self.side_units(a, |_| true) {
            self.units[i].facing = 0.0;
        }
        let infantry = self.side_units(a, |u| u.category == UnitCategory::Infantry);
        let shooters = self.side_units(a, |u| u.category == UnitCategory::Ranged && !u.mounted);
        let cavalry = self.side_units(a, |u| {
            u.category == UnitCategory::Cavalry || (u.mounted && u.category == UnitCategory::Ranged)
        });
        let towers = self.side_units(a, Unit::siege_tower);
        let engines = self.side_units(a, |u| {
            u.category == UnitCategory::Siege && !u.siege_tower() && !u.ram
        });
        let rams = self.side_units(a, |u| u.ram);
        // Out of bowshot of the wall walk; towers and ram closer in.
        let width = self.place_row(&infantry, front_z - 265.0, -1.0);
        self.place_row(&shooters, front_z - 215.0, -1.0);
        self.place_wings(&cavalry, width.max(200.0), front_z - 280.0);
        self.place_row(&engines, front_z - 240.0, -1.0);
        let front = works.front_walls();
        for (k, &i) in towers.iter().enumerate() {
            let (mx, mz) = works.pieces[front[k % front.len()]].midpoint();
            let unit = &mut self.units[i];
            unit.x = mx + 15.0 * (k / front.len()) as f64;
            unit.z = mz - 70.0;
        }
        let (gx, gz) = works.pieces[works.gate].midpoint();
        for &i in &rams {
            self.units[i].x = gx;
            self.units[i].z = gz - 110.0;
        }
        // Garrison.
        let d = SideId::Defender;
        for i in self.side_units(d, |_| true) {
            self.units[i].facing = std::f64::consts::PI;
        }
        let wall_shooters =
            self.side_units(d, |u| u.category == UnitCategory::Ranged && !u.mounted);
        let foot = self.side_units(d, |u| u.category == UnitCategory::Infantry && !u.mounted);
        let others = self.side_units(d, |u| {
            u.mounted || matches!(u.category, UnitCategory::Cavalry | UnitCategory::Siege)
        });
        let reserve_count = if foot.len() >= 2 {
            foot.len().div_ceil(3)
        } else {
            0
        };
        let (reserve, wall_foot) = foot.split_at(reserve_count);
        let mut on_walls = wall_shooters;
        on_walls.extend_from_slice(wall_foot);
        let mut order = front.clone();
        let mut rest: Vec<usize> = (0..works.pieces.len())
            .filter(|p| works.pieces[*p].kind == PieceKind::Wall && !front.contains(p))
            .collect();
        rest.sort_by(|&x, &y| {
            works.pieces[x]
                .midpoint()
                .1
                .total_cmp(&works.pieces[y].midpoint().1)
                .then(x.cmp(&y))
        });
        order.extend(rest);
        let mut used = vec![4.0; works.pieces.len()];
        let mut leftover: Vec<usize> = reserve.to_vec();
        for i in on_walls {
            let (w, _) = self.units[i].extent();
            let slot = order.iter().copied().find(|&p| {
                works.pieces[p].intact() && used[p] + w + 4.0 <= works.pieces[p].length()
            });
            let Some(p) = slot else {
                leftover.push(i);
                continue;
            };
            let piece = &works.pieces[p];
            let (tx, tz) = piece.tangent();
            let (nx, nz) = piece.outward();
            let along = used[p] + w * 0.5;
            used[p] += w + 6.0;
            let inset = works.thickness * 0.25;
            let unit = &mut self.units[i];
            unit.x = piece.a.0 + tx * along - nx * inset;
            unit.z = piece.a.1 + tz * along - nz * inset;
            unit.facing = angle_to(nx, nz);
            unit.on_wall = true;
        }
        // A guard behind the gate, the rest in the square.
        leftover.sort_unstable();
        let (nx, nz) = works.pieces[works.gate].outward();
        let mut square: Vec<usize> = Vec::new();
        for (k, &i) in leftover.iter().enumerate() {
            if k == 0 && self.units[i].category == UnitCategory::Infantry {
                let unit = &mut self.units[i];
                unit.x = gx - nx * 30.0;
                unit.z = gz - nz * 30.0;
                unit.facing = angle_to(nx, nz);
            } else {
                square.push(i);
            }
        }
        let (_, cz) = works.center;
        self.place_row(&square, cz - 15.0, 1.0);
        self.place_row(&others, cz + 30.0, 1.0);
    }

    /// Places `row` side by side, centred on the field, wrapping into extra rows
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
            let mut x = self.field.size.center_x() - total * 0.5;
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
        &mut self.field
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
            if unit.unit_type == "unit_bombard" {
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

    pub(crate) fn unit_label(&self, index: usize) -> String {
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
        let setup_order = matches!(
            command,
            Command::Formation { .. } | Command::FireAtWill { .. } | Command::SetMode { .. }
        );
        if self.deploying && !setup_order {
            return Err(CommandError::Deploying);
        }
        if let Command::LeaderOrder {
            side: order_side,
            order,
            units,
        } = &command
        {
            let giver = match (side, *order_side) {
                (Some(owner), Some(named)) if owner != named => {
                    return Err(CommandError::WrongSide)
                }
                (Some(owner), _) => owner,
                (None, Some(named)) => named,
                (None, None) => units
                    .first()
                    .and_then(|&id| self.units.get(id as usize))
                    .map(|u| u.side)
                    .ok_or(CommandError::NoSide)?,
            };
            return self.give_order(giver, order, units);
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
                queue,
                width,
                match_speed,
                group_tag,
            } => {
                if !self.field.inside(x, z) {
                    return Err(CommandError::OutsideField);
                }
                if queue {
                    self.check_queue_room(&units)?;
                }
                // CB-M3: a queued move spreads the group around where each
                // regiment will stand once its queue is done.
                let anchors: Vec<(f64, f64)> = if queue {
                    units
                        .iter()
                        .map(|&id| self.queue_anchor(id as usize))
                        .collect()
                } else {
                    units
                        .iter()
                        .map(|&id| (self.units[id as usize].x, self.units[id as usize].z))
                        .collect()
                };
                // CB1: each regiment's share of a dragged width, and the
                // frontage it will take (ranks within their bounds).
                let widths = self.move_widths(&units, width);
                let frontages = self.move_frontages(&units, widths.as_deref());
                let destinations = self.group_destinations_with(
                    &units,
                    &anchors,
                    x,
                    z,
                    facing,
                    frontages.as_deref(),
                );
                let group_tag = move_group_tag(&units, match_speed, group_tag);
                for (k, (id, (dx, dz))) in units.iter().zip(destinations).enumerate() {
                    let destination = (
                        dx.clamp(5.0, self.field.width - 5.0),
                        dz.clamp(5.0, self.field.depth - 5.0),
                    );
                    let shape = MoveShape {
                        width: widths.as_ref().map(|w| w[k]),
                        match_speed,
                        group_tag,
                    };
                    let index = *id as usize;
                    if queue && self.units[index].busy() {
                        self.units[index].order_queue.push_back(QueuedOrder::Move {
                            x: destination.0,
                            z: destination.1,
                            facing,
                            run,
                            width: shape.width,
                            match_speed,
                            group_tag,
                        });
                        continue;
                    }
                    self.units[index].order_queue.clear();
                    self.start_move(index, destination, facing, run, shape);
                }
            }
            Command::Attack {
                units,
                target,
                run,
                queue,
            } => {
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
                if queue {
                    self.check_queue_room(&units)?;
                }
                for &id in &units {
                    let index = id as usize;
                    if queue && self.units[index].busy() {
                        self.units[index]
                            .order_queue
                            .push_back(QueuedOrder::Attack { target, run });
                        continue;
                    }
                    self.units[index].order_queue.clear();
                    self.start_attack(index, target, run);
                }
            }
            Command::Halt { units } => {
                for &id in &units {
                    let unit = &mut self.units[id as usize];
                    unit.order_queue.clear();
                    unit.match_speed = false;
                    unit.group_tag = None;
                    unit.target = None;
                    unit.destination = None;
                    unit.destination_facing = None;
                    unit.running = false;
                    unit.disengaging = false;
                    stop_climbing(unit);
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
                    // CB1: a formation order drops a dragged width.
                    self.units[id as usize].formation = kind;
                    self.units[id as usize].line_files = None;
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
                    unit.order_queue.clear();
                    unit.pavise = None;
                    stop_climbing(unit);
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
            Command::TargetWall { units, piece } => {
                let Some(works) = &self.siege else {
                    return Err(CommandError::NotASiege);
                };
                if piece >= works.pieces.len() {
                    return Err(CommandError::UnknownPiece(piece));
                }
                for &id in &units {
                    if !self.units[id as usize].wall_breaker() {
                        return Err(CommandError::NotAnEngine(id));
                    }
                }
                for &id in &units {
                    let unit = &mut self.units[id as usize];
                    unit.wall_target = Some(piece);
                    unit.target = None;
                    unit.order_queue.clear();
                }
            }
            Command::Burn { units, house, gate } => self.command_burn(&units, house, gate)?,
            Command::SetMode {
                units,
                mode,
                enabled,
            } => self.set_mode(&units, mode, enabled)?,
            Command::UseAbility { units, ability } => self.use_ability(&units, &ability)?,
            Command::LeaderOrder { .. } => unreachable!("handled above"),
        }
        Ok(())
    }

    /// Destinations of a group move for regiments standing at `anchors`
    /// (where they are, or CB-M3 where their queued orders leave them):
    /// along a line perpendicular to `facing` when given (ordered by
    /// lateral position), else keeping the offsets to the group's centroid.
    /// CB1: `frontages` (metres, one per regiment) replace the present
    /// frontages of a dragged group.
    pub(crate) fn group_destinations_with(
        &self,
        ids: &[u32],
        anchors: &[(f64, f64)],
        x: f64,
        z: f64,
        facing: Option<f64>,
        frontages: Option<&[f64]>,
    ) -> Vec<(f64, f64)> {
        let n = ids.len() as f64;
        let (cx, cz) = anchors
            .iter()
            .fold((0.0, 0.0), |(sx, sz), &(ax, az)| (sx + ax / n, sz + az / n));
        match facing {
            None => anchors
                .iter()
                .map(|&(ax, az)| (x + ax - cx, z + az - cz))
                .collect(),
            Some(angle) => {
                let right = (angle.cos(), -angle.sin());
                let mut order: Vec<(usize, f64)> = anchors
                    .iter()
                    .enumerate()
                    .map(|(k, &(ax, az))| (k, (ax - cx) * right.0 + (az - cz) * right.1))
                    .collect();
                order.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
                let widths: Vec<f64> = match frontages {
                    Some(frontages) => frontages.iter().map(|w| w + 10.0).collect(),
                    None => ids
                        .iter()
                        .map(|id| self.units[*id as usize].extent().0 + 10.0)
                        .collect(),
                };
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
        let ai_ticks = (AI_PERIOD / DT).round() as u64;
        if self.ticks.is_multiple_of(ai_ticks) {
            self.check_sortie();
            self.tick_scenario();
            for side in SideId::BOTH {
                if self.ai_enabled[side.index()] {
                    for command in ai::plan(self, side) {
                        // EP7: held waves and posted regiments (historical maps).
                        if let Some(command) = self.scenario_filter(command) {
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
        self.tick_abilities();
        self.elapsed += DT;
        self.ticks += 1;
        self.release_reserves();
        self.record_siege_transitions();
        self.track_engagement();
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
                if gap_ab.max(gap_ba) < CONTACT_GAP && !self.wall_between(a, b) {
                    result[i].push(j);
                    result[j].push(i);
                }
            }
        }
        result
    }

    /// An intact wall separates the two regiments for melee: only climbers
    /// fight the defenders above them, and regiments on the wall walk fight
    /// each other.
    fn wall_between(&self, a: &Unit, b: &Unit) -> bool {
        let Some(works) = &self.siege else {
            return false;
        };
        if works.crosses_intact((a.x, a.z), (b.x, b.z)).is_none() {
            return false;
        }
        let bridged = (a.climbing.is_some() && b.on_wall)
            || (b.climbing.is_some() && a.on_wall)
            || (a.on_wall && b.on_wall);
        !bridged
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
        if unit.siege_tower() {
            // Pushed along by the assault troops.
            speed = speed.max(0.55);
        }
        if unit.running || unit.state == UnitState::Routing {
            speed *= if unit.is_cavalry() && unit.state == UnitState::Charging {
                2.5
            } else {
                2.0
            };
            // CB2: a run under the run mode (1 in the bundled data).
            speed *= unit.run_mode_speed();
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
        // EP3: fords, deep water, streams, banks, bridges, roads.
        speed *= self.water_speed(unit);
        if self.weather == Weather::Snow {
            speed *= 0.8;
        }
        speed *= self
            .field
            .site_speed_factor(unit.x, unit.z, unit.mounted, self.weather);
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
        let speed = speed * (1.0 - unit.fatigue / 200.0);
        // CB4: close ranks walk slower.
        let speed = speed * self.ability_effects(unit).map_or(1.0, |e| e.speed_factor);
        // CB1: a `match_speed` group keeps the pace of its slowest regiment.
        if unit.match_speed {
            speed * self.group_pace_factor(unit)
        } else {
            speed
        }
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
        let from = (unit.x, unit.z);
        let to = (unit.x + dir.0 * step, unit.z + dir.1 * step);
        let blocked = self.wall_block(index, from, to);
        let in_house = blocked.is_none()
            && (self.house_block(index, from, to)
                || (!may_leave && self.water_blocks(index, from, to)));
        let unit = &mut self.units[index];
        unit.blocked_by = blocked;
        if blocked.is_some() || in_house {
            unit.facing = turn_towards(unit.facing, heading, Self::turn_rate(unit) * 4.0);
            return dist;
        }
        unit.x = to.0;
        unit.z = to.1;
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

    /// The intact wall piece that stops a move from `from` to `to`, if any.
    /// Routing regiments slip through posterns and are never stopped;
    /// defenders step onto the wall walk from inside but not beyond it;
    /// attackers on the wall walk go where they please.
    fn wall_block(&self, index: usize, from: (f64, f64), to: (f64, f64)) -> Option<usize> {
        let works = self.siege.as_ref()?;
        let unit = &self.units[index];
        if unit.state == UnitState::Routing || unit.left_field {
            return None;
        }
        let defender_inside =
            unit.side == SideId::Defender && (unit.on_wall || works.inside(from.0, from.1));
        let band = works.band();
        for (k, piece) in works.pieces.iter().enumerate() {
            if !piece.intact() {
                continue;
            }
            // Sortie (F5a): the garrison opens its gate for its own regiments.
            if works.sortie && unit.side == SideId::Defender && piece.kind == PieceKind::Gate {
                continue;
            }
            if defender_inside {
                // Never further out than the middle of the wall walk.
                let out_to = piece.outside_offset(to.0, to.1);
                if piece.distance(to.0, to.1) < band
                    && out_to > 0.0
                    && out_to > piece.outside_offset(from.0, from.1)
                {
                    return Some(k);
                }
                continue;
            }
            if unit.on_wall {
                continue;
            }
            // Stopped only when closing in on the wall face (sliding along
            // it or backing away is free).
            let d_to = piece.distance(to.0, to.1);
            // Distance to the stretch itself (F5a): rounding the jamb of a
            // breach or gate is free.
            let closing = d_to < piece.distance(from.0, from.1) - 1e-9;
            if piece.crossed_by(from, to) || (d_to < band && closing) {
                return Some(k);
            }
        }
        None
    }

    /// A regiment stopped by an intact wall: foot soldiers of the attacker
    /// raise their ladders (or cross from a docked siege tower).
    fn start_climb(&mut self, i: usize) {
        let Some(piece) = self.units[i].blocked_by else {
            return;
        };
        let unit = &self.units[i];
        if unit.side != SideId::Attacker || !unit.can_climb() || unit.climbing.is_some() {
            return;
        }
        let Some(works) = &self.siege else {
            return;
        };
        let tower = works.pieces[piece].docked_tower.is_some_and(|t| {
            let t = &self.units[t as usize];
            (t.x - unit.x).powi(2) + (t.z - unit.z).powi(2) < 40.0 * 40.0
        });
        let unit = &mut self.units[i];
        unit.climbing = Some(piece);
        unit.climb_progress = 0.0;
        unit.state = UnitState::Climbing;
        let text = if tower {
            format!(
                "Les {} s'élancent de la tour de siège sur le rempart.",
                self.unit_label(i)
            )
        } else {
            format!(
                "Les {} dressent leurs échelles contre la muraille.",
                self.unit_label(i)
            )
        };
        let side = self.units[i].side;
        self.log(text, Some(side));
        if !tower {
            let unit = self.units[i].id;
            self.push_fx(crate::siege_fx::SiegeFxKind::LaddersRaised { unit, piece });
        }
    }

    /// One step of climbing; on the top the regiment stands on the wall walk
    /// and resumes its order.
    fn progress_climb(&mut self, i: usize, piece: usize, engaged: bool) {
        let Some(works) = &self.siege else {
            return;
        };
        let p = &works.pieces[piece];
        if !p.intact() {
            // The wall came down under them: walk through the breach.
            let unit = &mut self.units[i];
            unit.climbing = None;
            unit.climb_progress = 0.0;
            unit.state = UnitState::Marching;
            return;
        }
        let unit = &self.units[i];
        let tower = p.docked_tower.is_some_and(|t| {
            let t = &self.units[t as usize];
            (t.x - unit.x).powi(2) + (t.z - unit.z).powi(2) < 40.0 * 40.0
        });
        let duration = if tower {
            siege::TOWER_CLIMB_TIME
        } else {
            siege::LADDER_TIME
        };
        let mut rate = DT / duration * (1.0 - unit.fatigue / 200.0);
        if engaged {
            rate *= 0.25;
        }
        let (cx, cz) = p.closest_point(unit.x, unit.z);
        let (nx, nz) = p.outward();
        let inset = works.thickness * 0.25;
        let unit = &mut self.units[i];
        unit.climb_progress += rate;
        if unit.climb_progress >= 1.0 {
            unit.climbing = None;
            unit.climb_progress = 0.0;
            unit.on_wall = true;
            unit.x = cx - nx * inset;
            unit.z = cz - nz * inset;
            unit.state = UnitState::Marching;
            let text = format!("Les {} prennent pied sur le rempart !", self.unit_label(i));
            let side = self.units[i].side;
            self.log(text, Some(side));
            let unit = self.units[i].id;
            self.push_fx(crate::siege_fx::SiegeFxKind::OnWall { unit, piece });
        }
    }

    /// Siege battles, after movement: wall-walk status, docked towers, the
    /// ram against the gate, and the hold of the central square.
    fn resolve_siege_works(&mut self) {
        let Some(works) = self.siege.as_mut() else {
            return;
        };
        let band = works.band();
        let mut logs: Vec<(String, Option<SideId>)> = Vec::new();
        // Wall walk.
        for unit in self.units.iter_mut() {
            if !unit.present() {
                continue;
            }
            if unit.state == UnitState::Routing {
                unit.on_wall = false;
                continue;
            }
            let near = works.nearest_intact(unit.x, unit.z);
            unit.on_wall = match unit.side {
                SideId::Defender => near.is_some_and(|(p, d)| {
                    d < band + 1.0 && works.pieces[p].outside_offset(unit.x, unit.z) <= 0.5
                }),
                SideId::Attacker => unit.on_wall && near.is_some_and(|(_, d)| d < band + 3.0),
            };
        }
        // Siege towers dock against the wall they touch; SB: the "under
        // attack" marks wear off.
        for piece in works.pieces.iter_mut() {
            piece.docked_tower = None;
            piece.attacked_for = (piece.attacked_for - DT).max(0.0);
        }
        for unit in self.units.iter() {
            if !unit.siege_tower() || !unit.able() {
                continue;
            }
            if let Some((p, d)) = works.nearest_intact(unit.x, unit.z) {
                if d < band + 4.0 && works.pieces[p].kind == PieceKind::Wall {
                    works.pieces[p].docked_tower = Some(unit.id);
                }
            }
        }
        // The ram batters the gate, one blow every `RAM_PERIOD` seconds (SG1).
        let gate = works.gate;
        let mut blows: Vec<(u32, bool)> = Vec::new();
        for (index, unit) in self.units.iter_mut().enumerate() {
            if !unit.ram {
                continue;
            }
            let at_gate = unit.able()
                && works.pieces[gate].intact()
                && works.pieces[gate].distance(unit.x, unit.z) < band + 4.0;
            if at_gate {
                works.pieces[gate].mark_attacked(siege::UNDER_ATTACK_CONTACT_S);
            }
            let Some(seconds) = Self::ram_blow(&mut self.assault.ram_timers, index, at_gate) else {
                continue;
            };
            {
                let crew = unit.hp / f64::from(unit.initial_soldiers.max(1));
                works.pieces[gate].hp -=
                    siege::SiegeWorkRules::bundled().ram.damage_per_s * crew * seconds;
                blows.push((unit.id, works.pieces[gate].hp <= 0.0));
                if works.pieces[gate].hp <= 0.0 {
                    works.pieces[gate].hp = 0.0;
                    logs.push((
                        "La porte cède sous les coups du bélier !".to_owned(),
                        Some(SideId::Attacker),
                    ));
                }
            }
        }
        for (text, side) in logs {
            self.log(text, side);
        }
        for (unit, breached) in blows {
            self.push_fx(crate::siege_fx::SiegeFxKind::RamStrike {
                unit,
                piece: gate,
                breached,
            });
        }
    }

    fn resolve_movement(&mut self, contacts: &[Vec<usize>]) {
        for i in 0..self.units.len() {
            if !self.units[i].present() {
                continue;
            }
            let state = self.units[i].state;
            // Routing: flee towards the own edge. EP10: in a field battle,
            // straight to the rear of the army, swerving round the enemies
            // and the friends on the way; in a siege, also away from the
            // nearest enemy (unchanged).
            if state == UnitState::Routing {
                let (fx, fz) = self.flight_direction(i);
                let (x, z) = (self.units[i].x, self.units[i].z);
                self.advance(i, x + fx * 50.0, z + fz * 50.0, true);
                self.check_left_field(i);
                continue;
            }
            if self.units[i].withdrawing {
                if let Some((tx, tz)) = self.units[i].destination {
                    // In a siege, round the walls and the houses (F5a
                    // pathing; no ladders on the way out, and the wall
                    // walk is left straight inwards as before).
                    let (gx, gz) = if self.siege.is_some() && !self.units[i].on_wall {
                        self.grid_route(i, tx, tz).unwrap_or((tx, tz))
                    } else {
                        (tx, tz)
                    };
                    self.advance(i, gx, gz, true);
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
            if let Some(piece) = self.units[i].climbing {
                self.progress_climb(i, piece, in_contact);
                continue;
            }
            if in_contact && !self.units[i].disengaging {
                self.enter_melee(i, &contacts[i]);
                continue;
            }
            if !in_contact {
                self.units[i].disengaging = false;
            }
            if let Some(target) = self.units[i].target {
                let t = target as usize;
                // CB-M3: a target gone ends the attack; with orders queued,
                // so does a target in flight (the next order starts).
                let fleeing = self.units[t].state == UnitState::Routing
                    && !self.units[i].order_queue.is_empty();
                // CB2: a regiment on guard does not pursue.
                if !self.units[t].present() || fleeing || self.guard_releases(i, t, in_contact) {
                    self.units[i].target = None;
                    self.units[i].state = UnitState::Idle;
                    self.next_queued(i);
                    continue;
                }
                let (tx, tz) = (self.units[t].x, self.units[t].z);
                let unit = &self.units[i];
                let dist = ((tx - unit.x).powi(2) + (tz - unit.z).powi(2)).sqrt();
                // A target hidden (forest, walls, crest) is closed in on,
                // not waited for within range.
                if unit.shoots()
                    && unit.ammo > 0
                    && dist <= self.effective_range(unit, tx, tz)
                    && self.visible(unit, &self.units[t], dist)
                {
                    let unit = &mut self.units[i];
                    unit.state = UnitState::Shooting;
                    unit.facing = turn_towards(
                        unit.facing,
                        angle_to(tx - unit.x, tz - unit.z),
                        Self::turn_rate(unit) * 2.0,
                    );
                    continue;
                }
                if unit.pavise.is_some() {
                    // Behind the pavises: wait for the target to come in range.
                    if self.units[i].state != UnitState::Shooting {
                        self.units[i].state = UnitState::Idle;
                    }
                    continue;
                }
                let charge_distance = if unit.is_cavalry() { 120.0 } else { 40.0 };
                let charging = unit.running && dist < charge_distance && !unit.shoots();
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
                let (gx, gz) = self.route(i, tx, tz);
                self.advance(i, gx, gz, false);
                self.start_climb(i);
                continue;
            }
            if let Some((tx, tz)) = self.units[i].destination {
                let (gx, gz) = self.route(i, tx, tz);
                self.advance(i, gx, gz, false);
                self.start_climb(i);
                let unit = &self.units[i];
                let remaining = ((tx - unit.x).powi(2) + (tz - unit.z).powi(2)).sqrt();
                if self.units[i].climbing.is_some() {
                    continue;
                }
                if remaining < 1.5 {
                    let unit = &mut self.units[i];
                    unit.destination = None;
                    unit.running = false;
                    unit.state = UnitState::Idle;
                    if let Some(facing) = unit.destination_facing.take() {
                        unit.facing = facing;
                    }
                    self.next_queued(i);
                } else if !self.units[i].disengaging {
                    // Out of contact after breaking off a melee: marching
                    // again (still `Melee` while pulling out under blows).
                    self.units[i].state = UnitState::Marching;
                }
                continue;
            }
            // CB-M3: an order cut short (a melee halts the march) leaves
            // the queue: the next order starts.
            if !self.units[i].order_queue.is_empty() && self.next_queued(i) {
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

    /// EP10: unit vector from the front to the rear of `side`'s army (the
    /// attacker holds the low-z edge, the defender the high-z one, on the
    /// generated fields as on the historical maps).
    fn rear_of(side: SideId) -> (f64, f64) {
        match side {
            SideId::Attacker => (0.0, -1.0),
            SideId::Defender => (0.0, 1.0),
        }
    }

    /// Direction of flight of the routing regiment `i` (not normalised in
    /// a siege, as before EP10).
    fn flight_direction(&self, i: usize) -> (f64, f64) {
        let unit = &self.units[i];
        let rear = Self::rear_of(unit.side);
        if self.siege.is_some() {
            let (mut fx, mut fz) = rear;
            if let Some((j, d)) = self.nearest_enemy(i, false) {
                if d > 1e-6 {
                    let e = &self.units[j];
                    fx += (unit.x - e.x) / d;
                    fz += (unit.z - e.z) / d;
                }
            }
            return (fx, fz);
        }
        let reach = self
            .rout
            .flight
            .lookahead_m
            .max(self.rout.flight.friend_lookahead_m);
        let near = |u: &Unit| (u.x - unit.x).abs() < reach && (u.z - unit.z).abs() < reach;
        let enemies = self
            .units
            .iter()
            .filter(|u| u.side != unit.side && u.able() && near(u))
            .map(|u| (u.x, u.z));
        let friends = self
            .units
            .iter()
            .enumerate()
            .filter(|&(j, u)| j != i && u.side == unit.side && u.able() && near(u))
            .map(|(_, u)| (u.x, u.z));
        self.rout
            .flight
            .direction((unit.x, unit.z), rear, enemies, friends)
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
        let mass = impact::charge_mass(&self.units[i]);
        let heading = angle_to(
            self.units[p].x - self.units[i].x,
            self.units[p].z - self.units[i].z,
        );
        let point = self.contact_point(i, p);
        let base = ImpactEvent {
            time: self.elapsed,
            attacker: self.units[i].id,
            defender: self.units[p].id,
            kind: ImpactKind::Shock,
            point,
            heading,
            mass,
            knocked: 0,
            unhorsed: 0,
            depth: 0.0,
            cohesion: 0.0,
        };
        if cavalry && angle == 0 && self.units[p].stakes_planted {
            let defender_id = self.units[p].id;
            let unit = &mut self.units[i];
            let loss = unit.hp * 0.12;
            unit.hp -= loss;
            unit.tick_losses += loss;
            unit.morale -= 15.0;
            unit.charge_timer = 0.0;
            unit.loss_cause = LossCause::Stakes;
            unit.loss_by = Some(defender_id);
            let text = format!("Les {} s'empalent sur les pieux !", self.unit_label(i));
            let side = self.units[i].side;
            self.log(text, Some(side));
            self.record_impact(ImpactEvent {
                kind: ImpactKind::Stakes,
                unhorsed: loss.round() as u32,
                ..base
            });
            return;
        }
        // BV2: levelled pikes stop the horses and unhorse the first riders
        // (CB4: also pikes planted against a frontal charge).
        if impact::pikes_stop(&self.units[i], &self.units[p], angle)
            || self.ability_stops_charge(&self.units[i], &self.units[p], angle)
        {
            let defender_id = self.units[p].id;
            let unit = &mut self.units[i];
            let loss = unit.hp * impact::PIKE_STOP_LOSS;
            unit.hp -= loss;
            unit.tick_losses += loss;
            unit.morale -= 10.0;
            unit.charge_timer = 0.0;
            unit.loss_cause = LossCause::Pikes;
            unit.loss_by = Some(defender_id);
            let text = format!(
                "La charge des {} se brise sur les piques des {} !",
                self.unit_label(i),
                self.unit_label(p)
            );
            let side = self.units[i].side;
            self.log(text, Some(side));
            self.record_impact(ImpactEvent {
                kind: ImpactKind::Pikes,
                unhorsed: loss.round() as u32,
                ..base
            });
            return;
        }
        // B5: a hedge or a ditch in front of the target, or the lanes of a
        // village, break the impact of horsemen.
        let (from, to) = (
            (self.units[i].x, self.units[i].z),
            (self.units[p].x, self.units[p].z),
        );
        let water = if cavalry {
            self.water_breaks_charge(from, to)
        } else {
            None
        };
        let decor = if cavalry {
            self.field.decor_breaks_charge(to.0, to.1)
        } else {
            None
        };
        if cavalry
            && (self.field.breaks_charge(from, to)
                || self.field.in_village(to.0, to.1)
                || water.is_some()
                || decor.is_some())
        {
            self.units[i].charge_timer = 0.0;
            self.units[i].morale -= 5.0;
            let text = if let Some(how) = water {
                format!("La charge des {} {how}.", self.unit_label(i))
            } else if self.field.in_village(to.0, to.1) {
                format!(
                    "La charge des {} se brise dans le village.",
                    self.unit_label(i)
                )
            } else if let Some(place) = decor {
                format!("La charge des {} se brise {place}.", self.unit_label(i))
            } else {
                format!("La charge des {} se brise sur la haie.", self.unit_label(i))
            };
            let side = self.units[i].side;
            self.log(text, Some(side));
            self.record_impact(ImpactEvent {
                kind: ImpactKind::Broken,
                ..base
            });
            return;
        }
        self.units[i].charge_timer = CHARGE_IMPACT;
        // Loss of cohesion: the shock of the horses (B-rules, unchanged).
        // CB4: close ranks take the shock better.
        let braced = self.ability_charge_taken(&self.units[p]);
        let cohesion = if cavalry && self.units[p].formation != Formation::Square {
            impact::shock_morale(angle) * braced
        } else {
            0.0
        };
        self.units[p].morale -= cohesion;
        // BV2: the horses knock men down; they stop fighting until they are
        // back on their feet.
        let knocked = if cavalry && self.units[i].mounted {
            let count = impact::knocked_count(&self.units[i], &self.units[p], angle);
            if braced == 1.0 {
                count
            } else {
                (f64::from(count) * braced).round() as u32
            }
        } else {
            0
        };
        let depth = impact::drive_depth(mass, knocked, &self.units[p]);
        if knocked > 0 {
            let target = &mut self.units[p];
            target.knocked = f64::from(knocked);
            target.knocked_timer = impact::KNOCKDOWN_TIME;
        }
        self.record_impact(ImpactEvent {
            knocked,
            depth,
            cohesion,
            ..base
        });
    }

    /// Point where regiment `i` strikes regiment `p`: on `p`'s face, along
    /// the line between the two centres.
    fn contact_point(&self, i: usize, p: usize) -> (f64, f64) {
        let (a, b) = (&self.units[i], &self.units[p]);
        let (dx, dz) = (a.x - b.x, a.z - b.z);
        let len = (dx * dx + dz * dz).sqrt();
        if len < 1e-6 {
            return (b.x, b.z);
        }
        let dir = (dx / len, dz / len);
        let reach = b.support(dir).min(len);
        (b.x + dir.0 * reach, b.z + dir.1 * reach)
    }

    /// The enemy `i` strikes among `contacts`: its target, else the nearest.
    /// The contacts are those of the start of the step: regiments killed
    /// since (missiles, fire) are skipped.
    fn primary_opponent(&self, i: usize, contacts: &[usize]) -> Option<usize> {
        let alive = |j: usize| self.units[j].present();
        if let Some(t) = self.units[i].target {
            if contacts.contains(&(t as usize)) && alive(t as usize) {
                return Some(t as usize);
            }
        }
        let unit = &self.units[i];
        contacts
            .iter()
            .copied()
            .filter(|&j| alive(j))
            .min_by(|&a, &b| {
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

    pub(crate) fn defense_points(&self, unit: &Unit) -> f64 {
        f64::from(unit.stats.armor)
            + self
                .general_bonus(unit.side)
                .map_or(0.0, |g| g.defense_percent)
    }

    fn resolve_shooting(&mut self, contacts: &[Vec<usize>]) {
        for i in 0..self.units.len() {
            let unit = &self.units[i];
            if !unit.present()
                || !unit.shoots()
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
            if moving && !unit.shoots_on_move() {
                continue;
            }
            if self.units[i].reload > 0.0 {
                self.units[i].reload -= DT;
                continue;
            }
            if let Some(piece) = self.pick_wall_target(i) {
                self.fire_at_wall(i, piece);
                continue;
            }
            // CB2: an engine battering walls never shoots men.
            if self.units[i].breach {
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

    /// CB3: does any present, standing regiment of `side` see `target`? Reuses the missile-arc
    /// spotter range (`data/rules/missile_arc.json`, `spotter_range_m`, already the distance at
    /// which a friend directs an indirect volley) and the forest/wall/line-of-sight checks of
    /// [`Self::visible`]. Purely derived from the current state (not a field of [`Unit`]): it
    /// never enters `state_digest` or the replay format. Feeds the tactical view's fog of war
    /// (`spotted` in `get_units`); the normal view is unaffected.
    pub fn spotted_by(&self, target: &Unit, side: SideId) -> bool {
        if target.side == side {
            return true;
        }
        let rules = crate::missile_arc::MissileArcRules::bundled();
        let reach = rules.spotter_range_m * self.range_factor();
        self.units.iter().any(|u| {
            u.side == side && u.present() && !u.synthetic && u.state != UnitState::Routing && {
                let dist = (u.x - target.x).hypot(u.z - target.z);
                dist <= reach && self.visible(u, target, dist)
            }
        })
    }

    pub(crate) fn visible(&self, shooter: &Unit, target: &Unit, dist: f64) -> bool {
        if self.field.in_forest(target.x, target.z) && dist > 60.0 {
            return false;
        }
        if let Some(works) = &self.siege {
            // Walls hide what is behind them, except from (or of) the wall
            // walk; engines lob over them.
            if shooter.category != UnitCategory::Siege
                && !shooter.on_wall
                && !target.on_wall
                && target.climbing.is_none()
                && works
                    .crosses_intact((shooter.x, shooter.z), (target.x, target.z))
                    .is_some()
            {
                return false;
            }
        }
        // R4: direct at a target in sight, lobbed over a crest otherwise.
        self.fire_mode(shooter, target).is_some()
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
            // The ordered target cannot be shot (hidden, in a melee): at
            // will, the nearest enemy that can be.
            if !unit.fire_at_will {
                return None;
            }
        } else if !unit.fire_at_will {
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
        if shooter.on_wall {
            accuracy *= 1.25;
        }
        // CB4: the aimed shot aims better, above all at horses.
        let shooter_ability = self.ability_effects(shooter);
        if let Some(e) = shooter_ability {
            accuracy *= e.accuracy_factor;
        }
        let mut kills = shots * accuracy * f64::from(shooter.stats.ranged) / 100.0
            * armor_factor(self.defense_points(target))
            * RANGED_RATE;
        if let Some(e) = shooter_ability.filter(|_| target.mounted) {
            kills *= e.vs_mounted_factor;
        }
        if self.field.in_forest(target.x, target.z) {
            kills *= 0.5;
        }
        if self.field.in_village(target.x, target.z) {
            kills *= crate::site::VILLAGE_COVER;
        } else {
            // EP6: hamlets, churchyards, manors, orchards, vineyards, camps.
            kills *= self.field.decor_cover(target.x, target.z);
            if self
                .field
                .hedge_between((shooter.x, shooter.z), (target.x, target.z))
            {
                kills *= crate::site::HEDGE_COVER;
            }
        }
        kills *= self.missile_cover(target, attack_angle(target, shooter.x, shooter.z));
        if target.formation == Formation::Square {
            kills *= 1.2;
        }
        if target.on_wall && !shooter.on_wall {
            kills *= 0.5; // merlons
        }
        if self.on_ladders(target) {
            kills *= 1.5;
        }
        if target.ram || target.siege_tower() {
            kills *= 0.1; // roofed and hung with wet hides
        }
        if attack_angle(target, shooter.x, shooter.z) == 2 {
            kills *= 1.3;
        }
        kills *= self.smoke_factor(shooter, target);
        // R4: an indirect volley scatters over ground nobody aims at.
        let mode = self.fire_mode(shooter, target);
        kills *= mode.map_or(1.0, |m| {
            m.accuracy(crate::missile_arc::MissileArcRules::bundled())
        });
        let indirect = mode.is_some_and(|m| m.indirect());
        let aim = (target.x, target.z);
        let reload = shooter.reload_period() * shooter_ability.map_or(1.0, |e| e.reload_factor);
        let heading = angle_to(target.x - shooter.x, target.z - shooter.z);
        let kills = kills.min(self.units[t].hp);
        let cover = if target.on_wall {
            ShotCover::Wall
        } else if target.pavise.is_some()
            || (target.has(Ability::Pavise) && target.state != UnitState::Marching)
        {
            ShotCover::Pavise
        } else if target.stakes_planted {
            ShotCover::Stakes
        } else {
            ShotCover::None
        };
        let shot = ShotEvent {
            time: self.elapsed,
            shooter: shooter.id,
            target: Some(target.id),
            from: (shooter.x, shooter.z),
            aim: (target.x, target.z),
            missiles: Self::missiles(shooter),
            kills,
            kind: Self::missile_kind(shooter),
            incendiary: self.shoots_fire(i),
            cover,
            indirect,
        };
        self.record_shot(shot);
        if mode.is_some_and(|m| m != crate::missile_arc::FireMode::Remembered) {
            self.units[t].seen_at = self.elapsed;
        }
        let cause = Self::missile_cause(&self.units[i]);
        let shooter_id = self.units[i].id;
        self.units[t].hp -= kills;
        self.units[t].tick_losses += kills;
        self.units[i].kills += kills;
        if !self.units[t].synthetic {
            self.clock.missile_losses[self.units[t].side.index()] += kills;
        }
        // ADR 0052: the arrows wound the horses too, and they panic, unless
        // the riders are already locked in a melee.
        let target = &self.units[t];
        self.units[t].morale -= crate::missile_morale::MissileMoraleRules::bundled().panic(
            target.mounted && target.state != UnitState::Melee,
            kills,
            target.max_soldiers,
        );
        if kills > 0.0 {
            self.units[t].missile_timer = 0.0;
            self.units[t].loss_cause = cause;
            self.units[t].loss_by = Some(shooter_id);
        }
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
            let (x, z, id) = (self.units[i].x, self.units[i].z, self.units[i].id);
            self.alert(
                crate::alerts::AlertKind::AmmoOut,
                x,
                z,
                Some(side),
                Some(id),
            );
            if self.units[i].state == UnitState::Shooting {
                self.units[i].state = UnitState::Idle;
            }
        }
        if self.units[t].hp <= 0.0 {
            self.unit_destroyed(t);
        }
        self.incendiary_volley(i, aim);
    }

    /// Wall piece an engine batters this volley: its ordered piece, else
    /// (fire at will, no unit target) the nearest wall stretch in range
    /// facing it.
    fn pick_wall_target(&self, i: usize) -> Option<usize> {
        let works = self.siege.as_ref()?;
        let unit = &self.units[i];
        if !unit.wall_breaker() || (unit.target.is_some() && !unit.breach) {
            return None;
        }
        let range = f64::from(unit.stats.range) * self.range_factor();
        let in_range = |p: usize| {
            let piece = &works.pieces[p];
            piece.intact()
                && piece.distance(unit.x, unit.z) <= range
                && piece.outside_offset(unit.x, unit.z) > 0.0
        };
        if let Some(p) = unit.wall_target.filter(|&p| in_range(p)) {
            return Some(p);
        }
        if unit.breach {
            return self.breach_piece(i, range);
        }
        if !unit.fire_at_will {
            return None;
        }
        (0..works.pieces.len())
            .filter(|&p| works.pieces[p].kind == PieceKind::Wall && in_range(p))
            .min_by(|&a, &b| {
                let da = works.pieces[a].distance(unit.x, unit.z);
                let db = works.pieces[b].distance(unit.x, unit.z);
                da.total_cmp(&db).then(a.cmp(&b))
            })
    }

    /// An engine's shot at a wall piece; the defenders standing on it suffer
    /// a little, and all of them fall off when it comes down.
    fn fire_at_wall(&mut self, i: usize, piece: usize) {
        let unit = &self.units[i];
        let crew = unit.hp / f64::from(unit.initial_soldiers.max(1));
        let damage = f64::from(unit.stats.siege_attack.unwrap_or(0))
            * siege::SiegeWorkRules::bundled()
                .engine
                .wall_damage_per_siege_attack
            * crew
            // CB2: battering in breach (1 otherwise).
            * unit.breach_wall_damage();
        let heading = {
            let (mx, mz) = self.siege.as_ref().expect("siege").pieces[piece].midpoint();
            angle_to(mx - unit.x, mz - unit.z)
        };
        let aim = self.siege.as_ref().expect("siege").pieces[piece].midpoint();
        let shot = ShotEvent {
            time: self.elapsed,
            shooter: unit.id,
            target: None,
            from: (unit.x, unit.z),
            aim,
            missiles: Self::missiles(unit),
            kills: 0.0,
            kind: Self::missile_kind(unit),
            incendiary: self.shoots_fire(i),
            cover: ShotCover::Wall,
            indirect: false,
        };
        self.record_shot(shot);
        let shooter = &mut self.units[i];
        shooter.reload = crate::shot::ENGINE_RELOAD * shooter.breach_reload();
        shooter.ammo = shooter.ammo.saturating_sub(1);
        shooter.facing = turn_towards(shooter.facing, heading, 0.5);
        if shooter.state != UnitState::Marching {
            shooter.state = UnitState::Shooting;
        }
        let works = self.siege.as_mut().expect("siege");
        let band = works.band();
        let p = &mut works.pieces[piece];
        p.hp = (p.hp - damage).max(0.0);
        p.mark_attacked(siege::UNDER_ATTACK_SHOT_S);
        let breached = p.hp <= 0.0;
        let kind = p.kind;
        let p = works.pieces[piece].clone();
        let mut fell = Vec::new();
        for (j, u) in self.units.iter_mut().enumerate() {
            if !u.present() || !u.on_wall || p.distance(u.x, u.z) > band + 2.0 {
                continue;
            }
            let loss = if breached { u.hp * 0.12 } else { u.hp * 0.01 };
            u.hp -= loss;
            u.tick_losses += loss;
            if breached {
                u.on_wall = false;
                u.morale -= 10.0;
                fell.push(j);
            }
        }
        if breached {
            let text = match kind {
                PieceKind::Gate => "La porte vole en éclats !".to_owned(),
                PieceKind::Wall => {
                    "Un pan de muraille s'effondre : la brèche est ouverte !".to_owned()
                }
            };
            let side = self.units[i].side;
            self.log(text, Some(side));
        }
        for j in fell {
            if self.units[j].hp <= 0.0 {
                self.unit_destroyed(j);
            }
        }
        // SG1: where the stone struck (deterministic hash, no random draw).
        let id = self.units[i].id;
        let along = 0.15 + 0.7 * crate::siege_fx::hash01(self.ticks, u64::from(id));
        let height = 0.25 + 0.6 * crate::siege_fx::hash01(self.ticks ^ 0x5eed, u64::from(id));
        self.push_fx(crate::siege_fx::SiegeFxKind::EngineShot {
            unit: id,
            piece,
            x: p.a.0 + (p.b.0 - p.a.0) * along,
            z: p.a.1 + (p.b.1 - p.a.1) * along,
            height,
            breached,
        });
        self.incendiary_volley(i, p.midpoint());
    }

    fn melee_damage(&self, attacker: &Unit, defender: &Unit) -> f64 {
        let mut damage = attacker.fighting_soldiers() * f64::from(attacker.stats.melee) / 100.0
            * armor_factor(self.defense_points(defender))
            * MELEE_RATE
            * DT
            // EP6: walls, hedges and houses of the decor shelter the defender.
            / self.field.decor_defense(defender.x, defender.z);
        if attacker.charge_timer > 0.0 {
            let charge = attacker.charge_points();
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
        let angle = attack_angle(defender, attacker.x, attacker.z);
        damage *= match angle {
            0 => 1.0,
            1 => 1.5,
            _ => 2.0,
        };
        // CB4: close ranks, planted pikes.
        damage *= self.ability_melee_factor(attacker, defender, angle);
        if defender.state == UnitState::Routing {
            damage *= 1.5;
        }
        // Matchup of the types (CB-M2: also quoted by the hover comparison).
        damage *= horse_against_foot(attacker, defender);
        damage *= pikes_against_horse(attacker, defender);
        // Siege: ladders are a poor place to fight from.
        if self.on_ladders(attacker) {
            damage *= 0.3;
        } else if attacker.climbing.is_some() {
            damage *= 0.9;
        }
        if self.on_ladders(defender) {
            damage *= 2.0;
        }
        // EP3: fords, streams, deep water, bridges and bridgeheads.
        damage *= self.water_melee_factor(attacker, defender);
        // SG4: downhill strikes harder, uphill weaker (`battle_crest.json`).
        damage *= crate::crest::CrestRules::bundled().melee_factor(
            self.field.height(attacker.x, attacker.z) - self.field.height(defender.x, defender.z),
        );
        damage *= 1.0 - attacker.fatigue / 250.0;
        damage *= 1.0 + f64::from(attacker.experience) / 20.0;
        damage *= 0.6 + attacker.morale.max(0.0) / 250.0;
        // EP5: without its standard the regiment loses its rallying point.
        if matches!(attacker.standard, crate::unit::StandardState::Fallen { .. }) {
            damage *= self.standard_rules.fallen_melee_factor;
        }
        // CV3-2: the palisade of an entrenched camp shelters its defenders.
        damage /= self.palisade_defense(attacker, defender);
        // EP11: compression and wrapping files (`sim/push.rs`).
        damage * self.push_melee_factor(attacker, defender)
    }

    fn resolve_melee(&mut self, contacts: &[Vec<usize>]) {
        let n = self.units.len();
        let mut damage = vec![0.0; n];
        let mut flanked = vec![0u8; n];
        // CB5: rising edge of `flanked` (0 -> non-zero), collected here and
        // turned into alerts once the loop below has released its `&mut`
        // borrow of `self.units`.
        let mut newly_flanked: Vec<(f64, f64, SideId, u32)> = Vec::new();
        // BV2: heaviest blow per defender this tick (cause of its deaths).
        let mut heaviest: Vec<Option<(f64, usize)>> = vec![None; n];
        // UB1: (striker, victim, damage) to credit the kills once capped.
        let mut credit: Vec<(usize, usize, f64)> = Vec::new();
        let mut dealt_ratio = vec![0.0; n];
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
            let dealt = self.melee_damage(unit, defender);
            damage[p] += dealt;
            if heaviest[p].is_none_or(|(d, _)| dealt > d) {
                heaviest[p] = Some((dealt, i));
            }
            credit.push((i, p, dealt));
            match attack_angle(defender, unit.x, unit.z) {
                1 => flanked[p] |= 1,
                2 => flanked[p] |= 2,
                _ => {}
            }
        }
        let causes: Vec<Option<(LossCause, u32)>> = heaviest
            .iter()
            .map(|h| {
                h.map(|(_, a)| {
                    let attacker = &self.units[a];
                    let cause = if attacker.charge_timer > 0.0 {
                        LossCause::Charge
                    } else {
                        LossCause::Melee
                    };
                    (cause, attacker.id)
                })
            })
            .collect();
        for i in 0..n {
            let unit = &mut self.units[i];
            if unit.charge_timer > 0.0 {
                unit.charge_timer -= DT;
            }
            if unit.knocked_timer > 0.0 {
                unit.knocked_timer -= DT;
                if unit.knocked_timer <= 0.0 {
                    unit.knocked = 0.0;
                }
            }
            if flanked[i] != 0 {
                if !self.flanked_alerted[i] {
                    self.flanked_alerted[i] = true;
                    newly_flanked.push((unit.x, unit.z, unit.side, unit.id));
                }
            } else {
                self.flanked_alerted[i] = false;
            }
            unit.flanked = flanked[i];
            if damage[i] <= 0.0 || !unit.present() {
                continue;
            }
            let before = unit.hp;
            let dealt = damage[i].min(unit.hp);
            dealt_ratio[i] = dealt / damage[i];
            unit.hp -= dealt;
            unit.tick_losses += dealt;
            if let Some((cause, by)) = causes[i] {
                unit.loss_cause = cause;
                unit.loss_by = Some(by);
            }
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
        for (striker, victim, blow) in credit {
            self.units[striker].kills += blow * dealt_ratio[victim];
        }
        for (x, z, side, id) in newly_flanked {
            self.alert(
                crate::alerts::AlertKind::Flanked,
                x,
                z,
                Some(side),
                Some(id),
            );
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
        let general_pos = self
            .units
            .iter()
            .find(|u| u.side == side && u.is_general)
            .map(|u| (u.x, u.z, u.id));
        if let Some((x, z, id)) = general_pos {
            self.alert(
                crate::alerts::AlertKind::GeneralDown,
                x,
                z,
                Some(side),
                Some(id),
            );
        }
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
        // Rout and rally events, worded after the loop (the labels are only
        // formatted for the few regiments concerned).
        let mut new_events: Vec<(usize, &'static str)> = Vec::new();
        let siege = self.siege.is_some();
        // T4 (ADR 0104): the garrison's last stand on the square.
        let stand = &crate::capture::CaptureRules::bundled().last_stand;
        let last_stand: Vec<bool> = self
            .units
            .iter()
            .map(|u| siege && u.present() && self.in_last_stand(u))
            .collect();
        for i in 0..n {
            if !self.units[i].present() {
                continue;
            }
            let unit = &mut self.units[i];
            let mut morale = unit.morale;
            let (stand_loss, stand_contagion) = if last_stand[i] {
                (stand.loss_morale_factor, stand.contagion_factor)
            } else {
                (1.0, 1.0)
            };
            // Behind battlements the garrison takes its losses more calmly.
            let cover = if unit.ram || unit.siege_tower() {
                0.3
            } else if unit.on_wall {
                0.7
            } else {
                1.0
            };
            morale -= unit.tick_losses / f64::from(unit.max_soldiers)
                * LOSS_MORALE_FACTOR
                * cover
                * stand_loss;
            if unit.flanked & 1 != 0 {
                morale -= 1.5 * DT;
            }
            if unit.flanked & 2 != 0 {
                morale -= 3.0 * DT;
            }
            if unit.fatigue > EXHAUSTED_FATIGUE {
                morale -= (unit.fatigue - EXHAUSTED_FATIGUE) * 0.02 * DT;
            }
            if unit.state == UnitState::Melee && unit.hp < f64::from(unit.max_soldiers) * 0.5 {
                morale -= 0.3 * DT;
            }
            // EP10: routing friends weigh by where they are (fully beside or
            // in front, little once behind and running away); in a siege
            // every one within reach counts fully (unchanged).
            let contagion = &self.rout.contagion;
            let forward = {
                let (rx, rz) = Self::rear_of(unit.side);
                (-rx, -rz)
            };
            let mut routing_weight = 0.0;
            let mut nearest_enemy = f64::INFINITY;
            for (j, &(side, x, z, routing, able)) in snapshot.iter().enumerate() {
                if j == i {
                    continue;
                }
                let (dx, dz) = (x - unit.x, z - unit.z);
                if side == unit.side {
                    if routing {
                        routing_weight += if siege {
                            f64::from(u8::from(
                                dx * dx + dz * dz < contagion.radius_m * contagion.radius_m,
                            ))
                        } else {
                            contagion.weight(dx, dz, forward)
                        };
                    }
                } else if able {
                    nearest_enemy = nearest_enemy.min((dx * dx + dz * dz).sqrt());
                }
            }
            morale -= contagion.morale_rate(routing_weight) * DT * stand_contagion;
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
            } else if unit.on_wall && unit.side == SideId::Defender {
                // Behind their walls the burghers stand firm.
                if morale < unit.morale_cap {
                    morale = (morale + 0.2 * DT + aura).min(unit.morale_cap);
                }
            } else if last_stand[i] {
                // T4: the last stand, even in the melee.
                if morale < unit.morale_cap {
                    morale = (morale + stand.morale_per_s * DT + aura).min(unit.morale_cap);
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
                UnitState::Melee | UnitState::Climbing => 0.3,
                UnitState::Routing => 0.3,
            };
            if rate > 0.0 {
                // CB2: a run under the run mode (1 in the bundled data).
                rate *= unit.run_mode_fatigue();
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
                unit.order_queue.clear();
                unit.stakes_planted = false;
                unit.pavise = None;
                unit.charge_timer = 0.0;
                unit.climbing = None;
                unit.climb_progress = 0.0;
                if std::mem::take(&mut unit.on_wall) {
                    new_events.push((i, "abandonnent le rempart !"));
                }
                new_events.push((i, "sont en déroute !"));
            } else if unit.state == UnitState::Routing
                && unit.morale > RALLY_MORALE
                && nearest_enemy > RALLY_SAFE_DISTANCE
                && unit.hp >= f64::from(unit.max_soldiers) * 0.2
            {
                unit.state = UnitState::Rallied;
                unit.rally_timer = RALLY_PAUSE;
                new_events.push((i, "se rallient."));
            }
        }
        for (i, what) in new_events {
            let text = format!("Les {} {what}", self.unit_label(i));
            let side = self.units[i].side;
            self.log(text, Some(side));
            if what == "sont en déroute !" {
                let (x, z, id) = (self.units[i].x, self.units[i].z, self.units[i].id);
                self.alert(crate::alerts::AlertKind::Rout, x, z, Some(side), Some(id));
            }
        }
    }

    fn check_end(&mut self) {
        if !self.end_conditions {
            return;
        }
        let able = SideId::BOTH.map(|side| {
            self.units
                .iter()
                .filter(|u| u.side == side && u.able() && !u.synthetic)
                .count()
        });
        let timeout = self.elapsed >= MAX_DURATION - 1e-9;
        // T4 (ADR 0104): the market square held long enough.
        let square_held = self.square_taken();
        let (winner, end) = if able[0] > 0 && able[1] > 0 && !timeout && !square_held {
            // EP9: a broken army, a refused battle or a lull.
            match self.field_decision() {
                Some(decision) => decision,
                None => return,
            }
        } else {
            let winner = if (able[1] == 0 && able[0] > 0) || (square_held && able[0] > 0) {
                SideId::Attacker
            } else {
                SideId::Defender
            };
            let end = if square_held && winner == SideId::Attacker && able[1] > 0 {
                BattleEnd::SquareHeld
            } else if timeout && able[0] > 0 && able[1] > 0 {
                BattleEnd::Nightfall
            } else {
                BattleEnd::Rout
            };
            (winner, end)
        };
        self.apply_decision(winner.other(), end);
        self.end = Some(end);
        self.finished = true;
        self.winner = Some(winner);
        self.return_ram_crews();
        self.collect_field_standards(winner);
        let loser = winner.other();
        if self.general_alive[loser.index()] {
            if let Some(unit) = self.units.iter().find(|u| u.side == loser && u.is_general) {
                let (gx, gz, gid) = (unit.x, unit.z, unit.id);
                let caught = if unit.left_field {
                    unit.state == UnitState::Routing && !unit.withdrawing
                } else {
                    unit.state == UnitState::Routing
                };
                let chance = if unit.left_field { 0.2 } else { 0.35 };
                let taken = caught && self.rng.unit() < chance;
                if taken && self.no_quarter[winner.index()] {
                    // No quarter: the victors take no prisoner, not even for ransom.
                    self.kill_general(loser);
                } else if taken {
                    self.general_captured[loser.index()] = true;
                    let name = self
                        .setup
                        .side(loser)
                        .general
                        .as_ref()
                        .map_or_else(|| "Le général".to_owned(), |g| g.name.clone());
                    self.log(format!("{name} est fait prisonnier."), Some(loser));
                    self.alert(
                        crate::alerts::AlertKind::GeneralDown,
                        gx,
                        gz,
                        Some(loser),
                        Some(gid),
                    );
                }
            }
        }
        let winner_name = self.setup.side(winner).faction_name.clone();
        let loser_name = self.setup.side(loser).faction_name.clone();
        let text = match end {
            BattleEnd::SquareHeld => {
                format!("{winner_name} tient la place centrale : la ville est prise !")
            }
            BattleEnd::Nightfall => format!("La nuit tombe : {winner_name} tient le terrain."),
            BattleEnd::Refused => format!(
                "Personne n'engage le combat : {loser_name} renonce et se retire, \
                 {winner_name} garde le champ. Bataille refusée."
            ),
            BattleEnd::Lull => {
                format!("Le combat cesse : {loser_name} se retire, {winner_name} garde le champ.")
            }
            BattleEnd::Broken => format!(
                "L'armée {} est brisée : déroute générale ! Victoire {} !",
                of_faction(&loser_name),
                of_faction(&winner_name)
            ),
            BattleEnd::Rout => format!("Victoire {} !", of_faction(&winner_name)),
        };
        self.log(text, Some(winner));
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
