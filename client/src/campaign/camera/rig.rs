//! Pure state and maths of the campaign RTS camera: port of `campaign_camera.gd` (focus point on
//! the ground, distance, yaw, pitch derived from the distance, damping, inertia, close camera
//! against the relief). No ECS here so that the behaviour is unit-tested; `mod.rs` feeds it the
//! input and applies the result to the Bevy camera.

use bevy::prelude::*;

use crate::campaign::terrain::profile::CloseCameraProfile;
use crate::campaign::terrain::pyramid::{ReliefPyramid, tile_units};

/// `CameraFeel` (`data/ui/camera_feel.json`, block `campaign`).
#[derive(Debug, Clone, Resource)]
pub struct CameraFeel {
    pub pan_accel: f32,
    pub pan_friction: f32,
    pub follow_damping: f32,
    pub zoom_damping: f32,
    pub rotate_damping: f32,
    pub focus_glide_s: f32,
    pub drag_release_inertia: f32,
    pub rotate_mouse_deg_per_px: f32,
    pub follow_release_px: f32,
}

impl Default for CameraFeel {
    fn default() -> Self {
        CameraFeel {
            pan_accel: 14.0,
            pan_friction: 5.5,
            follow_damping: 12.0,
            zoom_damping: 9.0,
            rotate_damping: 10.0,
            focus_glide_s: 0.4,
            drag_release_inertia: 0.6,
            rotate_mouse_deg_per_px: 0.25,
            follow_release_px: 4.0,
        }
    }
}

impl CameraFeel {
    /// Reads `data/ui/camera_feel.json`; missing keys keep the fallback of `camera_feel.gd`.
    pub fn from_json(json: &serde_json::Value) -> CameraFeel {
        let mut feel = CameraFeel::default();
        let block = &json["campaign"];
        let read = |key: &str, slot: &mut f32| {
            if let Some(v) = block[key].as_f64() {
                *slot = v as f32;
            }
        };
        read("pan_accel", &mut feel.pan_accel);
        read("pan_friction", &mut feel.pan_friction);
        read("follow_damping", &mut feel.follow_damping);
        read("zoom_damping", &mut feel.zoom_damping);
        read("rotate_damping", &mut feel.rotate_damping);
        read("focus_glide_s", &mut feel.focus_glide_s);
        read("drag_release_inertia", &mut feel.drag_release_inertia);
        read("rotate_mouse_deg_per_px", &mut feel.rotate_mouse_deg_per_px);
        read("follow_release_px", &mut feel.follow_release_px);
        feel
    }
}

/// One frame of input, already translated from keys, mouse and gestures.
#[derive(Debug, Clone, Default)]
pub struct RigInput {
    /// Pan request (x right, y down), keys and screen edges, each axis in -1..1.
    pub pan: Vec2,
    /// Yaw request (-1 left, +1 right).
    pub rotate: f32,
    /// Zoom factors to apply in order (< 1 zooms in).
    pub zoom_factors: Vec<f32>,
    pub middle_pressed: bool,
    pub middle_released: bool,
    /// Alt or Shift held when the middle button went down: rotate instead of drag.
    pub rotate_modifier: bool,
    /// Mouse motion since the last frame (pixels).
    pub motion: Vec2,
    pub viewport_height: f32,
}

/// Ground point under the focus and the relief tiles, used by the close-camera rules.
pub struct Ground<'a> {
    /// Displayed ground height (world units) at (x, z).
    pub height: &'a dyn Fn(f32, f32) -> f32,
    /// Relief tiles (finest level under the focus point); `None` without a pyramid.
    pub pyramid: Option<&'a ReliefPyramid>,
}

impl Ground<'_> {
    pub fn flat() -> Ground<'static> {
        Ground {
            height: &|_, _| 0.0,
            pyramid: None,
        }
    }
}

#[derive(Debug, Clone, Resource)]
pub struct CampaignRig {
    pub focus: Vec3,
    pub distance: f32,
    pub yaw: f32,
    pub target_focus: Vec3,
    pub target_distance: f32,
    pub target_yaw: f32,
    /// World rectangle (min x, min z, max x, max z); `None` = no bounds.
    pub bounds: Option<[f32; 4]>,
    pub min_distance: f32,
    pub max_distance: f32,
    /// Hard floor of the generalised map (0 = none), `camera_floor_distance` of `campaign_map.json`.
    pub floor_distance: f32,
    pub pitch_near_deg: f32,
    pub pitch_far_deg: f32,
    pub pitch_far_distance: f32,
    pub pan_speed: f32,
    pub rotate_speed_deg: f32,
    pub zoom_step: f32,
    pub edge_pan_enabled: bool,
    pub edge_margin_px: f32,
    pub close_min_distance: f32,
    /// Circles (x, z, radius) of the emblematic towns.
    pub close_zones: Vec<Vec3>,
    pub floor_zones: Vec<Vec3>,
    pub floor_zones_set: bool,
    pub pan_velocity: Vec3,
    /// Eye position of the last pose.
    pub eye: Vec3,
    /// Followed ground point (x, z) until the next manual move.
    pub follow: Option<Vec2>,
    pub profile: CloseCameraProfile,
    pub feel: CameraFeel,
    dragging: bool,
    rotating: bool,
    follow_moved_px: f32,
    drag_accum: Vec3,
    drag_velocity: Vec3,
    glide: Option<Glide>,
    occlusion_lift: f32,
    snapping: bool,
    soft_min_key: Option<(i32, i32, i32)>,
    soft_min_value: f32,
}

#[derive(Debug, Clone, Copy)]
struct Glide {
    from_focus: Vec3,
    from_distance: f32,
    with_distance: bool,
    duration: f32,
    elapsed: f32,
}

/// Where the camera is and what it looks at.
#[derive(Debug, Clone, Copy)]
pub struct EyePose {
    pub eye: Vec3,
    pub look_at: Vec3,
    pub near: f32,
    pub far: f32,
    pub pitch_deg: f32,
}

impl CampaignRig {
    pub fn new(profile: CloseCameraProfile, feel: CameraFeel) -> CampaignRig {
        CampaignRig {
            focus: Vec3::ZERO,
            distance: 500.0,
            yaw: 0.0,
            target_focus: Vec3::ZERO,
            target_distance: 500.0,
            target_yaw: 0.0,
            bounds: None,
            min_distance: 22.0,
            max_distance: 2600.0,
            floor_distance: 0.0,
            pitch_near_deg: 30.0,
            pitch_far_deg: 70.0,
            pitch_far_distance: 2600.0,
            pan_speed: 1.2,
            rotate_speed_deg: 90.0,
            zoom_step: 0.15,
            edge_pan_enabled: true,
            edge_margin_px: 14.0,
            close_min_distance: 7.0,
            close_zones: Vec::new(),
            floor_zones: Vec::new(),
            floor_zones_set: false,
            pan_velocity: Vec3::ZERO,
            eye: Vec3::ZERO,
            follow: None,
            profile,
            feel,
            dragging: false,
            rotating: false,
            follow_moved_px: 0.0,
            drag_accum: Vec3::ZERO,
            drag_velocity: Vec3::ZERO,
            glide: None,
            occlusion_lift: 0.0,
            snapping: false,
            soft_min_key: None,
            soft_min_value: 0.0,
        }
    }

    /// `setup`: bounds from the map size, focus on the centre, snap.
    pub fn setup(&mut self, map_size: Vec2, initial_distance: f32, ground: &Ground) {
        self.bounds = Some([0.0, 0.0, map_size.x, map_size.y]);
        self.pitch_far_distance = (map_size.x.max(map_size.y) * 0.45).clamp(
            self.pitch_reference() + 1.0,
            self.max_distance.max(self.pitch_reference() + 1.0),
        );
        self.target_focus = Vec3::new(map_size.x * 0.5, 0.0, map_size.y * 0.5);
        self.target_distance = initial_distance.clamp(self.min_distance, self.max_distance);
        self.snap(ground);
    }

    pub fn pitch_reference(&self) -> f32 {
        self.profile.pitch_reference_distance
    }

    /// Applies the targets at once (no damping).
    pub fn snap(&mut self, ground: &Ground) {
        self.ground_target(ground);
        self.focus = self.target_focus;
        self.distance = self.target_distance;
        self.yaw = self.target_yaw;
        self.glide = None;
        self.pan_velocity = Vec3::ZERO;
        self.snapping = true;
        let _ = self.pose(ground);
        self.snapping = false;
    }

    /// Recentres on `point`; `new_distance <= 0` keeps the zoom. Glides unless the feel says cut.
    pub fn look_at_point(&mut self, point: Vec3, new_distance: f32, ground: &Ground) {
        self.follow = None;
        self.target_focus = point;
        if new_distance > 0.0 {
            self.target_distance =
                new_distance.clamp(self.min_distance_at(point, ground), self.max_distance);
        }
        if self.feel.focus_glide_s <= 0.0 {
            self.snap(ground);
            return;
        }
        self.glide = Some(Glide {
            from_focus: self.focus,
            from_distance: self.distance,
            with_distance: new_distance > 0.0,
            duration: self.feel.focus_glide_s,
            elapsed: 0.0,
        });
        self.pan_velocity = Vec3::ZERO;
    }

    pub fn is_gliding(&self) -> bool {
        self.glide.is_some()
    }

    /// Minimum distance above `point`: emblematic towns, relief stage under the point, floor.
    pub fn min_distance_at(&mut self, point: Vec3, ground: &Ground) -> f32 {
        let mut result = self.min_distance;
        let p2 = Vec2::new(point.x, point.z);
        for zone in &self.close_zones {
            if p2.distance(Vec2::new(zone.x, zone.y)) <= zone.z {
                result = self.close_min_distance.min(self.min_distance);
                break;
            }
        }
        if let Some(pyramid) = ground.pyramid {
            result = result.min(self.soft_min(point, pyramid));
            let zones = if self.floor_zones_set {
                &self.floor_zones
            } else {
                &self.close_zones
            };
            let landmark = landmark_floor(&self.profile, p2, zones);
            result = result.max(landmark.min(self.min_distance));
        }
        result.max(self.floor_distance)
    }

    /// Memoised while the point stays within 2 % of the current distance (`_soft_min`).
    fn soft_min(&mut self, point: Vec3, pyramid: &ReliefPyramid) -> f32 {
        let cell = (self.distance * 0.02).max(0.01);
        let key = (
            (point.x / cell).round() as i32,
            (point.z / cell).round() as i32,
            (cell * 1000.0) as i32,
        );
        if self.soft_min_key != Some(key) {
            self.soft_min_key = Some(key);
            self.soft_min_value =
                soft_min_distance(&self.profile, Vec2::new(point.x, point.z), pyramid);
        }
        self.soft_min_value
    }

    pub fn zoom_by(&mut self, factor: f32, ground: &Ground) {
        let min = self.min_distance_at(self.target_focus, ground);
        self.target_distance =
            (self.target_distance * factor).clamp(min, self.max_distance.max(min));
    }

    /// Ground-plane vector of a screen vector (x right, y down) at the current yaw.
    pub fn plane_vector(&self, screen: Vec2) -> Vec3 {
        let right = Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin());
        let forward = Vec3::new(-self.yaw.sin(), 0.0, -self.yaw.cos());
        right * screen.x - forward * screen.y
    }

    fn move_target(&mut self, offset: Vec3) {
        self.target_focus += offset;
        let Some([x0, z0, x1, z1]) = self.bounds else {
            return;
        };
        let cx = self.target_focus.x.clamp(x0, x1);
        let cz = self.target_focus.z.clamp(z0, z1);
        if cx != self.target_focus.x {
            self.pan_velocity.x = 0.0;
        }
        if cz != self.target_focus.z {
            self.pan_velocity.z = 0.0;
        }
        self.target_focus.x = cx;
        self.target_focus.z = cz;
    }

    fn ground_target(&mut self, ground: &Ground) {
        self.target_focus.y = (ground.height)(self.target_focus.x, self.target_focus.z);
    }

    /// Pitch (degrees): historic curve above the reference distance, ever flatter below it.
    pub fn pitch_deg(&self) -> f32 {
        let reference = self.pitch_reference();
        let t =
            ((self.distance - reference) / (self.pitch_far_distance - reference)).clamp(0.0, 1.0);
        let pitch = self.pitch_near_deg + (self.pitch_far_deg - self.pitch_near_deg) * t;
        if self.distance < reference {
            return self
                .profile
                .close_pitch_deg(self.distance, self.pitch_near_deg);
        }
        pitch
    }

    /// One frame (`_process`), then the new pose.
    pub fn step(&mut self, input: &RigInput, delta: f32, ground: &Ground) -> EyePose {
        // Mouse buttons and wheel (`_unhandled_input`).
        for factor in &input.zoom_factors {
            self.zoom_by(*factor, ground);
        }
        if input.middle_released {
            if self.dragging {
                self.pan_velocity = self.drag_velocity * self.feel.drag_release_inertia;
            }
            self.rotating = false;
            self.dragging = false;
        }
        if input.middle_pressed {
            self.pan_velocity = Vec3::ZERO;
            self.drag_accum = Vec3::ZERO;
            self.drag_velocity = Vec3::ZERO;
            self.rotating = input.rotate_modifier;
            self.follow_moved_px = 0.0;
            self.dragging = !self.rotating;
        }
        if input.motion != Vec2::ZERO {
            if self.rotating {
                self.target_yaw -=
                    (input.motion.x * self.feel.rotate_mouse_deg_per_px).to_radians();
            } else if self.dragging {
                self.follow_moved_px += input.motion.length();
                if self.follow_moved_px > self.feel.follow_release_px {
                    self.follow = None;
                }
                let units_per_px = self.distance * 1.6 / input.viewport_height.max(1.0);
                let before = self.target_focus;
                let v =
                    self.plane_vector(Vec2::new(-input.motion.x, -input.motion.y) * units_per_px);
                self.move_target(v);
                self.drag_accum += self.target_focus - before;
            }
        }

        if input.pan != Vec2::ZERO {
            self.follow = None;
        }
        self.update_follow();
        self.update_pan_velocity(input.pan, delta);
        self.target_yaw += self.rotate_speed_deg.to_radians() * input.rotate * delta;
        let min = self.min_distance_at(self.target_focus, ground);
        self.target_distance = self.target_distance.max(min);
        self.ground_target(ground);

        let zoom_t = 1.0 - (-self.feel.zoom_damping * delta).exp();
        let yaw_t = 1.0 - (-self.feel.rotate_damping * delta).exp();
        if let Some(mut glide) = self.glide {
            glide.elapsed += delta;
            let k = (glide.elapsed / glide.duration.max(1e-4)).clamp(0.0, 1.0);
            let eased = k * k * (3.0 - 2.0 * k);
            self.focus = glide.from_focus.lerp(self.target_focus, eased);
            self.distance = if glide.with_distance {
                glide.from_distance + (self.target_distance - glide.from_distance) * eased
            } else {
                self.distance + (self.target_distance - self.distance) * zoom_t
            };
            self.glide = if k >= 1.0 { None } else { Some(glide) };
        } else {
            let follow_t = 1.0 - (-self.feel.follow_damping * delta).exp();
            self.focus = self.focus.lerp(self.target_focus, follow_t);
            self.distance += (self.target_distance - self.distance) * zoom_t;
        }
        self.yaw = lerp_angle(self.yaw, self.target_yaw, yaw_t);
        self.pose(ground)
    }

    fn update_follow(&mut self) {
        let Some(point) = self.follow else {
            return;
        };
        self.glide = None;
        self.target_focus = Vec3::new(point.x, self.target_focus.y, point.y);
        self.move_target(Vec3::ZERO);
    }

    /// Inertia: the speed rises towards the requested one, then decays exponentially.
    fn update_pan_velocity(&mut self, pan: Vec2, delta: f32) {
        if self.dragging {
            if delta > 0.0 {
                self.drag_velocity = self.drag_velocity.lerp(self.drag_accum / delta, 0.5);
            }
            self.drag_accum = Vec3::ZERO;
            self.glide = None;
            return;
        }
        if pan != Vec2::ZERO {
            self.glide = None;
            let wanted = self.plane_vector(pan.normalize() * self.pan_speed * self.distance);
            let k = 1.0 - (-self.feel.pan_accel * delta).exp();
            self.pan_velocity = self.pan_velocity.lerp(wanted, k);
        } else {
            self.pan_velocity *= (-self.feel.pan_friction * delta).exp();
            if self.pan_velocity.length() < self.distance * 0.002 {
                self.pan_velocity = Vec3::ZERO;
            }
        }
        if self.pan_velocity != Vec3::ZERO {
            if pan == Vec2::ZERO {
                self.glide = None;
            }
            self.move_target(self.pan_velocity * delta);
        }
    }

    /// Eye and aim (`_apply_transform`): never under the relief, never behind a ridge.
    pub fn pose(&mut self, ground: &Ground) -> EyePose {
        let pitch = self.pitch_deg().to_radians();
        let horizontal = pitch.cos() * self.distance;
        let offset = Vec3::new(
            self.yaw.sin() * horizontal,
            pitch.sin() * self.distance,
            self.yaw.cos() * horizontal,
        );
        let focus = self.focus;
        let mut eye = focus + offset;
        let profile = &self.profile;
        let height = ground.height;
        let clear = profile.clearance(self.distance);
        let mut needed = height(eye.x, eye.z) + clear;
        let samples = profile.occlusion_samples.max(0);
        for k in 1..=samples {
            let t = profile.occlusion_min_t
                + (1.0 - profile.occlusion_min_t) * k as f32 / samples as f32;
            let p = focus.lerp(eye, t);
            let g = height(p.x, p.z) + clear;
            needed = needed.max(focus.y + (g - focus.y) / t);
        }
        let radius = profile.crest_radius_factor * self.distance;
        let crest = profile.crest_samples.max(0);
        for k in 0..crest {
            let angle = std::f32::consts::TAU * k as f32 / crest as f32;
            let ring = Vec2::new(angle.cos(), angle.sin()) * radius;
            needed = needed.max(height(eye.x + ring.x, eye.z + ring.y) + clear);
            needed = needed.max(
                height(focus.x + ring.x, focus.z + ring.y) * profile.crest_focus_weight + clear,
            );
        }
        let lift = (needed - eye.y).max(0.0);
        let rate = if self.snapping {
            1.0
        } else if lift > self.occlusion_lift {
            0.5
        } else {
            0.06
        };
        self.occlusion_lift += (lift - self.occlusion_lift) * rate;
        eye.y += self.occlusion_lift;
        eye.y = eye.y.max(height(eye.x, eye.z) + clear);
        self.eye = eye;
        EyePose {
            eye,
            look_at: focus + Vec3::Y * profile.look_up(self.distance),
            near: profile.near_plane(self.distance),
            far: profile.far_plane(self.distance),
            pitch_deg: self.pitch_deg(),
        }
    }
}

/// `CloseCameraProfile.landmark_floor`.
pub fn landmark_floor(profile: &CloseCameraProfile, p: Vec2, zones: &[Vec3]) -> f32 {
    if profile.landmark_min_distance <= 0.0 {
        return 0.0;
    }
    let slope = profile.min_distance_slope.max(1e-3);
    let mut best = 0.0f32;
    for zone in zones {
        let outside = (p.distance(Vec2::new(zone.x, zone.y)) - zone.z).max(0.0);
        best = best.max(profile.landmark_min_distance - slope * outside);
    }
    best
}

/// `CloseCameraProfile.soft_min_distance`: minimum over the stages k of
/// `level_min_distance[k] + slope * distance to the nearest tile of stage k`.
pub fn soft_min_distance(profile: &CloseCameraProfile, p: Vec2, pyramid: &ReliefPyramid) -> f32 {
    let mut best = profile.min_distance_for_level(pyramid.finest_level_at(p.x as f64, p.y as f64));
    let slope = profile.min_distance_slope.max(1e-3);
    let top = pyramid
        .max_level
        .min(profile.level_min_distance.len() as i32 - 1);
    for level in (1..=top).rev() {
        let floor_k = profile.min_distance_for_level(level);
        if floor_k >= best {
            continue;
        }
        let radius = (best - floor_k) / slope;
        let t = tile_units(level) as f32;
        let mut lo = ReliefPyramid::tile_at(level, (p.x - radius) as f64, (p.y - radius) as f64);
        let mut hi = ReliefPyramid::tile_at(level, (p.x + radius) as f64, (p.y + radius) as f64);
        // Guard: at most 24 x 24 tiles per stage.
        if hi.0 - lo.0 > 24 || hi.1 - lo.1 > 24 {
            let c = ReliefPyramid::tile_at(level, p.x as f64, p.y as f64);
            lo = (c.0 - 12, c.1 - 12);
            hi = (c.0 + 12, c.1 + 12);
        }
        for row in lo.1..=hi.1 {
            for col in lo.0..=hi.0 {
                if !pyramid.has_tile(level, col, row) {
                    continue;
                }
                let origin = ReliefPyramid::tile_origin(level, col, row);
                let nearest = Vec2::new(
                    p.x.clamp(origin.0 as f32, origin.0 as f32 + t),
                    p.y.clamp(origin.1 as f32, origin.1 as f32 + t),
                );
                best = best.min(floor_k + slope * nearest.distance(p));
            }
        }
    }
    best
}

/// `lerp_angle`: shortest-arc interpolation.
pub fn lerp_angle(from: f32, to: f32, t: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    let diff = (to - from).rem_euclid(tau);
    let diff = if diff > std::f32::consts::PI {
        diff - tau
    } else {
        diff
    };
    from + diff * t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::terrain::tres::Tres;

    fn rig() -> CampaignRig {
        let profile = CloseCameraProfile::from_tres(&Tres::default());
        let mut rig = CampaignRig::new(profile, CameraFeel::default());
        rig.setup(Vec2::new(7080.0, 6104.0), 7080.0 * 0.55, &Ground::flat());
        rig
    }

    fn frame(rig: &mut CampaignRig, input: RigInput, frames: usize) -> EyePose {
        let mut pose = None;
        for _ in 0..frames {
            pose = Some(rig.step(&input, 1.0 / 60.0, &Ground::flat()));
        }
        pose.unwrap()
    }

    #[test]
    fn setup_centres_on_the_map_and_clamps_the_distance() {
        let rig = rig();
        assert_eq!(rig.focus, Vec3::new(3540.0, 0.0, 3052.0));
        assert_eq!(rig.distance, 2600.0);
        // Far view: pitch at its far value (70 degrees).
        assert!((rig.pitch_deg() - 70.0).abs() < 1e-3);
    }

    #[test]
    fn pitch_follows_distance_and_goes_flat_close_up() {
        let mut rig = rig();
        rig.distance = rig.pitch_reference();
        assert!((rig.pitch_deg() - 30.0).abs() < 1e-3);
        rig.distance = 0.3;
        assert!((rig.pitch_deg() - 11.0).abs() < 1e-3);
        rig.distance = (rig.pitch_reference() + rig.pitch_far_distance) * 0.5;
        let mid = rig.pitch_deg();
        assert!(mid > 30.0 && mid < 70.0);
    }

    #[test]
    fn zoom_is_clamped_and_floor_distance_wins() {
        let mut rig = rig();
        rig.floor_distance = 20.0;
        let ground = Ground::flat();
        for _ in 0..200 {
            rig.zoom_by(0.85, &ground);
        }
        // Without a pyramid, min_distance (22) applies.
        assert!((rig.target_distance - 22.0).abs() < 1e-3);
        for _ in 0..200 {
            rig.zoom_by(1.15, &ground);
        }
        assert_eq!(rig.target_distance, rig.max_distance);
        assert!((rig.min_distance_at(rig.target_focus, &ground) - 22.0).abs() < 1e-3);
        rig.min_distance = 5.0;
        assert_eq!(rig.min_distance_at(rig.target_focus, &ground), 20.0);
    }

    #[test]
    fn keyboard_pan_moves_the_focus_with_inertia_and_stays_in_bounds() {
        let mut rig = rig();
        let start = rig.target_focus;
        let right = RigInput {
            pan: Vec2::new(1.0, 0.0),
            viewport_height: 800.0,
            ..default()
        };
        frame(&mut rig, right, 30);
        assert!(rig.target_focus.x > start.x + 10.0);
        // Released: decays to a stop.
        let idle = RigInput {
            viewport_height: 800.0,
            ..default()
        };
        frame(&mut rig, idle.clone(), 600);
        assert_eq!(rig.pan_velocity, Vec3::ZERO);
        // Push against the east edge.
        rig.target_focus.x = 7079.0;
        let right = RigInput {
            pan: Vec2::new(1.0, 0.0),
            viewport_height: 800.0,
            ..default()
        };
        frame(&mut rig, right, 120);
        assert!(rig.target_focus.x <= 7080.0);
        assert_eq!(rig.pan_velocity.x, 0.0);
    }

    #[test]
    fn rotation_keys_turn_the_yaw_and_pan_follows_it() {
        let mut rig = rig();
        let turn = RigInput {
            rotate: 1.0,
            viewport_height: 800.0,
            ..default()
        };
        frame(&mut rig, turn, 60);
        assert!((rig.target_yaw - std::f32::consts::FRAC_PI_2).abs() < 0.05);
        rig.yaw = std::f32::consts::FRAC_PI_2;
        // Rotated by 90 degrees: "right" on screen is now -z in the world... (cos, 0, -sin).
        let v = rig.plane_vector(Vec2::new(1.0, 0.0));
        assert!(v.x.abs() < 1e-5 && (v.z + 1.0).abs() < 1e-5);
    }

    #[test]
    fn middle_drag_pans_and_release_keeps_inertia() {
        let mut rig = rig();
        let start = rig.target_focus;
        let press = RigInput {
            middle_pressed: true,
            viewport_height: 800.0,
            ..default()
        };
        frame(&mut rig, press, 1);
        let drag = RigInput {
            motion: Vec2::new(-40.0, 0.0),
            viewport_height: 800.0,
            ..default()
        };
        frame(&mut rig, drag, 5);
        assert!(rig.target_focus.x > start.x, "dragging left moves the map");
        let release = RigInput {
            middle_released: true,
            viewport_height: 800.0,
            ..default()
        };
        frame(&mut rig, release, 1);
        assert!(rig.pan_velocity.length() > 0.0);
    }

    #[test]
    fn alt_middle_drag_rotates() {
        let mut rig = rig();
        let press = RigInput {
            middle_pressed: true,
            rotate_modifier: true,
            viewport_height: 800.0,
            ..default()
        };
        frame(&mut rig, press, 1);
        let drag = RigInput {
            motion: Vec2::new(100.0, 0.0),
            viewport_height: 800.0,
            ..default()
        };
        frame(&mut rig, drag, 1);
        assert!(rig.target_yaw < 0.0);
        assert!((rig.target_yaw + 25f32.to_radians()).abs() < 1e-4);
    }

    #[test]
    fn look_at_point_glides_then_arrives() {
        let mut rig = rig();
        rig.look_at_point(Vec3::new(1000.0, 0.0, 1000.0), 300.0, &Ground::flat());
        assert!(rig.is_gliding());
        let idle = RigInput {
            viewport_height: 800.0,
            ..default()
        };
        frame(&mut rig, idle, 60);
        assert!(!rig.is_gliding());
        assert!(rig.focus.distance(Vec3::new(1000.0, 0.0, 1000.0)) < 1.0);
        assert!((rig.distance - 300.0).abs() < 1.0);
    }

    #[test]
    fn camera_stays_above_a_hill_and_looks_at_the_focus() {
        let mut rig = rig();
        rig.target_distance = 50.0;
        rig.distance = 50.0;
        let hill = |x: f32, z: f32| {
            let d = ((x - 3540.0).powi(2) + (z - 3052.0).powi(2)).sqrt();
            (30.0 - d * 0.5).max(0.0)
        };
        let ground = Ground {
            height: &hill,
            pyramid: None,
        };
        rig.snap(&ground);
        let pose = rig.pose(&ground);
        assert!(pose.eye.y >= hill(pose.eye.x, pose.eye.z));
        assert!(rig.focus.y >= 0.0);
        assert!(pose.near > 0.0 && pose.far > pose.near);
    }

    #[test]
    fn soft_min_distance_uses_the_relief_stages() {
        // Fake pyramid: E0 everywhere in a 2 x 2 world, E2 on the tile (0, 0) only.
        let profile = CloseCameraProfile::from_tres(&Tres::default());
        let mut pyramid = ReliefPyramid::test_new(2, 2, 2);
        pyramid.test_set_tiles(vec![(0, vec![0, 1, 2, 3]), (2, vec![0])]);
        // Inside the E2 tile: E2 minimum distance (5.0).
        let inside = soft_min_distance(&profile, Vec2::new(10.0, 10.0), &pyramid);
        assert!((inside - 5.0).abs() < 1e-3, "{inside}");
        // Far from it: E0 value (22) reduced by the slope towards the E2 tile border.
        let outside = soft_min_distance(&profile, Vec2::new(300.0, 300.0), &pyramid);
        assert!(outside <= 22.0 && outside > inside);
        let near_border = soft_min_distance(&profile, Vec2::new(70.0, 10.0), &pyramid);
        assert!(near_border < outside);
    }

    #[test]
    fn lerp_angle_takes_the_short_way() {
        let a = lerp_angle(0.1, std::f32::consts::TAU - 0.1, 0.5);
        assert!(a.abs() < 1e-5);
    }

    #[test]
    fn feel_reads_the_json_block() {
        let json = serde_json::json!({"campaign": {"zoom_damping": 3.0, "focus_glide_s": 0.0}});
        let feel = CameraFeel::from_json(&json);
        assert_eq!(feel.zoom_damping, 3.0);
        assert_eq!(feel.focus_glide_s, 0.0);
        assert_eq!(feel.pan_accel, 14.0);
    }
}
