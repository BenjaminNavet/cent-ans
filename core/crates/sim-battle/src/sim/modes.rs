//! CB2: unit modes (see `crate::modes`). The `set_mode` command, which
//! regiment may take which mode, the skirmishers' step back, the guard's
//! hold, the breach of the engines and the display states of a regiment.

use data_model::{Ability, UnitCategory};

use super::BattleSim;
use crate::command::CommandError;
use crate::modes::{UnitMode, UnitModeRules, UnitStatus};
use crate::siege::PieceKind;
use crate::unit::{Unit, UnitState};

/// Margin kept from the field's edges by a skirmish step back (metres).
const SKIRMISH_EDGE_MARGIN: f64 = 10.0;

impl Unit {
    /// CB2: shoots its missiles (able to, and not in melee mode).
    pub fn shoots(&self) -> bool {
        self.can_shoot() && !self.melee_mode
    }

    /// CB2: shoots on the move: the `skirmish` ability, with the mode on.
    pub fn shoots_on_move(&self) -> bool {
        self.has(Ability::Skirmish) && self.skirmish && self.shoots()
    }

    /// CB2: whether `mode` is on.
    pub fn mode(&self, mode: UnitMode) -> bool {
        match mode {
            UnitMode::Run => self.mode_run,
            UnitMode::Guard => self.guard,
            UnitMode::Skirmish => self.skirmish,
            UnitMode::Melee => self.melee_mode,
            UnitMode::Breach => self.breach,
        }
    }

    /// CB2: the modes set away from their default (the regiment's own
    /// `skirmish` ability turns skirmish on by default); the replay digest
    /// only reads these, so digests of battles without modes are unchanged.
    pub fn modes_changed(&self) -> bool {
        self.mode_run
            || self.guard
            || self.melee_mode
            || self.breach
            || self.skirmish != self.has(Ability::Skirmish)
    }

    /// CB2: multiplier of the speed of a run under the run mode.
    pub fn run_mode_speed(&self) -> f64 {
        if self.mode_run && self.running {
            UnitModeRules::bundled().run.speed_multiplier
        } else {
            1.0
        }
    }

    /// CB2: multiplier of the fatigue of a run under the run mode.
    pub fn run_mode_fatigue(&self) -> f64 {
        if self.mode_run && self.running && self.state == UnitState::Marching {
            UnitModeRules::bundled().run.fatigue_multiplier
        } else {
            1.0
        }
    }

    /// CB2: multiplier of the damage of an engine's shot at a wall.
    pub fn breach_wall_damage(&self) -> f64 {
        if self.breach {
            UnitModeRules::bundled().breach.wall_damage(&self.unit_type)
        } else {
            1.0
        }
    }

    /// CB2: multiplier of the reload of an engine's shot at a wall.
    pub fn breach_reload(&self) -> f64 {
        if self.breach {
            UnitModeRules::bundled().breach.reload_multiplier
        } else {
            1.0
        }
    }
}

impl BattleSim {
    /// CB2: whether regiment `unit` may take `mode`: run and guard for every
    /// regiment but the ram and the towers, skirmish and melee for
    /// shooters (engines excepted), breach for wall-battering engines in a
    /// siege.
    pub fn mode_available(&self, unit: &Unit, mode: UnitMode) -> bool {
        if unit.synthetic || unit.siege_tower() {
            return false;
        }
        match mode {
            UnitMode::Run | UnitMode::Guard => unit.category != UnitCategory::Siege,
            UnitMode::Skirmish | UnitMode::Melee => {
                unit.can_shoot() && unit.category != UnitCategory::Siege
            }
            UnitMode::Breach => self.siege.is_some() && unit.wall_breaker(),
        }
    }

    /// The modes regiment `unit` may take, in [`UnitMode::ALL`] order.
    pub fn available_modes(&self, unit: &Unit) -> Vec<UnitMode> {
        UnitMode::ALL
            .into_iter()
            .filter(|&mode| self.mode_available(unit, mode))
            .collect()
    }

    /// `Command::SetMode`: all regiments must be able to take the mode.
    /// Guard and skirmish exclude each other, as do skirmish and melee.
    pub(super) fn set_mode(
        &mut self,
        units: &[u32],
        mode: UnitMode,
        enabled: bool,
    ) -> Result<(), CommandError> {
        for &id in units {
            if !self.mode_available(&self.units[id as usize], mode) {
                return Err(CommandError::ModeUnavailable { unit: id, mode });
            }
        }
        for &id in units {
            let unit = &mut self.units[id as usize];
            match mode {
                UnitMode::Run => {
                    unit.mode_run = enabled;
                    // The move under way changes pace at once.
                    if unit.destination.is_some() && !unit.withdrawing {
                        unit.running = enabled;
                    }
                }
                UnitMode::Guard => {
                    unit.guard = enabled;
                    if enabled {
                        unit.skirmish = false;
                    }
                }
                UnitMode::Skirmish => {
                    unit.skirmish = enabled;
                    if enabled {
                        unit.guard = false;
                        unit.melee_mode = false;
                    }
                }
                UnitMode::Melee => {
                    unit.melee_mode = enabled;
                    if enabled {
                        unit.skirmish = false;
                        if unit.state == UnitState::Shooting {
                            unit.state = UnitState::Idle;
                        }
                    }
                }
                UnitMode::Breach => {
                    unit.breach = enabled;
                    if enabled {
                        // No shot at men while battering.
                        unit.target = None;
                    }
                }
            }
        }
        Ok(())
    }

    /// CB2, guard: regiment `i` attacking `t` gives up instead of pursuing
    /// when the target flees, or when the melee with it has broken (it held
    /// the contact and does not follow). The caller ends the attack.
    pub(super) fn guard_releases(&self, i: usize, t: usize, in_contact: bool) -> bool {
        let unit = &self.units[i];
        unit.guard
            && (self.units[t].state == UnitState::Routing
                || (unit.state == UnitState::Melee && !in_contact))
    }

    /// CB2, skirmish: shooters in skirmish mode, standing (or shooting)
    /// with no move under way, step back from the nearest enemy melee troop
    /// closing in within `skirmish.trigger_m`: a run of `retreat_m` straight
    /// away from it, turning to face it on arrival. Nothing when the field's
    /// edge leaves less than `min_retreat_m`.
    pub(super) fn resolve_skirmish(&mut self, contacts: &[Vec<usize>]) {
        let rules = &UnitModeRules::bundled().skirmish;
        for i in 0..self.units.len() {
            let unit = &self.units[i];
            if !unit.skirmish
                || !unit.shoots()
                || !unit.present()
                || !contacts[i].is_empty()
                || unit.withdrawing
                || unit.destination.is_some()
                || !unit.order_queue.is_empty()
                || unit.climbing.is_some()
                || unit.on_wall
                || unit.pavise.is_some()
                || matches!(
                    unit.state,
                    UnitState::Routing | UnitState::Melee | UnitState::Rallied
                )
            {
                continue;
            }
            let Some((j, dist)) = self.skirmish_threat(i, rules.trigger_m, rules.approach_cos)
            else {
                continue;
            };
            let threat = &self.units[j];
            let away = if dist > 1e-6 {
                ((unit.x - threat.x) / dist, (unit.z - threat.z) / dist)
            } else {
                (-unit.forward().0, -unit.forward().1)
            };
            let (x, z) = self.field.clamp_inside(
                unit.x + away.0 * rules.retreat_m,
                unit.z + away.1 * rules.retreat_m,
                SKIRMISH_EDGE_MARGIN,
            );
            if (x - unit.x).hypot(z - unit.z) < rules.min_retreat_m {
                continue;
            }
            let facing = super::angle_to(-away.0, -away.1);
            let unit = &mut self.units[i];
            unit.target = None;
            unit.destination = Some((x, z));
            unit.destination_facing = Some(facing);
            unit.running = true;
            unit.state = UnitState::Marching;
        }
    }

    /// The nearest enemy melee troop (not shooting its missiles) closing in
    /// on regiment `i` within `reach`: marching or charging at it, or
    /// towards it (front within `approach_cos`).
    fn skirmish_threat(&self, i: usize, reach: f64, approach_cos: f64) -> Option<(usize, f64)> {
        let unit = &self.units[i];
        self.units
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                e.side != unit.side
                    && e.able()
                    && !e.synthetic
                    && e.category != UnitCategory::Siege
                    && !(e.shoots() && e.ammo > 0)
                    && matches!(e.state, UnitState::Marching | UnitState::Charging)
            })
            .filter_map(|(j, e)| {
                let (dx, dz) = (unit.x - e.x, unit.z - e.z);
                let dist = dx.hypot(dz);
                if dist > reach {
                    return None;
                }
                let (fx, fz) = e.forward();
                let heading = if dist > 1e-6 {
                    (fx * dx + fz * dz) / dist
                } else {
                    1.0
                };
                (e.target == Some(unit.id) || heading > approach_cos).then_some((j, dist))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
    }

    /// CB2, breach: an engine battering walls picks the nearest intact wall
    /// or gate within range when none is ordered (and never shoots men).
    pub(super) fn breach_piece(&self, i: usize, range: f64) -> Option<usize> {
        let works = self.siege.as_ref()?;
        let unit = &self.units[i];
        (0..works.pieces.len())
            .filter(|&p| {
                let piece = &works.pieces[p];
                matches!(piece.kind, PieceKind::Wall | PieceKind::Gate)
                    && piece.intact()
                    && piece.distance(unit.x, unit.z) <= range
                    && piece.outside_offset(unit.x, unit.z) > 0.0
            })
            .min_by(|&a, &b| {
                let da = works.pieces[a].distance(unit.x, unit.z);
                let db = works.pieces[b].distance(unit.x, unit.z);
                da.total_cmp(&db).then(a.cmp(&b))
            })
    }

    /// CB2: display states of regiment `unit` (badges), thresholds from
    /// `data/rules/unit_modes.json`.
    pub fn unit_status(&self, unit: &Unit) -> UnitStatus {
        let rules = &UnitModeRules::bundled().status;
        let routing = unit.state == UnitState::Routing;
        UnitStatus {
            charging: unit.state == UnitState::Charging,
            under_fire: unit.present() && unit.missile_timer < rules.under_fire_seconds,
            engaged: unit.state == UnitState::Melee,
            wavering: unit.present() && !routing && unit.morale < rules.wavering_morale,
        }
    }
}
