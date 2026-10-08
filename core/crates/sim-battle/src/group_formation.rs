//! Group formations (lot CB6, plan `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md`,
//! historian's review `docs/research/cb6-formations.md`).
//!
//! A preset of `data/rules/group_formations.json` (« Ligne de bataille »,
//! « La herse », « Trois batailles »…) gives each role of an army (foot,
//! foot shooters, horse, engines, the general's regiment) a place relative
//! to the front line: a row ahead of it, on it or behind it, centred, on
//! the wings or on one flank. [`BattleSim::formation_slots`] turns a preset
//! into one place per regiment; it is pure (no order is given): the
//! interface turns the places into individual `Move` orders (CB1 width,
//! `match_speed`, `group_tag`) or `deploy_unit` calls, so replays are not
//! touched.
//!
//! The initial deployment of a field battle goes through « Ligne de
//! bataille » ([`layout`], called by `BattleSim::deploy`) and yields the
//! very placement it had before CB6: rows centred on the field, the
//! shooters 45 m behind the foot (or on the line without foot), the horse
//! on the wings, alternately right and left, the engines behind.

use data_model::UnitCategory;
use serde::{Deserialize, Serialize};

use crate::setup::SideId;
use crate::sim::BattleSim;
use crate::unit::Unit;

/// Role of a regiment in a group formation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Infantry,
    FootRanged,
    Cavalry,
    Siege,
    General,
}

impl Role {
    pub const ALL: [Role; 5] = [
        Role::Infantry,
        Role::FootRanged,
        Role::Cavalry,
        Role::Siege,
        Role::General,
    ];

    fn index(self) -> usize {
        self as usize
    }

    /// Role of `unit` by its category, as the initial deployment reads it
    /// (mounted shooters ride with the horse); never `General`.
    pub fn of(unit: &Unit) -> Role {
        match unit.category {
            UnitCategory::Infantry => Role::Infantry,
            UnitCategory::Ranged if unit.mounted => Role::Cavalry,
            UnitCategory::Ranged => Role::FootRanged,
            UnitCategory::Cavalry => Role::Cavalry,
            UnitCategory::Siege => Role::Siege,
        }
    }
}

/// Posture of a preset (the picker sorts them under these headings).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stance {
    Attack,
    Defense,
    March,
}

impl Stance {
    pub fn key(self) -> &'static str {
        match self {
            Stance::Attack => "attack",
            Stance::Defense => "defense",
            Stance::March => "march",
        }
    }
}

/// Row of a role (descriptive; `depth_m` places it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Row {
    Front,
    Line,
    Behind,
    Reserve,
}

/// Lateral arrangement of a role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lateral {
    /// One row centred on the formation.
    Center,
    /// Two wings beyond the edges of the front.
    Wings,
    /// A single (right) flank beyond the edge of the front.
    Flank,
    /// A centred row stretched to the width of the front.
    Spread,
}

/// Place of a role in a block preset.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Place {
    pub row: Row,
    pub lateral: Lateral,
    /// Metres from the line of the front role, positive towards the enemy.
    pub depth_m: f64,
    #[serde(default)]
    pub gap_m: Option<f64>,
    #[serde(default)]
    pub wing_offset_m: Option<f64>,
    #[serde(default)]
    pub turn_in_deg: f64,
    /// Frontage in files (CB1 `line_files`); the formation is kept without.
    #[serde(default)]
    pub files: Option<u32>,
    #[serde(default)]
    pub echelons: Option<u32>,
    #[serde(default)]
    pub echelon_step_m: f64,
}

/// Places of the five roles.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RolePlaces {
    pub infantry: Place,
    pub foot_ranged: Place,
    pub cavalry: Place,
    pub siege: Place,
    pub general: Place,
}

impl RolePlaces {
    pub fn get(&self, role: Role) -> &Place {
        match role {
            Role::Infantry => &self.infantry,
            Role::FootRanged => &self.foot_ranged,
            Role::Cavalry => &self.cavalry,
            Role::Siege => &self.siege,
            Role::General => &self.general,
        }
    }
}

/// Which horse a column segment takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentFilter {
    #[default]
    All,
    /// Mounted shooters (scouts).
    Light,
    Heavy,
}

/// Which half of a role a column segment takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentPart {
    #[default]
    All,
    FirstHalf,
    SecondHalf,
}

/// One segment of a marching column, head first.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Segment {
    pub role: Role,
    #[serde(default)]
    pub filter: SegmentFilter,
    #[serde(default)]
    pub part: SegmentPart,
}

/// Layout kind of a preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layout {
    Blocks,
    Column,
}

/// Files of each role in a column.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ColumnFiles {
    #[serde(default)]
    pub infantry: Option<u32>,
    #[serde(default)]
    pub foot_ranged: Option<u32>,
    #[serde(default)]
    pub cavalry: Option<u32>,
    #[serde(default)]
    pub siege: Option<u32>,
    #[serde(default)]
    pub general: Option<u32>,
}

impl ColumnFiles {
    fn get(&self, role: Role) -> Option<u32> {
        match role {
            Role::Infantry => self.infantry,
            Role::FootRanged => self.foot_ranged,
            Role::Cavalry => self.cavalry,
            Role::Siege => self.siege,
            Role::General => self.general,
        }
    }
}

/// A preset of `data/rules/group_formations.json`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preset {
    pub id: String,
    pub name_fr: String,
    pub description_fr: String,
    pub stance: Stance,
    pub layout: Layout,
    #[serde(default)]
    pub front_roles: Vec<Role>,
    #[serde(default)]
    pub roles: Option<RolePlaces>,
    #[serde(default)]
    pub column_gap_m: f64,
    #[serde(default)]
    pub column_files: ColumnFiles,
    #[serde(default)]
    pub column: Vec<Segment>,
}

/// `data/rules/group_formations.json`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupFormationRules {
    #[serde(default)]
    pub description: String,
    pub unit_gap_m: f64,
    pub wrap_depth_m: f64,
    pub row_margin_m: f64,
    pub wing_clamp_m: f64,
    pub general_min_group: usize,
    pub presets: Vec<Preset>,
}

data_model::bundled_rules!(GroupFormationRules, "rules/group_formations.json");

/// Id of the preset of the initial deployment.
pub const BATTLE_LINE: &str = "battle_line";

impl GroupFormationRules {

    pub fn preset(&self, id: &str) -> Option<&Preset> {
        self.presets.iter().find(|p| p.id == id)
    }

    /// « Ligne de bataille », the preset of the initial deployment.
    pub fn battle_line(&self) -> &Preset {
        self.preset(BATTLE_LINE)
            .expect("group_formations.json has the battle line")
    }
}

/// The place proposed to one regiment.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct FormationSlot {
    pub id: u32,
    pub x: f64,
    pub z: f64,
    /// Radians (`atan2(dx, dz)` convention of the battle).
    pub facing: f64,
    /// CB1 frontage (metres) to order; `None` keeps the formation.
    pub width: Option<f64>,
}

/// Reference frame of a formation: world = `right · s + forward · t`,
/// with `(s, t)` absolute coordinates along the two axes (the origin at
/// `(s0, t0)`). Working in absolute coordinates keeps the arithmetic of the
/// initial deployment bit for bit.
#[derive(Debug, Clone, Copy)]
pub struct Frame {
    pub right: (f64, f64),
    pub forward: (f64, f64),
    pub s0: f64,
    pub t0: f64,
    pub facing: f64,
}

impl Frame {
    /// Frame of a formation whose front centre is `(x, z)`, facing
    /// `facing` (right-hand axis `(cos f, −sin f)`, as group orders).
    pub fn facing(x: f64, z: f64, facing: f64) -> Frame {
        let right = (facing.cos(), -facing.sin());
        let forward = (facing.sin(), facing.cos());
        Frame::with_axes(x, z, right, forward, facing)
    }

    /// Frame with explicit axes (the initial deployment keeps +x as its
    /// lateral axis on both sides).
    pub fn with_axes(x: f64, z: f64, right: (f64, f64), forward: (f64, f64), facing: f64) -> Frame {
        Frame {
            right,
            forward,
            s0: x * right.0 + z * right.1,
            t0: x * forward.0 + z * forward.1,
            facing,
        }
    }

    fn world(&self, s: f64, t: f64) -> (f64, f64) {
        (
            self.right.0 * s + self.forward.0 * t,
            self.right.1 * s + self.forward.1 * t,
        )
    }

    /// Facing turned `turn` radians towards `-right` (inwards for a right
    /// wing when positive).
    fn turned(&self, turn: f64) -> f64 {
        if turn == 0.0 {
            return self.facing;
        }
        let (c, s) = (turn.cos(), turn.sin());
        let dx = self.forward.0 * c - self.right.0 * s;
        let dz = self.forward.1 * c - self.right.1 * s;
        dx.atan2(dz)
    }
}

/// How the regiments of a pair of wings are dealt out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WingFill {
    /// Alternately right then left (the initial deployment).
    Alternate,
    /// The left part on the left wing, the rest on the right, keeping
    /// their left-to-right order.
    Ordered,
}

/// Options of a [`layout`].
#[derive(Debug, Clone, Copy)]
pub struct LayoutOptions {
    pub field_width: f64,
    /// The general's regiment takes the `general` place (from
    /// `general_min_group` regiments).
    pub separate_general: bool,
    pub wings: WingFill,
}

/// One placed regiment (index into the unit slice).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placed {
    pub index: usize,
    pub x: f64,
    pub z: f64,
    pub facing: f64,
    pub width: Option<f64>,
}

struct Builder<'a> {
    rules: &'a GroupFormationRules,
    units: &'a [Unit],
    frame: Frame,
    opts: LayoutOptions,
    out: Vec<Placed>,
}

impl Builder<'_> {
    fn width_of(&self, i: usize, files: Option<u32>) -> Option<f64> {
        files.map(|f| f64::from(f) * self.units[i].spacing().0)
    }

    fn frontage(&self, i: usize, width: Option<f64>) -> (f64, f64) {
        self.units[i].extent_for_width(width)
    }

    fn put(&mut self, i: usize, s: f64, t: f64, facing: f64, width: Option<f64>, clamp: bool) {
        let (mut x, mut z) = self.frame.world(s, t);
        if clamp {
            let m = self.rules.wing_clamp_m;
            x = x.clamp(m, self.opts.field_width - m);
            if self.frame.forward.0 != 0.0 {
                // A rotated frame: its wings may run off the depth too.
                z = z.max(m);
            }
        }
        self.out.push(Placed {
            index: i,
            x,
            z,
            facing,
            width,
        });
    }

    /// Side-by-side rows centred on `s_center` at `t`, wrapping into extra
    /// rows behind when wider than the field (`min_span`: a spread row).
    /// Returns the width of the first row.
    fn row(
        &mut self,
        list: &[usize],
        t: f64,
        gap: f64,
        files: Option<u32>,
        min_span: Option<f64>,
    ) -> f64 {
        let widths: Vec<Option<f64>> = list.iter().map(|&i| self.width_of(i, files)).collect();
        let fronts: Vec<f64> = list
            .iter()
            .zip(&widths)
            .map(|(&i, &w)| self.frontage(i, w).0)
            .collect();
        let max_width = self.opts.field_width - 2.0 * self.rules.row_margin_m;
        let mut lines: Vec<Vec<usize>> = vec![Vec::new()];
        let mut width = 0.0;
        for (k, &w) in fronts.iter().enumerate() {
            if width + w > max_width && !lines.last().is_some_and(Vec::is_empty) {
                lines.push(Vec::new());
                width = 0.0;
            }
            width += w + gap;
            lines.last_mut().expect("non-empty").push(k);
        }
        let mut first_width = 0.0;
        for (line_index, line) in lines.iter().enumerate() {
            let mut line_gap = gap;
            let mut total: f64 = line.iter().map(|&k| fronts[k] + gap).sum::<f64>() - gap;
            if let (Some(span), true) = (min_span, line_index == 0 && line.len() > 1) {
                if span > total {
                    line_gap = gap + (span - total) / (line.len() - 1) as f64;
                    total = span;
                }
            }
            if line_index == 0 {
                first_width = total.max(0.0);
            }
            let mut s = self.frame.s0 - total * 0.5;
            let t_line = t + -self.rules.wrap_depth_m * line_index as f64;
            for &k in line {
                let w = fronts[k];
                let facing = self.frame.facing;
                self.put(list[k], s + w * 0.5, t_line, facing, widths[k], false);
                s += w + line_gap;
            }
        }
        first_width
    }

    /// Wings beyond a front `front_width` wide (`single`: the right flank
    /// only).
    fn wings(
        &mut self,
        list: &[usize],
        front_width: f64,
        t: f64,
        place: &Place,
        gap: f64,
        single: bool,
    ) {
        let offset = place.wing_offset_m.unwrap_or(20.0);
        let turn = place.turn_in_deg.to_radians();
        let mut left = self.frame.s0 - front_width * 0.5 - offset;
        let mut right = self.frame.s0 + front_width * 0.5 + offset;
        // (unit, on the right wing)
        let assignment: Vec<(usize, bool)> = if single {
            list.iter().map(|&i| (i, true)).collect()
        } else {
            match self.opts.wings {
                WingFill::Alternate => list
                    .iter()
                    .enumerate()
                    .map(|(k, &i)| (i, k % 2 == 0))
                    .collect(),
                WingFill::Ordered => {
                    let n_left = list.len() / 2;
                    // Innermost first on each side: the left wing from its
                    // last (rightmost) regiment outwards.
                    let mut out: Vec<(usize, bool)> =
                        list[..n_left].iter().rev().map(|&i| (i, false)).collect();
                    out.extend(list[n_left..].iter().map(|&i| (i, true)));
                    out
                }
            }
        };
        for (i, on_right) in assignment {
            let width = self.width_of(i, place.files);
            let (w, _) = self.frontage(i, width);
            let s = if on_right {
                right += w * 0.5;
                let s = right;
                right += w * 0.5 + gap;
                s
            } else {
                left -= w * 0.5;
                let s = left;
                left -= w * 0.5 + gap;
                s
            };
            let facing = self.frame.turned(if on_right { turn } else { -turn });
            self.put(i, s, t, facing, width, true);
        }
    }

    /// Lays one role at its place; returns the width of its first row.
    fn role(&mut self, list: &[usize], place: &Place, front_width: f64, as_front: bool) -> f64 {
        if list.is_empty() {
            return 0.0;
        }
        let gap = place.gap_m.unwrap_or(self.rules.unit_gap_m);
        let depth = if as_front { 0.0 } else { place.depth_m };
        let t = self.frame.t0 + depth;
        let lateral = if as_front {
            match place.lateral {
                Lateral::Spread => Lateral::Spread,
                _ => Lateral::Center,
            }
        } else {
            place.lateral
        };
        match lateral {
            Lateral::Wings | Lateral::Flank => {
                self.wings(list, front_width, t, place, gap, lateral == Lateral::Flank);
                0.0
            }
            Lateral::Center | Lateral::Spread => {
                let span = (lateral == Lateral::Spread && !as_front).then_some(front_width);
                let echelons = place.echelons.unwrap_or(1).max(1) as usize;
                if echelons == 1 {
                    return self.row(list, t, gap, place.files, span);
                }
                let mut first = 0.0;
                for (k, chunk) in echelon_chunks(list, echelons).iter().enumerate() {
                    if chunk.is_empty() {
                        continue;
                    }
                    let t_k = t + place.echelon_step_m * k as f64;
                    let w = self.row(chunk, t_k, gap, place.files, span);
                    if k == 0 {
                        first = w;
                    }
                }
                first
            }
        }
    }
}

/// Splits `list` (in order) into `n` successive lines, the remainder going
/// first to the middle line (« la plus forte au centre »).
fn echelon_chunks(list: &[usize], n: usize) -> Vec<Vec<usize>> {
    let base = list.len() / n;
    let mut sizes = vec![base; n];
    let mut priority: Vec<usize> = Vec::with_capacity(n);
    let mid = (n - 1) / 2;
    priority.push(mid);
    for d in 1..n {
        if mid >= d {
            priority.push(mid - d);
        }
        if mid + d < n {
            priority.push(mid + d);
        }
    }
    for &k in priority.iter().take(list.len() % n) {
        sizes[k] += 1;
    }
    let mut out = Vec::with_capacity(n);
    let mut start = 0;
    for size in sizes {
        out.push(list[start..start + size].to_vec());
        start += size;
    }
    out
}

/// Lays `ids` (indices into `units`, in their left-to-right order) out in
/// `preset` within `frame`.
pub fn layout(
    rules: &GroupFormationRules,
    preset: &Preset,
    units: &[Unit],
    ids: &[usize],
    frame: Frame,
    opts: LayoutOptions,
) -> Vec<Placed> {
    let mut by_role: [Vec<usize>; 5] = Default::default();
    let general = (opts.separate_general && ids.len() >= rules.general_min_group)
        .then(|| {
            ids.iter()
                .copied()
                .find(|&i| units[i].is_general && !units[i].on_wall)
        })
        .flatten();
    for &i in ids {
        let role = if Some(i) == general {
            Role::General
        } else {
            Role::of(&units[i])
        };
        by_role[role.index()].push(i);
    }
    let mut builder = Builder {
        rules,
        units,
        frame,
        opts,
        out: Vec::with_capacity(ids.len()),
    };
    match (preset.layout, &preset.roles) {
        (Layout::Blocks, Some(places)) => {
            let front = preset
                .front_roles
                .iter()
                .copied()
                .find(|r| !by_role[r.index()].is_empty());
            let mut front_width = 0.0;
            if let Some(front) = front {
                let list = by_role[front.index()].clone();
                front_width = builder.role(&list, places.get(front), 0.0, true);
            }
            for role in Role::ALL {
                if Some(role) == front {
                    continue;
                }
                let list = by_role[role.index()].clone();
                builder.role(&list, places.get(role), front_width, false);
            }
        }
        _ => column(&mut builder, preset, &by_role),
    }
    builder.out
}

/// A marching column along the facing, its head at the origin.
fn column(builder: &mut Builder, preset: &Preset, by_role: &[Vec<usize>; 5]) {
    let mut taken = vec![false; builder.units.len()];
    let mut order: Vec<(usize, Role)> = Vec::new();
    for segment in &preset.column {
        let list = &by_role[segment.role.index()];
        let filtered: Vec<usize> = list
            .iter()
            .copied()
            .filter(|&i| {
                let light =
                    builder.units[i].mounted && builder.units[i].category == UnitCategory::Ranged;
                match segment.filter {
                    SegmentFilter::All => true,
                    SegmentFilter::Light => light,
                    SegmentFilter::Heavy => !light,
                }
            })
            .collect();
        let half = filtered.len().div_ceil(2);
        let part: &[usize] = match segment.part {
            SegmentPart::All => &filtered,
            SegmentPart::FirstHalf => &filtered[..half],
            SegmentPart::SecondHalf => &filtered[half..],
        };
        for &i in part {
            if !taken[i] {
                taken[i] = true;
                order.push((i, segment.role));
            }
        }
    }
    // Anything the segments left out closes the column.
    for role in Role::ALL {
        for &i in &by_role[role.index()] {
            if !taken[i] {
                taken[i] = true;
                order.push((i, role));
            }
        }
    }
    let mut t = builder.frame.t0;
    for (k, (i, role)) in order.into_iter().enumerate() {
        let width = builder.width_of(i, preset.column_files.get(role));
        let (_, depth) = builder.frontage(i, width);
        if k > 0 {
            t -= preset.column_gap_m;
        }
        t -= depth * 0.5;
        let facing = builder.frame.facing;
        let s0 = builder.frame.s0;
        builder.put(i, s0, t, facing, width, false);
        t -= depth * 0.5;
    }
}

impl BattleSim {
    /// CB6: the places `preset_id` proposes for regiments `ids`, the
    /// centre of their front line at `(x, z)`, facing `facing` (radians).
    /// Pure: nothing moves. The regiments keep their left-to-right order
    /// (projection of their present positions on the front); during the
    /// deployment the places stay inside the side's zone; never in deep
    /// water (fords excepted). Unknown or absent regiments are left out;
    /// an unknown preset gives no place.
    pub fn formation_slots(
        &self,
        preset_id: &str,
        ids: &[u32],
        x: f64,
        z: f64,
        facing: f64,
    ) -> Vec<FormationSlot> {
        let rules = GroupFormationRules::bundled();
        let Some(preset) = rules.preset(preset_id) else {
            return Vec::new();
        };
        if !(x.is_finite() && z.is_finite() && facing.is_finite()) {
            return Vec::new();
        }
        let units = self.units();
        let mut seen = vec![false; units.len()];
        let mut list: Vec<usize> = Vec::new();
        for &id in ids {
            let i = id as usize;
            if i < units.len() && !seen[i] && units[i].present() {
                seen[i] = true;
                list.push(i);
            }
        }
        let frame = Frame::facing(x, z, facing);
        let lateral = |i: usize| units[i].x * frame.right.0 + units[i].z * frame.right.1;
        list.sort_by(|&a, &b| lateral(a).total_cmp(&lateral(b)));
        let field = self.field();
        let placed = layout(
            rules,
            preset,
            units,
            &list,
            frame,
            LayoutOptions {
                field_width: field.width,
                separate_general: true,
                wings: WingFill::Ordered,
            },
        );
        placed
            .into_iter()
            .map(|p| {
                let unit = &units[p.index];
                let (px, pz) = self.settle(unit.side, unit.z, p.x, p.z, (x, z));
                FormationSlot {
                    id: p.index as u32,
                    x: px,
                    z: pz,
                    facing: p.facing,
                    width: p.width,
                }
            })
            .collect()
    }

    /// A proposed place brought onto the field, into the deployment zone
    /// during the deployment, and out of deep water.
    fn settle(&self, side: SideId, from_z: f64, x: f64, z: f64, anchor: (f64, f64)) -> (f64, f64) {
        let field = self.field();
        let margin = 10.0;
        let mut x = x.clamp(margin, field.width - margin);
        let mut z = z.clamp(margin, field.depth - margin);
        if self.is_deploying() {
            let zones = self.deployment_zones(side);
            let zone = zones
                .iter()
                .find(|zone| zone.contains(anchor.0, anchor.1))
                .or(zones.first())
                .copied()
                .unwrap_or_else(|| self.deployment_zone(side));
            (x, z) = zone.clamp(x, z);
            let back = if side == SideId::Attacker { -1.0 } else { 1.0 };
            let edge = if back < 0.0 { zone.z0 } else { zone.z1 };
            z = crate::ai::dry_z(field, x, z, edge, back).clamp(zone.z0, zone.z1);
        } else {
            let forward = if z >= from_z { 1.0 } else { -1.0 };
            z = crate::ai::dry_z(field, x, z, from_z, forward);
        }
        (x, z)
    }
}

/// The presets, for the interface.
pub fn presets() -> &'static [Preset] {
    &GroupFormationRules::bundled().presets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_presets_load() {
        let rules = GroupFormationRules::bundled();
        assert_eq!(rules.presets.len(), 6);
        assert!(rules.battle_line().roles.is_some());
        assert!(rules
            .presets
            .iter()
            .any(|p| p.layout == Layout::Column && p.stance == Stance::March));
    }

    #[test]
    fn echelons_put_the_remainder_in_the_middle() {
        let list: Vec<usize> = (0..7).collect();
        let chunks = echelon_chunks(&list, 3);
        let sizes: Vec<usize> = chunks.iter().map(Vec::len).collect();
        assert_eq!(sizes, vec![2, 3, 2]);
        let chunks = echelon_chunks(&list[..5], 3);
        let sizes: Vec<usize> = chunks.iter().map(Vec::len).collect();
        assert_eq!(sizes, vec![2, 2, 1]);
        assert_eq!(chunks.concat(), (0..5).collect::<Vec<_>>());
    }
}
