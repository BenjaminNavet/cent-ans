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

mod modes;

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
use crate::unit::{Unit, UnitState};

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

mod cover;
pub use self::cover::*;
mod horse;
use self::horse::*;
mod plan_field;
pub use self::plan_field::*;
mod roles;
use self::roles::*;
mod shooter;
pub use self::shooter::*;
mod siege;
use self::siege::*;
mod view;
pub use self::view::*;
