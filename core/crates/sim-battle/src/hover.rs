//! Contextual cursor and face-to-face comparison (lot CB-M2, spec
//! `docs/superpowers/specs/2026-09-27-controles-bataille-tw-design.md`,
//! « Curseur contextuel » and « Comparaison au survol »).
//!
//! [`BattleSim::hover_context`] tells the interface what a right click at
//! (x, z) would do with the current selection: move, close in, shoot (or
//! not: out of range, no line of sight), batter or scale the walls, nothing
//! (forbidden ground, a friend). Range, line of sight and what may attack a
//! wall are the rules of the simulation, evaluated here and never in
//! GDScript. Read-only and cheap (no path search): called as the mouse
//! moves. Numbers from `data/rules/battle_hover.json` ([`HoverRules`]).

use serde::{Deserialize, Serialize};

use crate::hydro::Water;
use crate::setup::SideId;
use crate::siege::PieceKind;
use crate::sim::{horse_against_foot, pikes_against_horse, BattleSim};
use crate::unit::{Unit, UnitState};

/// What a right click under the cursor would do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HoverKind {
    /// Walk there.
    Move,
    /// Close with the enemy (crossed swords).
    Melee,
    /// Shoot at the enemy.
    Ranged,
    /// A shooter's target out of range or out of sight.
    RangedBlocked,
    /// Batter, ram or scale the wall, tower or gate.
    Siege,
    /// Impassable ground, off the field or outside the deployment zone.
    Forbidden,
    /// A friend, or nothing selected.
    None,
}

impl HoverKind {
    /// Key of the context for the interface (`move`, `ranged_blocked`...).
    pub fn key(self) -> &'static str {
        match self {
            HoverKind::Move => "move",
            HoverKind::Melee => "melee",
            HoverKind::Ranged => "ranged",
            HoverKind::RangedBlocked => "ranged_blocked",
            HoverKind::Siege => "siege",
            HoverKind::Forbidden => "forbidden",
            HoverKind::None => "none",
        }
    }
}

/// One regiment's figures in the comparison (same format on both sides).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompareSide {
    pub unit: u32,
    /// Living soldiers.
    pub soldiers: u32,
    pub melee: f64,
    /// Armour plus the general's defence bonus.
    pub defense: f64,
    pub charge: f64,
    pub ranged: f64,
    /// Effective range against the other regiment (0 without missiles).
    pub range: f64,
    pub morale: f64,
    pub fatigue: f64,
    /// Multiplier of the blows struck at the other regiment by the matchup
    /// of the types (pikes against horse...), in percent (100 = none).
    pub bonus_vs: f64,
}

/// A line of the comparison where one side has a net advantage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Advantage {
    Even,
    /// The selected regiment.
    Ours,
    /// The enemy under the cursor.
    Theirs,
}

/// Face-to-face figures of the selected regiment and the hovered enemy,
/// with the net advantage of each line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Compare {
    pub ours: CompareSide,
    pub theirs: CompareSide,
    /// `(line, advantage)` for each line, in display order (`soldiers`,
    /// `melee`, `defense`, `charge`, `ranged`, `range`, `morale`, `fatigue`,
    /// `bonus_vs`); fatigue is an advantage when lower.
    pub advantages: Vec<(String, Advantage)>,
}

/// What the cursor is over and what a right click would do.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HoverContext {
    pub context: HoverKind,
    /// The regiment under the cursor (friend or enemy).
    pub target: Option<u32>,
    /// The wall piece (wall, tower or gate) under the cursor.
    pub piece: Option<usize>,
    /// With a single regiment selected and an enemy under the cursor.
    pub compare: Option<Compare>,
}

/// `data/rules/battle_hover.json`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HoverRules {
    #[serde(default)]
    pub description: String,
    pub pick: PickRules,
    pub compare: CompareRules,
    pub preview: PreviewRules,
    pub range_arc: RangeArcRules,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PickRules {
    /// Margin around a regiment's rectangle (metres).
    pub unit_margin_m: f64,
    /// Margin beyond half the wall's thickness (metres).
    pub piece_margin_m: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompareRules {
    /// Share of the larger value the gap must exceed.
    pub net_advantage_ratio: f64,
    /// Absolute gap the line must exceed.
    pub net_advantage_min: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewRules {
    /// Recompute the path when the aimed point moved farther (metres).
    pub recompute_distance_m: f64,
    /// At most this many recomputes per second.
    pub max_recomputes_per_s: f64,
    /// Beyond this many selected regiments, one path from the centre.
    pub max_individual_paths: u32,
}

/// CB-M4: the shooting range drawn on the ground.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RangeArcRules {
    /// Half-angle of the sector drawn ahead of a shooter (degrees). The
    /// core does not restrict the angle of fire (a shooter turns to its
    /// target); this is the sector covered without turning.
    pub fire_half_angle_deg: f64,
}

impl RangeArcRules {
    /// [`Self::fire_half_angle_deg`] in radians.
    pub fn fire_half_angle(&self) -> f64 {
        self.fire_half_angle_deg.to_radians()
    }
}

data_model::bundled_rules!(HoverRules, "rules/battle_hover.json");

impl BattleSim {
    /// CB-M2: what a right click at (x, z) would do with regiments
    /// `selected` of `side` (the player's side). Regiments of another side,
    /// gone or routing are ignored; the regiment under the cursor is
    /// reported (`target`) even with nothing selected. Reachability of a
    /// far destination is the preview's business ([`BattleSim::preview_path`]:
    /// no path turns the cursor `forbidden`); here only the ground under
    /// the cursor is judged.
    pub fn hover_context(&self, x: f64, z: f64, selected: &[u32], side: SideId) -> HoverContext {
        let rules = HoverRules::bundled();
        let target = self.unit_at(x, z, rules.pick.unit_margin_m);
        let mut hover = HoverContext {
            context: HoverKind::None,
            target: target.map(|u| u.id),
            piece: None,
            compare: None,
        };
        let chosen = || {
            selected
                .iter()
                .filter_map(|&id| self.unit(id))
                .filter(move |u| u.side == side && u.present() && u.state != UnitState::Routing)
        };
        if chosen().next().is_none() || !x.is_finite() || !z.is_finite() {
            return hover;
        }
        if !self.field().inside(x, z) {
            hover.context = HoverKind::Forbidden;
            return hover;
        }
        if self.is_deploying() {
            hover.context = if self.deployable(side, x, z) {
                HoverKind::Move
            } else {
                HoverKind::Forbidden
            };
            return hover;
        }
        if let Some(enemy) = target {
            if enemy.side == side {
                return hover;
            }
            let shooters = || chosen().filter(|u| u.can_shoot() && u.ammo > 0);
            hover.context = if chosen().all(|u| u.can_shoot() && u.ammo > 0) {
                let reach = shooters().any(|u| {
                    let dist = (enemy.x - u.x).hypot(enemy.z - u.z);
                    dist <= self.effective_range(u, enemy.x, enemy.z)
                        && self.visible(u, enemy, dist)
                });
                if reach {
                    HoverKind::Ranged
                } else {
                    HoverKind::RangedBlocked
                }
            } else {
                HoverKind::Melee
            };
            let mut one = chosen();
            if let (Some(ours), None) = (one.next(), one.next()) {
                hover.compare = Some(self.compare(ours, enemy));
            }
            return hover;
        }
        if side == SideId::Attacker {
            if let Some((piece, gate)) = self.piece_at(x, z, rules.pick.piece_margin_m) {
                hover.piece = Some(piece);
                let can_siege = chosen().any(|u| {
                    u.wall_breaker() || u.siege_tower() || u.can_climb() || (gate && u.ram)
                });
                hover.context = if can_siege {
                    HoverKind::Siege
                } else {
                    HoverKind::Melee
                };
                return hover;
            }
        }
        hover.context = if self.impassable_for(x, z, chosen()) {
            HoverKind::Forbidden
        } else {
            HoverKind::Move
        };
        hover
    }

    /// The regiment whose rectangle (front × depth, turned to its facing,
    /// widened by `margin`) holds (x, z); the one whose centre is nearest
    /// when several do.
    fn unit_at(&self, x: f64, z: f64, margin: f64) -> Option<&Unit> {
        self.units()
            .iter()
            .filter(|u| u.present())
            .filter_map(|u| {
                let (w, d) = u.extent();
                let (dx, dz) = (x - u.x, z - u.z);
                let (fx, fz) = u.forward();
                let (rx, rz) = u.right();
                let along = dx * fx + dz * fz;
                let across = dx * rx + dz * rz;
                (across.abs() <= w * 0.5 + margin && along.abs() <= d * 0.5 + margin)
                    .then_some((u, dx * dx + dz * dz))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.id.cmp(&b.0.id)))
            .map(|(u, _)| u)
    }

    /// The intact wall piece (or the piece nearest a tower) under (x, z),
    /// and whether it is the gate.
    fn piece_at(&self, x: f64, z: f64, margin: f64) -> Option<(usize, bool)> {
        let works = self.siege()?;
        let reach = works.band() + margin;
        let on_tower = works
            .towers
            .iter()
            .any(|t| (t.x - x).hypot(t.z - z) <= t.radius + margin);
        let (piece, d) = works
            .pieces
            .iter()
            .enumerate()
            .filter(|(_, p)| p.intact())
            .map(|(k, p)| (k, p.distance(x, z)))
            .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))?;
        (d <= reach || on_tower).then(|| (piece, works.pieces[piece].kind == PieceKind::Gate))
    }

    /// Ground no selected regiment may stand on: a house or a prop of the
    /// town, deep water off a bridge for a selection of horsemen and engines.
    fn impassable_for<'a>(
        &self,
        x: f64,
        z: f64,
        mut chosen: impl Iterator<Item = &'a Unit>,
    ) -> bool {
        if let Some(works) = self.siege() {
            if works.house_at(x, z, 0.0).is_some() || works.prop_at(x, z, 0.0) {
                return true;
            }
        }
        let field = self.field();
        let deep =
            field.water_kind(x, z).is_some_and(Water::deep) && field.bridge_at(x, z).is_none();
        deep && chosen.all(BattleSim::stopped_by_deep_water)
    }

    /// CB-M4: the range drawn around `unit` on the ground — its effective
    /// range against ground at its own feet (weather, time of day, the
    /// height it stands at: a wall walk or a hilltop reaches farther), 0
    /// for a regiment that cannot shoot or has no missiles left. Same
    /// figure as the comparison's `range` line against a target level with
    /// the shooter.
    pub fn ground_range(&self, unit: &Unit) -> f64 {
        if unit.can_shoot() && unit.ammo > 0 {
            self.effective_range(unit, unit.x, unit.z)
        } else {
            0.0
        }
    }

    /// CB-M2: face-to-face figures of `ours` and `theirs`.
    pub fn compare(&self, ours: &Unit, theirs: &Unit) -> Compare {
        let a = self.compare_side(ours, theirs);
        let b = self.compare_side(theirs, ours);
        let rules = &HoverRules::bundled().compare;
        let judge = |mine: f64, other: f64, lower_is_better: bool| {
            let gap = (mine - other).abs();
            let big = mine.abs().max(other.abs());
            if gap <= rules.net_advantage_ratio * big || gap < rules.net_advantage_min {
                Advantage::Even
            } else if (mine > other) != lower_is_better {
                Advantage::Ours
            } else {
                Advantage::Theirs
            }
        };
        let lines = [
            (
                "soldiers",
                f64::from(a.soldiers),
                f64::from(b.soldiers),
                false,
            ),
            ("melee", a.melee, b.melee, false),
            ("defense", a.defense, b.defense, false),
            ("charge", a.charge, b.charge, false),
            ("ranged", a.ranged, b.ranged, false),
            ("range", a.range, b.range, false),
            ("morale", a.morale, b.morale, false),
            ("fatigue", a.fatigue, b.fatigue, true),
            ("bonus_vs", a.bonus_vs, b.bonus_vs, false),
        ];
        Compare {
            advantages: lines
                .iter()
                .map(|&(name, m, o, low)| (name.to_owned(), judge(m, o, low)))
                .collect(),
            ours: a,
            theirs: b,
        }
    }

    fn compare_side(&self, unit: &Unit, other: &Unit) -> CompareSide {
        let shoots = unit.can_shoot() && unit.ammo > 0;
        CompareSide {
            unit: unit.id,
            soldiers: unit.soldiers(),
            melee: f64::from(unit.stats.melee),
            defense: self.defense_points(unit),
            charge: unit.charge_points(),
            ranged: if shoots {
                f64::from(unit.stats.ranged)
            } else {
                0.0
            },
            range: if shoots {
                self.effective_range(unit, other.x, other.z)
            } else {
                0.0
            },
            morale: unit.morale,
            fatigue: unit.fatigue,
            bonus_vs: 100.0 * horse_against_foot(unit, other) * pikes_against_horse(unit, other),
        }
    }
}
