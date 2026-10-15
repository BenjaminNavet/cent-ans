//! Flattened and bloodied grass, driven by the simulation (port of
//! game/scripts/battle/battle_grass_flatten.gd; the Rust `StampMap` of the bridge becomes
//! [`FlattenMap`]).
//!
//! An RG8 map at 1 m per texel over the field and 100 m of margin: R = grass trampled (0-1),
//! G = blood on the grass (0-1). It is read by the vertex stage of `wind.wesl` (grass cards fold
//! down, turn pale and yellow, blood tints them). Sources, every half second of battle time:
//! the footprint of marching regiments (capped: trodden, not shaved), the front of melees (laid
//! flat quickly), the bodies that fall (flat under the body, blood around it). The falls are
//! the drops of `Unit::soldiers()` between two frames ([`Losses`], also read by `blood.rs`).

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use sim_battle::UnitState;

use crate::app::AppState;
use crate::battle::scene::{BattleSession, StepSet};
use crate::battle::units::corpses::hash4;

/// Margin (m) covered around the field.
pub const MARGIN: f32 = 100.0;
/// Seconds of battle time between two map updates.
const STEP_S: f64 = 0.5;
/// Caps (0-255) of the trodden grass.
const MARCH_CAP: i32 = 150;
const IDLE_CAP: i32 = 70;
/// Most bodies marked per regiment and per frame.
const MAX_MARKS_PER_UNIT: u32 = 12;

/// The map and its bookkeeping.
#[derive(Resource)]
pub struct FlattenMap {
    pub width: usize,
    pub height: usize,
    /// World position (x, z) of the map corner.
    pub origin: Vec2,
    pub texel: f32,
    bytes: Vec<u8>,
    pub image: Handle<Image>,
    dirty: bool,
    /// Battle time of the last map update and last position of each regiment.
    last_time: f64,
    last_positions: HashMap<u32, Vec2>,
    pub corpse_marks: usize,
    pub uploads: usize,
}

impl FlattenMap {
    /// Map of a field `width` x `depth` m; the image goes into `images`.
    pub fn new(field_width: f32, field_depth: f32, images: &mut Assets<Image>) -> Self {
        let texel = if field_width * field_depth <= 1200.0 * 800.0 * 1.5 {
            1.0
        } else {
            2.0
        };
        let width = ((field_width + 2.0 * MARGIN) / texel) as usize;
        let height = ((field_depth + 2.0 * MARGIN) / texel) as usize;
        let bytes = vec![0u8; width * height * 2];
        let image = Image::new(
            Extent3d {
                width: width as u32,
                height: height as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            bytes.clone(),
            TextureFormat::Rg8Unorm,
            RenderAssetUsages::default(),
        );
        FlattenMap {
            width,
            height,
            origin: Vec2::splat(-MARGIN),
            texel,
            bytes,
            image: images.add(image),
            dirty: false,
            last_time: 0.0,
            last_positions: HashMap::new(),
            corpse_marks: 0,
            uploads: 0,
        }
    }

    /// Rectangle (x, z, width, depth) in metres, as the shader reads it.
    pub fn rect(&self) -> Vec4 {
        Vec4::new(
            self.origin.x,
            self.origin.y,
            self.width as f32 * self.texel,
            self.height as f32 * self.texel,
        )
    }

    fn index(&self, ix: usize, iz: usize) -> usize {
        (iz * self.width + ix) * 2
    }

    fn span(centre: f32, reach: f32, n: usize) -> std::ops::Range<usize> {
        let lo = ((centre - reach) as i64).max(0);
        let hi = ((centre + reach) as i64 + 1).min(n as i64);
        if hi <= lo {
            0..0
        } else {
            lo as usize..hi as usize
        }
    }

    /// Oriented rectangle (`half` = half sides, x along the front): adds `add_r` / `add_g`, R
    /// capped at `cap_r`.
    pub fn stamp_box(
        &mut self,
        centre: Vec2,
        facing: f32,
        half: Vec2,
        add_r: i32,
        add_g: i32,
        cap_r: i32,
    ) {
        let axis_x = Vec2::new(facing.cos(), -facing.sin());
        let axis_z = Vec2::new(facing.sin(), facing.cos());
        let reach = half.length();
        let c = (centre - self.origin) / self.texel;
        let r = reach / self.texel;
        for iz in Self::span(c.y, r, self.height) {
            for ix in Self::span(c.x, r, self.width) {
                let d = self.origin + (Vec2::new(ix as f32, iz as f32) + 0.5) * self.texel - centre;
                if d.dot(axis_x).abs() > half.x || d.dot(axis_z).abs() > half.y {
                    continue;
                }
                let i = self.index(ix, iz);
                let old = i32::from(self.bytes[i]);
                if old < cap_r {
                    self.bytes[i] = (old + add_r).min(cap_r).clamp(0, 255) as u8;
                    self.dirty = true;
                }
                if add_g > 0 {
                    let g = i32::from(self.bytes[i + 1]);
                    self.bytes[i + 1] = (g + add_g).min(255) as u8;
                    self.dirty = true;
                }
            }
        }
    }

    /// Soft-edged disc: R and G rise towards `r_value` / `g_value` (never fall).
    pub fn stamp_disc(&mut self, centre: Vec2, radius: f32, r_value: u8, g_value: u8) {
        let feather = radius * 0.5;
        let reach = radius + feather;
        let c = (centre - self.origin) / self.texel;
        let r = reach / self.texel;
        for iz in Self::span(c.y, r, self.height) {
            for ix in Self::span(c.x, r, self.width) {
                let d = (self.origin + (Vec2::new(ix as f32, iz as f32) + 0.5) * self.texel
                    - centre)
                    .length();
                if d > reach {
                    continue;
                }
                let edge = ((d - (radius - feather * 0.5)) / (reach - (radius - feather * 0.5)))
                    .clamp(0.0, 1.0);
                let k = 1.0 - edge * edge * (3.0 - 2.0 * edge);
                let i = self.index(ix, iz);
                for (channel, value) in [(0usize, r_value), (1, g_value)] {
                    let v = (f32::from(value) * k) as u8;
                    if v > self.bytes[i + channel] {
                        self.bytes[i + channel] = v;
                        self.dirty = true;
                    }
                }
            }
        }
    }

    /// Texel value (0-1) of channel `channel` at the world position.
    pub fn sample(&self, x: f32, z: f32, channel: usize) -> f32 {
        let ix = ((x - self.origin.x) / self.texel).floor();
        let iz = ((z - self.origin.y) / self.texel).floor();
        if ix < 0.0 || iz < 0.0 || ix as usize >= self.width || iz as usize >= self.height {
            return 0.0;
        }
        f32::from(self.bytes[self.index(ix as usize, iz as usize) + channel]) / 255.0
    }

    /// Texels with R above a threshold (dump).
    pub fn flattened_texels(&self) -> usize {
        self.bytes
            .as_chunks::<2>()
            .0
            .iter()
            .filter(|t| t[0] > 20)
            .count()
    }

    pub fn bloody_texels(&self) -> usize {
        self.bytes
            .as_chunks::<2>()
            .0
            .iter()
            .filter(|t| t[1] > 20)
            .count()
    }
}

/// A body that fell this frame (drop of a regiment's head count).
#[derive(Clone, Copy, Debug)]
pub struct Fall {
    pub pos: Vec2,
    pub cavalry: bool,
    pub unit: u32,
    pub routing: bool,
}

/// What the regiments did since the last frame, in a form `flatten` and `blood` share.
#[derive(Resource, Default)]
pub struct Losses {
    pub falls: Vec<Fall>,
    /// Regiments routing with heavy losses: (id, centre, frontage, depth, facing, total lost).
    pub fleeing: Vec<(u32, Vec2, f32, f32, f32, f32)>,
    heads: HashMap<u32, u32>,
    lost_total: HashMap<u32, f32>,
}

/// Regiment state of one frame, copied out of the simulation.
struct Frame {
    id: u32,
    pos: Vec2,
    facing: f32,
    half: Vec2,
    state: UnitState,
    cavalry: bool,
}

pub struct FlattenPlugin;

impl Plugin for FlattenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Losses>().add_systems(
            Update,
            (
                collect_losses,
                update_flatten.run_if(resource_exists::<FlattenMap>),
            )
                .chain()
                .after(StepSet)
                .run_if(in_state(AppState::Battle))
                .run_if(resource_exists::<BattleSession>),
        );
        app.add_systems(OnExit(AppState::Battle), |mut commands: Commands| {
            commands.remove_resource::<FlattenMap>();
            commands.insert_resource(Losses::default());
        });
    }
}

fn frames_of(session: &mut BattleSession) -> (f64, Vec<(Frame, u32)>) {
    let sim = session.sim_mut();
    let now = sim.elapsed();
    let frames = sim
        .units()
        .iter()
        .filter(|u| !u.synthetic)
        .map(|u| {
            let (frontage, depth) = u.extent();
            (
                Frame {
                    id: u.id,
                    pos: Vec2::new(u.x as f32, u.z as f32),
                    facing: u.facing as f32,
                    half: Vec2::new(frontage as f32, depth as f32) * 0.5,
                    state: u.state,
                    cavalry: u.category == data_model::UnitCategory::Cavalry,
                },
                if u.present() { u.soldiers() } else { 0 },
            )
        })
        .collect();
    (now, frames)
}

/// Compares the head counts with the previous frame and lists the bodies that fell.
pub fn collect_losses(mut session: ResMut<BattleSession>, mut losses: ResMut<Losses>) {
    losses.falls.clear();
    losses.fleeing.clear();
    let (_, frames) = frames_of(&mut session);
    for (frame, heads) in frames {
        let before = losses.heads.insert(frame.id, heads);
        let Some(before) = before else { continue };
        if heads < before {
            let lost = (before - heads).min(MAX_MARKS_PER_UNIT);
            *losses.lost_total.entry(frame.id).or_default() += (before - heads) as f32;
            let forward = Vec2::new(frame.facing.sin(), frame.facing.cos());
            let right = Vec2::new(frame.facing.cos(), -frame.facing.sin());
            for k in 0..lost {
                let h =
                    hash4(u64::from(frame.id) << 32 | u64::from(before - k) << 8 | u64::from(k));
                let across = (h[0] * 2.0 - 1.0) * frame.half.x;
                let mut along = (h[1] * 2.0 - 1.0) * frame.half.y;
                if frame.state == UnitState::Melee {
                    // The dead of a melee lie along the front rank.
                    along = frame.half.y * (0.6 + 0.4 * h[1]);
                }
                losses.falls.push(Fall {
                    pos: frame.pos + right * across + forward * along,
                    cavalry: frame.cavalry,
                    unit: frame.id,
                    routing: frame.state == UnitState::Routing,
                });
            }
        }
        if heads > 0
            && frame.state == UnitState::Routing
            && losses.lost_total.get(&frame.id).copied().unwrap_or(0.0) > 8.0
        {
            let total = losses.lost_total[&frame.id];
            losses.fleeing.push((
                frame.id,
                frame.pos,
                frame.half.x * 2.0,
                frame.half.y * 2.0,
                frame.facing,
                total,
            ));
        }
    }
}

fn update_flatten(
    mut session: ResMut<BattleSession>,
    mut map: ResMut<FlattenMap>,
    losses: Res<Losses>,
    mut images: ResMut<Assets<Image>>,
    settings: Option<Res<crate::settings::Settings>>,
) {
    let blood_level = settings.map_or(1, |s| s.blood);
    for fall in &losses.falls {
        let radius = if fall.cavalry { 1.6 } else { 1.0 };
        map.stamp_disc(fall.pos, radius, 255, 0);
        if blood_level > 0 {
            map.stamp_disc(fall.pos, radius * 1.9, 0, 200);
        }
        map.corpse_marks += 1;
    }
    let (now, frames) = frames_of(&mut session);
    let step = now - map.last_time;
    if step >= STEP_S {
        map.last_time = now;
        for (frame, heads) in frames {
            let previous = map.last_positions.insert(frame.id, frame.pos);
            if heads == 0 {
                continue;
            }
            let speed = frame.pos.distance(previous.unwrap_or(frame.pos)) / step as f32;
            if frame.state == UnitState::Melee {
                // Front of the melee: a band either side of the first rank, laid flat fast.
                let forward = Vec2::new(frame.facing.sin(), frame.facing.cos());
                let front = frame.pos + forward * frame.half.y;
                map.stamp_box(
                    front,
                    frame.facing,
                    Vec2::new(frame.half.x + 1.5, 3.0),
                    40,
                    0,
                    255,
                );
                map.stamp_box(frame.pos, frame.facing, frame.half, 8, 0, MARCH_CAP);
            } else if speed > 0.4 {
                map.stamp_box(
                    frame.pos,
                    frame.facing,
                    frame.half + Vec2::splat(0.5),
                    (speed * 4.0).clamp(4.0, 30.0) as i32,
                    0,
                    MARCH_CAP,
                );
            } else {
                map.stamp_box(frame.pos, frame.facing, frame.half, 2, 0, IDLE_CAP);
            }
        }
    }
    if map.dirty {
        map.dirty = false;
        if let Some(mut image) = images.get_mut(&map.image) {
            image.data = Some(map.bytes.clone());
            map.uploads += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map() -> FlattenMap {
        let mut images = Assets::<Image>::default();
        FlattenMap::new(1200.0, 800.0, &mut images)
    }

    #[test]
    fn map_covers_field_and_margin() {
        let m = map();
        assert_eq!((m.width, m.height), (1400, 1000));
        assert_eq!(m.rect(), Vec4::new(-100.0, -100.0, 1400.0, 1000.0));
    }

    #[test]
    fn march_is_capped_and_a_body_lays_grass_flat() {
        let mut m = map();
        for _ in 0..50 {
            m.stamp_box(
                Vec2::new(100.0, 100.0),
                0.0,
                Vec2::new(10.0, 4.0),
                30,
                0,
                MARCH_CAP,
            );
        }
        let trodden = m.sample(100.0, 100.0, 0);
        assert!((trodden - MARCH_CAP as f32 / 255.0).abs() < 0.01);
        m.stamp_disc(Vec2::new(100.0, 100.0), 1.0, 255, 0);
        assert!(m.sample(100.0, 100.0, 0) > 0.99);
        assert_eq!(m.sample(500.0, 500.0, 0), 0.0);
    }

    #[test]
    fn blood_disc_only_stains_green_channel() {
        let mut m = map();
        m.stamp_disc(Vec2::new(50.0, 50.0), 2.0, 0, 200);
        assert!(m.sample(50.0, 50.0, 1) > 0.7);
        assert_eq!(m.sample(50.0, 50.0, 0), 0.0);
        assert!(m.bloody_texels() > 5);
    }
}
