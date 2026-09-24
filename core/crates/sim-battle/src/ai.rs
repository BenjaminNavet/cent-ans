//! Tactical battle AI (spec `docs/design/m9-ai.md` § 2).
//!
//! Evaluated every [`crate::sim::AI_PERIOD`] simulated seconds for each AI
//! side; it only emits [`Command`]s through the same API as the player, and
//! is deterministic (index order everywhere, no hidden state: every decision
//! is derived from the battle state).
//!
//! # Field battles
//!
//! - **Roles**: the infantry forms the line in the centre (the last foot
//!   regiment is held back as a reserve when there are four or more), foot
//!   shooters stand in front of the line, cavalry on the wings, engines
//!   behind, the general's regiment behind the centre.
//! - **Posture**: a side much weaker than its enemy (and a defender not
//!   clearly stronger) stands on the defensive: it takes the best high
//!   ground near its deployment, shooters in front plant their stakes
//!   (Crécy, Agincourt), the line waits and counter-charges at close range.
//!   Otherwise the side advances: shooters lead and duel at range, then the
//!   line closes and each regiment engages the enemy regiment opposite.
//! - **Shooters** fall back behind the line as soon as enemy foot or horse
//!   come close, and disengage from a melee.
//! - **Cavalry** charges isolated shooters, the flanks or rear of enemy
//!   regiments already engaged, answers enemy cavalry, pursues routing
//!   regiments, and never charges pikes or planted stakes head on.
//! - **Reactions**: the reserve plugs a gap (a line regiment routed or
//!   wavering) or strikes an enemy attacking a flank; a regiment attacked
//!   on the flank turns to face its attacker; a shaken regiment in melee is
//!   pulled out before it breaks.
//!
//! # Siege battles
//!
//! - **Besiegers**: engines batter the weakest stretch of the front wall
//!   (then shoot the wall walk), the ram goes for the gate, towers roll to
//!   the front walls, shooters duel with the wall walk. The foot waits out
//!   of bowshot while the engines work, then storms through the first
//!   opening towards the central square, climbs from the docked towers, or
//!   (no engine, no tower, or too long) raises ladders along the front.
//! - **Garrison**: shooters and foot hold the wall walk; wall foot attack
//!   climbers and attackers on the walls nearby; the reserve and the gate
//!   guard block the openings, then hunt the attackers inside the walls.
//!
//! # Leader's orders (F10b)
//!
//! Given when the conditions of the order's `ai` block hold (all numbers in
//! `data/battle_orders/`): the war cry when the regiments around the
//! general close with the enemy, rally as soon as regiments flee near him,
//! dismount when the side stands on the defensive (the garrison of a siege
//! too), pavises for crossbowmen standing under fire, and no quarter only
//! for an outnumbered army facing its hereditary enemy.

use data_model::{Ability, BattleOrder, BattleOrderKind, BattleOrderScope, UnitCategory};

use crate::command::Command;
use crate::setup::SideId;
use crate::siege::SiegeWorks;
use crate::sim::{attack_angle, BattleSim};
use crate::unit::{Formation, Unit, UnitState};

/// Distance at which the line closes in at the run.
pub const CHARGE_DISTANCE: f64 = 60.0;
/// A defensive line counter-charges enemies this close.
pub const COUNTER_CHARGE_DISTANCE: f64 = 45.0;
/// Shooters fall back when enemy melee troops come this close.
pub const SHOOTER_SAFETY: f64 = 70.0;
/// Enemy shooters farther than this from their own melee troops are
/// "isolated" (a cavalry target).
pub const ISOLATION_DISTANCE: f64 = 80.0;
/// Radius within which cavalry looks for targets.
pub const CAVALRY_REACH: f64 = 450.0;
/// Longest archery duel before the line advances regardless (seconds).
pub const DUEL_TIME: f64 = 480.0;
/// A clearly stronger attacker (its enemy stands on the defensive) gives up
/// the archery duel after this long and closes in (F5d: two AI armies
/// always end up engaging).
pub const ATTACKER_DUEL_TIME: f64 = 60.0;
/// AI destinations keep this far from the edge of deep water (F5d).
pub const RIVER_MARGIN: f64 = 12.0;
/// Closing in, the cavalry charges enemy horse this close to the line.
pub const ASSAULT_RANGE: f64 = 250.0;
/// A weaker attacker waits this long for the defender to come to it.
pub const ATTACKER_PATIENCE: f64 = 240.0;
/// A defensive side gives up waiting after this long.
pub const DEFENDER_PATIENCE: f64 = 480.0;
/// Besiegers wait for their engines at most this long before escalading.
pub const ENGINE_PATIENCE: f64 = 420.0;

fn dist(a: &Unit, b: &Unit) -> f64 {
    ((a.x - b.x).powi(2) + (a.z - b.z).powi(2)).sqrt()
}

fn dist_to(a: &Unit, x: f64, z: f64) -> f64 {
    ((a.x - x).powi(2) + (a.z - z).powi(2)).sqrt()
}

/// Rough fighting value of a regiment (for the posture decision).
pub fn unit_power(unit: &Unit) -> f64 {
    let s = &unit.stats;
    let quality = f64::from(s.melee)
        + f64::from(s.ranged) * 0.8
        + f64::from(s.armor) * 0.6
        + f64::from(s.charge.unwrap_or(0)) * 0.3;
    unit.hp.max(0.0) * quality / 100.0 * (0.5 + unit.morale / 200.0)
}

fn is_shooter(unit: &Unit) -> bool {
    unit.can_shoot() && unit.ammo > 0 && !unit.mounted && unit.category != UnitCategory::Siege
}

fn is_horse(unit: &Unit) -> bool {
    unit.mounted || unit.category == UnitCategory::Cavalry
}

fn is_melee_troop(unit: &Unit) -> bool {
    matches!(
        unit.category,
        UnitCategory::Infantry | UnitCategory::Cavalry
    ) && !unit.can_shoot()
}

/// Dangerous to charge head on for cavalry.
fn bristling(unit: &Unit) -> bool {
    unit.stakes_planted || unit.formation == Formation::Square || unit.has(Ability::PikeSquare)
}

/// Snapshot of the battle from one side's point of view.
struct View<'a> {
    sim: &'a BattleSim,
    side: SideId,
    units: &'a [Unit],
    own: Vec<usize>,
    enemies: Vec<usize>,
    /// +1 when the enemy lies towards +z (attacker), -1 otherwise.
    forward: f64,
    /// F5d: a clearly stronger attacker closing in (no duel) with the enemy
    /// within [`ASSAULT_RANGE`]: the horse goes for the enemy horse.
    assault: bool,
    commands: Vec<Command>,
}

impl<'a> View<'a> {
    fn new(sim: &'a BattleSim, side: SideId) -> Self {
        let units = sim.units();
        let own = (0..units.len())
            .filter(|&i| units[i].side == side && units[i].able())
            .collect();
        let enemies = (0..units.len())
            .filter(|&j| units[j].side != side && units[j].present() && !units[j].synthetic)
            .filter(|&j| units[j].able() || units[j].state == UnitState::Routing)
            .collect();
        View {
            sim,
            side,
            units,
            own,
            enemies,
            forward: if side == SideId::Attacker { 1.0 } else { -1.0 },
            assault: false,
            commands: Vec::new(),
        }
    }

    fn able_enemies(&self) -> impl Iterator<Item = usize> + '_ {
        self.enemies
            .iter()
            .copied()
            .filter(|&j| self.units[j].able())
    }

    fn engaged(&self, i: usize) -> bool {
        self.units[i].state == UnitState::Melee
    }

    /// Nearest able enemy satisfying `keep`, with its distance.
    fn nearest_enemy(&self, i: usize, keep: impl Fn(&Unit) -> bool) -> Option<(usize, f64)> {
        let unit = &self.units[i];
        self.able_enemies()
            .filter(|&j| keep(&self.units[j]))
            .map(|j| (j, dist(unit, &self.units[j])))
            .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
    }

    /// Can `i` reach `j` without an intact wall in between (siege)?
    fn reachable(&self, i: usize, j: usize) -> bool {
        let (a, b) = (&self.units[i], &self.units[j]);
        self.sim
            .siege()
            .is_none_or(|w| w.crosses_intact((a.x, a.z), (b.x, b.z)).is_none())
    }

    fn power(&self, side_own: bool) -> f64 {
        let list: Vec<usize> = if side_own {
            self.own.clone()
        } else {
            self.able_enemies().collect()
        };
        list.iter()
            .filter(|&&i| !self.units[i].synthetic)
            .map(|&i| unit_power(&self.units[i]))
            .sum()
    }

    fn centroid(&self, list: &[usize]) -> Option<(f64, f64)> {
        if list.is_empty() {
            return None;
        }
        let n = list.len() as f64;
        Some(list.iter().fold((0.0, 0.0), |(x, z), &i| {
            (x + self.units[i].x / n, z + self.units[i].z / n)
        }))
    }

    // ----- command helpers -----------------------------------------------

    fn free(&self, i: usize) -> bool {
        let u = &self.units[i];
        !matches!(
            u.state,
            UnitState::Melee | UnitState::Rallied | UnitState::Climbing
        ) && u.climbing.is_none()
    }

    /// Moves `i` to (x, z) unless it is already going (or standing) there.
    fn move_to(&mut self, i: usize, x: f64, z: f64, run: bool, facing: Option<f64>) {
        let u = &self.units[i];
        if !self.free(i) {
            return;
        }
        let field = self.sim.field();
        let (x, z) = (
            x.clamp(10.0, field.width - 10.0),
            z.clamp(10.0, field.depth - 10.0),
        );
        let z = dry_z(field, x, z, u.z, self.forward);
        let far = match u.destination {
            Some((dx, dz)) => (dx - x).powi(2) + (dz - z).powi(2) > 36.0,
            None => dist_to(u, x, z) > 6.0,
        };
        if far || u.target.is_some() || (u.destination.is_some() && u.running != run) {
            self.commands.push(Command::Move {
                units: vec![u.id],
                x,
                z,
                run,
                facing,
            });
        }
    }

    fn attack(&mut self, i: usize, j: usize, run: bool) {
        let u = &self.units[i];
        if u.climbing.is_some() {
            return;
        }
        if u.target != Some(self.units[j].id) || (u.running != run && !self.engaged(i)) {
            self.commands.push(Command::Attack {
                units: vec![u.id],
                target: self.units[j].id,
                run,
            });
        }
    }

    fn halt(&mut self, i: usize) {
        let u = &self.units[i];
        if (u.destination.is_some() || u.target.is_some()) && self.free(i) {
            self.commands.push(Command::Halt { units: vec![u.id] });
        }
    }

    /// Slots along a line centred on `center`, perpendicular to `facing`,
    /// in lateral order of the regiments.
    fn line_slots(&self, ids: &[usize], center: (f64, f64), facing: f64) -> Vec<(usize, f64, f64)> {
        let right = (facing.cos(), -facing.sin());
        let mut order: Vec<(usize, f64)> = ids
            .iter()
            .map(|&i| (i, self.units[i].x * right.0 + self.units[i].z * right.1))
            .collect();
        order.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
        let widths: Vec<f64> = order
            .iter()
            .map(|&(i, _)| self.units[i].extent().0 + 10.0)
            .collect();
        let total: f64 = widths.iter().sum();
        let mut offset = -total * 0.5;
        order
            .iter()
            .zip(widths)
            .map(|(&(i, _), w)| {
                let lateral = offset + w * 0.5;
                offset += w;
                (
                    i,
                    center.0 + right.0 * lateral,
                    center.1 + right.1 * lateral,
                )
            })
            .collect()
    }
}

/// F5d: a destination in deep water (fords excepted) moves to the bank on
/// the side where the regiment stands, or across when it is already wading
/// (`forward` = +1 towards +z).
pub fn dry_z(field: &crate::field::Battlefield, x: f64, z: f64, from_z: f64, forward: f64) -> f64 {
    let Some(river) = &field.river else {
        return z;
    };
    let center = river.center_z(x);
    let reach = river.width * 0.5 + RIVER_MARGIN;
    if river.in_ford(x) || (z - center).abs() > reach {
        return z;
    }
    let side = if (from_z - center).abs() <= reach {
        forward
    } else {
        (from_z - center).signum()
    };
    (center + side * (reach + 1.0)).clamp(10.0, field.depth - 10.0)
}

/// Commands of `side` for this decision step.
pub fn plan(sim: &BattleSim, side: SideId) -> Vec<Command> {
    let mut view = View::new(sim, side);
    if view.own.is_empty() || view.able_enemies().next().is_none() {
        return Vec::new();
    }
    match (sim.siege(), side) {
        (Some(works), SideId::Attacker) => plan_siege_attack(&mut view, works),
        (Some(works), SideId::Defender) => {
            plan_siege_defence(&mut view, works);
            let sortie = crate::formation_ai::plan_sortie(sim, side);
            view.commands.extend(sortie);
        }
        (None, _) => {
            plan_field(&mut view);
            crate::formation_ai::coordinate_flanks(sim, side, &mut view.commands);
            let formations = crate::formation_ai::plan_formations(sim, side);
            view.commands.extend(formations);
        }
    }
    if sim.siege().is_some() {
        plan_orders(&mut view, side == SideId::Defender);
    }
    view.commands
}

// ----- field battles ---------------------------------------------------------

struct Roles {
    line: Vec<usize>,
    reserve: Option<usize>,
    shooters: Vec<usize>,
    horse: Vec<usize>,
    engines: Vec<usize>,
}

fn roles(view: &View) -> Roles {
    let u = view.units;
    let mut line: Vec<usize> = view
        .own
        .iter()
        .copied()
        .filter(|&i| {
            let unit = &u[i];
            !unit.mounted
                && (unit.category == UnitCategory::Infantry
                    || (unit.category == UnitCategory::Ranged && unit.ammo == 0))
        })
        .collect();
    let reserve = if line.len() >= 4 {
        // The rearmost foot regiment (highest id on ties) is the reserve.
        let pick = *line
            .iter()
            .max_by(|&&a, &&b| {
                (-u[a].z * view.forward)
                    .total_cmp(&(-u[b].z * view.forward))
                    .then(a.cmp(&b))
            })
            .expect("non-empty");
        line.retain(|&i| i != pick);
        Some(pick)
    } else {
        None
    };
    Roles {
        line,
        reserve,
        shooters: view
            .own
            .iter()
            .copied()
            .filter(|&i| is_shooter(&u[i]))
            .collect(),
        horse: view
            .own
            .iter()
            .copied()
            .filter(|&i| is_horse(&u[i]) && u[i].category != UnitCategory::Siege)
            .collect(),
        engines: view
            .own
            .iter()
            .copied()
            .filter(|&i| u[i].category == UnitCategory::Siege)
            .collect(),
    }
}

/// Best high ground within reach of the own deployment (defensive posture).
fn high_ground(view: &View, around: (f64, f64)) -> (f64, f64) {
    let field = view.sim.field();
    let base = field.height(around.0, around.1);
    let mut best = (around, 0.0);
    for ix in -8..=8 {
        for iz in -4..=4 {
            let x = around.0 + f64::from(ix) * 25.0;
            let z = around.1 + f64::from(iz) * 25.0;
            if !(300.0..=900.0).contains(&x) || !(60.0..=field.depth - 60.0).contains(&z) {
                continue;
            }
            // Never step towards the enemy to find a hill.
            if (z - around.1) * view.forward > 30.0 {
                continue;
            }
            let gain = field.height(x, z) - base - (x - around.0).abs() * 0.01;
            if gain > best.1 + 0.5 && !field.in_forest(x, z) && !field.in_mud(x, z) {
                best = ((x, z), gain);
            }
        }
    }
    if best.1 > 2.0 {
        best.0
    } else {
        around
    }
}

fn plan_field(view: &mut View) {
    let roles = roles(view);
    let elapsed = view.sim.elapsed();
    let own_power = view.power(true);
    let enemy_power = view.power(false).max(1.0);
    let ratio = own_power / enemy_power;
    let enemy_melee: Vec<usize> = view
        .able_enemies()
        .filter(|&j| is_melee_troop(&view.units[j]))
        .collect();
    // Distance from our line to the nearest able enemy.
    let front_ref: Vec<usize> = if roles.line.is_empty() {
        view.own.clone()
    } else {
        roles.line.clone()
    };
    let contact = front_ref
        .iter()
        .filter_map(|&i| view.nearest_enemy(i, |_| true).map(|(_, d)| d))
        .fold(f64::INFINITY, f64::min);
    let defensive = match view.side {
        SideId::Defender => ratio < 0.85 && elapsed < DEFENDER_PATIENCE,
        SideId::Attacker => ratio < 0.8 && elapsed < ATTACKER_PATIENCE,
    };
    let shooters_have_ammo = roles
        .shooters
        .iter()
        .any(|&i| view.units[i].ammo > view.units[i].stats.ammo / 5);
    let enemy_shooters = view
        .able_enemies()
        .filter(|&j| is_shooter(&view.units[j]))
        .count();
    let own_ranged: f64 = roles
        .shooters
        .iter()
        .map(|&i| unit_power(&view.units[i]))
        .sum();
    let enemy_ranged: f64 = view
        .able_enemies()
        .filter(|&j| is_shooter(&view.units[j]))
        .map(|j| unit_power(&view.units[j]))
        .sum();
    // The archery duel: hold the line while our shooters are winning it.
    // F5d: a clearly stronger attacker facing a defender that waits for it
    // takes the initiative (short duel, the horse rides at the enemy horse).
    let press = view.side == SideId::Attacker && ratio * 0.85 > 1.0;
    let duel = !roles.shooters.is_empty()
        && shooters_have_ammo
        && contact < 320.0
        && elapsed < if press { ATTACKER_DUEL_TIME } else { DUEL_TIME }
        && (enemy_shooters == 0 || own_ranged >= enemy_ranged * 0.8);

    let line_center = view
        .centroid(&roles.line)
        .or_else(|| view.centroid(&view.own))
        .expect("own not empty");
    let facing = if view.forward > 0.0 {
        0.0
    } else {
        std::f64::consts::PI
    };
    // Only to open the fight (first minutes, nobody locked yet).
    let melee = view.units.iter().any(|u| u.state == UnitState::Melee);
    view.assault = press
        && !defensive
        && !duel
        && !melee
        && elapsed < ATTACKER_PATIENCE
        && contact < ASSAULT_RANGE;
    // Where the line stands this step.
    let anchor = if defensive {
        high_ground(view, line_center)
    } else if duel && contact < 260.0 {
        line_center
    } else {
        advance(view, line_center)
    };

    // Line.
    let slots = view.line_slots(&roles.line, anchor, facing);
    for (i, x, z) in slots {
        if !view.free(i) {
            continue;
        }
        let target = view.nearest_enemy(i, |e| e.state != UnitState::Routing);
        match target {
            Some((j, d)) if !defensive && !duel && d < CHARGE_DISTANCE * 2.0 => {
                let j = opposite(view, i).unwrap_or(j);
                view.attack(i, j, d < CHARGE_DISTANCE);
            }
            Some((j, d)) if d < COUNTER_CHARGE_DISTANCE => view.attack(i, j, true),
            _ => view.move_to(i, x, z, false, Some(facing)),
        }
    }

    // Shooters: in front of the line, halt in range, fall back when threatened.
    for &i in &roles.shooters {
        plan_shooter(view, i, anchor, line_center.1, facing, defensive);
    }

    // Reserve.
    if let Some(r) = roles.reserve {
        plan_reserve(view, r, &roles.line, anchor, facing);
    }

    // Cavalry.
    for &i in &roles.horse {
        plan_horse(view, i, &roles, anchor, facing, defensive, &enemy_melee);
    }

    // Engines: stay behind the line, shoot at will.
    for &i in &roles.engines {
        let unit = &view.units[i];
        if let Some((j, d)) = view.nearest_enemy(i, |_| true) {
            let e = &view.units[j];
            let range = view.sim.effective_range(unit, e.x, e.z);
            if d > range * 0.95 && unit.ammo > 0 {
                let x = unit.x;
                let z = (anchor.1 - view.forward * 70.0).max(0.0);
                if (z - unit.z) * view.forward > 5.0 {
                    view.move_to(i, x, z, false, None);
                }
            } else {
                view.halt(i);
            }
        }
    }

    react(view, &roles);
    plan_orders(view, defensive);
}

/// Next step of an advancing line: 45 m towards the enemy's centroid (F5d:
/// armies that slipped past each other turn back instead of marching on to
/// the far edge).
fn advance(view: &View, from: (f64, f64)) -> (f64, f64) {
    let able: Vec<usize> = view.able_enemies().collect();
    let Some((ex, ez)) = view.centroid(&able) else {
        return (from.0, from.1 + view.forward * 45.0);
    };
    let (dx, dz) = (ex - from.0, ez - from.1);
    let d = dx.hypot(dz);
    if dz * view.forward > 0.5 * d {
        // Ahead: the usual straight advance.
        return (from.0, from.1 + view.forward * 45.0);
    }
    let step = d.min(45.0) / d.max(1e-6);
    (from.0 + dx * step, from.1 + dz * step)
}

/// Enemy regiment opposite `i` (smallest lateral offset, a bit of depth).
fn opposite(view: &View, i: usize) -> Option<usize> {
    let u = &view.units[i];
    view.able_enemies()
        .filter(|&j| !is_horse(&view.units[j]) || view.units[j].state == UnitState::Melee)
        .filter(|&j| view.reachable(i, j))
        .map(|j| {
            let e = &view.units[j];
            (j, (e.x - u.x).abs() + 0.3 * (e.z - u.z).abs())
        })
        .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
        .map(|(j, _)| j)
}

fn plan_shooter(
    view: &mut View,
    i: usize,
    anchor: (f64, f64),
    line_z: f64,
    facing: f64,
    defensive: bool,
) {
    let unit = &view.units[i];
    let threat = view.nearest_enemy(i, is_melee_troop);
    // Engaged or about to be: fall back behind the line.
    if let Some((_, d)) = threat {
        if d < SHOOTER_SAFETY || view.engaged(i) {
            let rear = (line_z * view.forward).min(anchor.1 * view.forward) * view.forward;
            let behind = (unit.x, rear - view.forward * 45.0);
            if (unit.z - behind.1) * view.forward > 8.0 || view.engaged(i) {
                view.commands.push(Command::Move {
                    units: vec![unit.id],
                    x: behind.0,
                    z: behind.1,
                    run: true,
                    facing: Some(facing),
                });
            }
            return;
        }
    }
    if !view.free(i) {
        return;
    }
    let front_z = anchor.1 + view.forward * 30.0;
    if let Some((j, d)) = view.nearest_enemy(i, |_| true) {
        let e = &view.units[j];
        let range = view.sim.effective_range(unit, e.x, e.z);
        if d <= range * 0.95 {
            view.halt(i);
            return;
        }
        if defensive {
            view.move_to(i, unit.x, front_z, false, Some(facing));
            return;
        }
        // Advance to shooting range, never far ahead of the line.
        let wanted = (unit.z * view.forward + d - range * 0.85).min(front_z * view.forward + 40.0)
            * view.forward;
        if (wanted - unit.z) * view.forward > 5.0 {
            view.move_to(i, unit.x, wanted, false, None);
        }
    }
}

fn plan_reserve(view: &mut View, r: usize, line: &[usize], anchor: (f64, f64), facing: f64) {
    if !view.free(r) {
        return;
    }
    // Plug a gap: a line regiment routed, destroyed or wavering in melee.
    let units = view.units;
    let gap = view
        .sim
        .units()
        .iter()
        .enumerate()
        .filter(|(_, u)| u.side == view.side && u.category == UnitCategory::Infantry && !u.mounted)
        .filter(|(k, _)| *k != r)
        .find(|(k, u)| {
            (u.state == UnitState::Routing && u.present())
                || (line.contains(k) && u.state == UnitState::Melee && u.morale < 35.0)
        })
        .map(|(k, _)| k);
    if let Some(k) = gap {
        let (gx, gz) = (units[k].x, units[k].z);
        let enemy = view
            .able_enemies()
            .filter(|&j| view.reachable(r, j))
            .min_by(|&a, &b| {
                dist_to(&units[a], gx, gz)
                    .total_cmp(&dist_to(&units[b], gx, gz))
                    .then(a.cmp(&b))
            });
        if let Some(j) = enemy {
            if dist_to(&units[j], gx, gz) < 120.0 {
                view.attack(r, j, true);
                return;
            }
        }
    }
    // A flank attack on a friend: strike the attacker.
    if let Some(j) = flanker(view) {
        if dist(&units[r], &units[j]) < 250.0 {
            view.attack(r, j, true);
            return;
        }
    }
    if let Some((j, d)) = view.nearest_enemy(r, |e| e.state != UnitState::Routing) {
        if d < COUNTER_CHARGE_DISTANCE {
            view.attack(r, j, true);
            return;
        }
    }
    view.move_to(
        r,
        anchor.0,
        anchor.1 - view.forward * 60.0,
        false,
        Some(facing),
    );
}

/// An enemy regiment attacking one of ours on the flank or rear.
fn flanker(view: &View) -> Option<usize> {
    let units = view.units;
    for &i in &view.own {
        let u = &units[i];
        if u.flanked == 0 && u.state != UnitState::Melee {
            continue;
        }
        let found = view.able_enemies().find(|&j| {
            let e = &units[j];
            e.state == UnitState::Melee && dist(u, e) < 60.0 && attack_angle(u, e.x, e.z) > 0
        });
        if found.is_some() {
            return found;
        }
    }
    None
}

#[allow(clippy::too_many_arguments)]
fn plan_horse(
    view: &mut View,
    i: usize,
    roles: &Roles,
    anchor: (f64, f64),
    facing: f64,
    defensive: bool,
    enemy_melee: &[usize],
) {
    if !view.free(i) {
        return;
    }
    let units = view.units;
    let unit = &units[i];
    let general_only = unit.is_general && roles.horse.len() > 1;
    // Mounted shooters skirmish like foot shooters but keep their distance.
    if unit.can_shoot() && unit.ammo > 0 {
        if let Some((j, d)) = view.nearest_enemy(i, |_| true) {
            let e = &units[j];
            let range = view.sim.effective_range(unit, e.x, e.z);
            if d < SHOOTER_SAFETY && is_melee_troop(e) {
                view.move_to(i, unit.x, unit.z - view.forward * 80.0, true, None);
            } else if d <= range * 0.95 {
                view.halt(i);
            } else {
                view.move_to(i, e.x, e.z - view.forward * range * 0.85, false, None);
            }
        }
        return;
    }
    // 1. Enemy cavalry close to us or our shooters: counter-charge.
    let threatened = view.able_enemies().find(|&j| {
        is_horse(&units[j])
            && matches!(units[j].state, UnitState::Charging | UnitState::Marching)
            && dist(unit, &units[j]) < 160.0
    });
    if let Some(j) = threatened {
        if !bristling(&units[j]) {
            view.attack(i, j, true);
            return;
        }
    }
    // 1b. Closing in: ride at the enemy horse (not bristling) within reach.
    if view.assault && !general_only && !unit.is_general {
        let horse = view
            .able_enemies()
            .filter(|&j| is_horse(&units[j]) && !bristling(&units[j]))
            .filter(|&j| units[j].category != UnitCategory::Siege)
            .map(|j| (j, dist(unit, &units[j])))
            .filter(|&(_, d)| d < CAVALRY_REACH)
            .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
        if let Some((j, d)) = horse {
            view.attack(i, j, d < CHARGE_DISTANCE * 3.0);
            return;
        }
    }
    // 2. Isolated shooters (not behind stakes facing us).
    let isolated = view
        .able_enemies()
        .filter(|&j| is_shooter(&units[j]) && units[j].state != UnitState::Melee)
        .filter(|&j| {
            let e = &units[j];
            enemy_melee
                .iter()
                .all(|&m| dist(&units[m], e) > ISOLATION_DISTANCE)
                && !(e.stakes_planted && attack_angle(e, unit.x, unit.z) == 0)
        })
        .map(|j| (j, dist(unit, &units[j])))
        .filter(|&(_, d)| d < CAVALRY_REACH)
        .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    if let (Some((j, _)), false) = (isolated, general_only) {
        view.attack(i, j, true);
        return;
    }
    // 3. Exposed flank: an enemy regiment locked in melee with our troops.
    let exposed = view
        .able_enemies()
        .filter(|&j| {
            let e = &units[j];
            e.state == UnitState::Melee
                && e.formation != Formation::Square
                && !e.has(Ability::PikeSquare)
        })
        .map(|j| (j, dist(unit, &units[j])))
        .filter(|&(_, d)| d < CAVALRY_REACH)
        .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    if let (Some((j, d)), false) = (exposed, general_only) {
        let e = &units[j];
        if attack_angle(e, unit.x, unit.z) > 0 || d < 25.0 {
            view.attack(i, j, true);
        } else {
            // Ride round to its flank first.
            let (rx, rz) = e.right();
            let side = if (unit.x - e.x) * rx + (unit.z - e.z) * rz >= 0.0 {
                1.0
            } else {
                -1.0
            };
            let (w, _) = e.extent();
            let (fx, fz) = e.forward();
            let px = e.x + rx * side * (w * 0.5 + 35.0) - fx * 15.0;
            let pz = e.z + rz * side * (w * 0.5 + 35.0) - fz * 15.0;
            view.move_to(i, px, pz, true, None);
        }
        return;
    }
    // 4. Pursuit of routing regiments.
    let routing = view
        .enemies
        .iter()
        .copied()
        .filter(|&j| units[j].state == UnitState::Routing && units[j].present())
        .map(|j| (j, dist(unit, &units[j])))
        .filter(|&(_, d)| d < 350.0)
        .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    if let Some((j, _)) = routing {
        if unit.target != Some(units[j].id) {
            view.commands.push(Command::Attack {
                units: vec![unit.id],
                target: units[j].id,
                run: true,
            });
        }
        return;
    }
    // 5. A shaken or bled regiment right in front (not bristling): ride it down.
    let shaken =
        |e: &Unit| !bristling(e) && (e.morale < 40.0 || e.hp < f64::from(e.initial_soldiers) * 0.5);
    if let Some((j, d)) = view.nearest_enemy(i, shaken) {
        if d < 110.0 && !defensive && !general_only {
            view.attack(i, j, true);
            return;
        }
    }
    // Otherwise: hold the wing (the general's regiment behind the centre).
    let half = roles
        .line
        .iter()
        .map(|&k| units[k].extent().0 + 10.0)
        .sum::<f64>()
        * 0.5;
    let (x, z) = if general_only || unit.is_general {
        (anchor.0, anchor.1 - view.forward * 50.0)
    } else {
        let side = if unit.x >= anchor.0 { 1.0 } else { -1.0 };
        (
            anchor.0 + side * (half + 45.0),
            anchor.1 - view.forward * 15.0,
        )
    };
    view.move_to(i, x, z, false, Some(facing));
}

/// Reactions common to every plan: face flank attacks, pull out wavering
/// regiments.
fn react(view: &mut View, roles: &Roles) {
    let units = view.units;
    let own = view.own.clone();
    for i in own {
        let u = &units[i];
        if u.state != UnitState::Melee || u.climbing.is_some() {
            continue;
        }
        // Shaken and bled: pull out before it breaks (if a reserve exists).
        if u.morale < 28.0
            && u.hp < f64::from(u.initial_soldiers) * 0.5
            && roles.reserve.is_some_and(|r| r != i)
            && !u.is_general
        {
            view.commands.push(Command::Move {
                units: vec![u.id],
                x: u.x,
                z: u.z - view.forward * 70.0,
                run: true,
                facing: None,
            });
            continue;
        }
        // Attacked on the flank: face the attacker.
        if u.flanked != 0 {
            let flanker = view.able_enemies().find(|&j| {
                let e = &units[j];
                dist(u, e) < 50.0 && attack_angle(u, e.x, e.z) > 0 && e.state == UnitState::Melee
            });
            if let Some(j) = flanker {
                if u.target != Some(units[j].id) {
                    view.commands.push(Command::Attack {
                        units: vec![u.id],
                        target: units[j].id,
                        run: false,
                    });
                }
            }
        }
    }
}

// ----- leader's orders ---------------------------------------------------------

/// Is `id` already given a move (or attack, withdraw) in this step?
fn ordered_to_move(view: &View, id: u32) -> bool {
    view.commands.iter().any(|c| {
        matches!(
            c,
            Command::Move { .. } | Command::Attack { .. } | Command::Withdraw { .. }
        ) && c.units().contains(&id)
    })
}

fn rivals(order: &BattleOrder, own: &str, enemy: &str) -> bool {
    order.ai.as_ref().is_none_or(|ai| {
        ai.rivals.is_empty()
            || ai.rivals.iter().any(|[a, b]| {
                (a.as_str() == own && b.as_str() == enemy)
                    || (a.as_str() == enemy && b.as_str() == own)
            })
    })
}

/// Leader's orders of this step (after the movement plan).
fn plan_orders(view: &mut View, defensive: bool) {
    let sim = view.sim;
    let side = view.side;
    let own_faction = &sim.setup().side(side).faction;
    let enemy_faction = &sim.setup().side(side.other()).faction;
    let mut orders: Vec<&BattleOrder> = sim.order_catalog().iter().collect();
    orders.sort_by(|a, b| a.rank.cmp(&b.rank).then_with(|| a.id.cmp(&b.id)));
    for order in orders {
        let Some(ai) = &order.ai else {
            continue;
        };
        if sim.order_unavailable(side, order).is_some()
            || !rivals(order, own_faction, enemy_faction)
        {
            continue;
        }
        if let Some(max) = ai.max_strength_ratio {
            if view.power(true) / view.power(false).max(1.0) >= max {
                continue;
            }
        }
        let mut targets = sim.order_targets(side, order, &[]);
        if let Some(reach) = ai.enemy_within {
            let close = targets.iter().any(|&i| {
                view.nearest_enemy(i, |_| true)
                    .is_some_and(|(_, d)| d < reach)
            });
            if !close {
                continue;
            }
        }
        match order.kind {
            BattleOrderKind::Dismount => {
                if ai.when_defensive && !defensive {
                    continue;
                }
                let units = view.units;
                targets.retain(|&i| view.free(i) && units[i].state != UnitState::Charging);
            }
            BattleOrderKind::Pavise => {
                let window = ai.under_fire_within.unwrap_or(f64::INFINITY);
                let units = view.units;
                targets.retain(|&i| {
                    let u = &units[i];
                    u.missile_timer <= window
                        && u.destination.is_none()
                        && matches!(u.state, UnitState::Idle | UnitState::Shooting)
                        && !ordered_to_move(view, u.id)
                });
            }
            BattleOrderKind::WarCry | BattleOrderKind::NoQuarter | BattleOrderKind::Rally => {}
        }
        if targets.len() < ai.min_units as usize {
            continue;
        }
        let units = if order.scope == BattleOrderScope::Selected {
            targets.iter().map(|&i| view.units[i].id).collect()
        } else {
            Vec::new()
        };
        view.commands.push(Command::LeaderOrder {
            side: Some(side),
            order: order.id.clone(),
            units,
        });
    }
}

// ----- siege battles ---------------------------------------------------------

fn outer_point(works: &SiegeWorks, piece: usize, offset: f64) -> (f64, f64) {
    let p = &works.pieces[piece];
    let (mx, mz) = p.midpoint();
    let (nx, nz) = p.outward();
    (mx + nx * offset, mz + nz * offset)
}

fn plan_siege_attack(view: &mut View, works: &SiegeWorks) {
    let units = view.units;
    let elapsed = view.sim.elapsed();
    let front = works.front_walls();
    let band = works.band();
    let own = view.own.clone();
    let engines: Vec<usize> = own
        .iter()
        .copied()
        .filter(|&i| units[i].wall_breaker())
        .collect();
    let towers: Vec<usize> = own
        .iter()
        .copied()
        .filter(|&i| units[i].siege_tower())
        .collect();
    let openings = works.openings();
    let engines_working = engines.iter().any(|&i| units[i].ammo > 0) && openings.is_empty();
    let docked: Vec<usize> = (0..works.pieces.len())
        .filter(|&p| works.pieces[p].intact() && works.pieces[p].docked_tower.is_some())
        .collect();
    let towers_rolling = towers.iter().any(|&t| {
        let u = &units[t];
        u.destination.is_some() || u.state == UnitState::Marching
    });
    let storm = !openings.is_empty();
    // Nobody left who can get in: sound the retreat.
    let climbers_left = own.iter().any(|&i| units[i].can_climb());
    let ram_left = own.iter().any(|&i| units[i].ram) && works.pieces[works.gate].intact();
    let engines_left = engines.iter().any(|&i| units[i].ammo > 0);
    if !storm && !climbers_left && !ram_left && !engines_left {
        let all: Vec<u32> = own
            .iter()
            .filter(|&&i| !units[i].withdrawing)
            .map(|&i| units[i].id)
            .collect();
        if !all.is_empty() {
            view.commands.push(Command::Withdraw { units: all });
        }
        return;
    }
    let escalade = !storm
        && (!docked.is_empty()
            || elapsed > ENGINE_PATIENCE
            || (!engines_working && !towers_rolling && elapsed > 20.0));

    // Engines: concentrate on the weakest front wall, then shoot the wall walk.
    let target_piece = front
        .iter()
        .copied()
        .filter(|&p| works.pieces[p].intact())
        .min_by(|&a, &b| {
            works.pieces[a]
                .hp
                .total_cmp(&works.pieces[b].hp)
                .then(a.cmp(&b))
        });
    for &i in &engines {
        let unit = &units[i];
        let range = f64::from(unit.stats.range) * view.sim.weather().range_factor();
        match target_piece {
            Some(p) if openings.len() < 2 => {
                let d = works.pieces[p].distance(unit.x, unit.z);
                if d > range * 0.95 {
                    let (x, z) = outer_point(works, p, range * 0.8);
                    view.move_to(i, x, z, false, None);
                } else {
                    if unit.wall_target != Some(p) {
                        view.commands.push(Command::TargetWall {
                            units: vec![unit.id],
                            piece: p,
                        });
                    }
                    view.halt(i);
                }
            }
            _ => {
                if let Some((j, _)) = view.nearest_enemy(i, |e| e.on_wall) {
                    view.attack(i, j, false);
                }
            }
        }
    }
    // Ram: to the gate while it stands.
    for &i in own.iter().filter(|&&i| units[i].ram) {
        if works.pieces[works.gate].intact() {
            let (x, z) = outer_point(works, works.gate, band + 1.0);
            view.move_to(i, x, z, false, None);
        } else {
            let (x, z) = outer_point(works, works.gate, 60.0);
            view.move_to(i, x + 25.0, z, false, None);
        }
    }
    // Towers: one per front wall.
    let mut tower_pieces: Vec<usize> = Vec::new();
    let intact_front: Vec<usize> = front
        .iter()
        .copied()
        .filter(|&p| works.pieces[p].intact())
        .collect();
    for (k, &i) in towers.iter().enumerate() {
        if intact_front.is_empty() {
            break;
        }
        let p = intact_front[k % intact_front.len()];
        tower_pieces.push(p);
        let (x, z) = outer_point(works, p, band + 2.0);
        view.move_to(i, x, z, false, None);
    }

    // Foot and horse.
    let waiting_z = front_z(works) - 250.0;
    let square = works.center;
    let mut ladder_slot = 0usize;
    for &i in &own {
        let unit = &units[i];
        if unit.category == UnitCategory::Siege || !view.free(i) {
            continue;
        }
        if is_shooter(unit) {
            // Duel with the wall walk, out of the melee.
            if let Some((j, d)) = view.nearest_enemy(i, |e| e.on_wall) {
                let e = &units[j];
                let range = view.sim.effective_range(unit, e.x, e.z);
                if d <= range * 0.9 {
                    view.halt(i);
                } else {
                    let (dx, dz) = ((e.x - unit.x) / d, (e.z - unit.z) / d);
                    let step = d - range * 0.8;
                    view.move_to(i, unit.x + dx * step, unit.z + dz * step, false, None);
                }
            } else if storm {
                view.move_to(i, square.0, square.1, false, None);
            }
            continue;
        }
        // Enemies within reach (no wall in between): fight them.
        if let Some((j, d)) = view.nearest_enemy(i, |e| e.state != UnitState::Routing) {
            if d < 50.0 && view.reachable(i, j) {
                view.attack(i, j, true);
                continue;
            }
        }
        if storm {
            view.move_to(i, square.0, square.1, true, None);
        } else if escalade && unit.can_climb() {
            // Climb at a docked tower if any, else ladders along the front.
            let piece = if !docked.is_empty() {
                docked[ladder_slot % docked.len()]
            } else if !intact_front.is_empty() {
                intact_front[ladder_slot % intact_front.len()]
            } else {
                works.gate
            };
            ladder_slot += 1;
            let (x, z) = if let Some(t) = works.pieces[piece].docked_tower {
                let t = &units[t as usize];
                let (nx, nz) = works.pieces[piece].outward();
                (t.x - nx * 25.0, t.z - nz * 25.0)
            } else {
                outer_point(works, piece, -25.0)
            };
            view.move_to(i, x, z, true, None);
        } else if unit.on_wall || works.inside(unit.x, unit.z) {
            view.move_to(i, square.0, square.1, true, None);
        } else {
            // Wait out of bowshot for the engines and towers.
            view.move_to(i, unit.x, waiting_z.min(unit.z), false, Some(0.0));
        }
    }
}

fn front_z(works: &SiegeWorks) -> f64 {
    works
        .vertices
        .iter()
        .map(|v| v.1)
        .fold(f64::INFINITY, f64::min)
}

fn plan_siege_defence(view: &mut View, works: &SiegeWorks) {
    let units = view.units;
    let band = works.band();
    let own = view.own.clone();
    let openings: Vec<usize> = works.openings();
    // Attackers on the wall walk or inside the town.
    let inside: Vec<usize> = view
        .able_enemies()
        .filter(|&j| {
            let e = &units[j];
            e.on_wall || works.inside(e.x, e.z)
        })
        .collect();
    let climbers: Vec<usize> = view
        .able_enemies()
        .filter(|&j| units[j].climbing.is_some())
        .collect();
    let mut blockers = 0usize;
    for &i in &own {
        let unit = &units[i];
        if !view.free(i) {
            continue;
        }
        if is_shooter(unit) && unit.on_wall {
            // Hold the wall walk; shoot at will. Defend against climbers
            // only with the melee troops.
            view.halt(i);
            continue;
        }
        let near_inside = inside
            .iter()
            .copied()
            .filter(|&j| view.reachable(i, j))
            .map(|j| (j, dist(unit, &units[j])))
            .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
        if unit.on_wall {
            // Wall foot: throw back climbers and attackers on the walls nearby.
            let near_climber = climbers
                .iter()
                .chain(inside.iter())
                .copied()
                .map(|j| (j, dist(unit, &units[j])))
                .filter(|&(_, d)| d < 70.0)
                .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
            if let Some((j, _)) = near_climber {
                view.attack(i, j, false);
            } else {
                view.halt(i);
            }
            continue;
        }
        if let Some((j, d)) = near_inside {
            if d < 250.0 || is_horse(unit) {
                view.attack(i, j, true);
                continue;
            }
        }
        // Block the openings from inside, one regiment per opening first.
        if !openings.is_empty() {
            let p = openings[blockers % openings.len()];
            blockers += 1;
            let (x, z) = outer_point(works, p, -(band + 22.0));
            let (nx, nz) = works.pieces[p].outward();
            if let Some((j, d)) = view.nearest_enemy(i, |e| e.state != UnitState::Routing) {
                if d < 45.0 && view.reachable(i, j) {
                    view.attack(i, j, true);
                    continue;
                }
            }
            view.move_to(i, x, z, true, Some(nx.atan2(nz)));
            continue;
        }
        // Attackers on the walls: send the reserve.
        if let Some(&j) = climbers.first() {
            if !is_horse(unit) {
                let p = units[j].climbing.expect("climbing");
                let (x, z) = outer_point(works, p, -(band + 6.0));
                view.move_to(i, x, z, true, None);
                continue;
            }
        }
        // Otherwise: the gate guard stays, the reserve holds the square.
        let gate_guard = dist_to(
            unit,
            works.pieces[works.gate].midpoint().0,
            works.pieces[works.gate].midpoint().1,
        ) < 60.0;
        if !gate_guard && !works.in_square(unit.x, unit.z) && is_horse(unit) {
            view.move_to(i, works.center.0, works.center.1 + 25.0, false, None);
        }
    }
}
