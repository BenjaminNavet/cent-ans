//! CB-M3: queued orders (see `crate::queue`). Starting a move or an attack,
//! the room left in a regiment's queue, where the queue leaves it, and the
//! next order taken when the current one ends.

use super::{stop_climbing, BattleSim, MoveShape};
use crate::command::CommandError;
use crate::queue::{QueueRules, QueuedOrder};
use crate::unit::{Unit, UnitState};

impl Unit {
    /// CB-M3: the regiment has an order under way (or waiting): a queued
    /// order goes behind it instead of starting now.
    pub fn busy(&self) -> bool {
        self.destination.is_some() || self.target.is_some() || !self.order_queue.is_empty()
    }
}

impl BattleSim {
    /// Regiment `index` walks to `destination` (already clamped into the
    /// field), facing `facing` on arrival: the effect of a `Move` order.
    pub(super) fn start_move(
        &mut self,
        index: usize,
        destination: (f64, f64),
        facing: Option<f64>,
        run: bool,
        shape: MoveShape,
    ) {
        let unit = &mut self.units[index];
        // CB1: the Line a drag asked for, and the pace of the group.
        unit.set_width(shape.width);
        unit.match_speed = shape.match_speed;
        unit.group_tag = shape.group_tag;
        unit.destination = Some(destination);
        unit.destination_facing = facing;
        unit.target = None;
        // CB2: the run mode runs every move.
        unit.running = run || unit.mode_run;
        unit.withdrawing = false;
        unit.pavise = None;
        stop_climbing(unit);
        unit.disengaging = unit.state == UnitState::Melee;
        if unit.state != UnitState::Melee {
            unit.state = UnitState::Marching;
        }
    }

    /// Regiment `index` closes with (or shoots at) `target`: the effect of
    /// an `Attack` order. Pavises stay up while the target is within
    /// bowshot and in sight (RS-J: a target hidden in a wood, behind walls
    /// or a crest is closed in on, like the other shooters do).
    pub(super) fn start_attack(&mut self, index: usize, target: u32, run: bool) {
        let aim = &self.units[target as usize];
        let (tx, tz) = (aim.x, aim.z);
        let unit = &self.units[index];
        let dist = ((tx - unit.x).powi(2) + (tz - unit.z).powi(2)).sqrt();
        let in_range = unit.shoots()
            && unit.ammo > 0
            && dist <= self.effective_range(unit, tx, tz)
            && self.visible(unit, aim, dist);
        let unit = &mut self.units[index];
        if !in_range {
            unit.pavise = None;
        }
        unit.target = Some(target);
        // CB2: an attack order is the player's call on men: it ends the
        // engines' battering.
        unit.breach = false;
        unit.match_speed = false;
        unit.group_tag = None;
        unit.destination = None;
        unit.destination_facing = None;
        unit.running = run || unit.mode_run;
        unit.withdrawing = false;
        unit.disengaging = false;
        stop_climbing(unit);
    }

    /// Refuses a queued order when one of `units` has no room left.
    pub(super) fn check_queue_room(&self, units: &[u32]) -> Result<(), CommandError> {
        let max = QueueRules::bundled().max_queued_orders;
        for &id in units {
            if self.units[id as usize].order_queue.len() >= max as usize {
                return Err(CommandError::QueueFull { unit: id, max });
            }
        }
        Ok(())
    }

    /// Where regiment `index` will be once its orders are done: the last
    /// queued point (an attack's target where it stands now), else its
    /// current destination or target, else where it stands.
    pub fn queue_anchor(&self, index: usize) -> (f64, f64) {
        let unit = &self.units[index];
        let at = |id: u32| {
            self.units
                .get(id as usize)
                .map_or((unit.x, unit.z), |t| (t.x, t.z))
        };
        match unit.order_queue.back() {
            Some(QueuedOrder::Move { x, z, .. }) => (*x, *z),
            Some(QueuedOrder::Attack { target, .. }) => at(*target),
            None => match (unit.target, unit.destination) {
                (Some(target), _) => at(target),
                (None, Some(destination)) => destination,
                (None, None) => (unit.x, unit.z),
            },
        }
    }

    /// The current order of regiment `index` has ended: start the next
    /// queued one still possible (an attack on a regiment gone or fleeing
    /// is skipped). `true` if an order started.
    pub(super) fn next_queued(&mut self, index: usize) -> bool {
        while let Some(order) = self.units[index].order_queue.pop_front() {
            match order {
                QueuedOrder::Move {
                    x,
                    z,
                    facing,
                    run,
                    width,
                    match_speed,
                    group_tag,
                } => {
                    let shape = MoveShape {
                        width,
                        match_speed,
                        group_tag,
                    };
                    self.start_move(index, (x, z), facing, run, shape);
                    return true;
                }
                QueuedOrder::Attack { target, run } => {
                    let valid = self.units.get(target as usize).is_some_and(|t| {
                        t.present()
                            && t.state != UnitState::Routing
                            && t.side != self.units[index].side
                    });
                    if valid {
                        self.start_attack(index, target, run);
                        return true;
                    }
                }
            }
        }
        false
    }
}
