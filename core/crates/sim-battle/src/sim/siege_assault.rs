//! SG1 — siege assault: renderer events ([`crate::siege_fx`]), the ram's
//! blows, boiling oil from the gate, and the transitions of the works (gate
//! broken, wall breached, siege tower docked) recorded once per change.

use super::{armor_factor, BattleSim, DT};
use crate::setup::SideId;
use crate::siege::PieceKind;
use crate::siege_fx::{
    SiegeFx, SiegeFxKind, OIL_GUARD_RANGE, OIL_KILLS, OIL_MORALE, OIL_PERIOD, OIL_RAM_KILLS,
    OIL_REACH, RAM_PERIOD,
};
use crate::unit::UnitState;

/// Per-battle state of the assault events (derived from the works, plus the
/// ram and oil timers).
#[derive(Debug, Clone, Default)]
pub(crate) struct AssaultState {
    fx: Vec<SiegeFx>,
    fx_read: usize,
    /// Seconds the ram has been swinging since its last blow, per unit index.
    pub(crate) ram_timers: Vec<f64>,
    oil_timer: f64,
    /// Last seen `docked_tower` / `intact` of each piece (`None` before the
    /// first step).
    docked: Vec<Option<u32>>,
    intact: Vec<bool>,
    primed: bool,
    fell_back: bool,
}

impl BattleSim {
    /// SG1: siege events recorded since the last call (for the renderer).
    pub fn take_new_siege_fx(&mut self) -> Vec<SiegeFx> {
        let new = self.assault.fx[self.assault.fx_read..].to_vec();
        self.assault.fx_read = self.assault.fx.len();
        new
    }

    /// Every siege event of the battle so far (tests, replays).
    pub fn siege_fx(&self) -> &[SiegeFx] {
        &self.assault.fx
    }

    pub(super) fn push_fx(&mut self, kind: SiegeFxKind) {
        if self.siege.is_some() {
            self.assault.fx.push(SiegeFx {
                time: self.elapsed,
                kind,
            });
        }
    }

    /// The ram at the gate: `true` (and the damage of the blow) when the
    /// blow falls this tick. Called by `resolve_siege_works` per ram.
    pub(super) fn ram_blow(timers: &mut Vec<f64>, index: usize, at_gate: bool) -> Option<f64> {
        if timers.len() <= index {
            timers.resize(index + 1, 0.0);
        }
        if !at_gate {
            timers[index] = 0.0;
            return None;
        }
        timers[index] += DT;
        if timers[index] + 1e-9 >= RAM_PERIOD {
            timers[index] -= RAM_PERIOD;
            Some(RAM_PERIOD)
        } else {
            None
        }
    }

    /// Boiling oil from the gate's machicolations: while the gate stands and
    /// a defender guards it from inside, a pot every [`OIL_PERIOD`] seconds
    /// scalds the besiegers pressed against it (the ram, the regiments at its
    /// foot). Armour protects half as well as against blows.
    pub(super) fn boiling_oil(&mut self) {
        let Some(works) = &self.siege else {
            return;
        };
        let gate = works.gate;
        let piece = works.pieces[gate].clone();
        if !piece.intact() {
            return;
        }
        let (mx, mz) = piece.midpoint();
        let guarded = self.units.iter().any(|u| {
            u.side == SideId::Defender
                && u.able()
                && !u.synthetic
                && piece.outside_offset(u.x, u.z) <= 0.5
                && ((u.x - mx).powi(2) + (u.z - mz).powi(2)).sqrt() < OIL_GUARD_RANGE
        });
        if !guarded {
            self.assault.oil_timer = 0.0;
            return;
        }
        self.assault.oil_timer = (self.assault.oil_timer + DT).min(OIL_PERIOD);
        if self.assault.oil_timer + 1e-9 < OIL_PERIOD {
            return;
        }
        let reach = works.band() + OIL_REACH;
        let targets: Vec<usize> = (0..self.units.len())
            .filter(|&j| {
                let u = &self.units[j];
                u.side == SideId::Attacker
                    && u.present()
                    && u.state != UnitState::Routing
                    && !u.on_wall
                    && piece.outside_offset(u.x, u.z) > 0.0
                    && piece.distance(u.x, u.z) < reach
            })
            .collect();
        if targets.is_empty() {
            return;
        }
        self.assault.oil_timer = 0.0;
        let mut destroyed = Vec::new();
        for &j in &targets {
            let kills = if self.units[j].ram {
                OIL_RAM_KILLS
            } else {
                OIL_KILLS * armor_factor(self.defense_points(&self.units[j]) * 0.5)
            };
            let u = &mut self.units[j];
            let kills = kills.min(u.hp);
            u.hp -= kills;
            u.tick_losses += kills;
            // The ram's crew works under its roof: scalded, not frightened.
            if !u.ram {
                u.morale -= OIL_MORALE;
            }
            if u.hp <= 0.0 {
                destroyed.push(j);
            }
        }
        let (nx, nz) = piece.outward();
        let at = works.band() * 0.5 + 2.0;
        let ids = targets.iter().map(|&j| self.units[j].id).collect();
        self.push_fx(SiegeFxKind::BoilingOil {
            piece: gate,
            x: mx + nx * at,
            z: mz + nz * at,
            targets: ids,
        });
        self.log(
            "De l'huile bouillante se déverse des mâchicoulis de la porte !".to_owned(),
            Some(SideId::Defender),
        );
        for j in destroyed {
            self.unit_destroyed(j);
        }
    }

    /// Records the changes of the works since the last step: gate broken,
    /// wall breached, siege towers docked or gone; the first time the gate
    /// falls, an AI garrison's reserve falls back to the square.
    pub(super) fn record_siege_transitions(&mut self) {
        let Some(works) = &self.siege else {
            return;
        };
        let now_intact: Vec<bool> = works.pieces.iter().map(|p| p.intact()).collect();
        let now_docked: Vec<Option<u32>> = works.pieces.iter().map(|p| p.docked_tower).collect();
        let kinds: Vec<PieceKind> = works.pieces.iter().map(|p| p.kind).collect();
        if !self.assault.primed {
            self.assault.primed = true;
            self.assault.intact = now_intact;
            self.assault.docked = now_docked;
            return;
        }
        let mut new = Vec::new();
        for p in 0..now_intact.len() {
            if self.assault.intact[p] && !now_intact[p] {
                new.push(match kinds[p] {
                    PieceKind::Gate => SiegeFxKind::GateBroken { piece: p },
                    PieceKind::Wall => SiegeFxKind::WallBreached { piece: p },
                });
            }
            let (before, after) = (self.assault.docked[p], now_docked[p]);
            if before != after {
                if let Some(unit) = before {
                    new.push(SiegeFxKind::TowerUndocked { unit, piece: p });
                }
                if let Some(unit) = after {
                    new.push(SiegeFxKind::TowerDocked { unit, piece: p });
                }
            }
        }
        let gate_fell = new
            .iter()
            .any(|k| matches!(k, SiegeFxKind::GateBroken { .. }));
        self.assault.intact = now_intact;
        self.assault.docked = now_docked;
        for kind in new {
            self.push_fx(kind);
        }
        if gate_fell && !self.assault.fell_back && self.ai_enabled[SideId::Defender.index()] {
            self.assault.fell_back = true;
            self.push_fx(SiegeFxKind::DefendersFallBack);
            self.log(
                "La porte est tombée : la garnison se replie sur la place !".to_owned(),
                Some(SideId::Defender),
            );
        }
    }
}
