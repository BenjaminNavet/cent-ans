//! Morale, fatigue, routs and end-of-battle checks.

use super::*;

impl BattleSim {
    pub(super) fn resolve_morale_and_fatigue(&mut self, contacts: &[Vec<usize>]) {
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
        let Pace {
            loss_morale_factor,
            flank_morale_per_s,
            rear_morale_per_s,
            rout_morale,
            rally_morale,
            melee_fatigue_per_s,
            contagion_factor,
            melee_resolve_per_s,
            ..
        } = *self.pace();
        // T4 (ADR 0108): the garrison's last stand on the square.
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
                * loss_morale_factor
                * cover
                * stand_loss
                * unit.morale_loss_factor();
            if unit.flanked & 1 != 0 {
                morale -= flank_morale_per_s * DT;
            }
            if unit.flanked & 2 != 0 {
                morale -= rear_morale_per_s * DT;
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
            morale -=
                contagion.morale_rate(routing_weight) * DT * stand_contagion * contagion_factor;
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
            if unit.state == UnitState::Melee
                && unit.flanked == 0
                && unit.hp >= f64::from(unit.max_soldiers) * 0.5
                && morale < unit.morale_cap
            {
                morale = (morale + melee_resolve_per_s * DT).min(unit.morale_cap);
            }
            unit.morale = morale.clamp(0.0, 100.0);

            // Fatigue.
            let mut rate: f64 = match unit.state {
                UnitState::Idle | UnitState::Rallied => -0.8,
                UnitState::Shooting => 0.02,
                UnitState::Marching if unit.running || unit.withdrawing => 0.25,
                UnitState::Marching => 0.05,
                UnitState::Charging => 0.4,
                UnitState::Melee | UnitState::Climbing => melee_fatigue_per_s,
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
            if unit.state != UnitState::Routing && unit.morale < rout_morale && !unit.withdrawing {
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
                && unit.morale > rally_morale
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

    pub(super) fn check_end(&mut self) {
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
        // T4 (ADR 0108): the market square held long enough.
        let square_held = self.square_taken();
        let (winner, end) = if able[0] > 0 && able[1] > 0 && !timeout && !square_held {
            // EP9: a broken army, a refused battle or a lull.
            match self.field_decision() {
                Some(decision) => decision,
                None => return,
            }
        } else {
            let winner = if able[0] > 0 && (able[1] == 0 || square_held) {
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
}
