//! PB3c / RJ-b: figure buffers and regiment positions handed to the
//! renderer, blended between the two latest simulation steps
//! (`battle_pose_lerp.rs`, `docs/wip/rj-b.md`).
//!
//! The poses of a step are computed once (PB3c cache); each frame then
//! blends them with those of the step before for the frame's
//! `step_fraction`. A regiment that did not move keeps the same buffer (same
//! version: not sent again to its `MultiMesh`, PB3e).

use godot::prelude::*;
use sim_battle::DT;

use crate::battle_pose_lerp::{self as lerp, LooseRanks, Pose, TELEPORT_M_PER_STEP};
use crate::battle_sim::BattleSim;

/// `(ticks, pose epoch, figure scale bits)` of the poses built.
type PoseKey = (u64, u64, u64);

/// What a new key means for the poses built before it.
#[derive(Debug, PartialEq)]
enum Shift {
    /// Some steps later: the current poses become the previous ones (ticks).
    Advance(u64),
    /// Same step (an order, the deployment…): the previous poses stand.
    Keep,
    /// New battle, replay jump, other figure scale: nothing to blend from.
    Reset,
}

fn shift(old: Option<PoseKey>, new: PoseKey) -> Shift {
    let Some((ticks, _, scale)) = old else {
        return Shift::Reset;
    };
    if scale != new.2 {
        return Shift::Reset;
    }
    if new.0 == ticks {
        Shift::Keep
    } else if new.0 > ticks && new.0 - ticks <= lerp::MAX_SPAN {
        Shift::Advance(ticks)
    } else {
        Shift::Reset
    }
}

/// PB3c: per-regiment `MultiMesh.buffer`s of the latest
/// [`BattleSim::get_soldier_buffers`]: poses of the two latest steps built,
/// buffers rebuilt when the step changes or, for a moving regiment, when the
/// blend factor does (RJ-b); otherwise the same packed arrays are handed out
/// again (copy-on-write: the renderer may alter its copy).
#[derive(Default)]
pub(crate) struct PoseCache {
    /// Key the current poses were built for.
    key: Option<PoseKey>,
    prev_ticks: u64,
    cur_ticks: u64,
    /// Unit id -> poses at `prev_ticks` (`None`: not built then).
    prev: Vec<Option<Vec<Pose>>>,
    /// Unit id -> poses at `cur_ticks` (`None`: not requested yet).
    cur: Vec<Option<Vec<Pose>>>,
    /// Unit id -> `prev` and `cur` differ (the buffer follows the blend).
    moving: Vec<bool>,
    /// Unit id -> blend factor bits of the buffer (`None`: to build).
    built: Vec<Option<u64>>,
    /// Unit id -> the transforms drawn (12 floats per figure).
    raw: Vec<Vec<f32>>,
    /// Unit id -> the buffer handed out (`raw` zero-padded to the capacity).
    padded: Vec<PackedFloat32Array>,
    /// PB3e: unit id -> serial of `padded[id]` (changes whenever it is
    /// rebuilt): the renderer skips re-sending an unchanged buffer.
    versions: Vec<i64>,
    serial: i64,
}

impl PoseCache {
    /// Poses of a new step (or of a changed battle) to build.
    fn renew(&mut self, key: PoseKey, units: usize) {
        match shift(self.key, key) {
            Shift::Advance(ticks) => {
                self.prev = std::mem::take(&mut self.cur);
                self.prev_ticks = ticks;
            }
            Shift::Keep => {}
            Shift::Reset => self.prev.clear(),
        }
        self.key = Some(key);
        self.cur_ticks = key.0;
        self.cur.clear();
        self.cur.resize(units, None);
        self.prev.resize(units, None);
        self.moving.clear();
        self.moving.resize(units, false);
        self.built.clear();
        self.built.resize(units, None);
        self.raw.resize(units, Vec::new());
        self.padded.resize(units, PackedFloat32Array::new());
        self.versions.resize(units, -1);
    }

    /// Forgets everything (loose ranks changed): rebuilt at the next call.
    pub(crate) fn clear(&mut self) {
        *self = PoseCache {
            serial: self.serial,
            ..PoseCache::default()
        };
    }
}

/// RJ-b: regiment centres `(x, y, z, facing)` of the two latest steps, for
/// the positions of [`BattleSim::get_units`].
#[derive(Default)]
pub(crate) struct UnitFrames {
    key: Option<PoseKey>,
    prev_ticks: u64,
    cur_ticks: u64,
    prev: Vec<Option<Pose>>,
    cur: Vec<Pose>,
    /// Blend factor bits written in the cached dictionaries.
    shown: Option<u64>,
}

impl UnitFrames {
    fn renew(&mut self, key: PoseKey, centres: Vec<Pose>) {
        match shift(self.key, key) {
            Shift::Advance(ticks) => {
                self.prev = self.cur.iter().copied().map(Some).collect();
                self.prev_ticks = ticks;
            }
            Shift::Keep => {}
            Shift::Reset => self.prev.clear(),
        }
        self.key = Some(key);
        self.cur_ticks = key.0;
        self.cur = centres;
        self.prev.resize(self.cur.len(), None);
        self.shown = Some(1f64.to_bits());
    }

    fn span(&self) -> u64 {
        self.cur_ticks.saturating_sub(self.prev_ticks).max(1)
    }
}

/// The zero-padded copy of `raw` (`capacity` figures at least, never cut).
fn padded_buffer(raw: &[f32], capacity: usize) -> PackedFloat32Array {
    let len = raw.len().max(capacity * 12);
    let mut out = PackedFloat32Array::from(raw);
    if len > raw.len() {
        out.resize(len);
    }
    out
}

/// `[a, b]` as a Godot array.
fn pair(a: &PackedInt32Array, b: &VarArray) -> VarArray {
    let mut out = VarArray::new();
    out.push(&a.to_variant());
    out.push(&b.to_variant());
    out
}

impl BattleSim {
    /// RJ-b: part of the next step already elapsed (replay: of the next
    /// recorded step); 1 without a battle.
    pub(crate) fn step_fraction_now(&self) -> f64 {
        match (&self.sim, &self.player) {
            (Some(sim), Some(player)) => player.step_fraction(sim),
            (Some(sim), None) => sim.step_fraction(),
            _ => 1.0,
        }
    }

    /// Blend factor of the frame for poses built at `prev_ticks` and
    /// `cur_ticks` (1 when blending is off).
    fn frame_blend(&self, prev_ticks: u64, cur_ticks: u64) -> f64 {
        if !self.pose_lerp {
            return 1.0;
        }
        lerp::blend_factor(prev_ticks, cur_ticks, self.step_fraction_now())
    }

    /// [`Self::get_units`]: dictionaries rebuilt at each step (PB3c), their
    /// `x`, `y`, `z` and `facing` blended each frame (RJ-b), plus
    /// `ground_speed` (m/s of the centre over the latest step).
    pub(crate) fn units_now(&mut self) -> VarArray {
        let Some(sim) = &self.sim else {
            return VarArray::new();
        };
        let key = (sim.ticks(), self.pose_epoch, self.figure_scale.to_bits());
        let cached = match &self.units_cache {
            Some((cached, units)) if *cached == key => Some(units.clone()),
            _ => None,
        };
        let units = match cached {
            Some(units) => units,
            None => {
                let centres: Vec<Pose> = sim
                    .units()
                    .iter()
                    .map(|u| [u.x, sim.standing_height(u, u.x, u.z), u.z, u.facing])
                    .collect();
                self.unit_frames.renew(key, centres);
                let units = self.build_units();
                let frames = &self.unit_frames;
                for (id, cur) in frames.cur.iter().enumerate() {
                    let Some(mut dict) =
                        units.get(id).and_then(|v| v.try_to::<VarDictionary>().ok())
                    else {
                        continue;
                    };
                    let prev = frames.prev.get(id).and_then(Option::as_ref);
                    dict.set(
                        "ground_speed",
                        lerp::ground_speed(prev, cur, frames.span(), DT),
                    );
                }
                self.units_cache = Some((key, units.clone()));
                units
            }
        };
        let frames = &self.unit_frames;
        let t = self.frame_blend(frames.prev_ticks, frames.cur_ticks);
        if self.unit_frames.shown != Some(t.to_bits()) {
            let frames = &self.unit_frames;
            let teleport = TELEPORT_M_PER_STEP * frames.span() as f64;
            for (id, cur) in frames.cur.iter().enumerate() {
                let Some(prev) = frames.prev.get(id).and_then(Option::as_ref) else {
                    continue;
                };
                if prev == cur {
                    continue;
                }
                let Some(mut dict) = units.get(id).and_then(|v| v.try_to::<VarDictionary>().ok())
                else {
                    continue;
                };
                let [x, y, z, facing] = lerp::blend_pose(prev, cur, t, teleport);
                dict.set("x", x);
                dict.set("y", y);
                dict.set("z", z);
                dict.set("facing", facing);
            }
            self.unit_frames.shown = Some(t.to_bits());
        }
        units
    }
}

#[godot_api(secondary)]
impl BattleSim {
    /// RJ-b: draws the figures and regiments blended between the two latest
    /// simulation steps (one step behind the battle, continuous). Off by
    /// default (tests, headless runs: the poses of the current step).
    #[func]
    fn set_pose_lerp(&mut self, enabled: bool) {
        self.pose_lerp = enabled;
    }

    #[func]
    fn get_pose_lerp(&self) -> bool {
        self.pose_lerp
    }

    /// RJ-b: part of the next simulation step already elapsed (0-1).
    #[func]
    fn get_step_fraction(&self) -> f64 {
        self.step_fraction_now()
    }

    /// PO4 loose ranks drawn by the core (RJ-b): each figure of
    /// [`Self::get_soldier_buffers`] off its place by up to `offset_m` along
    /// x and z and `yaw_deg` of yaw, stable per (regiment, rank). 0, 0: off.
    #[func]
    fn set_loose_ranks(&mut self, offset_m: f64, yaw_deg: f64) {
        let loose = (offset_m > 0.0 || yaw_deg > 0.0).then(|| LooseRanks {
            offset_m: offset_m.max(0.0),
            yaw_rad: yaw_deg.max(0.0).to_radians(),
        });
        if loose != self.loose {
            self.loose = loose;
            self.poses.clear();
        }
    }

    /// PB3c: every regiment's `MultiMesh.buffer` in one call. `capacities[id]`
    /// is the instance count of regiment `id`'s `MultiMesh` (< 0 or missing:
    /// not drawn, no buffer). → `[counts: PackedInt32Array, buffers: Array]`
    /// indexed by unit id: `counts[id]` figures (12 floats each, as
    /// `get_soldier_buffer`), `buffers[id]` zero-padded to
    /// `max(counts[id], capacities[id])` figures. Poses are computed once per
    /// simulation step; a buffer changes only after a step, a change of the
    /// battle or, for a moving regiment with `set_pose_lerp`, a new blend
    /// factor; otherwise the same buffer comes back (copy-on-write). PB3e: a
    /// third element `versions: PackedInt64Array` numbers each buffer; it
    /// changes when the buffer does.
    #[func]
    fn get_soldier_buffers(&mut self, capacities: PackedInt32Array) -> VarArray {
        let Some(sim) = &self.sim else {
            return pair(&PackedInt32Array::new(), &VarArray::new());
        };
        let key = (sim.ticks(), self.pose_epoch, self.figure_scale.to_bits());
        let units = sim.units();
        if self.poses.key != Some(key) || self.poses.cur.len() != units.len() {
            self.poses.renew(key, units.len());
        }
        let t = self.frame_blend(self.poses.prev_ticks, self.poses.cur_ticks);
        let (scale, loose) = (self.figure_scale, self.loose);
        let capacities = capacities.as_slice();
        let wanted = |id: usize| capacities.get(id).is_some_and(|&c| c >= 0);
        let cache = &mut self.poses;
        let teleport =
            TELEPORT_M_PER_STEP * cache.cur_ticks.saturating_sub(cache.prev_ticks).max(1) as f64;
        let mut versions = PackedInt64Array::new();
        versions.resize(units.len());
        let mut counts = PackedInt32Array::new();
        counts.resize(units.len());
        let mut buffers = VarArray::new();
        for (id, unit) in units.iter().enumerate() {
            if !wanted(id) {
                cache.cur[id] = None;
                cache.built[id] = None;
                cache.raw[id] = Vec::new();
                cache.padded[id] = PackedFloat32Array::new();
                buffers.push(&PackedFloat32Array::new().to_variant());
                continue;
            }
            let capacity = capacities[id].max(0) as usize;
            if cache.cur[id].is_none() {
                // SG1: climbers drawn on their ladders / the tower bridge.
                let poses = sim.soldier_poses(unit, scale);
                cache.moving[id] = cache.prev[id].as_ref().is_some_and(|p| *p != poses);
                cache.cur[id] = Some(poses);
                cache.built[id] = None;
            }
            let shown = if cache.moving[id] { t } else { 1.0 };
            if cache.built[id] != Some(shown.to_bits()) {
                let cur = cache.cur[id].as_deref().unwrap_or_default();
                let prev = cache.prev[id].as_deref();
                lerp::write_figures(
                    &mut cache.raw[id],
                    unit.id,
                    prev,
                    cur,
                    shown,
                    teleport,
                    loose,
                );
                cache.padded[id] = padded_buffer(&cache.raw[id], capacity);
                cache.built[id] = Some(shown.to_bits());
                cache.serial += 1;
                cache.versions[id] = cache.serial;
            }
            let raw = &cache.raw[id];
            let len = raw.len().max(capacity * 12);
            if cache.padded[id].len() != len {
                cache.padded[id] = padded_buffer(raw, capacity);
                cache.serial += 1;
                cache.versions[id] = cache.serial;
            }
            versions[id] = cache.versions[id];
            counts[id] = (raw.len() / 12) as i32;
            buffers.push(&cache.padded[id].to_variant());
        }
        let mut out = pair(&counts, &buffers);
        out.push(&versions.to_variant());
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_shift_the_poses_only_forward_and_close() {
        let old = Some((10, 3, 7));
        assert_eq!(shift(None, (10, 3, 7)), Shift::Reset);
        assert_eq!(shift(old, (11, 3, 7)), Shift::Advance(10));
        assert_eq!(shift(old, (12, 4, 7)), Shift::Advance(10));
        assert_eq!(shift(old, (10, 4, 7)), Shift::Keep);
        assert_eq!(shift(old, (9, 3, 7)), Shift::Reset);
        assert_eq!(shift(old, (10 + lerp::MAX_SPAN + 1, 3, 7)), Shift::Reset);
        assert_eq!(shift(old, (11, 3, 8)), Shift::Reset);
    }
}
