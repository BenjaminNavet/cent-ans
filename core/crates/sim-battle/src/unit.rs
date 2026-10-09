//! Battle regiments: state, formation geometry and soldier positions.

use data_model::key_enum;
use std::collections::VecDeque;

use data_model::{Ability, Missile, UnitCategory, UnitStats};
use serde::{Deserialize, Serialize};

use crate::impact::LossCause;
use crate::queue::QueuedOrder;
use crate::rng::jitter;
use crate::setup::{SideId, UnitSetup};

pub use crate::formations::{Formation, Reform};
use crate::formations::{FormationRules, FormationShape};

key_enum! {
/// What a regiment is doing (spec § 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitState {
    Idle => "idle",
    Marching => "marching",
    Charging => "charging",
    Melee => "melee",
    Shooting => "shooting",
    Routing => "routing",
    /// Just rallied; back to `Idle` after a few seconds.
    Rallied => "rallied",
    /// Scaling a town wall (ladders or siege tower bridge).
    Climbing => "climbing",
}
}

key_enum! {
/// What became of a regiment, as shown on the end-of-battle screen (Q2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitFate {
    /// No soldier left.
    Destroyed => "destroyed",
    /// Fleeing, on the field or already off it.
    Routed => "routed",
    /// Ordered off the field (retreat or general retreat), in good order.
    Withdrawn => "withdrawn",
    /// Never committed (reinforcement waiting off the field).
    Reserve => "reserve",
    /// Still standing on the field.
    Held => "held",
}
}

/// The regiment's standard (lot EP5, ADR 0034).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum StandardState {
    /// Flying above the regiment, held by its bearer.
    #[default]
    Carried,
    /// On the ground at `(x, z)`; raised or taken when `timer` runs out.
    Fallen { x: f64, z: f64, timer: f64 },
    /// Taken by the enemy regiment `by` (a trophy).
    Captured { by: u32 },
    /// Left on the ground at `(x, z)` when the regiment fled or perished and
    /// no enemy was there to take it (the victor collects it at the end).
    Lost { x: f64, z: f64 },
}

impl StandardState {
    pub fn key(self) -> &'static str {
        match self {
            StandardState::Carried => "carried",
            StandardState::Fallen { .. } => "fallen",
            StandardState::Captured { .. } => "captured",
            StandardState::Lost { .. } => "lost",
        }
    }

    /// Ground position of a fallen or lost standard.
    pub fn ground(self) -> Option<(f64, f64)> {
        match self {
            StandardState::Fallen { x, z, .. } | StandardState::Lost { x, z } => Some((x, z)),
            _ => None,
        }
    }
}

/// BA9: what a figure layout depends on (formation, width, scale, head
/// count and the unit class that picks ranks and width bounds).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct LayoutKey {
    formation: &'static str,
    line_files: Option<u32>,
    scale_bits: u64,
    figures: u32,
    soldiers: u32,
    siege: bool,
    ranged: bool,
    mounted: bool,
    pikemen: bool,
}

/// Layouts kept per thread; emptied when full (a battle uses a few dozen).
const LAYOUT_CACHE_MAX: usize = 512;

type FigureLayout = std::rc::Rc<(Vec<(f64, f64)>, f64)>;

thread_local! {
    static LAYOUT_CACHE: std::cell::RefCell<std::collections::HashMap<LayoutKey, FigureLayout>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// A regiment on the field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Unit {
    pub id: u32,
    pub side: SideId,
    /// Index in the side's [`crate::SideSetup::units`].
    pub setup_index: usize,
    pub unit_type: std::sync::Arc<str>,
    pub name: std::sync::Arc<str>,
    pub category: UnitCategory,
    pub mounted: bool,
    pub stats: UnitStats,
    pub abilities: std::sync::Arc<[Ability]>,
    /// Missile loosed by a shooting unit (lot UR2: data-driven, see
    /// `BattleSim::missile_kind`).
    #[serde(default)]
    pub missile: Option<Missile>,
    pub experience: u8,
    /// Soldiers at the start of the battle.
    pub initial_soldiers: u32,
    pub max_soldiers: u32,
    /// Living soldiers as a float (fractional casualties accumulate).
    pub hp: f64,
    pub x: f64,
    pub z: f64,
    /// Facing angle in radians; the front points to `(sin, cos)` (0 = +z).
    pub facing: f64,
    pub formation: Formation,
    pub state: UnitState,
    pub morale: f64,
    /// Morale the unit recovers towards (campaign morale + general bonus).
    pub morale_cap: f64,
    pub fatigue: f64,
    pub ammo: u32,
    /// Explicit attack target.
    pub target: Option<u32>,
    /// Move destination.
    pub destination: Option<(f64, f64)>,
    /// Facing to take on arrival.
    pub destination_facing: Option<f64>,
    pub running: bool,
    pub fire_at_will: bool,
    pub is_general: bool,
    pub withdrawing: bool,
    /// Ordered to move out of a melee: keeps moving while in contact.
    pub disengaging: bool,
    /// Left the field (routed off or withdrawn); survivors return to the campaign.
    pub left_field: bool,
    /// Remaining seconds of charge impact bonus.
    pub charge_timer: f64,
    /// Seconds before the next volley.
    pub reload: f64,
    /// Seconds spent standing still (archers plant stakes after a while).
    pub still_time: f64,
    pub stakes_planted: bool,
    /// Seconds left in the `Rallied` state.
    pub rally_timer: f64,
    /// Siege battles: standing on the wall walk (shooting and cover bonus).
    #[serde(default)]
    pub on_wall: bool,
    /// Siege battles: wall piece being scaled and progress (0-1).
    #[serde(default)]
    pub climbing: Option<usize>,
    #[serde(default)]
    pub climb_progress: f64,
    /// Siege battles: wall piece an engine is ordered to batter.
    #[serde(default)]
    pub wall_target: Option<usize>,
    /// Battle-only regiment (the ram): not a campaign unit, no losses reported.
    #[serde(default)]
    pub synthetic: bool,
    /// F5d: waiting off the field (beyond the regiments a side may field at
    /// once); not [`Unit::present`] until it marches in.
    #[serde(default)]
    pub reserve: bool,
    /// The battering ram.
    #[serde(default)]
    pub ram: bool,
    /// Wall piece that stopped the last move (siege battles).
    #[serde(skip)]
    pub blocked_by: Option<usize>,
    /// Casualties taken during the current tick (for morale).
    #[serde(skip)]
    pub tick_losses: f64,
    /// UB1: enemy soldiers this unit struck down (volleys and melee), for
    /// the result screen only; no rule reads it.
    #[serde(default)]
    pub kills: f64,
    /// Attacked on the flank / rear during the current tick.
    #[serde(skip)]
    pub flanked: u8,
    /// Leader's orders (F10b): temporary morale added by a war cry (also
    /// added to `morale_cap`) and the seconds it still lasts.
    #[serde(default)]
    pub order_morale: f64,
    #[serde(default)]
    pub order_morale_timer: f64,
    /// Pavises raised: multiplier of the missile casualties taken; the
    /// regiment stands still until its next move order.
    #[serde(default)]
    pub pavise: Option<f64>,
    /// Seconds since the regiment last took missile casualties.
    #[serde(default = "never")]
    pub missile_timer: f64,
    /// Heavy horse fighting on foot (order or siege assault).
    #[serde(default)]
    pub dismounted: bool,
    /// BV2: cause of the latest casualties and the regiment that inflicted them.
    #[serde(default)]
    pub loss_cause: LossCause,
    #[serde(default)]
    pub loss_by: Option<u32>,
    /// BV2: men knocked down by a charge (not fighting) and the seconds left
    /// before they are back on their feet.
    #[serde(default)]
    pub knocked: f64,
    #[serde(default)]
    pub knocked_timer: f64,
    /// EP5: the regiment's standard.
    #[serde(default)]
    pub standard: StandardState,
    /// R4: simulated time at which enemy shooters last saw the regiment
    /// (a target of indirect volleys for a few seconds after).
    #[serde(default = "unseen")]
    pub seen_at: f64,
    /// EP11: push state and shape of the front in melee.
    #[serde(default)]
    pub push: crate::push::PushShape,
    /// CB-M3: orders waiting behind the current one (Shift + right click),
    /// at most `QueueRules::max_queued_orders`.
    #[serde(default, skip_serializing_if = "VecDeque::is_empty")]
    pub order_queue: VecDeque<QueuedOrder>,
    /// CB1: files of the Line set by a right-drag (`formation_width`),
    /// `None` for the default depth of the Line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_files: Option<u32>,
    /// RJ-a (ADR 0174): change of formation under way, `None` once formed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reform: Option<Reform>,
    /// RJ-a: seconds since the formation last changed (the AI holds a new
    /// formation a while before changing again).
    #[serde(default = "never")]
    pub formed_for: f64,
    /// CB1: tag of the grouped order the regiment walks under (a locked
    /// group, or one drag); with `match_speed`, the group keeps the pace
    /// of its slowest regiment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_tag: Option<u32>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub match_speed: bool,
    /// CB2 (`crate::modes`): persistent run, guard, skirmish, melee (shooters
    /// close in) and breach (engines batter walls only). Left out of the JSON
    /// when off. `skirmish` starts on for regiments with the `skirmish`
    /// ability.
    #[serde(default, skip_serializing_if = "is_false")]
    pub mode_run: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub guard: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub skirmish: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub melee_mode: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub breach: bool,
    /// CB4 (`crate::abilities`): the active ability in use, cooldowns, and
    /// why the last one ended. Left out of the JSON when empty.
    #[serde(
        default,
        skip_serializing_if = "crate::abilities::UnitAbilities::is_empty"
    )]
    pub ability_state: crate::abilities::UnitAbilities,
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// `missile_timer` of a regiment never shot at.
fn never() -> f64 {
    1.0e6
}

/// `seen_at` of a regiment enemy shooters never saw.
fn unseen() -> f64 {
    -1.0e6
}

impl Unit {
    pub fn from_setup(id: u32, side: SideId, setup_index: usize, setup: &UnitSetup) -> Self {
        let ranged_capable = setup.stats.ranged > 0 && setup.stats.range > 0;
        Unit {
            id,
            side,
            setup_index,
            unit_type: setup.unit_type.as_str().into(),
            name: setup.name.as_str().into(),
            category: setup.category,
            mounted: setup.mounted,
            stats: setup.stats.clone(),
            abilities: setup.abilities.as_slice().into(),
            missile: setup.missile,
            experience: setup.experience,
            initial_soldiers: setup.soldiers,
            max_soldiers: setup.max_soldiers.max(setup.soldiers).max(1),
            hp: f64::from(setup.soldiers),
            x: 0.0,
            z: 0.0,
            facing: 0.0,
            formation: Formation::default_formation(),
            state: UnitState::Idle,
            morale: f64::from(setup.morale),
            morale_cap: f64::from(setup.morale),
            fatigue: 0.0,
            ammo: if ranged_capable { setup.stats.ammo } else { 0 },
            target: None,
            destination: None,
            destination_facing: None,
            running: false,
            fire_at_will: ranged_capable,
            is_general: false,
            withdrawing: false,
            disengaging: false,
            left_field: false,
            reserve: false,
            charge_timer: 0.0,
            reload: 0.0,
            still_time: 0.0,
            stakes_planted: false,
            rally_timer: 0.0,
            on_wall: false,
            climbing: None,
            climb_progress: 0.0,
            wall_target: None,
            synthetic: false,
            ram: false,
            blocked_by: None,
            tick_losses: 0.0,
            kills: 0.0,
            flanked: 0,
            order_morale: 0.0,
            order_morale_timer: 0.0,
            pavise: None,
            missile_timer: never(),
            dismounted: false,
            loss_cause: LossCause::Other,
            loss_by: None,
            knocked: 0.0,
            knocked_timer: 0.0,
            standard: StandardState::Carried,
            seen_at: unseen(),
            push: Default::default(),
            order_queue: VecDeque::new(),
            line_files: None,
            reform: None,
            formed_for: never(),
            group_tag: None,
            match_speed: false,
            mode_run: false,
            guard: false,
            // CB2: the shot on the move of the ability goes with the mode.
            skirmish: setup.abilities.contains(&Ability::Skirmish),
            melee_mode: false,
            breach: false,
            ability_state: Default::default(),
        }
    }

    /// The riders leave their horses (irreversible): foot soldiers, no
    /// charge, speed at most `speed_max`, `armor` points of armour added.
    /// Shared by the "pied à terre" order and the siege assault.
    pub fn dismount(&mut self, speed_max: u8, armor: u8) {
        self.mounted = false;
        self.dismounted = true;
        if self.category == UnitCategory::Cavalry {
            self.category = UnitCategory::Infantry;
        }
        self.stats.speed = self.stats.speed.min(speed_max);
        self.stats.armor = self.stats.armor.saturating_add(armor).min(120);
        self.stats.charge = None;
        self.charge_timer = 0.0;
        if !self.formation.def().allows(self) {
            self.formation = Formation::default_formation();
            self.line_files = None;
            self.reform = None;
        }
        if self.state == UnitState::Charging {
            self.state = UnitState::Marching;
        }
    }

    /// Living soldiers.
    pub fn soldiers(&self) -> u32 {
        self.hp.max(0.0).ceil() as u32
    }

    /// Strips `ability` from the regiment (laboratory set-ups).
    pub fn remove_ability(&mut self, ability: Ability) {
        self.abilities = self
            .abilities
            .iter()
            .copied()
            .filter(|a| *a != ability)
            .collect();
    }

    pub fn has(&self, ability: Ability) -> bool {
        self.abilities.contains(&ability)
    }

    pub fn is_cavalry(&self) -> bool {
        self.category == UnitCategory::Cavalry
    }

    /// Charge bonus in percent (20 when the type gives none).
    pub fn charge_points(&self) -> f64 {
        f64::from(self.stats.charge.unwrap_or(20))
    }

    pub fn can_shoot(&self) -> bool {
        self.stats.ranged > 0 && self.stats.range > 0
    }

    /// On the field with soldiers left.
    pub fn present(&self) -> bool {
        !self.left_field && !self.reserve && self.hp > 0.0
    }

    /// Fate of the regiment (end-of-battle screen): a withdrawing unit has
    /// withdrawn even if the battle ended before it reached the edge.
    pub fn fate(&self) -> UnitFate {
        if self.soldiers() == 0 {
            UnitFate::Destroyed
        } else if self.state == UnitState::Routing {
            UnitFate::Routed
        } else if self.withdrawing || self.left_field {
            UnitFate::Withdrawn
        } else if self.reserve {
            UnitFate::Reserve
        } else {
            UnitFate::Held
        }
    }

    /// Present and not routing: counts for the end of the battle.
    pub fn able(&self) -> bool {
        self.present() && self.state != UnitState::Routing && !self.withdrawing
    }

    /// Seconds between two shots of this regiment ([`Self::reload`] restarts
    /// from it after each volley): engines 12 s, pavise crossbowmen 9 s,
    /// other shooters 6 s.
    pub fn reload_period(&self) -> f64 {
        if self.category == UnitCategory::Siege {
            crate::shot::ENGINE_RELOAD
        } else if self.has(Ability::Pavise) {
            crate::shot::PAVISE_RELOAD
        } else {
            crate::shot::VOLLEY_RELOAD
        }
    }

    /// Engine able to batter walls (`siege_attack`).
    pub fn wall_breaker(&self) -> bool {
        self.stats.siege_attack.unwrap_or(0) > 0 && self.can_shoot()
    }

    /// Siege tower (`wall_assault`).
    pub fn siege_tower(&self) -> bool {
        self.has(Ability::WallAssault)
    }

    /// Foot soldiers who can scale walls with ladders (SG4: archers and
    /// crossbowmen on foot too, once their quivers are empty).
    pub fn can_climb(&self) -> bool {
        !self.mounted
            && !self.synthetic
            && (self.category == UnitCategory::Infantry
                || (self.category == UnitCategory::Ranged && self.ammo == 0))
    }

    pub fn forward(&self) -> (f64, f64) {
        (self.facing.sin(), self.facing.cos())
    }

    pub fn right(&self) -> (f64, f64) {
        (self.facing.cos(), -self.facing.sin())
    }

    /// Lateral and depth spacing between soldiers, in metres (current
    /// formation).
    pub fn spacing(&self) -> (f64, f64) {
        self.spacing_in(self.formation)
    }

    /// Spacing in `formation`: the base spacing of the troops times the
    /// formation's factors (`unit_formations.json`).
    pub fn spacing_in(&self, formation: Formation) -> (f64, f64) {
        let (sx, sz) = if self.category == UnitCategory::Siege {
            (7.0, 7.0)
        } else if self.mounted {
            (2.4, 3.4)
        } else {
            (1.1, 1.5)
        };
        let k = formation.def().spacing;
        (sx * k.lateral, sz * k.depth)
    }

    /// `(ranks, files)` of the current formation for `n` soldiers.
    pub fn ranks_files(&self, n: u32) -> (u32, u32) {
        self.ranks_files_in(self.formation, self.line_files, n)
    }

    /// `(ranks, files)` of `formation` (dragged to `line_files` files) for
    /// `n` soldiers.
    pub fn ranks_files_in(
        &self,
        formation: Formation,
        line_files: Option<u32>,
        n: u32,
    ) -> (u32, u32) {
        let n = n.max(1);
        let def = formation.def();
        match def.shape {
            FormationShape::Line | FormationShape::Herse => {
                if let (true, Some(files)) = (def.width_adjustable, line_files) {
                    let bounds =
                        crate::formation_width::FormationWidthRules::bundled().bounds(self);
                    return crate::formation_width::line_shape(n, files, bounds);
                }
                let ranks = def.ranks_for(self);
                (ranks.min(n), n.div_ceil(ranks))
            }
            FormationShape::Column => {
                let files = if self.mounted {
                    def.files.mounted
                } else {
                    def.files.foot
                }
                .max(1)
                .min(n);
                (n.div_ceil(files), files)
            }
            FormationShape::Square => {
                let side = (f64::from(n).sqrt().ceil() as u32).max(1);
                (n.div_ceil(side), side)
            }
            FormationShape::Wedge => {
                let rows = (f64::from(n).sqrt().ceil() as u32).max(1);
                (rows, 2 * rows - 1)
            }
        }
    }

    /// Frontage and depth of the formation, in metres.
    pub fn extent(&self) -> (f64, f64) {
        self.extent_in(self.formation, self.line_files)
    }

    fn extent_in(&self, formation: Formation, line_files: Option<u32>) -> (f64, f64) {
        let (ranks, files) = self.ranks_files_in(formation, line_files, self.soldiers());
        let (sx, sz) = self.spacing_in(formation);
        (f64::from(files) * sx, f64::from(ranks) * sz)
    }

    /// Soldiers fighting in the front ranks.
    pub fn fighting_soldiers(&self) -> f64 {
        let (_, files) = self.ranks_files(self.soldiers());
        // A square fights on every face, a wedge along both slanted faces
        // (`fighting_ranks`).
        let front = f64::from(files) * self.formation.def().modifiers.fighting_ranks;
        // BV2: men knocked down by a charge do not fight until they get up.
        let down = if self.knocked_timer > 0.0 {
            self.knocked
        } else {
            0.0
        };
        (front - down).max(0.0).min(self.hp.max(0.0))
    }

    /// RJ-a: the regiment is changing formation.
    pub fn reforming(&self) -> bool {
        self.reform.is_some()
    }

    /// RJ-a: multiplier of the pace (formation, and the change under way;
    /// riders at the charge close up on the move, at full gallop).
    pub fn formation_speed(&self) -> f64 {
        let mut k = self.formation.def().modifiers.speed;
        if self.reforming() && self.state != UnitState::Charging {
            k *= FormationRules::bundled().reform.speed;
        }
        k
    }

    /// RJ-a: multiplier of the melee damage the regiment takes.
    pub fn melee_taken_factor(&self) -> f64 {
        let mut k = self.formation.def().modifiers.melee_taken;
        if self.reforming() {
            k *= FormationRules::bundled().reform.melee_taken;
        }
        k
    }

    /// RJ-a: multiplier of the missile casualties the regiment takes.
    pub fn missile_taken_factor(&self) -> f64 {
        let mut k = self.formation.def().modifiers.missile_taken;
        if self.reforming() {
            k *= FormationRules::bundled().reform.missile_taken;
        }
        k
    }

    /// RJ-a: multiplier of the morale lost to casualties.
    pub fn morale_loss_factor(&self) -> f64 {
        let mut k = self.formation.def().modifiers.morale_loss;
        if self.reforming() {
            k *= FormationRules::bundled().reform.morale_loss;
        }
        k
    }

    /// RJ-a: multiplier of the charge bonus and weight.
    pub fn formation_charge(&self) -> f64 {
        self.formation.def().modifiers.charge
    }

    /// RJ-a: multiplier of the casualties the regiment's shots inflict.
    pub fn formation_shooting(&self) -> f64 {
        self.formation.def().modifiers.shooting
    }

    /// RJ-a: no flank nor rear (schiltron).
    pub fn all_round(&self) -> bool {
        self.formation.def().all_round
    }

    /// RJ-a: braced against horse (schiltron).
    pub fn braced(&self) -> bool {
        self.formation.def().braced
    }

    /// RJ-a: orders the change to `to` (no-op when already there). The men
    /// walk to their new places over [`FormationRules::reform_duration`];
    /// ordering back the formation being left turns them round where they
    /// are. `instant` skips the walk (deployment, scenario set-up).
    pub fn change_formation(&mut self, to: Formation, instant: bool) {
        let from = self.formation;
        let from_files = self.line_files;
        if from != to {
            self.formed_for = 0.0;
        }
        self.formation = to;
        // CB1: a formation order drops a dragged width.
        self.line_files = None;
        if instant {
            self.reform = None;
            return;
        }
        let rules = FormationRules::bundled();
        let duration = rules.reform_duration(self, to);
        self.reform = match self.reform {
            // Back to the formation being left: the same walk, reversed.
            Some(r) if r.from == to && from_files.is_none() => {
                let done = r.progress();
                (done > 0.0).then_some(Reform {
                    from,
                    from_files: None,
                    elapsed: duration * (1.0 - done),
                    duration,
                })
            }
            Some(_) if from == to && from_files.is_none() => self.reform,
            None if from == to && from_files.is_none() => None,
            _ => (duration > 0.0).then_some(Reform {
                from,
                from_files,
                elapsed: 0.0,
                duration,
            }),
        };
    }

    /// RJ-a: advances the change of formation by `dt` seconds.
    pub fn tick_reform(&mut self, dt: f64) {
        self.formed_for += dt;
        if let Some(r) = &mut self.reform {
            r.elapsed += dt;
            if r.elapsed >= r.duration {
                self.reform = None;
            }
        }
    }

    /// Support of the formation rectangle along the unit vector `dir`.
    pub fn support(&self, dir: (f64, f64)) -> f64 {
        let (w, d) = self.extent();
        let (fx, fz) = self.forward();
        let (rx, rz) = self.right();
        (dir.0 * fx + dir.1 * fz).abs() * d * 0.5 + (dir.0 * rx + dir.1 * rz).abs() * w * 0.5
    }

    /// Distance from the point to this unit's rectangle (0 inside).
    pub fn distance_to_rect(&self, x: f64, z: f64) -> f64 {
        let (w, d) = self.extent();
        let (dx, dz) = (x - self.x, z - self.z);
        let (fx, fz) = self.forward();
        let (rx, rz) = self.right();
        let lx = (dx * rx + dz * rz).abs() - w * 0.5;
        let lz = (dx * fx + dz * fz).abs() - d * 0.5;
        let (ox, oz) = (lx.max(0.0), lz.max(0.0));
        (ox * ox + oz * oz).sqrt()
    }

    /// Local position (lateral, forward) of soldier `i` of `n` in
    /// `formation`.
    fn slot_in(&self, formation: Formation, line_files: Option<u32>, i: u32, n: u32) -> (f64, f64) {
        let (ranks, files) = self.ranks_files_in(formation, line_files, n);
        let (sx, sz) = self.spacing_in(formation);
        match formation.shape() {
            FormationShape::Wedge => {
                // Row k (0 = tip) holds 2k + 1 riders.
                let mut k = 0u32;
                let mut first = 0u32;
                while first + 2 * k < i {
                    first += 2 * k + 1;
                    k += 1;
                }
                let j = i - first;
                let lx = (f64::from(j) - f64::from(k)) * sx;
                let lz = (f64::from(ranks) - 1.0) * 0.5 * sz - f64::from(k) * sz;
                (lx, lz)
            }
            shape => {
                let rank = i / files;
                let file = i % files;
                // Herse: odd ranks stand in the gaps of the rank ahead.
                let stagger = match (shape, rank % 2) {
                    (FormationShape::Herse, 0) => -0.25,
                    (FormationShape::Herse, _) => 0.25,
                    _ => 0.0,
                };
                let lx = (f64::from(file) - (f64::from(files) - 1.0) * 0.5 + stagger) * sx;
                let lz = ((f64::from(ranks) - 1.0) * 0.5 - f64::from(rank)) * sz;
                (lx, lz)
            }
        }
    }

    /// Figures drawn for this regiment at the visual unit-size multiplier
    /// `scale` (BV1, ADR 0016): `round(soldiers × scale)`, at least one while
    /// a soldier stands. Rendering only: the rules count [`Self::soldiers`].
    pub fn figure_count(&self, scale: f64) -> u32 {
        let n = self.soldiers();
        if n == 0 || !self.present() {
            return 0;
        }
        if (scale - 1.0).abs() < 1e-9 {
            return n;
        }
        ((f64::from(n) * scale).round() as u32).max(1)
    }

    /// World (x, z, angle) of the figures drawn at unit-size multiplier
    /// `scale` (BV1, ADR 0016). The figures fill the regiment's simulated
    /// rectangle ([`Self::extent`]): more figures stand closer together
    /// rather than widening the formation, so what the player sees still
    /// matches the footprint the rules use for contact and collisions. At
    /// `scale` = 1 these are the simulated soldiers, one figure each.
    pub fn figure_positions(&self, scale: f64) -> Vec<(f64, f64, f64)> {
        let m = self.figure_count(scale);
        if m == 0 {
            return Vec::new();
        }
        let layout = self.figure_layout(self.formation, self.line_files, scale, m);
        let (mut local, squeeze) = (layout.0.clone(), layout.1);
        // RJ-a: changing formation, each man walks from his old place to
        // his new one (all arrive when the change ends).
        if let Some(r) = self.reform {
            let old = &self.figure_layout(r.from, r.from_files, scale, m).0;
            let rules = &FormationRules::bundled().reform;
            let walk = if self.mounted {
                rules.walk_mps.mounted
            } else {
                rules.walk_mps.foot
            };
            let farthest = old
                .iter()
                .zip(&local)
                .map(|(a, b)| (b.0 - a.0).hypot(b.1 - a.1))
                .fold(0.0, f64::max);
            let pace = walk.max(farthest / r.duration.max(1e-6));
            let walked = r.elapsed * pace;
            for (to, from) in local.iter_mut().zip(old.iter()) {
                let d = (to.0 - from.0).hypot(to.1 - from.1);
                let p = if d <= 1e-9 {
                    1.0
                } else {
                    (walked / d).min(1.0)
                };
                *to = (from.0 + (to.0 - from.0) * p, from.1 + (to.1 - from.1) * p);
            }
        }
        let (fx, fz) = self.forward();
        let (rx, rz) = self.right();
        let spread = match self.state {
            UnitState::Routing => 5.0,
            UnitState::Melee => 1.2,
            _ => 0.35,
        } * squeeze;
        local
            .into_iter()
            .enumerate()
            .map(|(i, (lx, lz))| {
                let i = i as u64;
                let jx = jitter(u64::from(self.id), i * 2) * spread;
                let jz = jitter(u64::from(self.id), i * 2 + 1) * spread;
                let (lx, lz) = (lx + jx, lz + jz);
                let angle = if self.state == UnitState::Routing {
                    self.facing + jitter(u64::from(self.id) + 7, i) * 1.5
                } else {
                    self.facing
                };
                (
                    self.x + rx * lx + fx * lz,
                    self.z + rz * lx + fz * lz,
                    angle,
                )
            })
            .collect()
    }

    /// Local places (lateral, forward) of the `m` figures drawn at `scale`
    /// in `formation`, and the factor of the jitter (figures squeezed into
    /// the simulated rectangle jitter less).
    fn figure_layout(
        &self,
        formation: Formation,
        line_files: Option<u32>,
        scale: f64,
        m: u32,
    ) -> std::rc::Rc<(Vec<(f64, f64)>, f64)> {
        let key = LayoutKey {
            formation: formation.key(),
            line_files,
            scale_bits: scale.to_bits(),
            figures: m,
            soldiers: self.soldiers(),
            siege: self.category == UnitCategory::Siege,
            ranged: self.category == UnitCategory::Ranged,
            mounted: self.mounted,
            pikemen: self.has(Ability::PikeSquare),
        };
        LAYOUT_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            if let Some(hit) = cache.get(&key) {
                return std::rc::Rc::clone(hit);
            }
            if cache.len() >= LAYOUT_CACHE_MAX {
                cache.clear();
            }
            let layout =
                std::rc::Rc::new(self.compute_figure_layout(formation, line_files, scale, m));
            cache.insert(key, std::rc::Rc::clone(&layout));
            layout
        })
    }

    fn compute_figure_layout(
        &self,
        formation: Formation,
        line_files: Option<u32>,
        scale: f64,
        m: u32,
    ) -> (Vec<(f64, f64)>, f64) {
        if (scale - 1.0).abs() < 1e-9 {
            let slots = (0..m)
                .map(|i| self.slot_in(formation, line_files, i, m))
                .collect();
            return (slots, 1.0);
        }
        let (width, depth) = self.extent_in(formation, line_files);
        let (sx, sz) = self.spacing_in(formation);
        let (ranks, files) = self.figure_ranks_files_in(formation, line_files, scale, m);
        let grid = matches!(
            formation.shape(),
            FormationShape::Line | FormationShape::Column
        );
        let (fw, fd) = if grid {
            (f64::from(files) * sx, f64::from(ranks) * sz)
        } else {
            let (r, f) = self.ranks_files_in(formation, line_files, m);
            (f64::from(f) * sx, f64::from(r) * sz)
        };
        let kx = width / fw.max(1e-6);
        let kz = depth / fd.max(1e-6);
        let slots = (0..m)
            .map(|i| {
                let (lx, lz) = if grid {
                    let rank = i / files;
                    let file = i % files;
                    (
                        (f64::from(file) - (f64::from(files) - 1.0) * 0.5) * sx,
                        ((f64::from(ranks) - 1.0) * 0.5 - f64::from(rank)) * sz,
                    )
                } else {
                    self.slot_in(formation, line_files, i, m)
                };
                (lx * kx, lz * kz)
            })
            .collect();
        (slots, kx.min(kz).min(1.0))
    }

    /// `(ranks, files)` of the figure grid for `m` figures drawn at `scale`.
    /// Figure layout: the formation's own shape for m figures, squeezed back
    /// into the simulated rectangle. Lines and columns gain ranks as well as
    /// files (√scale each way) so that a large regiment does not turn into a
    /// single file of shoulder-to-shoulder men; riders keep at least a horse
    /// length between ranks.
    fn figure_ranks_files_in(
        &self,
        formation: Formation,
        line_files: Option<u32>,
        scale: f64,
        m: u32,
    ) -> (u32, u32) {
        if (scale - 1.0).abs() < 1e-9 {
            return self.ranks_files_in(formation, line_files, m);
        }
        match formation.shape() {
            FormationShape::Line | FormationShape::Column => {
                let (r, _) = self.ranks_files_in(formation, line_files, self.soldiers());
                let (_, depth) = self.extent_in(formation, line_files);
                let min_depth = if self.mounted { 2.7 } else { 0.8 };
                let most = ((depth / min_depth).floor() as u32).max(1);
                let r = ((f64::from(r) * scale.sqrt()).round() as u32).clamp(1, most.max(r));
                let r = r.min(m);
                (r, m.div_ceil(r))
            }
            _ => self.ranks_files_in(formation, line_files, m),
        }
    }

    /// Shooters stand in a line-like formation (their standard in the
    /// middle rank so the front rank can see).
    fn shooters_in_line(&self) -> bool {
        self.category == UnitCategory::Ranged
            && matches!(
                self.formation.shape(),
                FormationShape::Line | FormationShape::Herse
            )
    }

    /// EP5: indices, in the figure buffer drawn at `scale`, of the figures
    /// that carry the regiment's standards (`bearers`, 1 or 2): the centre
    /// of the front rank (of the middle rank for shooters in line, whose
    /// front rank must see; the tip of a wedge). Rendering only.
    pub fn standard_slots(&self, scale: f64, bearers: u32) -> Vec<u32> {
        let m = self.figure_count(scale);
        if m == 0 || bearers == 0 {
            return Vec::new();
        }
        let bearers = bearers.min(m);
        if self.formation.shape() == FormationShape::Wedge {
            // Row k holds 2k + 1 riders from index k²: the tip, then the
            // second row's ends.
            return match bearers {
                1 => vec![0],
                _ => vec![1.min(m - 1), 3.min(m - 1)],
            };
        }
        let (ranks, files) = self.figure_ranks_files_in(self.formation, self.line_files, scale, m);
        let rank = if self.shooters_in_line() && ranks > 2 {
            ranks / 2
        } else {
            0
        };
        (0..bearers)
            .map(|b| {
                let file = files * (2 * b + 1) / (2 * bearers);
                (rank * files + file.min(files.saturating_sub(1))).min(m - 1)
            })
            .collect()
    }

    /// EP5: world `(x, z)` where the bearer of the regiment's standard
    /// stands (front centre, the middle for shooters in line).
    pub fn standard_point(&self) -> (f64, f64) {
        let (_, depth) = self.extent();
        let (fx, fz) = self.forward();
        let ahead = if self.shooters_in_line() {
            0.0
        } else {
            depth * 0.5
        };
        (self.x + fx * ahead, self.z + fz * ahead)
    }

    /// EP5: the regiment carries a standard (not siege engines nor the
    /// battle-only ram).
    pub fn has_standard(&self) -> bool {
        !self.synthetic && self.category != UnitCategory::Siege
    }
}

#[cfg(test)]
mod key_enum_tests {
    use super::*;
    use data_model::key_enum::assert_keys_match_serde;

    #[test]
    fn keys_match_serde_names() {
        assert_keys_match_serde::<UnitState>();
        assert_keys_match_serde::<UnitFate>();
    }
}
