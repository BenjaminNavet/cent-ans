//! Battle regiments: state, formation geometry and soldier positions.

use data_model::{Ability, Missile, UnitCategory, UnitStats};
use serde::{Deserialize, Serialize};

use crate::impact::LossCause;
use crate::rng::jitter;
use crate::setup::{SideId, UnitSetup};

/// Formation of a regiment (spec § 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Formation {
    Line,
    Column,
    /// Carré / schiltron: no flank, strong against cavalry, very slow.
    Square,
    /// Coin: cavalry only, stronger charge.
    Wedge,
}

impl Formation {
    pub fn key(self) -> &'static str {
        match self {
            Formation::Line => "line",
            Formation::Column => "column",
            Formation::Square => "square",
            Formation::Wedge => "wedge",
        }
    }

    pub fn label_fr(self) -> &'static str {
        match self {
            Formation::Line => "ligne",
            Formation::Column => "colonne",
            Formation::Square => "schiltron",
            Formation::Wedge => "coin",
        }
    }
}

/// What a regiment is doing (spec § 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitState {
    Idle,
    Marching,
    Charging,
    Melee,
    Shooting,
    Routing,
    /// Just rallied; back to `Idle` after a few seconds.
    Rallied,
    /// Scaling a town wall (ladders or siege tower bridge).
    Climbing,
}

impl UnitState {
    pub fn key(self) -> &'static str {
        match self {
            UnitState::Idle => "idle",
            UnitState::Marching => "marching",
            UnitState::Charging => "charging",
            UnitState::Melee => "melee",
            UnitState::Shooting => "shooting",
            UnitState::Routing => "routing",
            UnitState::Rallied => "rallied",
            UnitState::Climbing => "climbing",
        }
    }
}

/// What became of a regiment, as shown on the end-of-battle screen (Q2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitFate {
    /// No soldier left.
    Destroyed,
    /// Fleeing, on the field or already off it.
    Routed,
    /// Ordered off the field (retreat or general retreat), in good order.
    Withdrawn,
    /// Never committed (reinforcement waiting off the field).
    Reserve,
    /// Still standing on the field.
    Held,
}

impl UnitFate {
    pub fn key(self) -> &'static str {
        match self {
            UnitFate::Destroyed => "destroyed",
            UnitFate::Routed => "routed",
            UnitFate::Withdrawn => "withdrawn",
            UnitFate::Reserve => "reserve",
            UnitFate::Held => "held",
        }
    }
}

/// A regiment on the field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Unit {
    pub id: u32,
    pub side: SideId,
    /// Index in the side's [`crate::SideSetup::units`].
    pub setup_index: usize,
    pub unit_type: String,
    pub name: String,
    pub category: UnitCategory,
    pub mounted: bool,
    pub stats: UnitStats,
    pub abilities: Vec<Ability>,
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
}

/// `missile_timer` of a regiment never shot at.
fn never() -> f64 {
    1.0e6
}

impl Unit {
    pub fn from_setup(id: u32, side: SideId, setup_index: usize, setup: &UnitSetup) -> Self {
        let ranged_capable = setup.stats.ranged > 0 && setup.stats.range > 0;
        Unit {
            id,
            side,
            setup_index,
            unit_type: setup.unit_type.clone(),
            name: setup.name.clone(),
            category: setup.category,
            mounted: setup.mounted,
            stats: setup.stats.clone(),
            abilities: setup.abilities.clone(),
            missile: setup.missile,
            experience: setup.experience,
            initial_soldiers: setup.soldiers,
            max_soldiers: setup.max_soldiers.max(setup.soldiers).max(1),
            hp: f64::from(setup.soldiers),
            x: 0.0,
            z: 0.0,
            facing: 0.0,
            formation: Formation::Line,
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
        if self.formation == Formation::Wedge {
            self.formation = Formation::Line;
        }
        if self.state == UnitState::Charging {
            self.state = UnitState::Marching;
        }
    }

    /// Living soldiers.
    pub fn soldiers(&self) -> u32 {
        self.hp.max(0.0).ceil() as u32
    }

    pub fn has(&self, ability: Ability) -> bool {
        self.abilities.contains(&ability)
    }

    pub fn is_cavalry(&self) -> bool {
        self.category == UnitCategory::Cavalry
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

    /// Foot soldiers who can scale walls with ladders.
    pub fn can_climb(&self) -> bool {
        !self.mounted && !self.synthetic && self.category == UnitCategory::Infantry
    }

    pub fn forward(&self) -> (f64, f64) {
        (self.facing.sin(), self.facing.cos())
    }

    pub fn right(&self) -> (f64, f64) {
        (self.facing.cos(), -self.facing.sin())
    }

    /// Lateral and depth spacing between soldiers, in metres.
    pub fn spacing(&self) -> (f64, f64) {
        if self.category == UnitCategory::Siege {
            (7.0, 7.0)
        } else if self.mounted {
            (2.4, 3.4)
        } else {
            (1.1, 1.5)
        }
    }

    /// `(ranks, files)` of the current formation for `n` soldiers.
    pub fn ranks_files(&self, n: u32) -> (u32, u32) {
        let n = n.max(1);
        match self.formation {
            Formation::Line => {
                let ranks = if self.category == UnitCategory::Siege {
                    1
                } else if self.mounted {
                    2
                } else if self.category == UnitCategory::Ranged {
                    3
                } else {
                    4
                };
                (ranks.min(n), n.div_ceil(ranks))
            }
            Formation::Column => {
                let files = if self.mounted { 4 } else { 6 }.min(n);
                (n.div_ceil(files), files)
            }
            Formation::Square => {
                let side = (f64::from(n).sqrt().ceil() as u32).max(1);
                (n.div_ceil(side), side)
            }
            Formation::Wedge => {
                let rows = (f64::from(n).sqrt().ceil() as u32).max(1);
                (rows, 2 * rows - 1)
            }
        }
    }

    /// Frontage and depth of the formation, in metres.
    pub fn extent(&self) -> (f64, f64) {
        let (ranks, files) = self.ranks_files(self.soldiers());
        let (sx, sz) = self.spacing();
        (f64::from(files) * sx, f64::from(ranks) * sz)
    }

    /// Soldiers fighting in the front ranks.
    pub fn fighting_soldiers(&self) -> f64 {
        let (_, files) = self.ranks_files(self.soldiers());
        let front = match self.formation {
            Formation::Square => f64::from(files) * 4.0,
            // The wedge drives in: riders fight along both slanted faces.
            Formation::Wedge => f64::from(files) * 3.0,
            _ => f64::from(files) * 2.0,
        };
        // BV2: men knocked down by a charge do not fight until they get up.
        let down = if self.knocked_timer > 0.0 {
            self.knocked
        } else {
            0.0
        };
        (front - down).max(0.0).min(self.hp.max(0.0))
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

    /// Local position (lateral, forward) of soldier `i` in the formation.
    fn slot(&self, i: u32, n: u32) -> (f64, f64) {
        let (ranks, files) = self.ranks_files(n);
        let (sx, sz) = self.spacing();
        match self.formation {
            Formation::Wedge => {
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
            _ => {
                let rank = i / files;
                let file = i % files;
                let lx = (f64::from(file) - (f64::from(files) - 1.0) * 0.5) * sx;
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
    /// `scale` = 1 this is exactly [`Self::soldier_positions`].
    pub fn figure_positions(&self, scale: f64) -> Vec<(f64, f64, f64)> {
        if (scale - 1.0).abs() < 1e-9 {
            return self.soldier_positions();
        }
        let m = self.figure_count(scale);
        if m == 0 {
            return Vec::new();
        }
        let n = self.soldiers();
        let (width, depth) = self.extent();
        let (sx, sz) = self.spacing();
        // Figure layout: the formation's own shape for m figures, squeezed
        // back into the simulated rectangle. Lines and columns gain ranks as
        // well as files (√scale each way) so that a large regiment does not
        // turn into a single file of shoulder-to-shoulder men; riders keep
        // at least a horse length between ranks.
        let (ranks, files) = match self.formation {
            Formation::Line | Formation::Column => {
                let (r, _) = self.ranks_files(n);
                let min_depth = if self.mounted { 2.7 } else { 0.8 };
                let most = ((depth / min_depth).floor() as u32).max(1);
                let r = ((f64::from(r) * scale.sqrt()).round() as u32).clamp(1, most.max(r));
                let r = r.min(m);
                (r, m.div_ceil(r))
            }
            _ => self.ranks_files(m),
        };
        let (fw, fd) = match self.formation {
            Formation::Line | Formation::Column => (f64::from(files) * sx, f64::from(ranks) * sz),
            _ => {
                let (r, f) = self.ranks_files(m);
                (f64::from(f) * sx, f64::from(r) * sz)
            }
        };
        let kx = width / fw.max(1e-6);
        let kz = depth / fd.max(1e-6);
        let (fx, fz) = self.forward();
        let (rx, rz) = self.right();
        let spread = match self.state {
            UnitState::Routing => 5.0,
            UnitState::Melee => 1.2,
            _ => 0.35,
        } * kx.min(kz).min(1.0);
        (0..m)
            .map(|i| {
                let (lx, lz) = match self.formation {
                    Formation::Line | Formation::Column => {
                        let rank = i / files;
                        let file = i % files;
                        (
                            (f64::from(file) - (f64::from(files) - 1.0) * 0.5) * sx,
                            ((f64::from(ranks) - 1.0) * 0.5 - f64::from(rank)) * sz,
                        )
                    }
                    _ => self.slot(i, m),
                };
                let jx = jitter(u64::from(self.id), u64::from(i) * 2) * spread;
                let jz = jitter(u64::from(self.id), u64::from(i) * 2 + 1) * spread;
                let (lx, lz) = (lx * kx + jx, lz * kz + jz);
                let angle = if self.state == UnitState::Routing {
                    self.facing + jitter(u64::from(self.id) + 7, u64::from(i)) * 1.5
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

    /// World (x, z, angle) of every living soldier, with a small stable jitter
    /// (larger when routing or in melee).
    pub fn soldier_positions(&self) -> Vec<(f64, f64, f64)> {
        if !self.present() {
            return Vec::new();
        }
        let n = self.soldiers();
        let (fx, fz) = self.forward();
        let (rx, rz) = self.right();
        let spread = match self.state {
            UnitState::Routing => 5.0,
            UnitState::Melee => 1.2,
            _ => 0.35,
        };
        (0..n)
            .map(|i| {
                let (lx, lz) = self.slot(i, n);
                let jx = jitter(u64::from(self.id), u64::from(i) * 2) * spread;
                let jz = jitter(u64::from(self.id), u64::from(i) * 2 + 1) * spread;
                let (lx, lz) = (lx + jx, lz + jz);
                let angle = if self.state == UnitState::Routing {
                    self.facing + jitter(u64::from(self.id) + 7, u64::from(i)) * 1.5
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
}
