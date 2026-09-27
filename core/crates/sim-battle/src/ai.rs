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
//! - **Site (B6)**: a defensive side with a hedge, a ditch, a fence or a
//!   village within reach of its deployment line leans on it instead of
//!   the high ground: shooters just behind the obstacle (inside the edge of
//!   the village), the line behind them (or right behind the obstacle when
//!   it has no shooters); behind a hedge, a ditch or houses the shooters
//!   ignore horsemen, whose charge would break. Cavalry never charges
//!   through a hedge or a ditch, nor into a village: it rides round the end
//!   of the obstacle, or waits on its wing.
//! - **Relief (R2b)**, read once per battle ([`crate::relief_ai`]): a
//!   defensive side takes a true crest with a glacis in front (not a scarp),
//!   and its line steps back onto the reverse slope, out of sight of enemy
//!   crossbows, while its shooters hold the crest; a defender clearly above
//!   the enemy keeps its heights; shooters advance to a spot from which
//!   they see their target, preferably higher and out of reach of the enemy
//!   shooters; an advancing line shifts each step aside to go round a steep
//!   rise, runs under arrows, waits for its laggards, and does not charge
//!   at the run up a steep rise from afar.
//! - **Water (EP3)**: a defender with the river between itself and the
//!   enemy holds it (unless much stronger): shooters on its bank at the
//!   crossing the enemy would take, the line just behind them (the
//!   bridgehead). An advancing side picks a crossing (bridge, ford or a
//!   detour) by the march, the width it must file through and the enemy
//!   shooters covering the far end; it waits on its own bank while its
//!   shooters duel with a covered crossing (for a while), then crosses and
//!   forms beyond it. Horsemen do not charge into water or up a steep bank.
//! - **Shooters** fall back behind the line as soon as enemy foot or horse
//!   come close, and disengage from a melee.
//! - **Cavalry** charges isolated shooters, the flanks or rear of enemy
//!   regiments already engaged, answers enemy cavalry, pursues routing
//!   regiments, and never charges pikes or planted stakes head on (R2b: nor
//!   rides a rout or a flank in front of planted stakes).
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

#[path = "ai_modes.rs"]
mod modes;

#[path = "ai_abilities.rs"]
mod abilities;

use crate::command::Command;
use crate::crest::CrestDefenceRules;
use crate::horse_wait::HorseWaitRules;
use crate::position::{military_crest, score_position, Front};
use crate::relief_ai::ReliefMap;
use crate::setup::SideId;
use crate::siege::SiegeWorks;
use crate::sim::{attack_angle, BattleSim};
use crate::site::{Obstacle, OBSTACLE_REACH};
use crate::unit::{Formation, Unit, UnitState};

/// Distance at which the line closes in at the run.
pub const CHARGE_DISTANCE: f64 = 60.0;
/// A defensive line counter-charges enemies this close.
pub const COUNTER_CHARGE_DISTANCE: f64 = 45.0;
/// SG4: shooters behind their planted stakes on the military crest of a
/// defensive position fall back when enemy foot comes this close.
pub const CREST_STAKES_SAFETY: f64 = 45.0;
/// Shooters fall back when enemy melee troops come this close.
pub const SHOOTER_SAFETY: f64 = 70.0;
/// R4: shooters behind a hedge, a ditch or in a village fall back when enemy
/// foot comes this close.
pub const COVER_SAFETY: f64 = 20.0;
/// Distance kept from the field's edges by the AI's moves (a move outside
/// the field is refused).
pub(crate) const FIELD_MARGIN: f64 = 10.0;
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
/// The archery duel is fought when the lines stand this close (metres);
/// EP9b: closer than this, a line closing in after the duel advances in two
/// echelons.
pub const DUEL_RANGE: f64 = 320.0;
/// A side whose losses exceed the enemy's by more than this share is
/// losing the archery duel and closes in (B4).
pub const DUEL_LOSS_MARGIN: f64 = 0.04;
/// AI destinations keep this far from the edge of deep water (F5d).
pub const RIVER_MARGIN: f64 = 12.0;
/// Closing in, the cavalry charges enemy horse this close to the line.
pub const ASSAULT_RANGE: f64 = 250.0;
/// Window in which a clearly stronger attacker rides at the enemy horse
/// to open the fight (F5d).
pub const ATTACKER_PATIENCE: f64 = 240.0;
/// A weaker attacker waits this long for the defender to come to it, then
/// engages anyway (B4: it is the side that sought the battle; armies meet
/// within one to three minutes).
pub const ATTACKER_WAIT: f64 = 90.0;
/// A defensive side gives up waiting after this long ...
pub const DEFENDER_PATIENCE: f64 = 480.0;
/// ... unless nobody has fought for this long: the attacker does not come,
/// the defender keeps its ground and lets the battle be refused (EP9,
/// ADR 0056).
pub const DEFENDER_QUIET: f64 = 60.0;
/// Besiegers wait for their engines at most this long before escalading.
pub const ENGINE_PATIENCE: f64 = 420.0;
/// SG4: a ram whose crew falls below this share of its full crew calls a
/// foot regiment to take it over.
pub const RAM_RELIEF_CREW: f64 = 0.6;
/// SG4: the relieving regiment stands this far behind the ram, away from
/// the gate (outside the reach of the boiling oil).
pub const RAM_RELIEF_STAND: f64 = 14.0;
/// SG4: an assault with no progress (ram blow, ladders, wall walk gained,
/// tower docked, works down) for this long is abandoned.
pub const ASSAULT_STALL: f64 = 300.0;
/// SG4: an assault is abandoned when the besiegers' strength falls below
/// this share of the garrison's (no opening, nobody on the walls).
pub const ASSAULT_HOPELESS: f64 = 0.35;
/// SG4: engines choose the weakest front wall; each metre of distance from
/// the engines weighs as this many HP (the nearest of equal walls).
pub const ENGINE_TARGET_HP_PER_M: f64 = 2.0;

/// R2b: a rise steeper than this (metres per metre over 20 m) is not
/// charged at the run from afar: the regiment walks up and charges close.
pub const STEEP_CLIMB: f64 = 0.20;
/// R2b: a regiment hit by missiles within this many seconds is under fire.
pub const UNDER_FIRE: f64 = 6.0;
/// R2b: a line regiment this far ahead of the line's centre waits for it.
pub const LINE_SLACK: f64 = 30.0;
/// R2b: below this distance a regiment charges whatever the slope.
pub const CLOSE_CHARGE: f64 = 25.0;

/// B8: horsemen give up a pursuit once the routing target has fled this far
/// from the battle line's anchor, and fall back to it instead.
pub const PURSUIT_LEASH: f64 = 280.0;

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

/// SG4: enemy horse this close to one of our shooters draws our horse's
/// counter-charge.
pub const SHOOTER_GUARD: f64 = 160.0;

/// R2b: horsemen keep this far from the front of planted stakes.
pub const STAKES_GUARD: f64 = 35.0;

/// R2b: would horsemen riding from `from` at `target` pass in front of
/// enemy stakes (another regiment's planted stakes within [`STAKES_GUARD`]
/// of the ride, the horsemen coming from their front)? The ride ends on
/// the stakes: a routing regiment flees through its archers, a melee drifts.
fn stakes_in_path(units: &[Unit], from: (f64, f64), target: &Unit) -> bool {
    stakes_on_ride(
        units,
        target.side,
        from,
        (target.x, target.z),
        Some(target.id),
    )
}

/// R2b: does a ride from `from` to `to` pass in front of planted stakes of
/// `foe` (`except` the regiment charged)?
fn stakes_on_ride(
    units: &[Unit],
    foe: SideId,
    from: (f64, f64),
    to: (f64, f64),
    except: Option<u32>,
) -> bool {
    units.iter().any(|k| {
        Some(k.id) != except
            && k.side == foe
            && k.stakes_planted
            && k.able()
            && attack_angle(k, from.0, from.1) == 0
            && segment_distance((k.x, k.z), from, to) < STAKES_GUARD
    })
}

/// Distance from `p` to the segment `a`-`b`.
fn segment_distance(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dz) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dz * dz;
    let t = if len2 < 1e-9 {
        0.0
    } else {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dz) / len2).clamp(0.0, 1.0)
    };
    (p.0 - a.0 - dx * t).hypot(p.1 - a.1 - dz * t)
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
        let (x, z) = field.clamp_inside(x, z, FIELD_MARGIN);
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
                queue: false,
                width: None,
                match_speed: false,
                group_tag: None,
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
                queue: false,
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
    let reach = river.width_at(x) * 0.5 + RIVER_MARGIN;
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
        // CB4: the abilities allowed in sieges (the pavises).
        abilities::plan_abilities(&mut view, side == SideId::Defender, true);
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

/// R4: the ground a defensive side takes: the best of its deployment spot,
/// the heights around it (R2b) and the covers of the site (B6), all scored
/// by [`score_position`] (height, glacis, reverse slope, cover, flanks): a
/// hedge on a crest beats a bare crest and a hedge in a hollow. Returns the
/// front the shooters hold (a crest, or the line behind a cover) and the
/// cover when it is one.
///
/// R2b: the relief is read, not only the height: a true crest (ground above
/// its surroundings) and a glacis in front (ground the enemy must climb)
/// are worth more than a gentle tilt, and a scarp too steep to stand on in
/// order is avoided.
fn defensive_ground(view: &View, roles: &Roles) -> ((f64, f64), Option<Cover>) {
    let field = view.sim.field();
    let map = view.sim.relief_map();
    let around = deployment_center(field, view.side);
    let base = field.height(around.0, around.1);
    let width = front_width(view, roles);
    let reverse = view.able_enemies().any(|j| is_shooter(&view.units[j]));
    let score = |center: (f64, f64)| {
        let front = Front {
            center,
            forward: view.forward,
            width,
        };
        score_position(field, map, front, base, reverse)
    };
    // R4: a position the enemy would reach first (or hardly later) is no
    // position: the line would be caught marching up to it.
    let first = race(view, roles);
    let mut best = (around, None, score(around).ground() + GROUND_MARGIN);
    for ix in -8..=8 {
        for iz in -4..=4 {
            let x = around.0 + f64::from(ix) * 25.0;
            let z = around.1 + f64::from(iz) * 25.0;
            // The centre half of the field's width (300-900 m on the standard field).
            let (center, half) = (field.size.center_x(), 300.0 * field.size.sx());
            if !(center - half..=center + half).contains(&x)
                || !(60.0..=field.depth - 60.0).contains(&z)
            {
                continue;
            }
            // Never step towards the enemy to find a hill.
            if (z - around.1) * view.forward > 30.0 {
                continue;
            }
            // R4: the shooters of a bare crest stand on its military crest:
            // its field of fire is theirs.
            let mut part = score((x, z));
            let front = Front {
                center: (x, z),
                forward: view.forward,
                width,
            };
            part.fire = crate::position::fire_points(
                field,
                Front {
                    center: military_crest(field, front),
                    ..front
                },
            );
            let value = part.ground() - (x - around.0).abs() * 0.01;
            if value > best.2 + 0.5
                && !field.in_forest(x, z)
                && !field.in_mud(x, z)
                && first((x, z))
            {
                best = ((x, z), None, value);
            }
        }
    }
    for (cover, _) in cover_candidates(field, view.side) {
        let (x, z) = cover.center;
        let value = score(cover.center).total()
            - COVER_LATERAL_COST * (x - around.0).abs()
            - COVER_DEPTH_COST * (z - around.1).abs();
        if value > best.2 && first(cover.center) {
            best = (cover.center, Some(cover), value);
        }
    }
    (best.0, best.1)
}

/// R4: the own line must reach a position in less than this share of the
/// time the nearest enemy needs to get there.
const RACE_MARGIN: f64 = 0.6;

/// R4: can the regiments that hold the front (the shooters, else the line;
/// at the pace of the slowest, uphill slowed as in the simulation) reach a
/// spot from their deployment line well before the enemy's battle line (its
/// foot, at the pace of the slowest; the AI's horse waits for its line)
/// from its own? Both measured from the deployment lines, so that the
/// choice holds all battle long instead of flipping as the enemy nears.
fn race<'v>(view: &'v View, roles: &Roles) -> impl Fn((f64, f64)) -> bool + 'v {
    let field = view.sim.field();
    let walkers: &[usize] = if roles.shooters.is_empty() {
        &roles.line
    } else {
        &roles.shooters
    };
    let from = deployment_center(field, view.side);
    let enemy_from = deployment_center(field, view.side.other());
    let own_speed = walkers
        .iter()
        .map(|&i| f64::from(view.units[i].stats.speed))
        .fold(f64::INFINITY, f64::min);
    let enemy_foot = view
        .able_enemies()
        .filter(|&j| !view.units[j].mounted)
        .map(|j| f64::from(view.units[j].stats.speed))
        .fold(f64::INFINITY, f64::min);
    let enemy_speed = if enemy_foot.is_finite() {
        enemy_foot
    } else {
        view.able_enemies()
            .map(|j| f64::from(view.units[j].stats.speed))
            .fold(1.0, f64::max)
    }
    .max(1.0);
    move |spot: (f64, f64)| {
        if !own_speed.is_finite() || own_speed <= 0.0 {
            return true;
        }
        let ours = ReliefMap::march_cost(field, from, spot) / own_speed;
        let theirs = (enemy_from.0 - spot.0).hypot(enemy_from.1 - spot.1) / enemy_speed;
        ours < RACE_MARGIN * theirs
    }
}

/// R4: a spot of the heights must beat the deployment spot by this much.
const GROUND_MARGIN: f64 = 2.0;
/// R4: a cover away from the deployment line costs this much per metre
/// aside and in depth (B6's distance penalty, in points of height).
const COVER_LATERAL_COST: f64 = 0.03;
const COVER_DEPTH_COST: f64 = 0.035;

/// Width of the line of a side (the front whose flanks and cover count).
fn front_width(view: &View, roles: &Roles) -> f64 {
    let list = if roles.line.is_empty() {
        &view.own
    } else {
        &roles.line
    };
    list.iter()
        .map(|&i| view.units[i].extent().0 + 10.0)
        .sum::<f64>()
        .max(40.0)
}

/// R2b: centre of the deployment line of `side`, from which a defensive
/// side looks for its ground (a fixed reference: searching from the moving
/// line would let the reverse slope drag the line back step after step).
/// EP3: read from the field's dimensions (never a fixed 1200 × 800 m).
fn deployment_center(field: &crate::field::Battlefield, side: SideId) -> (f64, f64) {
    match side {
        SideId::Attacker => (field.size.center_x(), field.attacker_line_z()),
        SideId::Defender => (field.size.center_x(), field.defender_line_z()),
    }
}

/// R2b: the line of a defensive side on a crest steps back onto the reverse
/// slope, out of sight, when the enemy has shooters and its own shooters
/// hold the crest in front of it. R4: longbows too, since a volley at a
/// target out of sight must be directed by a friend who sees it and
/// scatters (ADR 0046).
fn reverse_slope_anchor(view: &View, crest: (f64, f64), shooters: bool) -> (f64, f64) {
    let front = Front {
        center: crest,
        forward: view.forward,
        width: 0.0,
    };
    if !shooters {
        // R4: a line without shooters sees its glacis from the military
        // crest.
        return military_crest(view.sim.field(), front);
    }
    if !view.able_enemies().any(|j| is_shooter(&view.units[j])) {
        return crest;
    }
    crate::position::reverse_slope(view.sim.field(), front).unwrap_or(crest)
}

/// B6: a defensive side looks for cover this far on either side of the
/// centre of its deployment line.
pub const COVER_LATERAL: f64 = 110.0;
/// ... this far ahead of its deployment line (towards the enemy) ...
pub const COVER_AHEAD: f64 = 110.0;
/// ... and this far behind it.
pub const COVER_BEHIND: f64 = 90.0;
/// Shooters stand this far behind a hedge, a fence or a ditch (well within
/// [`crate::site::HEDGE_COVER_REACH`]).
pub const COVER_SETBACK: f64 = 7.0;
/// R4: setbacks tried in turn behind a hedge on a crest (the last one still
/// clear of the obstacle's [`OBSTACLE_REACH`]).
const COVER_SETBACKS: [f64; 3] = [COVER_SETBACK, 5.5, 4.5];
/// Shooters stand this far inside the edge of a village.
pub const VILLAGE_SETBACK: f64 = 16.0;
/// Shooters stand this far in front of the line (the usual defensive order).
const SHOOTERS_AHEAD: f64 = 30.0;

/// What a defensive side leans on (B6).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CoverKind {
    Obstacle(crate::site::ObstacleKind),
    Village,
    /// EP3: the bank of the river at a crossing.
    River,
}

/// EP3: shooters hold the bank this far back from the water.
pub const BANK_SETBACK: f64 = 55.0;
/// EP3: a crossing farther than this from the deployment line is not held.
pub const RIVER_REACH: f64 = 420.0;
/// EP3: an advancing side waits this long on its bank while its shooters
/// duel with the enemy shooters covering the crossing.
pub const CROSSING_PATIENCE: f64 = 60.0;
/// EP3: each enemy shooter covering a crossing's far end costs this many
/// metres of march.
pub const CROSSING_EXPOSURE: f64 = 90.0;
/// EP3: the line forms this far beyond a crossing.
pub const BRIDGEHEAD_DEPTH: f64 = 45.0;

/// EP3: the bank a defensive `side` holds when the river lies between its
/// deployment line and the enemy (`enemy`: its centroid): the crossing the
/// enemy would take (cheapest from the enemy to the deployment line), its
/// own-side end, shooters [`BANK_SETBACK`] back from the water, along the
/// river.
pub fn river_hold(
    field: &crate::field::Battlefield,
    side: SideId,
    enemy: (f64, f64),
) -> Option<Cover> {
    let river = field.river.as_ref()?;
    let home = deployment_center(field, side);
    if !ReliefMap::river_between(field, home, enemy) {
        return None;
    }
    let own_north = river.north_of(home.0, home.1);
    let crossing = field
        .crossings()
        .into_iter()
        .map(|c| {
            let cost = ReliefMap::crossing_cost(field, enemy, &c, home, 40.0);
            (c, cost)
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))?
        .0;
    let end = crossing.end(own_north);
    if (end.0 - home.0).hypot(end.1 - home.1) > RIVER_REACH {
        return None;
    }
    let x = end.0;
    let away = if own_north { 1.0 } else { -1.0 };
    let mut center = (
        x,
        river.center_z(x) + away * (river.width_at(x) * 0.5 + BANK_SETBACK),
    );
    // Off the bridge and the road ramp itself: a little aside when needed.
    if field.water_at(center.0, center.1).is_some() {
        center.1 += away * 10.0;
    }
    let slope = river.slope(x);
    let n = (1.0 + slope * slope).sqrt();
    Some(Cover {
        kind: CoverKind::River,
        center,
        along: (1.0 / n, slope / n),
        width: 110.0,
        breaks_charge: true,
    })
}

/// EP3: how an advancing side crosses the river.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CrossingPlan {
    /// End of the crossing on the own bank, and on the enemy's.
    pub near: (f64, f64),
    pub far: (f64, f64),
    /// Enemy shooters covering the far end.
    pub covered: usize,
    pub bridge: bool,
}

/// EP3: the crossing an advancing side takes from `from` towards the enemy
/// (march with the relief, width filed through, enemy shooters covering the
/// far end); `None` when the river does not lie between.
fn crossing_plan(view: &View, from: (f64, f64), line: &[usize]) -> Option<CrossingPlan> {
    let field = view.sim.field();
    let river = field.river.as_ref()?;
    let able: Vec<usize> = view.able_enemies().collect();
    let enemy = view.centroid(&able)?;
    if !ReliefMap::river_between(field, from, enemy) {
        return None;
    }
    let frontage = line
        .iter()
        .map(|&i| view.units[i].extent().0)
        .fold(20.0, f64::max);
    let weather = view.sim.range_factor();
    let foes: Vec<(f64, f64, f64)> = able
        .iter()
        .filter(|&&k| is_shooter(&view.units[k]))
        .map(|&k| {
            let e = &view.units[k];
            (e.x, e.z, f64::from(e.stats.range) * weather)
        })
        .collect();
    let north = river.north_of(from.0, from.1);
    view.sim
        .crossings()
        .iter()
        .map(|c| {
            let far = c.end(!north);
            let covered = ReliefMap::covered(far, &foes);
            let cost = ReliefMap::crossing_cost(field, from, c, enemy, frontage)
                + CROSSING_EXPOSURE * covered as f64;
            (
                CrossingPlan {
                    near: c.end(north),
                    far,
                    covered,
                    bridge: c.bridge.is_some(),
                },
                cost,
            )
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(plan, _)| plan)
}

/// EP3: in the river or on a bridge (a regiment finishing its crossing).
fn crossing_now(view: &View, i: usize) -> bool {
    let (u, field) = (&view.units[i], view.sim.field());
    field.water_kind(u.x, u.z).is_some() || field.bridge_at(u.x, u.z).is_some()
}

/// A defensive position drawn from the site (B6): the front the shooters
/// hold, just behind a hedge, a ditch, a fence, or inside the edge of a
/// village facing the enemy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cover {
    pub kind: CoverKind,
    /// Centre of the shooters' front (x, z), behind the obstacle.
    pub center: (f64, f64),
    /// Unit vector along the front.
    pub along: (f64, f64),
    /// Length of the covered front, in metres.
    pub width: f64,
    /// The obstacle (or the village) breaks cavalry charges.
    pub breaks_charge: bool,
}

impl Cover {
    /// z of the front at `x` (the front follows the obstacle's slope).
    fn z_at(&self, x: f64) -> f64 {
        if self.along.0.abs() < 1e-6 {
            return self.center.1;
        }
        self.center.1 + (x - self.center.0) * self.along.1 / self.along.0
    }
}

/// Best cover for `side` within reach of its deployment line, if any
/// (deterministic: obstacles in index order, strict improvements only).
pub fn defensive_cover(field: &crate::field::Battlefield, side: SideId) -> Option<Cover> {
    let mut best: Option<(Cover, f64)> = None;
    for (cover, score) in cover_candidates(field, side) {
        if best.is_none_or(|(_, s)| score > s) {
            best = Some((cover, score));
        }
    }
    best.map(|(cover, _)| cover)
}

/// B6 score a cover must reach to be considered at all.
const COVER_THRESHOLD: f64 = 15.0;

/// Every cover `side` may lean on within reach of its deployment line, with
/// its B6 score (weight x length minus distance; the village edge last).
fn cover_candidates(field: &crate::field::Battlefield, side: SideId) -> Vec<(Cover, f64)> {
    let (line_z, forward) = match side {
        SideId::Attacker => (field.attacker_line_z(), 1.0),
        SideId::Defender => (field.defender_line_z(), -1.0),
    };
    let reference = (field.size.center_x(), line_z);
    let within = |x: f64, z: f64| {
        let ahead = (z - reference.1) * forward;
        (x - reference.0).abs() <= COVER_LATERAL && (-COVER_BEHIND..=COVER_AHEAD).contains(&ahead)
    };
    let standable = |x: f64, z: f64| {
        field.inside(x, z)
            && !field.in_forest(x, z)
            && !field.in_mud(x, z)
            && field.water_at(x, z).is_none()
    };
    let penalty =
        |x: f64, z: f64| 0.25 * (x - reference.0).abs() + 0.3 * ((z - reference.1) * forward).abs();
    let mut found: Vec<(Cover, f64)> = Vec::new();
    for obstacle in &field.obstacles {
        let len = obstacle.length();
        if len < 30.0 {
            continue;
        }
        let along = (
            (obstacle.b.0 - obstacle.a.0) / len,
            (obstacle.b.1 - obstacle.a.1) / len,
        );
        // The front must face the enemy (roughly across the field).
        if along.0.abs() < 0.7 {
            continue;
        }
        let along = if along.0 < 0.0 {
            (-along.0, -along.1)
        } else {
            along
        };
        let weight = crate::position::obstacle_weight(obstacle.kind);
        let mid = (
            (obstacle.a.0 + obstacle.b.0) * 0.5,
            (obstacle.a.1 + obstacle.b.1) * 0.5,
        );
        // R4: on a crest, the shooters stand closer to the hedge when the
        // ground just behind it would hide the glacis from them.
        // (Ground seen in front of the hedge, from right below it to bowshot.)
        let seen = |back: f64| {
            let spot = (mid.0, mid.1 - forward * back);
            [10.0, 20.0, 40.0, 70.0, 100.0, 150.0]
                .iter()
                .filter(|&&d| {
                    crate::relief_ai::ReliefMap::sees(field, spot, (mid.0, mid.1 + forward * d))
                })
                .count()
        };
        let mut setback = COVER_SETBACK;
        let mut most = seen(COVER_SETBACK);
        for back in COVER_SETBACKS {
            let n = seen(back);
            if n > most {
                (setback, most) = (back, n);
            }
        }
        let center = (mid.0, mid.1 - forward * setback);
        if !within(mid.0, mid.1) || !standable(center.0, center.1) {
            continue;
        }
        // A croft hedge with the houses in front of it would mask the
        // shooting.
        let masked = field.village.as_ref().is_some_and(|v| {
            (v.zone.z - mid.1) * forward > 0.0 && (v.zone.x - mid.0).abs() < v.zone.radius
        });
        if masked {
            continue;
        }
        let score = weight * len.min(120.0) - penalty(mid.0, mid.1);
        if score > COVER_THRESHOLD {
            found.push((
                Cover {
                    kind: CoverKind::Obstacle(obstacle.kind),
                    center,
                    along,
                    width: len,
                    breaks_charge: obstacle.kind.breaks_charge(),
                },
                score,
            ));
        }
    }
    if let Some(village) = &field.village {
        let zone = village.zone;
        let edge = (
            zone.x,
            zone.z + forward * (zone.radius - VILLAGE_SETBACK).max(0.0),
        );
        if within(edge.0, edge.1) && standable(edge.0, edge.1) {
            let width = (zone.radius * 1.4).max(30.0);
            let score = width.min(120.0) - penalty(edge.0, edge.1);
            if score > COVER_THRESHOLD {
                found.push((
                    Cover {
                        kind: CoverKind::Village,
                        center: edge,
                        along: (1.0, 0.0),
                        width,
                        breaks_charge: true,
                    },
                    score,
                ));
            }
        }
    }
    // EP6: the edge of a hamlet, churchyard, manor or farm of the decor
    // facing the enemy (walls, hedges and houses: cover like the village).
    let effects = &crate::decor::DecorRules::bundled().effects;
    for area in &field.decor.areas {
        let effect = effects.of(area.kind);
        if effect.cover > VILLAGE_COVER_MAX || !effect.breaks_charge {
            continue;
        }
        let fp = area.footprint();
        // Half extent of the area along z (towards the enemy).
        let (ax, az) = fp.axis();
        let (fx, fz) = fp.front();
        let half_z = fp.half_length * az.abs() + fp.half_depth * fz.abs();
        let half_x = fp.half_length * ax.abs() + fp.half_depth * fx.abs();
        let edge = (
            area.x,
            area.z + forward * (half_z - VILLAGE_SETBACK).max(0.0),
        );
        if !within(edge.0, edge.1) || !standable(edge.0, edge.1) || !area.contains(edge.0, edge.1) {
            continue;
        }
        let width = (half_x * 1.6).max(30.0);
        let score = width.min(120.0) * (1.0 - effect.cover) * 2.0 - penalty(edge.0, edge.1);
        if score > COVER_THRESHOLD {
            found.push((
                Cover {
                    kind: CoverKind::Village,
                    center: edge,
                    along: (1.0, 0.0),
                    width,
                    breaks_charge: true,
                },
                score,
            ));
        }
    }
    found
}

/// EP6: decor areas whose missile cover is at most this good count as a
/// defensive position (hamlets, churchyards, manors, farms).
const VILLAGE_COVER_MAX: f64 = 0.7;

/// Slots of the shooters along the cover front, in lateral order (B6).
fn cover_slots(view: &View, shooters: &[usize], cover: &Cover) -> Vec<(usize, f64, f64)> {
    let mut order: Vec<usize> = shooters.to_vec();
    order.sort_by(|&a, &b| view.units[a].x.total_cmp(&view.units[b].x).then(a.cmp(&b)));
    let widths: Vec<f64> = order
        .iter()
        .map(|&i| view.units[i].extent().0 + 6.0)
        .collect();
    let total: f64 = widths.iter().sum();
    let mut offset = -total * 0.5;
    order
        .iter()
        .zip(widths)
        .map(|(&i, w)| {
            let lateral = offset + w * 0.5;
            offset += w;
            let x = cover.center.0 + cover.along.0 * lateral;
            (i, x, cover.z_at(x))
        })
        .collect()
}

/// B6: does a charge of `i` at `j` break on the site (a hedge or a ditch in
/// front of the target, the lanes of a village)?
fn charge_breaks(view: &View, i: usize, j: usize) -> bool {
    let (u, e) = (&view.units[i], &view.units[j]);
    let field = view.sim.field();
    field.breaks_charge((u.x, u.z), (e.x, e.z))
        || field.in_village(e.x, e.z)
        || charge_breaks_on_water(field, e)
        || ReliefMap::river_between(field, (u.x, u.z), (e.x, e.z))
}

/// EP3: a target standing in the water, on a bridge or on a steep bank.
fn charge_breaks_on_water(field: &crate::field::Battlefield, e: &Unit) -> bool {
    matches!(
        field.water_kind(e.x, e.z),
        Some(
            crate::hydro::Water::Deep
                | crate::hydro::Water::Ford
                | crate::hydro::Water::Stream(_)
                | crate::hydro::Water::Oxbow
        )
    ) || field.bridge_at(e.x, e.z).is_some()
        || field.bank_kind(e.x, e.z) == Some(crate::hydro::BankKind::Steep)
}

/// B8: dense bocage can chain several hedges between a horse and its target;
/// this many are tried in turn before giving up and waiting.
const DETOUR_HOPS: u32 = 4;

/// The point beyond the nearer end of `blocking`, on the target's side, from
/// which `probe` can ride on towards `to` clear of that one obstacle.
fn detour_past(probe: (f64, f64), to: (f64, f64), blocking: &Obstacle) -> Option<(f64, f64)> {
    let len = blocking.length().max(1e-6);
    let side_of = |p: (f64, f64)| {
        (blocking.b.0 - blocking.a.0) * (p.1 - blocking.a.1)
            - (blocking.b.1 - blocking.a.1) * (p.0 - blocking.a.0)
    };
    // Normal pointing to the target's side of the obstacle.
    let mut normal = (
        -(blocking.b.1 - blocking.a.1) / len,
        (blocking.b.0 - blocking.a.0) / len,
    );
    if side_of(to) < 0.0 {
        normal = (-normal.0, -normal.1);
    }
    [(blocking.a, blocking.b), (blocking.b, blocking.a)]
        .into_iter()
        .map(|(end, other)| {
            let out = ((end.0 - other.0) / len, (end.1 - other.1) / len);
            (
                end.0 + out.0 * DETOUR_CLEARANCE + normal.0 * 10.0,
                end.1 + out.1 * DETOUR_CLEARANCE + normal.1 * 10.0,
            )
        })
        .map(|w| {
            let path = (w.0 - probe.0).hypot(w.1 - probe.1) + (to.0 - w.0).hypot(to.1 - w.1);
            (w, path)
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(w, _)| w)
}

/// B6/B8: a point beyond the hedges or ditches that break the charge of `i`
/// at `j`, from which the charge is clear; `None` when there is none (a
/// village, or no clear way round after [`DETOUR_HOPS`] tries): the horse
/// waits. B8: a dense network of hedges (bocage) is walked hedge by hedge
/// instead of only trying the ends of the first one in the way, which left
/// the horse waiting in front of the next hedge over.
fn detour(view: &View, i: usize, j: usize) -> Option<(f64, f64)> {
    let (u, e) = (&view.units[i], &view.units[j]);
    let field = view.sim.field();
    if field.in_village(e.x, e.z) {
        return None;
    }
    let to = (e.x, e.z);
    let mut probe = (u.x, u.z);
    let mut moved = false;
    for _ in 0..DETOUR_HOPS {
        let blocking = field.obstacles.iter().find(|o| {
            o.kind.breaks_charge()
                && o.distance(to.0, to.1) <= crate::site::HEDGE_COVER_REACH
                && (o.crosses(probe, to) || o.distance(probe.0, probe.1) <= 2.0 * OBSTACLE_REACH)
        });
        let Some(blocking) = blocking else {
            return moved.then_some(probe);
        };
        probe = detour_past(probe, to, blocking).filter(|&(x, z)| {
            field.inside(x, z) && !field.in_forest(x, z) && field.water_at(x, z).is_none()
        })?;
        moved = true;
    }
    // Still blocked after DETOUR_HOPS: a network too dense to clear.
    None
}

/// B6: horsemen ride this far past the end of a hedge before charging.
const DETOUR_CLEARANCE: f64 = 35.0;

/// B6: charge `j` when the charge is clear; otherwise ride round the
/// obstacle in the way. `false` when neither is possible (the caller moves
/// on to its next choice, or waits).
fn charge_or_detour(view: &mut View, i: usize, j: usize, run: bool) -> bool {
    if !charge_breaks(view, i, j) {
        view.attack(i, j, run);
        return true;
    }
    match detour(view, i, j) {
        Some((x, z)) => {
            view.move_to(i, x, z, true, None);
            true
        }
        None => false,
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
    // R2b: a defender standing clearly above the enemy does not give up its
    // ground to meet it (unless much stronger).
    let holds_heights =
        view.side == SideId::Defender && ratio < HOLD_RATIO && height_edge(view) > HOLD_HEIGHT;
    // EP3: a defender behind a river holds it (unless much stronger).
    let enemy_center = {
        let able: Vec<usize> = view.able_enemies().collect();
        view.centroid(&able)
    };
    let river_ahead = enemy_center.is_some_and(|e| {
        ReliefMap::river_between(
            view.sim.field(),
            deployment_center(view.sim.field(), view.side),
            e,
        )
    });
    let holds_river = view.side == SideId::Defender && ratio < HOLD_RATIO && river_ahead;
    // R4: an attacker clearly above an enemy made mostly of shooters waits
    // on its heights for a while rather than walking down into the arrows
    // (then attacks: no frozen battle).
    let attacker_holds = view.side == SideId::Attacker
        && ratio < HOLD_RATIO
        && elapsed < ATTACKER_HOLD_TIME
        && enemy_shooter_share(view) > SHOOTER_ARMY
        && side_losses(view.units, view.side)
            <= side_losses(view.units, view.side.other()) + DUEL_LOSS_MARGIN
        && height_edge(view) > HOLD_HEIGHT;
    // R4: an evenly matched defender whose enemy is marching on it waits
    // for it on its ground (behind its hedge, on its crest) rather than
    // leaving it as soon as its archers have thinned the enemy ranks.
    // An army made mostly of shooters needs the enemy to come to it
    // (Crécy, Agincourt): its strength counts only while it holds.
    let receives = view.side == SideId::Defender
        && (ratio < HOLD_RATIO
            || roles
                .shooters
                .iter()
                .map(|&i| unit_power(&view.units[i]))
                .sum::<f64>()
                > SHOOTER_ARMY * own_power)
        && enemy_melee.iter().any(|&j| {
            let e = &view.units[j];
            matches!(e.state, UnitState::Marching | UnitState::Charging)
                && front_ref
                    .iter()
                    .any(|&i| dist(&view.units[i], e) < RECEIVE_DISTANCE)
        });
    let defensive = match view.side {
        SideId::Defender => {
            (ratio < 0.85 || holds_heights || holds_river || receives)
                && (elapsed < DEFENDER_PATIENCE || view.sim.quiet_for(DEFENDER_QUIET))
        }
        SideId::Attacker => (ratio < 0.8 && elapsed < ATTACKER_WAIT) || attacker_holds,
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
    // B4: a side bleeding faster than its enemy under the arrows stops
    // trading volleys and closes in.
    let press = view.side == SideId::Attacker && ratio * 0.85 > 1.0;
    let losing = side_losses(view.units, view.side)
        > side_losses(view.units, view.side.other()) + DUEL_LOSS_MARGIN;
    // EP9 (ADR 0056): the side that sought the battle must attack; it trades
    // volleys only for a while. EP9b: longer while its shooters are clearly
    // winning the duel (sliding window, `data/rules/battle_duel.json`).
    let duel_rules = view.sim.duel_rules();
    let window = duel_rules.window_seconds;
    let winning_duel = duel_rules.winning(
        view.sim.recent_missile_losses(view.side, window),
        view.sim.recent_missile_losses(view.side.other(), window),
    );
    let duel = !roles.shooters.is_empty()
        && shooters_have_ammo
        && contact < DUEL_RANGE
        && elapsed
            < if press {
                ATTACKER_DUEL_TIME
            } else if view.side == SideId::Attacker {
                duel_rules.attacker_limit(winning_duel)
            } else {
                DUEL_TIME
            }
        && !losing
        && (enemy_shooters == 0 || own_ranged >= enemy_ranged * 0.8);

    // EP9b: closing in, the militia (low base morale) marches in a second
    // echelon behind the solid foot rather than leading the assault through
    // the arrows. The line is measured on its first echelon (the second
    // follows it).
    let (first, second): (Vec<usize>, Vec<usize>) = roles
        .line
        .iter()
        .partition(|&&i| view.units[i].morale_cap >= duel_rules.second_echelon_morale);
    let echelons = !defensive
        && !duel
        && contact < DUEL_RANGE
        && contact >= duel_rules.second_echelon_closes_m
        && !first.is_empty()
        && !second.is_empty();
    let line_center = view
        .centroid(if echelons { &first } else { &roles.line })
        .or_else(|| view.centroid(&view.own))
        .expect("own not empty");
    let facing = if view.forward > 0.0 {
        0.0
    } else {
        std::f64::consts::PI
    };
    // Only to open the fight (first minutes, nobody locked yet).
    let melee = view.units.iter().any(|u| u.state == UnitState::Melee);
    // B4: an attacker bleeding under the enemy arrows (and not waiting as
    // the weaker side) also sends its horse at the enemy horse.
    let opens = press || (view.side == SideId::Attacker && losing);
    view.assault = opens
        && !defensive
        && !duel
        && !melee
        && elapsed < ATTACKER_PATIENCE
        && contact < ASSAULT_RANGE;
    // B6/R4: a defensive side takes the best ground within reach, cover
    // and relief scored together: behind a hedge, a ditch or in a village
    // (shooters just behind it, the line behind them), or on a crest.
    // EP3 first: the river bank at the crossing the enemy would take when
    // the river lies between us and the enemy and the crossing is within
    // reach (ADR 0033, "Cohabitation avec R4").
    let ground = if defensive && !attacker_holds {
        let bank = enemy_center.and_then(|e| river_hold(view.sim.field(), view.side, e));
        Some(match bank {
            Some(bank) => (bank.center, Some(bank)),
            None => defensive_ground(view, &roles),
        })
    } else {
        None
    };
    let cover = ground.and_then(|(_, c)| c);
    // EP3: an advancing side with the river in front picks its crossing.
    let crossing = if defensive {
        None
    } else {
        crossing_plan(view, line_center, &roles.line)
    };
    let mut shooter_anchor = None;
    // Where the line stands this step.
    let anchor = if let Some(c) = cover {
        let back = if roles.shooters.is_empty() {
            5.0
        } else {
            SHOOTERS_AHEAD
        };
        (c.center.0, c.z_at(c.center.0) - view.forward * back)
    } else if let Some((crest, _)) = ground {
        let post = reverse_slope_anchor(view, crest, !roles.shooters.is_empty());
        // SG5: an army on its heights, with its shooters holding the crest
        // in front, stands its line back out of reach of their rout (and of the horse
        // fighting in front of them): a broken regiment there no longer
        // carries the line with it before the melee (ADR 0046 § Suite SG5).
        // Measured from the crest itself: the line stepping back must not
        // change the decision.
        let field = view.sim.field();
        let enemies: Vec<usize> = view.able_enemies().collect();
        let below = view.centroid(&enemies).is_some_and(|(x, z)| {
            field.height(crest.0, crest.1) - field.height(x, z) > HOLD_HEIGHT
        });
        // A small force keeps its line by its shooters (R4); an army
        // deploys in depth.
        let rules = CrestDefenceRules::bundled();
        // A historical deployment (EP7) keeps the regiments on their posts.
        let posted = roles
            .line
            .iter()
            .any(|&i| view.sim.scenario_post(i).is_some());
        if !roles.shooters.is_empty()
            && below
            && !posted
            && roles.line.len() >= rules.min_line_regiments
        {
            (post.0, post.1 - view.forward * rules.line_setback_m)
        } else {
            post
        }
    } else if attacker_holds {
        // R4: an attacker above an enemy of shooters waits on its heights.
        line_center
    } else if let Some(plan) = crossing {
        // Shooters cover the crossing from the own bank.
        let (dx, dz) = (plan.far.0 - plan.near.0, plan.far.1 - plan.near.1);
        let len = dx.hypot(dz).max(1e-6);
        let ahead = (dx / len, dz / len);
        shooter_anchor = Some((
            plan.near.0 - ahead.0 * 8.0,
            plan.near.1 - ahead.1 * 8.0 - view.forward * SHOOTERS_AHEAD,
        ));
        let wait = plan.covered > 0
            && elapsed < CROSSING_PATIENCE
            && shooters_have_ammo
            && !roles.shooters.is_empty()
            && !losing;
        if wait {
            // Do not file across under the arrows: duel from the bank first.
            (plan.near.0 - ahead.0 * 60.0, plan.near.1 - ahead.1 * 60.0)
        } else {
            (
                plan.far.0 + ahead.0 * BRIDGEHEAD_DEPTH,
                plan.far.1 + ahead.1 * BRIDGEHEAD_DEPTH,
            )
        }
    } else if duel && contact < 260.0 {
        line_center
    } else {
        advance(view, line_center)
    };

    // Line (EP9b: in two echelons when closing in).
    let slots = if !echelons {
        view.line_slots(&roles.line, anchor, facing)
    } else {
        let depth = duel_rules.second_echelon_depth_m;
        let behind = (anchor.0, anchor.1 - view.forward * depth);
        let mut slots = view.line_slots(&first, anchor, facing);
        slots.extend(view.line_slots(&second, behind, facing));
        slots
    };
    for (i, x, z) in slots {
        if !view.free(i) {
            continue;
        }
        // ADR 0052: archers out of arrows fall only on horsemen already held
        // in a melee (Agincourt); they do not walk alone into fresh knights.
        let spent_archers = view.units[i].category == UnitCategory::Ranged;
        let target = view.nearest_enemy(i, |e| {
            e.state != UnitState::Routing
                && !(spent_archers && is_horse(e) && e.state != UnitState::Melee)
        });
        // R2b: a regiment well ahead of the line waits for it rather than
        // arriving alone under the enemy arrows (fast archers out of
        // arrows outpace the men-at-arms).
        // EP3: a regiment in the river or on a bridge finishes crossing.
        let ahead =
            (view.units[i].z - line_center.1) * view.forward > LINE_SLACK && !crossing_now(view, i);
        match target {
            Some((_, d)) if ahead && !defensive && d >= CHARGE_DISTANCE => view.halt(i),
            Some((j, d)) if !defensive && !duel && d < CHARGE_DISTANCE * 2.0 => {
                let j = opposite(view, i).unwrap_or(j);
                let run = d < CHARGE_DISTANCE && !steep_charge(view, i, j);
                view.attack(i, j, run);
            }
            Some((j, d)) if d < COUNTER_CHARGE_DISTANCE => view.attack(i, j, true),
            // R4: a defensive line comes to the help of its shooters caught
            // in a melee in front of it (the men-at-arms beside the archers).
            _ if defensive && rescue(view, i, &roles.shooters).is_some() => {
                let j = rescue(view, i, &roles.shooters).expect("checked");
                view.attack(i, j, true);
            }
            _ => {
                // R2b: an advancing line under arrows closes at the run
                // rather than walking up to the enemy shooters.
                let u = &view.units[i];
                let run = !defensive
                    && !duel
                    && u.missile_timer < UNDER_FIRE
                    && (z - u.z) * view.forward > 5.0;
                view.move_to(i, x, z, run, Some(facing));
            }
        }
    }

    // Shooters: in front of the line, halt in range, fall back when threatened.
    let slots = cover
        .map(|c| cover_slots(view, &roles.shooters, &c))
        .unwrap_or_default();
    for &i in &roles.shooters {
        let slot = slots
            .iter()
            .find(|s| s.0 == i)
            .map(|&(_, x, z)| (x, z))
            .zip(cover);
        let at = shooter_anchor.unwrap_or(anchor);
        let line_z = if shooter_anchor.is_some() {
            at.1
        } else {
            line_center.1
        };
        let crest = ground.filter(|(_, c)| c.is_none()).map(|(p, _)| p);
        plan_shooter(view, i, at, line_z, facing, defensive, slot, crest);
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
    // CB2: guard for a defensive line, skirmish for light shooters.
    modes::plan_modes(view, &roles, defensive);
    // CB4: one simple rule per active ability.
    abilities::plan_abilities(view, defensive, false);
}

/// R2b: a defender this much higher than the enemy (mean ground under the
/// regiments, metres) holds its heights ...
pub const HOLD_HEIGHT: f64 = 6.0;
/// ... unless it is this much stronger.
pub const HOLD_RATIO: f64 = 1.25;

/// R4: a defensive line regiment helps its shooters in a melee this close.
pub const RESCUE_DISTANCE: f64 = 90.0;

/// R4: the nearest enemy in a melee with one of `shooters`, within
/// [`RESCUE_DISTANCE`] of line regiment `i`.
fn rescue(view: &View, i: usize, shooters: &[usize]) -> Option<usize> {
    let u = &view.units[i];
    view.able_enemies()
        .filter(|&j| view.units[j].state == UnitState::Melee)
        .filter(|&j| {
            shooters.iter().any(|&s| {
                view.units[s].state == UnitState::Melee
                    && dist(&view.units[s], &view.units[j]) < 30.0
            })
        })
        .map(|j| (j, dist(u, &view.units[j])))
        .filter(|&(_, d)| d < RESCUE_DISTANCE)
        .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
        .map(|(j, _)| j)
}

/// R4: shooters run to the military crest when the enemy is this close.
pub const POST_RUN: f64 = 400.0;
/// R4: a defender receives an enemy marching on it from this close.
pub const RECEIVE_DISTANCE: f64 = 250.0;
/// R4: an attacker above an enemy of shooters waits at most this long.
pub const ATTACKER_HOLD_TIME: f64 = 150.0;
/// R4: ... when shooters make more than this share of the enemy's power.
pub const SHOOTER_ARMY: f64 = 0.5;

/// Share of the able enemy's power in its shooters.
fn enemy_shooter_share(view: &View) -> f64 {
    let (shooters, all) = view
        .able_enemies()
        .filter(|&j| !view.units[j].synthetic)
        .fold((0.0, 0.0), |(s, a), j| {
            let p = unit_power(&view.units[j]);
            (s + if is_shooter(&view.units[j]) { p } else { 0.0 }, a + p)
        });
    if all > 0.0 {
        shooters / all
    } else {
        0.0
    }
}

/// R2b: mean ground under the own regiments minus that under the enemy's.
fn height_edge(view: &View) -> f64 {
    let field = view.sim.field();
    let mean = |list: &mut dyn Iterator<Item = usize>| {
        let (sum, n) = list
            .filter(|&i| !view.units[i].synthetic)
            .fold((0.0, 0.0), |(s, n), i| {
                let u = &view.units[i];
                (s + field.height(u.x, u.z), n + 1.0)
            });
        if n > 0.0 {
            sum / n
        } else {
            0.0
        }
    };
    mean(&mut view.own.iter().copied()) - mean(&mut view.able_enemies())
}

/// Share of its initial soldiers a side has lost (B4, archery duel).
fn side_losses(units: &[Unit], side: SideId) -> f64 {
    let (hp, initial) = units
        .iter()
        .filter(|u| u.side == side && !u.synthetic)
        .fold((0.0, 0.0), |(hp, initial), u| {
            (hp + u.hp.max(0.0), initial + f64::from(u.initial_soldiers))
        });
    if initial > 0.0 {
        1.0 - hp / initial
    } else {
        0.0
    }
}

/// B8: within the forward arc, the line leans this many metres (at most)
/// towards a defender offset sideways from dead ahead, instead of marching
/// straight past it. A defender squarely in front (`dx` ~ 0) is unaffected.
const ADVANCE_LEAN_MAX: f64 = 20.0;

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
    let straight = if dz * view.forward > 0.5 * d {
        // Ahead: march forward, leaning towards a defender offset
        // sideways (B8).
        let lean = dx.clamp(-ADVANCE_LEAN_MAX, ADVANCE_LEAN_MAX);
        (from.0 + lean, from.1 + view.forward * 45.0)
    } else {
        let step = d.min(45.0) / d.max(1e-6);
        (from.0 + dx * step, from.1 + dz * step)
    };
    relief_step(view, from, straight)
}

/// R2b: lateral shifts tried for each step of an advancing line.
const RELIEF_LEANS: [f64; 4] = [-15.0, 15.0, -30.0, 30.0];
/// R2b: a shifted step must save this much march (metres) to be taken.
const RELIEF_SAVING: f64 = 4.0;

/// R2b: of the step `from` -> `to` and the same step shifted aside, the one
/// the relief makes cheapest (round a steep rise by a valley or a shelf
/// rather than straight up it); the straight step on ties.
fn relief_step(view: &View, from: (f64, f64), to: (f64, f64)) -> (f64, f64) {
    let field = view.sim.field();
    let (dx, dz) = (to.0 - from.0, to.1 - from.1);
    let len = dx.hypot(dz).max(1e-6);
    let side = (dz / len, -dx / len);
    let cost = |p: (f64, f64)| {
        ReliefMap::march_cost(field, from, p)
            + 40.0 * (ReliefMap::climb(field, from, p) - STEEP_CLIMB).max(0.0)
    };
    let mut best = (to, cost(to) - RELIEF_SAVING);
    for lean in RELIEF_LEANS {
        let p = (to.0 + side.0 * lean, to.1 + side.1 * lean);
        if !field.inside(p.0, p.1)
            || field.in_forest(p.0, p.1)
            || field.water_at(p.0, p.1).is_some()
        {
            continue;
        }
        let c = cost(p);
        if c < best.1 {
            best = (p, c);
        }
    }
    best.0
}

/// R2b: would `i` charge `j` up a steep rise from afar? It then walks up
/// and charges once close.
fn steep_charge(view: &View, i: usize, j: usize) -> bool {
    let (u, e) = (&view.units[i], &view.units[j]);
    dist(u, e) > CLOSE_CHARGE
        && ReliefMap::climb(view.sim.field(), (u.x, u.z), (e.x, e.z)) > STEEP_CLIMB
}

/// Enemy regiment opposite `i` (smallest lateral offset, a bit of depth).
/// R4: the strength of the ground each one holds counts too, so that the
/// line strikes the weak point (the open ground rather than the hedge or
/// the crest).
fn opposite(view: &View, i: usize) -> Option<usize> {
    let u = &view.units[i];
    let field = view.sim.field();
    let map = view.sim.relief_map();
    let base = field.height(u.x, u.z);
    view.able_enemies()
        .filter(|&j| !is_horse(&view.units[j]) || view.units[j].state == UnitState::Melee)
        .filter(|&j| view.reachable(i, j))
        .map(|j| {
            let e = &view.units[j];
            let front = Front {
                center: (e.x, e.z),
                forward: -view.forward,
                width: e.extent().0,
            };
            let strength = score_position(field, map, front, base, false).total();
            (
                j,
                (e.x - u.x).abs() + 0.3 * (e.z - u.z).abs() + WEAK_POINT * strength.max(0.0),
            )
        })
        .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
        .map(|(j, _)| j)
}

/// R4: metres of lateral offset a line regiment accepts to strike an enemy
/// holding one point less of ground ([`score_position`]).
pub const WEAK_POINT: f64 = 1.5;

/// `cover`: the slot of the shooter behind the site's cover (B6); `crest`:
/// the bare crest the side holds (R4, military crest).
#[allow(clippy::too_many_arguments)]
fn plan_shooter(
    view: &mut View,
    i: usize,
    anchor: (f64, f64),
    line_z: f64,
    facing: f64,
    defensive: bool,
    cover: Option<((f64, f64), Cover)>,
    crest: Option<(f64, f64)>,
) {
    let unit = &view.units[i];
    // B6: behind a hedge, a ditch or in a village, horsemen are no threat
    // (their charge breaks on it).
    let threat = match cover {
        Some((_, c)) if c.breaks_charge => {
            view.nearest_enemy(i, |e| is_melee_troop(e) && !is_horse(e))
        }
        // R4: on the military crest of a defensive position, behind
        // planted stakes, horsemen in front are no threat either (their
        // charge breaks on the stakes; Crécy, Agincourt): the archers hold
        // the crest and keep shooting instead of falling back out of sight.
        _ if defensive && crest.is_some() && unit.stakes_planted => view.nearest_enemy(i, |e| {
            is_melee_troop(e) && (!is_horse(e) || attack_angle(unit, e.x, e.z) != 0)
        }),
        _ => view.nearest_enemy(i, is_melee_troop),
    };
    // Engaged or about to be: fall back behind the line.
    if let Some((_, d)) = threat {
        // R4: behind a hedge, a ditch or houses, the foot must cross them
        // too: the shooters keep shooting until it is close.
        // Behind their stakes on the military crest too: falling back would
        // leave the glacis out of sight.
        let safety = match cover {
            Some((_, c)) if c.breaks_charge => COVER_SAFETY,
            _ if defensive && crest.is_some() && unit.stakes_planted => CREST_STAKES_SAFETY,
            _ => SHOOTER_SAFETY,
        };
        if d < safety || view.engaged(i) {
            let rear = (line_z * view.forward).min(anchor.1 * view.forward) * view.forward;
            let behind = (unit.x, rear - view.forward * 45.0);
            if (unit.z - behind.1) * view.forward > 8.0 || view.engaged(i) {
                let (x, z) = view
                    .sim
                    .field()
                    .clamp_inside(behind.0, behind.1, FIELD_MARGIN);
                view.commands.push(Command::Move {
                    units: vec![unit.id],
                    x,
                    z,
                    run: true,
                    facing: Some(facing),
                    queue: false,
                    width: None,
                    match_speed: false,
                    group_tag: None,
                });
            }
            return;
        }
    }
    if !view.free(i) {
        return;
    }
    let front_z = anchor.1 + view.forward * SHOOTERS_AHEAD;
    if let Some((j, d)) = view.nearest_enemy(i, |_| true) {
        let e = &view.units[j];
        let range = view.sim.effective_range(unit, e.x, e.z);
        if let Some(((x, z), c)) = cover {
            // EP3: on the way to the bank, shoot as soon as in range.
            if c.kind == CoverKind::River && d <= range * 0.95 && dist_to(unit, x, z) < 60.0 {
                view.halt(i);
                return;
            }
            // B6: reach the cover first (the threat above sends them back
            // when the enemy closes in).
            // R4: right up to the slot (6 m short, behind a hedge on a
            // crest, would leave the glacis in dead ground).
            if dist_to(unit, x, z) > 2.0 {
                // R4: at the run when the enemy comes on.
                view.move_to(i, x, z, false, Some(facing));
            } else {
                view.halt(i);
            }
            return;
        }
        // R4: holding a crest, the shooters stand on its military crest
        // (never behind the usual post in front of the line), where they see
        // the glacis, rather than halting wherever a target first comes in
        // range.
        // A crest that sees its glacis needs none of this.
        let post = crest.filter(|_| defensive).and_then(|(_, cz)| {
            let front = Front {
                center: (unit.x, cz),
                forward: view.forward,
                width: unit.extent().0,
            };
            let mc = military_crest(view.sim.field(), front).1;
            (mc != cz).then_some(mc)
        });
        if let Some(mc) = post {
            let z = if (mc - front_z) * view.forward > 0.0 {
                mc
            } else {
                front_z
            };
            if (unit.z - z).abs() > 6.0 {
                // At the run when the enemy comes on: the crest must be
                // held before it arrives.
                let run = d < POST_RUN;
                view.move_to(i, unit.x, z, run, Some(facing));
            } else {
                view.halt(i);
            }
            return;
        }
        // R4: in range and able to shoot it (in sight, or lobbing at a
        // target a friend sees).
        if d <= range * 0.95 && view.sim.fire_mode(unit, e).is_some() {
            view.halt(i);
            return;
        }
        if defensive {
            view.move_to(i, unit.x, front_z, false, Some(facing));
            return;
        }
        // Advance to shooting range, never far ahead of the line. R2b:
        // to a spot from which the target is seen, preferably higher and
        // out of reach of the enemy shooters.
        let limit = front_z + view.forward * 40.0;
        if let Some((x, z)) = firing_spot(view, i, j, limit) {
            view.move_to(i, x, z, false, None);
            return;
        }
        let wanted = (unit.z * view.forward + d - range * 0.85).min(front_z * view.forward + 40.0)
            * view.forward;
        if (wanted - unit.z) * view.forward > 5.0 {
            view.move_to(i, unit.x, wanted, false, None);
        }
    }
}

/// R2b: lateral offsets of the firing spots tried by a shooter.
const FIRING_LATERALS: [f64; 5] = [0.0, -20.0, 20.0, -40.0, 40.0];
/// R2b: a firing spot within reach of this many enemy shooters costs this
/// much march (metres) each.
const EXPOSURE_COST: f64 = 60.0;

/// R2b: where shooter `i` should stand to shoot `j`, marching towards it no
/// farther than `limit` (z): a spot within its range (height counted), from
/// which it sees the target, preferably out of
/// reach of the enemy shooters and higher than the target; `None` when no
/// such spot lies ahead.
fn firing_spot(view: &View, i: usize, j: usize, limit: f64) -> Option<(f64, f64)> {
    let field = view.sim.field();
    let (u, t) = (&view.units[i], &view.units[j]);
    let weather = view.sim.range_factor();
    let reach = f64::from(u.stats.range) * weather;
    let ht = field.height(t.x, t.z);
    let foes: Vec<(f64, f64, f64, f64)> = view
        .able_enemies()
        .filter(|&k| is_shooter(&view.units[k]))
        .map(|k| {
            let e = &view.units[k];
            (
                e.x,
                e.z,
                field.height(e.x, e.z),
                f64::from(e.stats.range) * weather,
            )
        })
        .collect();
    let (dx, dz) = (t.x - u.x, t.z - u.z);
    let d = dx.hypot(dz).max(1e-6);
    let (ux, uz) = (dx / d, dz / d);
    let mut best: Option<((f64, f64), f64)> = None;
    for lateral in FIRING_LATERALS {
        for k in 0..=30 {
            let s = f64::from(k) * 10.0;
            let c = (u.x + ux * s + uz * lateral, u.z + uz * s - ux * lateral);
            if (c.1 - limit) * view.forward > 0.0 {
                break;
            }
            if !field.inside(c.0, c.1)
                || field.in_forest(c.0, c.1)
                || field.in_mud(c.0, c.1)
                || field.water_at(c.0, c.1).is_some()
            {
                continue;
            }
            let hc = field.height(c.0, c.1);
            let range = reach * (1.0 + (hc - ht).max(0.0) / 100.0);
            if (t.x - c.0).hypot(t.z - c.1) > range * 0.93 {
                continue;
            }
            // R4: volleys too aim at what they see (a lob over a crest is a
            // last resort, directed by a friend and scattered).
            if !ReliefMap::sees(field, c, (t.x, t.z)) {
                continue;
            }
            let exposed = foes
                .iter()
                .filter(|&&(x, z, h, r)| {
                    (x - c.0).hypot(z - c.1) <= r * (1.0 + (h - hc).max(0.0) / 100.0) + 5.0
                })
                .count();
            let score = ReliefMap::march_cost(field, (u.x, u.z), c)
                + EXPOSURE_COST * exposed as f64
                - 2.0 * (hc - ht).clamp(-10.0, 10.0);
            if best.is_none_or(|(_, b)| score < b) {
                best = Some((c, score));
            }
            // Farther along this ray only costs more march.
            break;
        }
    }
    best.map(|(c, _)| c)
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
    // 1. Enemy cavalry close to us or our shooters: counter-charge (SG4:
    // the shooters' guard too, not only the horse the enemy rides at).
    let threatened = view.able_enemies().find(|&j| {
        let e = &units[j];
        is_horse(e)
            && matches!(e.state, UnitState::Charging | UnitState::Marching)
            && (dist(unit, e) < 160.0
                || (!defensive
                    && dist(unit, e) < CAVALRY_REACH
                    && roles
                        .shooters
                        .iter()
                        .any(|&s| units[s].able() && dist(&units[s], e) < SHOOTER_GUARD)))
    });
    if let Some(j) = threatened {
        if !bristling(&units[j]) {
            view.attack(i, j, true);
            return;
        }
    }
    // 1b. Closing in: ride at the enemy horse (not bristling) within reach.
    if view.assault && !general_only && !unit.is_general {
        let mut horse: Vec<(usize, f64)> = view
            .able_enemies()
            .filter(|&j| is_horse(&units[j]) && !bristling(&units[j]))
            .filter(|&j| units[j].category != UnitCategory::Siege)
            .map(|j| (j, dist(unit, &units[j])))
            .filter(|&(_, d)| d < CAVALRY_REACH)
            .collect();
        horse.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
        // B6: horsemen behind a hedge are ridden round, or left alone.
        for (j, d) in horse {
            if waits_for_foot(view, roles, i, j) {
                continue;
            }
            if charge_or_detour(view, i, j, d < CHARGE_DISTANCE * 3.0) {
                return;
            }
        }
    }
    // 2. Isolated shooters (not behind stakes facing us).
    let mut isolated: Vec<(usize, f64)> = view
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
        .collect();
    isolated.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    if !general_only {
        // B6: shooters behind a hedge are ridden round, or left alone.
        for (j, _) in isolated {
            if charge_or_detour(view, i, j, true) {
                return;
            }
        }
    }
    // 3. Exposed flank: an enemy regiment locked in melee with our troops.
    let exposed = view
        .able_enemies()
        .filter(|&j| {
            let e = &units[j];
            e.state == UnitState::Melee
                && e.formation != Formation::Square
                && !e.has(Ability::PikeSquare)
                && !stakes_in_path(units, (unit.x, unit.z), e)
                && !charge_breaks(view, i, j)
        })
        .map(|j| (j, dist(unit, &units[j])))
        .filter(|&(_, d)| d < CAVALRY_REACH)
        .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    if let (Some((j, d)), false) = (exposed, general_only) {
        let e = &units[j];
        if attack_angle(e, unit.x, unit.z) > 0 || d < 25.0 {
            view.attack(i, j, true);
            return;
        }
        {
            // Ride round to its flank first.
            let (rx, rz) = e.right();
            let side = if (unit.x - e.x) * rx + (unit.z - e.z) * rz >= 0.0 {
                1.0
            } else {
                -1.0
            };
            let (w, _) = e.extent();
            let (fx, fz) = e.forward();
            let flank = |side: f64| {
                (
                    e.x + rx * side * (w * 0.5 + 35.0) - fx * 15.0,
                    e.z + rz * side * (w * 0.5 + 35.0) - fz * 15.0,
                )
            };
            // R2b: not by a flank that passes in front of stakes.
            let clear = |p: (f64, f64)| {
                !stakes_on_ride(units, e.side, (unit.x, unit.z), p, None)
                    && !stakes_on_ride(units, e.side, p, (e.x, e.z), Some(e.id))
            };
            let way = [flank(side), flank(-side)].into_iter().find(|&p| clear(p));
            if let Some((px, pz)) = way {
                view.move_to(i, px, pz, true, None);
                return;
            }
            // R2b: both flanks pass in front of stakes: next choice.
        }
    }
    // 4. Pursuit of routing regiments (B8: leashed — a rout that has
    // already fled too far from the battle line is left to run; chasing it
    // down would strand the horse and delay the main fight).
    let routing = view
        .enemies
        .iter()
        .copied()
        .filter(|&j| units[j].state == UnitState::Routing && units[j].present())
        .filter(|&j| !stakes_in_path(units, (unit.x, unit.z), &units[j]))
        .map(|j| (j, dist(unit, &units[j])))
        .filter(|&(_, d)| d < 350.0)
        .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    if let Some((j, _)) = routing {
        let e = &units[j];
        if (e.x - anchor.0).hypot(e.z - anchor.1) < PURSUIT_LEASH {
            if unit.target != Some(e.id) {
                view.commands.push(Command::Attack {
                    units: vec![unit.id],
                    target: e.id,
                    run: true,
                    queue: false,
                });
            }
            return;
        }
    }
    // 5. A shaken or bled regiment right in front (not bristling): ride it down.
    let shaken = |e: &Unit| {
        !bristling(e)
            && !stakes_in_path(units, (unit.x, unit.z), e)
            && (e.morale < 40.0 || e.hp < f64::from(e.initial_soldiers) * 0.5)
    };
    if let Some((j, d)) = view.nearest_enemy(i, shaken) {
        if d < 110.0
            && !defensive
            && !general_only
            && !charge_breaks(view, i, j)
            && !waits_for_foot(view, roles, i, j)
        {
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

/// EQ7 (suite of ADR 0052): the horse of an attacker with foot does not
/// ride at enemy horse or at a shaken regiment covered by enemy shooters who
/// still have arrows before its foot is close to that target, or already in
/// a melee (Crécy, Poitiers: the knights sent ahead alone broke under the
/// arrows). It holds the wing meanwhile, and goes on once committed close
/// to its target. The defender's horse, which waits for the enemy, and the
/// charge at isolated shooters (which pins them) are unchanged.
fn waits_for_foot(view: &View, roles: &Roles, i: usize, j: usize) -> bool {
    if view.side != SideId::Attacker {
        return false;
    }
    let rules = HorseWaitRules::bundled();
    let units = view.units;
    let (unit, target) = (&units[i], &units[j]);
    if dist(unit, target) < rules.committed_m {
        return false;
    }
    let covered = view.able_enemies().any(|s| {
        let shooter = &units[s];
        is_shooter(shooter)
            && shooter.state != UnitState::Melee
            && dist(shooter, target)
                <= view.sim.effective_range(shooter, target.x, target.z) * rules.range_margin
    });
    let foot: Vec<&Unit> = roles
        .line
        .iter()
        .map(|&k| &units[k])
        .filter(|u| u.able())
        .collect();
    covered
        && !foot.is_empty()
        && !foot
            .iter()
            .any(|u| u.state == UnitState::Melee || dist(u, target) < rules.foot_close_m)
}

/// Reactions common to every plan: face flank attacks, pull out wavering
/// regiments.
fn react(view: &mut View, roles: &Roles) {
    let units = view.units;
    let field = view.sim.field();
    let own = view.own.clone();
    for i in own {
        let u = &units[i];
        if u.state != UnitState::Melee || u.climbing.is_some() {
            continue;
        }
        // R2b: horsemen riding down a rout into the front of enemy stakes
        // break off.
        let rout_into_stakes = is_horse(u)
            && u.target
                .and_then(|t| units.get(t as usize))
                .is_some_and(|t| {
                    t.state == UnitState::Routing && stakes_in_path(units, (u.x, u.z), t)
                });
        if rout_into_stakes {
            let (x, z) = field.clamp_inside(u.x, u.z - view.forward * 60.0, FIELD_MARGIN);
            view.commands.push(Command::Move {
                units: vec![u.id],
                x,
                z,
                run: true,
                facing: None,
                queue: false,
                width: None,
                match_speed: false,
                group_tag: None,
            });
            continue;
        }
        // Shaken and bled: pull out before it breaks (if a reserve exists).
        if u.morale < 28.0
            && u.hp < f64::from(u.initial_soldiers) * 0.5
            && roles.reserve.is_some_and(|r| r != i)
            && !u.is_general
        {
            let (x, z) = field.clamp_inside(u.x, u.z - view.forward * 70.0, FIELD_MARGIN);
            view.commands.push(Command::Move {
                units: vec![u.id],
                x,
                z,
                run: true,
                facing: None,
                queue: false,
                width: None,
                match_speed: false,
                group_tag: None,
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
                        queue: false,
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

/// SG4: the attacker's progress in the assault: the last time a ram struck,
/// ladders went up, a regiment gained the wall walk, a tower docked, or the
/// works gave way (0 before any).
fn last_progress(sim: &BattleSim) -> f64 {
    use crate::siege_fx::SiegeFxKind as K;
    sim.siege_fx()
        .iter()
        .rev()
        .find(|fx| {
            matches!(
                fx.kind,
                K::RamStrike { .. }
                    | K::LaddersRaised { .. }
                    | K::OnWall { .. }
                    | K::TowerDocked { .. }
                    | K::GateBroken { .. }
                    | K::WallBreached { .. }
            )
        })
        .map_or(0.0, |fx| fx.time)
}

/// SG4: is the assault clearly lost? No way in (no opening, nobody of ours
/// on the walls, climbing or inside, no tower rolling) and either nothing
/// has moved for [`ASSAULT_STALL`] seconds since the escalade could start,
/// or our strength fell below [`ASSAULT_HOPELESS`] of the garrison's.
fn assault_lost(view: &View, works: &SiegeWorks, towers_rolling: bool) -> bool {
    let units = view.units;
    let elapsed = view.sim.elapsed();
    if elapsed <= ENGINE_PATIENCE || !works.openings().is_empty() || towers_rolling {
        return false;
    }
    let in_the_town = view.own.iter().any(|&i| {
        let u = &units[i];
        u.on_wall || u.climbing.is_some() || works.inside(u.x, u.z)
    });
    if in_the_town {
        return false;
    }
    let stalled = elapsed > ENGINE_PATIENCE + ASSAULT_STALL
        && elapsed - last_progress(view.sim) > ASSAULT_STALL;
    stalled || view.power(true) < view.power(false) * ASSAULT_HOPELESS
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
    let gate_intact = works.pieces[works.gate].intact();
    // SG4: rams of the side, manned or abandoned (their crew all dead).
    let rams: Vec<usize> = (0..units.len())
        .filter(|&i| {
            let u = &units[i];
            u.ram && u.side == view.side && !u.left_field && !u.withdrawing && !u.reserve
        })
        .collect();
    // Nobody left who can get in, or (SG4) the assault is clearly lost:
    // sound the retreat.
    let climbers_left = own
        .iter()
        .any(|&i| units[i].can_climb() || (is_shooter(&units[i]) && !units[i].mounted));
    let ram_left = !rams.is_empty() && gate_intact;
    let engines_left = engines.iter().any(|&i| units[i].ammo > 0);
    let hopeless = !storm && !climbers_left && !ram_left && !engines_left;
    if hopeless || assault_lost(view, works, towers_rolling) {
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

    // Engines: concentrate on the weakest front wall (SG4: the nearest of
    // equally battered ones), then shoot the wall walk.
    let engine_center = view.centroid(&engines);
    let target_piece = front
        .iter()
        .copied()
        .filter(|&p| works.pieces[p].intact())
        .min_by(|&a, &b| {
            let score = |p: usize| {
                let d = engine_center.map_or(0.0, |(x, z)| works.pieces[p].distance(x, z));
                works.pieces[p].hp + d * ENGINE_TARGET_HP_PER_M
            };
            score(a).total_cmp(&score(b)).then(a.cmp(&b))
        });
    for &i in &engines {
        let unit = &units[i];
        let range = f64::from(unit.stats.range) * view.sim.range_factor();
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
    for &i in rams.iter().filter(|&&i| units[i].able()) {
        if gate_intact {
            let (x, z) = outer_point(works, works.gate, band + 1.0);
            view.move_to(i, x, z, false, None);
        } else {
            let (x, z) = outer_point(works, works.gate, 60.0);
            view.move_to(i, x + 25.0, z, false, None);
        }
    }
    // SG4: a ram short of men (or abandoned): the nearest free foot
    // regiment comes to take it over (the core passes its men to the ram,
    // `siege_works.json` `ram.relief_*`), standing behind it, clear of the
    // boiling oil.
    let mut relief: Vec<usize> = Vec::new();
    if gate_intact {
        for &r in &rams {
            let ram = &units[r];
            if ram.hp >= f64::from(ram.initial_soldiers) * RAM_RELIEF_CREW {
                continue;
            }
            let (nx, nz) = works.pieces[works.gate].outward();
            let spot = (ram.x + nx * RAM_RELIEF_STAND, ram.z + nz * RAM_RELIEF_STAND);
            let donor = own
                .iter()
                .copied()
                .filter(|&i| {
                    let u = &units[i];
                    u.can_climb()
                        && view.free(i)
                        && !u.on_wall
                        && !works.inside(u.x, u.z)
                        && !relief.contains(&i)
                })
                .min_by(|&a, &b| {
                    dist_to(&units[a], spot.0, spot.1)
                        .total_cmp(&dist_to(&units[b], spot.0, spot.1))
                        .then(a.cmp(&b))
                });
            if let Some(i) = donor {
                relief.push(i);
                view.move_to(i, spot.0, spot.1, true, None);
            }
        }
    }
    // Towers: one per front wall.
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
        let (x, z) = outer_point(works, p, band + 2.0);
        view.move_to(i, x, z, false, None);
    }
    // SG4: ladders on several stretches of the front at once (one regiment
    // per stretch, nearest first), the stretches the garrison holds in
    // strength (more than twice the average) last: dilute the defence.
    let held = |p: usize| -> f64 {
        view.able_enemies()
            .filter(|&j| {
                units[j].on_wall && works.pieces[p].distance(units[j].x, units[j].z) < 30.0
            })
            .map(|j| units[j].hp)
            .sum()
    };
    let held_by: Vec<f64> = intact_front.iter().map(|&p| held(p)).collect();
    let average = held_by.iter().sum::<f64>() / held_by.len().max(1) as f64;
    let strong = |k: usize| average > 0.0 && held_by[k] > 2.0 * average;
    let ladder_pieces: Vec<usize> = (0..intact_front.len())
        .filter(|&k| !strong(k))
        .chain((0..intact_front.len()).filter(|&k| strong(k)))
        .map(|k| intact_front[k])
        .collect();

    // Foot and horse.
    let waiting_z = front_z(works) - 250.0;
    let square = works.center;
    let mut ladder_slot = 0usize;
    let mut tower_slot = 0usize;
    for &i in &own {
        let unit = &units[i];
        if unit.category == UnitCategory::Siege || !view.free(i) || relief.contains(&i) {
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
        // SG4 (BR3b too): over the wall or inside the town: take the square
        // (never wait at the ladders); enemies on the way are fought as they
        // come close.
        if unit.on_wall || works.inside(unit.x, unit.z) {
            view.move_to(i, square.0, square.1, true, None);
            continue;
        }
        if storm {
            // SG4: through the breach or the gate as soon as it opens.
            view.move_to(i, square.0, square.1, true, None);
        } else if (escalade || storm) && unit.can_climb() {
            // Climb at a docked tower if any (one regiment per tower at a
            // time), else ladders on the least-held stretches of the front.
            let piece = if let Some(&p) = docked.get(tower_slot) {
                tower_slot += 1;
                p
            } else if !ladder_pieces.is_empty() {
                let p = ladder_pieces[ladder_slot % ladder_pieces.len()];
                ladder_slot += 1;
                p
            } else {
                works.gate
            };
            let (x, z) = if let Some(t) = works.pieces[piece].docked_tower {
                let t = &units[t as usize];
                let (nx, nz) = works.pieces[piece].outward();
                (t.x - nx * 25.0, t.z - nz * 25.0)
            } else {
                outer_point(works, piece, -25.0)
            };
            view.move_to(i, x, z, true, None);
        } else {
            // Wait out of bowshot for the engines, towers and ram.
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

/// SG1: the `k`-th rallying point of the garrison on the central square.
fn square_point(works: &SiegeWorks, k: usize) -> (f64, f64) {
    let angle = k as f64 * 2.4;
    let r = works.square_radius * 0.45;
    (
        works.center.0 + r * angle.sin(),
        works.center.1 + r * angle.cos(),
    )
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
    let gate_down = !works.pieces[works.gate].intact();
    let mut square_slot = 0usize;
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
            } else if gate_down {
                // SG1: the gate is down, nobody climbs here: come down and
                // regroup on the square.
                let k = square_slot;
                square_slot += 1;
                let (x, z) = square_point(works, k);
                view.move_to(i, x, z, true, None);
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
        // Block the openings from inside, one regiment per opening first;
        // SG1: beyond `BLOCKERS_PER_OPENING` per opening, the foot regroups
        // on the central square (and holds it) instead of crowding the gap.
        if !openings.is_empty()
            && blockers >= openings.len() * crate::siege_fx::BLOCKERS_PER_OPENING
            && !is_horse(unit)
        {
            if !works.in_square(unit.x, unit.z) {
                let k = square_slot;
                square_slot += 1;
                let (x, z) = square_point(works, k);
                view.move_to(i, x, z, true, None);
            }
            continue;
        }
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
