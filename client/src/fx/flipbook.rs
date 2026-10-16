//! Flipbook billboards: the player's FA flame and smoke sheets (`textures/fx/flame_flipbook.png`,
//! `smoke_flipbook.png`, ADR 0164) played on camera-facing quads. Port of `fx_flipbook.gdshaderinc`,
//! `fire_flame`, `fire_smoke` and `life_flame` (`.gdshader`) and of the particle set-up of
//! `siege_fire_fx.gd` (`data/fx/siege_fire.json`).
//!
//! One shared quad and one [`FlipbookMaterial`] per look: the per-sprite values (phase or age,
//! spin, seed) travel in the [`MeshTag`] so that sprites of a look batch together. Three modes
//! (`FlipbookKind`): flame particles (looping, cooling with age), smoke (the age picks the image,
//! no loop) and static flames (time driven in the shader, for the campaign map).
//!
//! Use: spawn a [`FlipbookEmitter`] (particles), or [`spawn_static_flame`] (campaign flame), and
//! take the materials from [`FlipbookLooks`]. Not ported: the soft contact fade with the depth
//! texture, the lighting of smoke by scene lights (it keeps the sheet's own shading, G channel).

use std::collections::HashMap;
use std::f32::consts::TAU;

use bevy::asset::{AssetServer, embedded_asset};
use bevy::image::ImageLoaderSettings;
use bevy::mesh::MeshTag;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

use crate::agent::dump::DumpAppExt;

const FLAME_SHEET: &str = "textures/fx/flame_flipbook.png";
const SMOKE_SHEET: &str = "textures/fx/smoke_flipbook.png";
/// Particles alive at once, all emitters together.
pub const MAX_PARTICLES: usize = 900;

// ---------------------------------------------------------------------------------------------
// Frame timing (pure).
// ---------------------------------------------------------------------------------------------

/// `flipbook_loop`: phase in turns (fraction used) -> (current image, next image wrapping to the
/// first, weight of the next).
pub fn loop_frames(phase: f32, total: u32) -> (u32, u32, f32) {
    let total = total.max(1);
    let position = phase.rem_euclid(1.0) * total as f32;
    let first = (position.floor() as u32).min(total - 1);
    (first, (first + 1) % total, position - first as f32)
}

/// `flipbook_clamped`: position in `[0, total - 1]`; the last image has no successor.
pub fn clamped_frames(position: f32, total: u32) -> (u32, u32, f32) {
    let total = total.max(1);
    let position = position.clamp(0.0, (total - 1) as f32);
    let first = position.floor() as u32;
    (first, (first + 1).min(total - 1), position - first as f32)
}

/// Smoke: the age (0-1) of a puff picks the image.
pub fn smoke_frames(age: f32, total: u32) -> (u32, u32, f32) {
    clamped_frames(age.clamp(0.0, 1.0) * (total.max(1) - 1) as f32, total)
}

/// Flame particle: the `anim_loops` cycles per lifetime plus the random offset make the phase.
pub fn flame_phase(age_fraction: f32, loops: f32, offset: f32) -> f32 {
    (age_fraction * loops + offset).rem_euclid(1.0)
}

/// `(column, row)` of an image in the sheet.
pub fn cell(frame: u32, columns: u32) -> (u32, u32) {
    (frame % columns.max(1), frame / columns.max(1))
}

/// Packs the per-sprite values into the [`MeshTag`] (see `flipbook.wesl`).
pub fn pack_tag(first: f32, second: f32, spin_turns: f32) -> u32 {
    let q16 = (first.clamp(0.0, 1.0) * 65535.0).round() as u32;
    let q8 = (second.clamp(0.0, 1.0) * 255.0).round() as u32;
    let angle = ((spin_turns.rem_euclid(1.0) * 256.0) as u32).min(255);
    q16 | (q8 << 16) | (angle << 24)
}

/// Inverse of [`pack_tag`] (to within the quantisation).
pub fn unpack_tag(tag: u32) -> (f32, f32, f32) {
    (
        (tag & 0xFFFF) as f32 / 65535.0,
        ((tag >> 16) & 0xFF) as f32 / 255.0,
        (tag >> 24) as f32 / 256.0,
    )
}

/// Deterministic `0..1` hash of a counter.
fn hash01(n: u32) -> f32 {
    let mut x = n.wrapping_mul(0x9E37_79B1) ^ 0x85EB_CA6B;
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297A_2D39);
    x ^= x >> 15;
    (x & 0xFF_FFFF) as f32 / 16_777_216.0
}

// ---------------------------------------------------------------------------------------------
// Material.
// ---------------------------------------------------------------------------------------------

/// Which animation a material plays.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum FlipbookKind {
    /// Flame particles: looping, premultiplied, cools and thins with the age.
    Flame,
    /// Smoke puffs: the age picks the image, alpha blended.
    Smoke,
    /// Campaign flame: upright, time driven in the shader, flickers.
    StaticFlame,
    /// Embers (`fire_ember.gdshader`): small glowing sparks, additive, fading from hot to cold.
    Ember,
}

impl FlipbookKind {
    pub fn key(self) -> &'static str {
        match self {
            FlipbookKind::Flame => "flame",
            FlipbookKind::Smoke => "smoke",
            FlipbookKind::StaticFlame => "static_flame",
            FlipbookKind::Ember => "ember",
        }
    }
    fn mode(self) -> f32 {
        match self {
            FlipbookKind::Flame => 0.0,
            FlipbookKind::Smoke => 1.0,
            FlipbookKind::StaticFlame => 2.0,
            FlipbookKind::Ember => 3.0,
        }
    }
}

#[derive(ShaderType, Clone, Debug)]
pub struct FlipbookParams {
    pub frames: Vec4,
    pub shape: Vec4,
    pub extra: Vec4,
    pub cold: Vec4,
    pub mid: Vec4,
    pub hot: Vec4,
    pub core: Vec4,
    pub smoke: Vec4,
}

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct FlipbookMaterial {
    #[uniform(0)]
    pub params: FlipbookParams,
    #[texture(1)]
    #[sampler(2)]
    pub sheet: Handle<Image>,
    pub kind: FlipbookKindTag,
}

/// Wrapper so the material can derive `Debug`/`Clone` with the kind outside the bind group.
#[derive(Clone, Copy, Debug)]
pub struct FlipbookKindTag(pub FlipbookKind);

impl Material for FlipbookMaterial {
    fn vertex_shader() -> ShaderRef {
        "embedded://cent_ans_client/fx/flipbook.wesl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "embedded://cent_ans_client/fx/flipbook.wesl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        match self.kind.0 {
            FlipbookKind::Smoke => AlphaMode::Blend,
            FlipbookKind::Ember => AlphaMode::Add,
            _ => AlphaMode::Premultiplied,
        }
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        _layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

fn rgb(c: [f32; 3]) -> Vec4 {
    let l = Color::srgb(c[0], c[1], c[2]).to_linear();
    Vec4::new(l.red, l.green, l.blue, 1.0)
}

// ---------------------------------------------------------------------------------------------
// Data (`data/fx/siege_fire.json`, `data/fx/map_fire_wind.json`).
// ---------------------------------------------------------------------------------------------

/// Emitter set-up of the `flames` / `smoke` blocks of `siege_fire.json`.
#[derive(Clone, Debug)]
pub struct EmitterSpec {
    pub amount: f32,
    pub life: f32,
    pub size: [f32; 2],
    pub velocity: [f32; 2],
    pub spread_m: f32,
    /// Sheet cycles per lifetime (flames).
    pub loops: f32,
}

/// Look and emitters read from the data files (defaults of the Godot script when absent).
#[derive(Resource, Clone, Debug)]
pub struct FlipbookData {
    pub flame: EmitterSpec,
    pub smoke: EmitterSpec,
    pub flame_hot: [f32; 3],
    pub flame_cold: [f32; 3],
    pub flame_emission: f32,
    pub smoke_color: [f32; 4],
    pub smoke_age_darken: f32,
    pub smoke_young_density: f32,
    pub smoke_wind_drift: f32,
    pub flame_height_m: f32,
    /// Embers rising from the fires (`embers` of `siege_fire.json`) and their HDR energy.
    pub ember: EmitterSpec,
    pub ember_energy: f32,
    /// `fx/map_fire_wind.json` `fire`: campaign flame.
    pub map_frames: [u32; 2],
    pub map_loops_per_s: f32,
    pub map_emission: f32,
    pub map_opacity: f32,
    pub map_flicker_hz: f32,
    pub map_colors: [[f32; 3]; 4],
    pub map_size: [f32; 2],
    pub map_sheet: String,
}

impl Default for FlipbookData {
    fn default() -> Self {
        FlipbookData {
            flame: EmitterSpec {
                amount: 26.0,
                life: 1.5,
                size: [3.5, 7.0],
                velocity: [1.2, 2.8],
                spread_m: 4.5,
                loops: 1.5,
            },
            smoke: EmitterSpec {
                amount: 48.0,
                life: 10.0,
                size: [5.0, 14.0],
                velocity: [2.0, 4.0],
                spread_m: 4.0,
                loops: 0.0,
            },
            flame_hot: [1.0, 0.72, 0.28],
            flame_cold: [0.35, 0.03, 0.0],
            flame_emission: 3.2,
            smoke_color: [0.13, 0.12, 0.11, 0.7],
            smoke_age_darken: 1.0,
            smoke_young_density: 1.0,
            smoke_wind_drift: 3.5,
            flame_height_m: 7.5,
            ember: EmitterSpec {
                amount: 36.0,
                life: 3.0,
                size: [0.12, 0.3],
                velocity: [3.0, 7.0],
                spread_m: 4.0,
                loops: 0.0,
            },
            ember_energy: 6.0,
            map_frames: [8, 8],
            map_loops_per_s: 0.55,
            map_emission: 2.6,
            map_opacity: 0.85,
            map_flicker_hz: 1.17,
            map_colors: [
                [0.35, 0.03, 0.0],
                [1.0, 0.32, 0.04],
                [1.0, 0.72, 0.28],
                [1.0, 0.93, 0.78],
            ],
            map_size: [3.0, 4.6],
            map_sheet: FLAME_SHEET.to_owned(),
        }
    }
}

fn floats<const N: usize>(value: &serde_json::Value, into: &mut [f32; N]) {
    if let Some(array) = value.as_array() {
        for (slot, item) in into.iter_mut().zip(array) {
            if let Some(number) = item.as_f64() {
                *slot = number as f32;
            }
        }
    }
}

fn number(value: &serde_json::Value, into: &mut f32) {
    if let Some(n) = value.as_f64() {
        *into = n as f32;
    }
}

fn emitter(value: &serde_json::Value, spec: &mut EmitterSpec) {
    number(&value["amount"], &mut spec.amount);
    number(&value["lifetime_s"], &mut spec.life);
    floats(&value["size_m"], &mut spec.size);
    floats(&value["velocity_m_s"], &mut spec.velocity);
    number(&value["spread_m"], &mut spec.spread_m);
    number(&value["anim_loops"], &mut spec.loops);
}

impl FlipbookData {
    /// Parses the two data files (either may be `Null`).
    pub fn from_json(siege: &serde_json::Value, map: &serde_json::Value) -> Self {
        let mut data = FlipbookData::default();
        number(&siege["flame_height_m"], &mut data.flame_height_m);
        emitter(&siege["flames"], &mut data.flame);
        emitter(&siege["smoke"], &mut data.smoke);
        emitter(&siege["embers"], &mut data.ember);
        number(&siege["embers"]["energy"], &mut data.ember_energy);
        let mut hot4 = [data.flame_hot[0], data.flame_hot[1], data.flame_hot[2], 1.0];
        floats(&siege["flames"]["color_start"], &mut hot4);
        data.flame_hot = [hot4[0], hot4[1], hot4[2]];
        let mut cold4 = [
            data.flame_cold[0],
            data.flame_cold[1],
            data.flame_cold[2],
            1.0,
        ];
        floats(&siege["flames"]["color_end"], &mut cold4);
        data.flame_cold = [cold4[0], cold4[1], cold4[2]];
        number(&siege["flames"]["emission"], &mut data.flame_emission);
        floats(&siege["smoke"]["color"], &mut data.smoke_color);
        number(&siege["smoke"]["age_darken"], &mut data.smoke_age_darken);
        number(
            &siege["smoke"]["young_density"],
            &mut data.smoke_young_density,
        );
        number(
            &siege["smoke"]["wind_drift_m_s"],
            &mut data.smoke_wind_drift,
        );
        let fire = &map["fire"];
        if let Some(frames) = fire["frames"].as_array() {
            for (slot, item) in data.map_frames.iter_mut().zip(frames) {
                *slot = item.as_u64().unwrap_or(8).max(1) as u32;
            }
        }
        number(&fire["loops_per_s"], &mut data.map_loops_per_s);
        number(&fire["emission"], &mut data.map_emission);
        number(&fire["opacity"], &mut data.map_opacity);
        number(&fire["flicker_hz"], &mut data.map_flicker_hz);
        floats(&fire["flame_size"], &mut data.map_size);
        for (slot, key) in
            data.map_colors
                .iter_mut()
                .zip(["color_cold", "color_mid", "color_hot", "color_core"])
        {
            floats(&fire[key], slot);
        }
        if let Some(path) = fire["flipbook"].as_str() {
            data.map_sheet = path.trim_start_matches("res://assets/").to_owned();
        }
        data
    }

    fn load() -> Self {
        let read = |name: &str| -> serde_json::Value {
            crate::paths::find_root()
                .ok()
                .and_then(|root| std::fs::read_to_string(root.join("data/fx").join(name)).ok())
                .and_then(|text| serde_json::from_str(&text).ok())
                .unwrap_or(serde_json::Value::Null)
        };
        Self::from_json(&read("siege_fire.json"), &read("map_fire_wind.json"))
    }
}

/// Shared quad and materials of every look.
#[derive(Resource)]
pub struct FlipbookLooks {
    pub quad: Handle<Mesh>,
    /// Siege / battle flame particles.
    pub flame: Handle<FlipbookMaterial>,
    /// Siege / battle smoke puffs.
    pub smoke: Handle<FlipbookMaterial>,
    /// Campaign map flame (`map_fire_wind.json`).
    pub map_flame: Handle<FlipbookMaterial>,
    /// Embers of the fires (`fire_ember.gdshader`).
    pub ember: Handle<FlipbookMaterial>,
    /// Image counts of the sheets: (flame, smoke, map flame).
    pub totals: [u32; 3],
    /// Parameters and sheet of the smoke look, the base of the tinted variants.
    smoke_base: FlipbookParams,
    smoke_sheet: Handle<Image>,
    /// Smoke materials of other colours (battle camp fires, columns, plumes, ...), by colour.
    tints: HashMap<[u32; 4], Handle<FlipbookMaterial>>,
    /// Sprites alive, refreshed every frame and incremented by [`spawn_smoke_sprite`].
    live: usize,
}

impl FlipbookLooks {
    /// Sprites alive (refreshed every frame): lets a producer leave room for the fires.
    pub fn live(&self) -> usize {
        self.live
    }

    /// The smoke material of a colour (`smoke_color` of `fire_smoke.gdshader`: sRGB rgb, alpha =
    /// base opacity), created on first use.
    pub fn smoke_tint(
        &mut self,
        color: [f32; 4],
        materials: &mut Assets<FlipbookMaterial>,
    ) -> Handle<FlipbookMaterial> {
        let key = color.map(|c| (c.clamp(0.0, 1.0) * 1000.0).round() as u32);
        if let Some(handle) = self.tints.get(&key) {
            return handle.clone();
        }
        let mut params = self.smoke_base.clone();
        let l = Color::srgb(color[0], color[1], color[2]).to_linear();
        params.smoke = Vec4::new(l.red, l.green, l.blue, color[3]);
        let handle = materials.add(FlipbookMaterial {
            params,
            sheet: self.smoke_sheet.clone(),
            kind: FlipbookKindTag(FlipbookKind::Smoke),
        });
        self.tints.insert(key, handle.clone());
        handle
    }

    /// Number of tinted smoke materials built so far.
    pub fn tint_count(&self) -> usize {
        self.tints.len()
    }
}

/// One smoke sprite for [`spawn_smoke_sprite`] (positions and speeds in the world frame).
#[derive(Clone, Debug)]
pub struct SmokeSprite {
    pub position: Vec3,
    pub velocity: Vec3,
    pub accel: Vec3,
    pub damping: f32,
    pub life: f32,
    /// Size at the middle of the growth curve (it runs 0.5 -> 1.25 times this).
    pub size: f32,
    pub spin: f32,
    pub spin_rate: f32,
    /// sRGB colour and opacity; `None` = the siege smoke look.
    pub tint: Option<[f32; 4]>,
}

/// Spawns a smoke sprite outside any emitter. `None` when the particle budget is full.
pub fn spawn_smoke_sprite(
    commands: &mut Commands,
    looks: &mut FlipbookLooks,
    materials: &mut Assets<FlipbookMaterial>,
    sprite: &SmokeSprite,
) -> Option<Entity> {
    if looks.live >= MAX_PARTICLES {
        return None;
    }
    looks.live += 1;
    let material = match sprite.tint {
        Some(color) => looks.smoke_tint(color, materials),
        None => looks.smoke.clone(),
    };
    Some(
        commands
            .spawn((
                Name::new("FlipbookParticle"),
                FlipbookParticle {
                    kind: FlipbookKind::Smoke,
                    age: 0.0,
                    life: sprite.life.max(0.1),
                    position: sprite.position,
                    velocity: sprite.velocity,
                    accel: sprite.accel,
                    damping: sprite.damping,
                    size: sprite.size,
                    lift: 0.0,
                    loops: 0.0,
                    offset: 0.0,
                    spin: sprite.spin,
                    spin_rate: sprite.spin_rate,
                },
                Mesh3d(looks.quad.clone()),
                MeshMaterial3d(material),
                MeshTag::new(0),
                Transform::from_translation(sprite.position).with_scale(Vec3::splat(1e-3)),
                bevy::light::NotShadowCaster,
            ))
            .id(),
    )
}

/// One flame or ember sprite outside any emitter (fire arrows, impact sparks): `kind` is
/// [`FlipbookKind::Flame`] or [`FlipbookKind::Ember`]. `None` when the particle budget is full.
pub fn spawn_fire_sprite(
    commands: &mut Commands,
    looks: &mut FlipbookLooks,
    kind: FlipbookKind,
    sprite: &SmokeSprite,
    phase_offset: f32,
) -> Option<Entity> {
    if looks.live >= MAX_PARTICLES {
        return None;
    }
    looks.live += 1;
    let (material, lift, loops) = match kind {
        FlipbookKind::Ember => (looks.ember.clone(), 0.0, 0.0),
        _ => (looks.flame.clone(), 0.45, 1.5),
    };
    Some(
        commands
            .spawn((
                Name::new("FlipbookParticle"),
                FlipbookParticle {
                    kind,
                    age: 0.0,
                    life: sprite.life.max(0.05),
                    position: sprite.position,
                    velocity: sprite.velocity,
                    accel: sprite.accel,
                    damping: sprite.damping,
                    size: sprite.size,
                    lift,
                    loops,
                    offset: phase_offset,
                    spin: 0.0,
                    spin_rate: 0.0,
                },
                Mesh3d(looks.quad.clone()),
                MeshMaterial3d(material),
                MeshTag::new(0),
                Transform::from_translation(sprite.position).with_scale(Vec3::splat(1e-3)),
                bevy::light::NotShadowCaster,
            ))
            .id(),
    )
}

fn build_looks(
    mut commands: Commands,
    data: Res<FlipbookData>,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<FlipbookMaterial>>,
) {
    // Raw values (heat, coverage): linear, as Godot samples the sheet without `source_color`.
    let sheet = |path: String| -> Handle<Image> {
        assets
            .load_builder()
            .with_settings(|settings: &mut ImageLoaderSettings| {
                settings.is_srgb = false;
            })
            .load(path)
    };
    let zero = Vec4::ZERO;
    let flame = FlipbookParams {
        frames: Vec4::new(8.0, 8.0, FlipbookKind::Flame.mode(), 0.0),
        shape: Vec4::new(data.flame_emission, 0.55, 1.0, 0.0),
        extra: Vec4::new(1.0, 1.0, 0.0, 0.0),
        cold: rgb(data.flame_cold),
        mid: rgb([1.0, 0.32, 0.04]),
        hot: rgb(data.flame_hot),
        core: rgb([1.0, 0.93, 0.78]),
        smoke: zero,
    };
    let c = data.smoke_color;
    let smoke = FlipbookParams {
        frames: Vec4::new(8.0, 8.0, FlipbookKind::Smoke.mode(), 0.0),
        shape: Vec4::new(0.0, 1.0, 1.6, 0.0),
        extra: Vec4::new(data.smoke_young_density, data.smoke_age_darken, 0.15, 0.0),
        cold: zero,
        mid: zero,
        hot: zero,
        core: zero,
        smoke: {
            let l = Color::srgb(c[0], c[1], c[2]).to_linear();
            Vec4::new(l.red, l.green, l.blue, c[3])
        },
    };
    let mc = data.map_colors;
    let map_flame = FlipbookParams {
        frames: Vec4::new(
            data.map_frames[0] as f32,
            data.map_frames[1] as f32,
            FlipbookKind::StaticFlame.mode(),
            1.0,
        ),
        shape: Vec4::new(
            data.map_emission,
            data.map_opacity,
            1.0,
            data.map_loops_per_s,
        ),
        extra: Vec4::new(1.0, 1.0, 0.0, TAU * data.map_flicker_hz),
        cold: rgb(mc[0]),
        mid: rgb(mc[1]),
        hot: rgb(mc[2]),
        core: rgb(mc[3]),
        smoke: zero,
    };
    // Ember: colours of `fire_ember.gdshader` (hot orange to dark red), energy from the data.
    let ember = FlipbookParams {
        frames: Vec4::new(1.0, 1.0, FlipbookKind::Ember.mode(), 0.0),
        shape: Vec4::new(data.ember_energy, 1.0, 1.0, 0.0),
        extra: Vec4::new(1.0, 1.0, 0.0, 0.0),
        cold: rgb([0.6, 0.08, 0.0]),
        mid: zero,
        hot: rgb([1.0, 0.62, 0.2]),
        core: zero,
        smoke: zero,
    };
    let smoke_sheet = sheet(SMOKE_SHEET.to_owned());
    commands.insert_resource(FlipbookLooks {
        quad: meshes.add(Rectangle::new(1.0, 1.0)),
        flame: materials.add(FlipbookMaterial {
            params: flame,
            sheet: sheet(FLAME_SHEET.to_owned()),
            kind: FlipbookKindTag(FlipbookKind::Flame),
        }),
        smoke: materials.add(FlipbookMaterial {
            params: smoke.clone(),
            sheet: smoke_sheet.clone(),
            kind: FlipbookKindTag(FlipbookKind::Smoke),
        }),
        map_flame: materials.add(FlipbookMaterial {
            params: map_flame,
            sheet: sheet(data.map_sheet.clone()),
            kind: FlipbookKindTag(FlipbookKind::StaticFlame),
        }),
        ember: materials.add(FlipbookMaterial {
            params: ember,
            sheet: sheet(SMOKE_SHEET.to_owned()),
            kind: FlipbookKindTag(FlipbookKind::Ember),
        }),
        totals: [64, 64, data.map_frames[0] * data.map_frames[1]],
        smoke_base: smoke,
        smoke_sheet,
        tints: HashMap::new(),
        live: 0,
    });
}

// ---------------------------------------------------------------------------------------------
// Components and systems.
// ---------------------------------------------------------------------------------------------

/// A particle emitter (`GPUParticles3D` of `siege_fire_fx.gd`): spawns world-space sprites.
#[derive(Component, Clone, Debug)]
pub struct FlipbookEmitter {
    pub kind: FlipbookKind,
    pub spec: EmitterSpec,
    /// Godot `amount_ratio`.
    pub ratio: f32,
    /// Constant acceleration (m/s2): buoyancy plus the wind drift.
    pub accel: Vec3,
    /// Per-second velocity damping (smoke).
    pub damping: [f32; 2],
    /// Quad lift in sizes (flames: 0.45).
    pub lift: f32,
    /// Gate of the particle emission.
    pub active: bool,
    pub carry: f32,
    pub serial: u32,
    /// Smoke colour (sRGB rgb + opacity); `None` = the siege smoke look.
    pub tint: Option<[f32; 4]>,
}

impl FlipbookEmitter {
    /// Flame emitter as built by `_make_fire`.
    pub fn flames(data: &FlipbookData, radius: f32) -> Self {
        let mut spec = data.flame.clone();
        spec.spread_m = spec.spread_m.min(radius);
        FlipbookEmitter {
            kind: FlipbookKind::Flame,
            spec,
            ratio: 1.0,
            accel: Vec3::Y * 0.5,
            damping: [0.0, 0.0],
            lift: 0.45,
            active: true,
            carry: 0.0,
            serial: 0,
            tint: None,
        }
    }

    /// Ember emitter as built by `_make_embers`: sparks climbing in a 35 degree cone that the wind
    /// carries off (`wind` is the horizontal drift acceleration).
    pub fn embers(data: &FlipbookData, radius: f32, wind: Vec3) -> Self {
        let mut spec = data.ember.clone();
        spec.spread_m = radius * 0.7;
        FlipbookEmitter {
            kind: FlipbookKind::Ember,
            spec,
            ratio: 1.0,
            accel: Vec3::Y * 0.4 + wind * 0.8,
            damping: [0.5, 1.2],
            lift: 0.0,
            active: true,
            carry: 0.0,
            serial: 0,
            tint: None,
        }
    }

    /// Smoke emitter as built by `_make_fire`.
    pub fn smoke(data: &FlipbookData, radius: f32) -> Self {
        let mut spec = data.smoke.clone();
        spec.spread_m = spec.spread_m.min(radius);
        FlipbookEmitter {
            kind: FlipbookKind::Smoke,
            spec,
            ratio: 1.0,
            accel: Vec3::Y * 0.6,
            damping: [0.15, 0.3],
            lift: 0.0,
            active: true,
            carry: 0.0,
            serial: 0,
            tint: None,
        }
    }
}

/// One live sprite.
#[derive(Component, Debug)]
pub struct FlipbookParticle {
    pub kind: FlipbookKind,
    pub age: f32,
    pub life: f32,
    pub position: Vec3,
    pub velocity: Vec3,
    pub accel: Vec3,
    pub damping: f32,
    pub size: f32,
    pub lift: f32,
    pub loops: f32,
    pub offset: f32,
    pub spin: f32,
    pub spin_rate: f32,
}

/// Smoke growth `scale_curve` of `_make_fire` (0.5 -> 0.8 at 35 % -> 1.25).
pub fn smoke_growth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.35 {
        0.5 + (0.8 - 0.5) * t / 0.35
    } else {
        0.8 + (1.25 - 0.8) * (t - 0.35) / 0.65
    }
}

fn emit(
    mut commands: Commands,
    time: Res<Time>,
    looks: Option<ResMut<FlipbookLooks>>,
    mut materials: ResMut<Assets<FlipbookMaterial>>,
    mut emitters: Query<(&mut FlipbookEmitter, &GlobalTransform)>,
    live: Query<(), With<FlipbookParticle>>,
) {
    let Some(mut looks) = looks else { return };
    let mut count = live.iter().count();
    let dt = time.delta_secs().min(0.1);
    for (mut emitter, at) in &mut emitters {
        if !emitter.active || emitter.spec.life <= 0.0 {
            continue;
        }
        emitter.carry += dt * emitter.spec.amount / emitter.spec.life * emitter.ratio;
        while emitter.carry >= 1.0 {
            emitter.carry -= 1.0;
            if count >= MAX_PARTICLES {
                continue;
            }
            count += 1;
            emitter.serial = emitter.serial.wrapping_add(1);
            let n = emitter.serial.wrapping_mul(8).wrapping_add(
                (at.translation().x * 31.0 + at.translation().z * 17.0) as i32 as u32,
            );
            let r = |k: u32| hash01(n.wrapping_add(k));
            let spec = &emitter.spec;
            // Sphere of `spread_m` radius, a cone of 15 degrees around +Y.
            let dir = Vec3::new(r(0) * 2.0 - 1.0, r(1) * 2.0 - 1.0, r(2) * 2.0 - 1.0);
            let start = if dir.length_squared() > 1.0 {
                dir.normalize_or_zero() * 0.6
            } else {
                dir
            } * spec.spread_m;
            let cone = if emitter.kind == FlipbookKind::Ember {
                35f32
            } else {
                15f32
            };
            let tilt = cone.to_radians() * r(3);
            let around = r(4) * TAU;
            let heading = Vec3::new(
                tilt.sin() * around.cos(),
                tilt.cos(),
                tilt.sin() * around.sin(),
            );
            let speed = spec.velocity[0] + (spec.velocity[1] - spec.velocity[0]) * r(5);
            let size = spec.size[0] + (spec.size[1] - spec.size[0]) * r(6);
            let smoke = emitter.kind == FlipbookKind::Smoke;
            let (material, kind) = match (emitter.kind, emitter.tint) {
                (FlipbookKind::Smoke, Some(color)) => {
                    (looks.smoke_tint(color, &mut materials), FlipbookKind::Smoke)
                }
                (FlipbookKind::Smoke, None) => (looks.smoke.clone(), FlipbookKind::Smoke),
                (FlipbookKind::Ember, _) => (looks.ember.clone(), FlipbookKind::Ember),
                _ => (looks.flame.clone(), FlipbookKind::Flame),
            };
            let loops = spec.loops * (0.8 + 0.4 * r(7));
            let position = at.translation() + start;
            commands.spawn((
                Name::new("FlipbookParticle"),
                FlipbookParticle {
                    kind,
                    age: 0.0,
                    life: spec.life,
                    position,
                    velocity: heading * speed,
                    accel: emitter.accel,
                    damping: emitter.damping[0] + (emitter.damping[1] - emitter.damping[0]) * r(8),
                    size,
                    lift: emitter.lift,
                    loops,
                    offset: r(9),
                    spin: if smoke { (r(10) - 0.5) * TAU } else { 0.0 },
                    spin_rate: if smoke {
                        (r(11) - 0.5) * 24f32.to_radians()
                    } else {
                        0.0
                    },
                },
                Mesh3d(looks.quad.clone()),
                MeshMaterial3d(material),
                MeshTag::new(0),
                Transform::from_translation(position).with_scale(Vec3::splat(1e-3)),
                bevy::light::NotShadowCaster,
            ));
        }
    }
}

fn advance(
    mut commands: Commands,
    time: Res<Time>,
    mut particles: Query<(Entity, &mut FlipbookParticle, &mut Transform, &mut MeshTag)>,
) {
    let dt = time.delta_secs().min(0.1);
    for (entity, mut p, mut transform, mut tag) in &mut particles {
        p.age += dt;
        if p.age >= p.life {
            commands.entity(entity).despawn();
            continue;
        }
        let t = p.age / p.life;
        let accel = p.accel;
        p.velocity += accel * dt;
        let keep = (1.0 - p.damping * dt).max(0.0);
        p.velocity *= keep;
        let step = p.velocity * dt;
        p.position += step;
        p.spin += p.spin_rate * dt;
        let (size, value, second) = match p.kind {
            FlipbookKind::Smoke => (p.size * smoke_growth(t), t, 0.0),
            FlipbookKind::Ember => (p.size, t, 0.0),
            _ => (p.size, flame_phase(t, p.loops, p.offset), t),
        };
        transform.translation = p.position + Vec3::Y * size * p.lift;
        transform.scale = Vec3::splat(size);
        tag.value = pack_tag(value, second, p.spin / TAU);
    }
}

/// A campaign flame: upright billboard, time driven, base on the entity origin; the entity's
/// scale is the flame size (`flame_size` of `map_fire_wind.json`, times `prop_scale`).
pub fn spawn_static_flame(
    commands: &mut Commands,
    looks: &FlipbookLooks,
    seed: f32,
    transform: Transform,
) -> Entity {
    commands
        .spawn((
            Name::new("FlipbookFlame"),
            StaticFlame,
            Mesh3d(looks.quad.clone()),
            MeshMaterial3d(looks.map_flame.clone()),
            MeshTag::new(pack_tag(0.0, seed.rem_euclid(1.0), 0.0)),
            transform,
            bevy::light::NotShadowCaster,
        ))
        .id()
}

/// Marker of [`spawn_static_flame`] entities.
#[derive(Component)]
pub struct StaticFlame;

/// Counts refreshed every frame for the dump (the dump only sees `&World`).
#[derive(Resource, Default, Debug)]
pub struct FlipbookStats {
    /// kind key -> (emitters, live particles).
    pub by_kind: std::collections::BTreeMap<&'static str, (usize, usize)>,
    pub static_flames: usize,
    pub sample_frames: Vec<serde_json::Value>,
}

fn count_sprites(
    mut stats: ResMut<FlipbookStats>,
    looks: Option<ResMut<FlipbookLooks>>,
    emitters: Query<&FlipbookEmitter>,
    particles: Query<(&FlipbookParticle, &MeshTag)>,
    statics: Query<(), With<StaticFlame>>,
) {
    stats.by_kind.clear();
    stats.sample_frames.clear();
    for e in &emitters {
        stats.by_kind.entry(e.kind.key()).or_default().0 += 1;
    }
    let total = looks.as_ref().map_or(64, |l| l.totals[0]);
    if let Some(mut looks) = looks {
        looks.live = particles.iter().count();
    }
    for (p, tag) in &particles {
        stats.by_kind.entry(p.kind.key()).or_default().1 += 1;
        if stats.sample_frames.len() < 6 {
            let (v, _, _) = unpack_tag(tag.value);
            let (a, b, w) = match p.kind {
                FlipbookKind::Smoke | FlipbookKind::Ember => smoke_frames(v, total),
                _ => loop_frames(v, total),
            };
            stats
                .sample_frames
                .push(serde_json::json!({"kind": p.kind.key(), "frame": a, "next": b, "blend": w}));
        }
    }
    stats.static_flames = statics.iter().count();
}

fn dump_flipbook(world: &World) -> (String, serde_json::Value) {
    let totals = world.get_resource::<FlipbookLooks>().map(|l| l.totals);
    let data = world.get_resource::<FlipbookData>();
    let stats = world.resource::<FlipbookStats>();
    (
        "fx_flipbook".into(),
        serde_json::json!({
            "sheets": totals.map(|t| serde_json::json!({"flame_frames": t[0], "smoke_frames": t[1], "map_flame_frames": t[2]})),
            "emitters_by_kind": stats.by_kind.iter().map(|(k, v)| (k.to_string(), serde_json::json!({"emitters": v.0, "live_particles": v.1}))).collect::<serde_json::Map<_, _>>(),
            "static_flames": stats.static_flames,
            "sample_frames": stats.sample_frames,
            "flame_spec": data.map(|d| serde_json::json!({
                "amount": d.flame.amount, "life_s": d.flame.life, "loops": d.flame.loops,
                "fps_equivalent": d.flame.loops * 64.0 / d.flame.life,
                "map_fps_equivalent": d.map_loops_per_s * (d.map_frames[0] * d.map_frames[1]) as f32,
            })),
        }),
    )
}

pub struct FlipbookPlugin;

impl Plugin for FlipbookPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "flipbook.wesl");
        app.insert_resource(FlipbookData::load())
            .init_resource::<FlipbookStats>()
            .add_plugins(MaterialPlugin::<FlipbookMaterial>::default())
            .add_systems(Startup, build_looks)
            .add_systems(Update, (emit, advance, count_sprites).chain())
            .register_dump(dump_flipbook);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loop_wraps_to_the_first_image() {
        let (a, b, w) = loop_frames(63.5 / 64.0, 64);
        assert_eq!((a, b), (63, 0));
        assert!((w - 0.5).abs() < 1e-4);
        let (a, b, _) = loop_frames(1.25, 64);
        assert_eq!((a, b), (16, 17));
        let (a, _, _) = loop_frames(-0.25, 64);
        assert_eq!(a, 48);
    }

    #[test]
    fn smoke_stops_on_the_last_image() {
        assert_eq!(smoke_frames(0.0, 64), (0, 1, 0.0));
        let (a, b, w) = smoke_frames(1.0, 64);
        assert_eq!((a, b), (63, 63));
        assert_eq!(w, 0.0);
        let (a, b, w) = smoke_frames(0.5, 64);
        assert_eq!((a, b), (31, 32));
        assert!((w - 0.5).abs() < 1e-4);
        assert_eq!(smoke_frames(7.0, 64).0, 63);
    }

    #[test]
    fn cells_follow_the_row_major_atlas() {
        assert_eq!(cell(0, 8), (0, 0));
        assert_eq!(cell(9, 8), (1, 1));
        assert_eq!(cell(63, 8), (7, 7));
    }

    #[test]
    fn flame_phase_runs_the_loops_per_lifetime() {
        // 1.5 cycles in one lifetime at 64 images: 96 images, i.e. ~19.2 images/s at 5 s.
        assert!((flame_phase(1.0, 1.5, 0.0) - 0.5).abs() < 1e-5);
        assert!((flame_phase(0.5, 2.0, 0.25) - 0.25).abs() < 1e-5);
        let (a, _, _) = loop_frames(flame_phase(0.25, 1.0, 0.0), 64);
        assert_eq!(a, 16);
    }

    #[test]
    fn tag_round_trips() {
        let tag = pack_tag(0.5, 0.25, 0.75);
        let (a, b, c) = unpack_tag(tag);
        assert!((a - 0.5).abs() < 1e-4);
        assert!((b - 0.25).abs() < 5e-3);
        assert!((c - 0.75).abs() < 5e-3);
        assert_eq!(unpack_tag(pack_tag(2.0, -1.0, 1.0)).0, 1.0);
    }

    #[test]
    fn smoke_grows_along_the_curve() {
        assert_eq!(smoke_growth(0.0), 0.5);
        assert!((smoke_growth(0.35) - 0.8).abs() < 1e-5);
        assert!((smoke_growth(1.0) - 1.25).abs() < 1e-5);
    }

    #[test]
    fn data_reads_the_files() {
        let siege: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../data/fx/siege_fire.json"
            ))
            .unwrap(),
        )
        .unwrap();
        let map: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../data/fx/map_fire_wind.json"
            ))
            .unwrap(),
        )
        .unwrap();
        let data = FlipbookData::from_json(&siege, &map);
        assert_eq!(data.flame.amount, 26.0);
        assert_eq!(data.flame.size, [4.5, 8.5]);
        assert_eq!(data.smoke.life, 14.0);
        assert_eq!(data.map_frames, [8, 8]);
        assert_eq!(data.map_sheet, "textures/fx/flame_flipbook.png");
        assert!((data.smoke_age_darken - 0.72).abs() < 1e-6);
    }

    #[test]
    fn emitter_rate_follows_amount_over_life() {
        let data = FlipbookData::default();
        let e = FlipbookEmitter::flames(&data, 100.0);
        let per_second = e.spec.amount / e.spec.life * e.ratio;
        assert!((per_second - 26.0 / 1.5).abs() < 1e-4);
        assert_eq!(FlipbookEmitter::flames(&data, 2.0).spec.spread_m, 2.0);
    }
}
