//! State of the exaggerated relief shared by the campaign: port of `relief_state.gd`.
//!
//! Displayed height of an altitude `h` (m) at map point (x, z):
//! `y = s * (h - K*max(h-base,0) + g*(1-K)*max(h-floor,0) + c*D)`, `K = c*k`, where `s` is the
//! dynamic vertical scale, `g` the local gain, `c` the squash weight (all functions of the
//! camera distance) and `floor`, `base`, `k`, `D` the per-cell fields of the valley floor.

use bevy::prelude::*;

use super::profile::{CloseCameraProfile, HEIGHT_SCALE, ReliefProfile};
use super::relief_floor::FloorGrid;

#[derive(Resource, Debug, Clone)]
pub struct ReliefState {
    pub relief: ReliefProfile,
    pub camera: CloseCameraProfile,
    pub grid: FloorGrid,
    squash_max: f32,
    lift_max: f32,
    lift_m_max: f32,
    /// Current vertical scale (world units per metre), local gain and squash weight.
    pub vertical_scale: f32,
    pub gain: f32,
    pub squash: f32,
}

/// (floor, base, k) interpolated at a point.
pub type Fields = Vec3;

impl ReliefState {
    pub fn new(relief: ReliefProfile, camera: CloseCameraProfile) -> ReliefState {
        let mut state = ReliefState {
            relief,
            camera,
            grid: FloorGrid::default(),
            squash_max: 0.0,
            lift_max: 0.0,
            lift_m_max: 0.0,
            vertical_scale: HEIGHT_SCALE,
            gain: 0.0,
            squash: 0.0,
        };
        state.publish();
        state
    }

    /// Publishes the valley floor (`set_relief_floor`); an empty grid disables the exaggeration.
    pub fn set_floor(&mut self, grid: FloorGrid) {
        self.squash_max = grid.squash.iter().copied().fold(0.0, f32::max);
        self.lift_max = grid.squash.iter().map(|v| -v).fold(0.0, f32::max);
        self.lift_m_max = grid.lift.iter().copied().fold(0.0, f32::max);
        self.grid = grid;
        self.publish();
    }

    pub fn has_floor(&self) -> bool {
        !self.grid.floor.is_empty()
    }

    /// Lowest vertical scale of the close views.
    pub fn near_vertical_scale(&self) -> f32 {
        HEIGHT_SCALE * self.camera.near_exaggeration(&self.relief) / self.camera.exaggeration_far
    }

    pub fn squash_full_vertical_scale(&self) -> f32 {
        HEIGHT_SCALE * self.relief.mountain_squash_full_exaggeration / self.camera.exaggeration_far
    }

    pub fn gain_for_scale(&self, scale: f32) -> f32 {
        if !self.has_floor() {
            return 0.0;
        }
        self.relief
            .gain_for_scale(scale, self.near_vertical_scale())
    }

    pub fn squash_for_scale(&self, scale: f32) -> f32 {
        if !self.has_floor()
            || (self.squash_max <= 0.0 && self.lift_max <= 0.0 && self.lift_m_max <= 0.0)
        {
            return 0.0;
        }
        self.relief
            .squash_weight_for_scale(scale, self.squash_full_vertical_scale())
    }

    /// Bounding-box helpers for `relief_lod::View` (`up`, `down`, `lift`).
    pub fn up_factor(&self, scale: f32) -> f32 {
        (1.0 + self.gain_for_scale(scale)) * (1.0 + self.squash_for_scale(scale) * self.lift_max)
    }

    pub fn down_factor(&self, scale: f32) -> f32 {
        1.0 - self.squash_for_scale(scale) * self.squash_max
    }

    pub fn massif_max(&self, scale: f32) -> f32 {
        self.squash_for_scale(scale) * self.lift_m_max
    }

    fn publish(&mut self) {
        self.gain = self.gain_for_scale(self.vertical_scale);
        self.squash = self.squash_for_scale(self.vertical_scale);
    }

    /// Sets the vertical scale (publishes gain and squash); true if it changed.
    pub fn set_vertical_scale(&mut self, scale: f32) -> bool {
        let scale = scale.max(1e-6);
        if (scale - self.vertical_scale).abs() <= 1e-9 * scale.max(1.0) {
            return false;
        }
        self.vertical_scale = scale;
        self.publish();
        true
    }

    fn cell_pos(&self, x: f32, z: f32) -> Option<(usize, f32, f32)> {
        let (sx, sz) = self.grid.side;
        if sx < 2 || sz < 2 {
            return None;
        }
        let half = 0.5 * (self.grid.cell - 1.0);
        let fx = ((x - half) / self.grid.cell).clamp(0.0, sx as f32 - 1.0);
        let fz = ((z - half) / self.grid.cell).clamp(0.0, sz as f32 - 1.0);
        let i = (fx as usize).min(sx - 2);
        let j = (fz as usize).min(sz - 2);
        Some((j * sx + i, fx - i as f32, fz - j as f32))
    }

    fn bilerp(&self, data: &[f32], o: usize, tx: f32, tz: f32) -> f32 {
        let w = self.grid.side.0;
        let top = data[o] + (data[o + 1] - data[o]) * tx;
        let bottom = data[o + w] + (data[o + w + 1] - data[o + w]) * tx;
        top + (bottom - top) * tz
    }

    /// (floor, base, k) at (x, z); zero without a floor.
    pub fn fields_at(&self, x: f32, z: f32) -> Fields {
        let Some((o, tx, tz)) = self.cell_pos(x, z).filter(|_| self.has_floor()) else {
            return Vec3::ZERO;
        };
        Vec3::new(
            self.bilerp(&self.grid.floor, o, tx, tz),
            self.bilerp(&self.grid.base, o, tx, tz),
            self.bilerp(&self.grid.squash, o, tx, tz),
        )
    }

    /// Smooth massif lift D (m) at (x, z).
    pub fn lift_at(&self, x: f32, z: f32) -> f32 {
        if !self.has_floor() || self.lift_m_max <= 0.0 {
            return 0.0;
        }
        self.cell_pos(x, z)
            .map_or(0.0, |(o, tx, tz)| self.bilerp(&self.grid.lift, o, tx, tz))
    }

    /// `display_height_fields`.
    pub fn display_height_fields(
        h_m: f32,
        fields: Fields,
        scale: f32,
        gain: f32,
        squash: f32,
        lift_m: f32,
    ) -> f32 {
        let k = squash * fields.z;
        scale
            * (h_m - k * (h_m - fields.y).max(0.0)
                + gain * (1.0 - k) * (h_m - fields.x).max(0.0)
                + squash * lift_m)
    }

    /// Displayed height at the current scale (`MapData.display_height`).
    pub fn display_height(&self, h_m: f32, x: f32, z: f32) -> f32 {
        self.display_height_with(h_m, x, z, self.vertical_scale, self.gain, self.squash)
    }

    pub fn display_height_with(
        &self,
        h_m: f32,
        x: f32,
        z: f32,
        scale: f32,
        gain: f32,
        squash: f32,
    ) -> f32 {
        if gain == 0.0 && squash == 0.0 {
            return h_m * scale;
        }
        Self::display_height_fields(
            h_m,
            self.fields_at(x, z),
            scale,
            gain,
            squash,
            self.lift_at(x, z),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::terrain::tres::Tres;

    fn state_with_floor(floor: f32, base: f32, k: f32, lift: f32) -> ReliefState {
        let relief = ReliefProfile::from_tres(&Tres::parse(
            "[resource]\nnear_exaggeration = 2.5\ngain_far = 0.0\ngain_near = -0.1\nmountain_squash_far = 1.0\nmassif_lift = 5.5\n",
        ));
        let camera = CloseCameraProfile::from_tres(&Tres::default());
        let mut state = ReliefState::new(relief, camera);
        state.set_floor(FloorGrid {
            side: (3, 3),
            cell: 8.0,
            floor: vec![floor; 9],
            base: vec![base; 9],
            squash: vec![k; 9],
            lift: vec![lift; 9],
        });
        state
    }

    #[test]
    fn without_floor_height_is_linear() {
        let state = ReliefState::new(
            ReliefProfile::from_tres(&Tres::default()),
            CloseCameraProfile::from_tres(&Tres::default()),
        );
        assert!((state.display_height(1000.0, 5.0, 5.0) - 1000.0 * HEIGHT_SCALE).abs() < 1e-6);
    }

    #[test]
    fn display_height_applies_lift_and_negative_squash_amplifies() {
        let mut state = state_with_floor(100.0, 100.0, -0.5, 200.0);
        state.set_vertical_scale(HEIGHT_SCALE);
        let flat = 1000.0 * HEIGHT_SCALE;
        let shown = state.display_height(1000.0, 8.0, 8.0);
        // k = -0.5 amplifies above the base, plus the massif lift: higher than the linear value.
        assert!(shown > flat * 1.3, "{shown} vs {flat}");
        // Below the base nothing but the lift moves.
        let low = state.display_height(50.0, 8.0, 8.0);
        assert!((low - HEIGHT_SCALE * (50.0 + state.squash * 200.0)).abs() < 1e-4);
    }

    #[test]
    fn set_vertical_scale_reports_changes_and_updates_gain() {
        let mut state = state_with_floor(0.0, 0.0, 0.0, 0.0);
        assert!(!state.set_vertical_scale(HEIGHT_SCALE));
        let near = state.near_vertical_scale();
        assert!(state.set_vertical_scale(near));
        assert!((state.gain - (-0.1)).abs() < 1e-5);
        assert!(!state.set_vertical_scale(near));
    }
}
