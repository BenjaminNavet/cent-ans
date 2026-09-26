//! SG1 — siege assault: renderer events ([`crate::siege_fx`]), the ram's
//! blows, boiling oil from the gate, and the transitions of the works (gate
//! broken, wall breached, siege tower docked) recorded once per change.

use super::{armor_factor, BattleSim, DT};
use crate::setup::SideId;
use crate::siege::PieceKind;
use crate::siege_fx::{
    ladder_count, SiegeFx, SiegeFxKind, BRIDGE_CROSSERS, CLIMBERS_PER_LADDER, CLIMB_WAVES,
    LADDER_LEAN, OIL_GUARD_RANGE, OIL_KILLS, OIL_MORALE, OIL_PERIOD, OIL_RAM_KILLS, OIL_REACH,
    RAM_PERIOD,
};
use crate::unit::{Unit, UnitState};

/// One ladder of a climbing regiment: foot on the ground and top against
/// the crenels, both on the (x, z) plane, and their heights.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ladder {
    pub foot: (f64, f64),
    pub top: (f64, f64),
    pub foot_y: f64,
    pub top_y: f64,
}

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
    /// SG4: men lent to a ram by a foot regiment, `(ram index, regiment
    /// index, men)`: the survivors return to their regiment at the end.
    ram_loans: Vec<(usize, usize, f64)>,
}

impl AssaultState {
    /// PB3e: keeps the renderer's read cursor of `old` (the state a
    /// computed-ahead step replaces).
    pub(super) fn keep_read_cursor(&mut self, old: &AssaultState) {
        self.fx_read = old.fx_read;
    }
}

impl BattleSim {
    /// SG1: the ladders a regiment scaling a wall has raised (empty when it
    /// is not climbing with ladders), evenly spread over its frontage along
    /// the piece.
    pub fn ladders(&self, unit: &Unit) -> Vec<Ladder> {
        let (Some(piece), Some(works)) = (unit.climbing, &self.siege) else {
            return Vec::new();
        };
        let p = &works.pieces[piece];
        if p.docked_tower.is_some() {
            return Vec::new();
        }
        let (width, _) = unit.extent();
        let count = ladder_count(width);
        let (tx, tz) = p.tangent();
        let (nx, nz) = p.outward();
        let len = p.length();
        let (cx, cz) = p.closest_point(unit.x, unit.z);
        // Keep clear of the round towers at both ends of the piece.
        let margin = works
            .towers
            .iter()
            .map(|t| t.radius)
            .fold(0.0, f64::max)
            .max(1.0)
            + 1.5;
        let room = (len - 2.0 * margin).max(2.0);
        let span = width.min(room);
        let lo = (len * 0.5 - room * 0.5) + span * 0.5;
        let hi = (len * 0.5 + room * 0.5) - span * 0.5;
        // Regiments at the same spot do not share the very same ladders.
        let jitter =
            (crate::siege_fx::hash01(u64::from(unit.id), 0x1add) - 0.5) * span / count as f64 * 0.9;
        let along0 = ((cx - p.a.0) * tx + (cz - p.a.1) * tz + jitter).clamp(lo, hi.max(lo));
        let face = works.thickness * 0.5;
        (0..count)
            .map(|k| {
                let along = along0 - span * 0.5 + (k as f64 + 0.5) * span / count as f64;
                let (lx, lz) = (p.a.0 + tx * along, p.a.1 + tz * along);
                let top = (lx + nx * face, lz + nz * face);
                let foot = (
                    lx + nx * (face + LADDER_LEAN),
                    lz + nz * (face + LADDER_LEAN),
                );
                let ground = self.field.height(lx, lz);
                Ladder {
                    foot,
                    top,
                    foot_y: self.field.height(foot.0, foot.1),
                    top_y: ground + works.wall_height + 0.3,
                }
            })
            .collect()
    }

    /// SG1: how many of `unit`'s first soldiers are drawn on its ladders or
    /// on the bridge of a docked tower (the renderer animates them climbing).
    /// `scale` = figures per soldier (BV1, [`Unit::figure_count`]).
    pub fn climbers_shown(&self, unit: &Unit, scale: f64) -> usize {
        let n = unit.figure_count(scale) as usize;
        match (unit.climbing, &self.siege) {
            (Some(p), Some(works)) if works.pieces[p].docked_tower.is_some() => {
                n.min(BRIDGE_CROSSERS)
            }
            (Some(_), Some(_)) => {
                let (width, _) = unit.extent();
                n.min(ladder_count(width) * CLIMBERS_PER_LADDER)
            }
            _ => 0,
        }
    }

    /// `(x, y, z, facing)` of every living soldier of `unit` for the
    /// renderer. SG1: a regiment scaling a wall shows its first soldiers
    /// going up the ladders (or across the tower bridge) man after man, a
    /// growing share of the rest already fighting on the wall walk, the
    /// others pressed at the foot of the wall.
    /// `scale` = figures per soldier (BV1): one pose per figure of
    /// [`Unit::figure_positions`].
    pub fn soldier_poses(&self, unit: &Unit, scale: f64) -> Vec<[f64; 4]> {
        let mut positions = unit.figure_positions(scale);
        // EP11: bulging front, squeezed ranks, wrapping files.
        crate::push::deform_figures(unit, &self.push_rules, &mut positions);
        // BR3: no figure in a house or a prop.
        self.push_figures_out(unit, &mut positions);
        let (Some(piece), Some(works)) = (unit.climbing, &self.siege) else {
            return positions
                .iter()
                .map(|&(x, z, a)| [x, self.standing_height(unit, x, z), z, a])
                .collect();
        };
        let p = &works.pieces[piece];
        let (tx, tz) = p.tangent();
        let (nx, nz) = p.outward();
        let facing = (-nx).atan2(-nz);
        let progress = unit.climb_progress.clamp(0.0, 1.0);
        let shown = self.climbers_shown(unit, scale).min(positions.len());
        let ladders = self.ladders(unit);
        // Where a climber comes out on top: the ladder heads, else the bridge.
        let mut heads: Vec<(f64, f64, f64)> = ladders
            .iter()
            .map(|l| (l.top.0, l.top.1, l.top_y - 0.3))
            .collect();
        let bridge = p.docked_tower.map(|t| {
            let tower = &self.units[t as usize];
            let (cx, cz) = p.closest_point(tower.x, tower.z);
            let y = self.field.height(cx, cz) + works.wall_height;
            ((tower.x, tower.z), (cx, cz), y)
        });
        if let Some((_, (cx, cz), y)) = bridge {
            heads.push((cx, cz, y));
        }
        let on_top = ((positions.len().saturating_sub(shown)) as f64 * progress * 0.6) as usize;
        let inset = works.thickness * 0.25;
        positions
            .iter()
            .enumerate()
            .map(|(i, &(x, z, a))| {
                if i < shown {
                    if let Some(((bx, bz), (cx, cz), y)) = bridge {
                        let h = (progress * CLIMB_WAVES * 0.5 + i as f64 / shown as f64).fract();
                        let (px, pz) = (bx + (cx - bx) * h, bz + (cz - bz) * h);
                        return [px, y, pz, facing];
                    }
                    let l = &ladders[i % ladders.len()];
                    let rung = (i / ladders.len()) as f64 / CLIMBERS_PER_LADDER as f64;
                    let h = (progress * CLIMB_WAVES + rung).fract();
                    let px = l.foot.0 + (l.top.0 - l.foot.0) * h;
                    let pz = l.foot.1 + (l.top.1 - l.foot.1) * h;
                    // Half a metre off the rungs, towards the attacker.
                    let off = 0.45 * (1.0 - h);
                    return [
                        px + nx * off,
                        l.foot_y + (l.top_y - 0.2 - l.foot_y) * h,
                        pz + nz * off,
                        facing,
                    ];
                }
                if i < shown + on_top && !heads.is_empty() {
                    let k = i - shown;
                    let (hx, hz, y) = heads[k % heads.len()];
                    let row = k / heads.len();
                    let side = if row.is_multiple_of(2) { 1.0 } else { -1.0 };
                    let spread = 0.9 * (row / 2 + 1) as f64 * side;
                    let back = inset + 0.6 * (row % 3) as f64;
                    return [
                        hx + tx * spread - nx * back,
                        y,
                        hz + tz * spread - nz * back,
                        facing + side * 1.2,
                    ];
                }
                [x, self.field.height(x, z), z, a]
            })
            .collect()
    }

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

    /// SG4: relief of the ram's crew. While the gate stands, the nearest
    /// foot regiment of the attacker within `ram.relief_range_m` of a ram
    /// short of men (or abandoned, its crew all dead) passes men to it,
    /// `ram.relief_men_per_s` a second, up to the full crew; an abandoned
    /// ram is manned again.
    pub(super) fn relieve_rams(&mut self) {
        let Some(works) = &self.siege else {
            return;
        };
        if !works.pieces[works.gate].intact() {
            return;
        }
        let rules = &crate::siege::SiegeWorkRules::bundled().ram;
        if rules.relief_range_m <= 0.0 || rules.relief_men_per_s <= 0.0 {
            return;
        }
        for r in 0..self.units.len() {
            let ram = &self.units[r];
            if !ram.ram || ram.left_field || ram.reserve || ram.withdrawing {
                continue;
            }
            let full = f64::from(ram.initial_soldiers);
            let missing = full - ram.hp.max(0.0);
            if missing <= 1e-6 {
                continue;
            }
            let donor = (0..self.units.len())
                .filter(|&k| {
                    let u = &self.units[k];
                    u.side == ram.side
                        && k != r
                        && u.able()
                        && u.can_climb()
                        && u.climbing.is_none()
                        && !u.on_wall
                        && u.state != UnitState::Melee
                        && u.hp > 2.0
                })
                .map(|k| {
                    let u = &self.units[k];
                    (k, (u.x - ram.x).hypot(u.z - ram.z))
                })
                .filter(|&(_, d)| d <= rules.relief_range_m)
                .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
            let Some((k, _)) = donor else {
                continue;
            };
            let men = (rules.relief_men_per_s * DT)
                .min(missing)
                .min(self.units[k].hp - 1.0);
            if men <= 0.0 {
                continue;
            }
            let morale = self.units[k].morale;
            self.units[k].hp -= men;
            let abandoned = self.units[r].hp <= 0.0;
            let ram = &mut self.units[r];
            ram.hp = ram.hp.max(0.0) + men;
            if abandoned || ram.state == UnitState::Routing {
                ram.state = UnitState::Idle;
                ram.morale = ram.morale.max(morale);
                ram.destination = None;
                ram.target = None;
                let text = format!(
                    "Les {} reprennent le bélier abandonné !",
                    self.unit_label(k)
                );
                self.log(text, Some(SideId::Attacker));
            }
            match self
                .assault
                .ram_loans
                .iter_mut()
                .find(|l| l.0 == r && l.1 == k)
            {
                Some(loan) => loan.2 += men,
                None => self.assault.ram_loans.push((r, k, men)),
            }
        }
    }

    /// SG4: at the end of the battle the survivors of the men lent to a ram
    /// go back to their regiments (still on the field), in proportion to
    /// what each lent; the dead are losses of their regiment.
    pub(super) fn return_ram_crews(&mut self) {
        let loans = std::mem::take(&mut self.assault.ram_loans);
        for r in 0..self.units.len() {
            let lent: f64 = loans.iter().filter(|l| l.0 == r).map(|l| l.2).sum();
            if lent <= 0.0 {
                continue;
            }
            let back = self.units[r].hp.max(0.0).min(lent);
            if back <= 0.0 {
                continue;
            }
            self.units[r].hp -= back;
            for &(_, k, men) in loans.iter().filter(|l| l.0 == r) {
                if self.units[k].present() {
                    self.units[k].hp += back * men / lent;
                }
            }
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
