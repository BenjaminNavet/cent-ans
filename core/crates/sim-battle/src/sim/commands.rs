//! Player commands and group destinations.

use super::*;

impl BattleSim {
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
        if let Command::Concede { side: named } = command {
            let giver = match (side, named) {
                (Some(owner), Some(named)) if owner != named => {
                    return Err(CommandError::WrongSide)
                }
                (Some(owner), _) => owner,
                (None, Some(named)) => named,
                (None, None) => return Err(CommandError::NoSide),
            };
            self.concede(giver);
            return Ok(());
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
                    if !kind.def().allows(unit) {
                        return Err(CommandError::InvalidFormation {
                            unit: id,
                            formation: kind,
                        });
                    }
                }
                for &id in &units {
                    // RJ-a (ADR 0174): the men walk to their new places;
                    // CB1: a formation order drops a dragged width.
                    self.units[id as usize].change_formation(kind, self.deploying);
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
            Command::LeaderOrder { .. } | Command::Concede { .. } => {
                unreachable!("handled above")
            }
        }
        Ok(())
    }

    /// `Command::Concede`: every regiment of `side` still in order leaves
    /// the field (survivors saved, as with a general retreat) and the battle
    /// ends now, lost by `side`, even during deployment.
    pub(super) fn concede(&mut self, side: SideId) {
        self.deploying = false;
        let units: Vec<u32> = self
            .units
            .iter()
            .filter(|u| {
                u.side == side && u.present() && u.state != UnitState::Routing && !u.withdrawing
            })
            .map(|u| u.id)
            .collect();
        if !units.is_empty() {
            // Cannot fail: the battle runs and every unit is present, in order, of `side`.
            let _ = self.apply_command(Command::Withdraw { units }, Some(side));
        }
        self.log(
            "Retraite générale : l'armée quitte le champ.".to_owned(),
            Some(side),
        );
        let end_conditions = self.end_conditions;
        self.end_conditions = true;
        self.check_end();
        self.end_conditions = end_conditions;
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
}
