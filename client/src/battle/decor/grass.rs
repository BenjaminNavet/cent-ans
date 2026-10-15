//! Grass of the battlefield (reduced port of game/scripts/battle/battle_vegetation.gd).
//!
//! The Godot grass is two camera-following MultiMesh grids animated in `battle_grass.gdshader`.
//! Here the tufts (two crossed alpha-masked cards with the `grass_blades.png` fan) are scattered
//! once over the whole field on a jittered grid, merged per 48 m chunk, and each chunk fades out
//! at distance with a visibility range. Tufts keep off water, mud, roads, ploughland, woods and
//! hamlets (the splat of the ground decides). Wind: `wind.wesl` (amplitude, speed and gusts in `battle_decor_look.json`). Not ported: flatten / blood map, snow tint
//! shader parameters, FA7 atlas of photographed tufts, per-biome cards, near / far layers.

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::VisibilityRange;
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use sim_battle::{BattleSeason, Weather};

use super::ground::splat_at;
use super::hash01;
use super::look::DecorLook;
use super::wind::{WindExtension, WindMaterial, WindParams};
use crate::app::AppState;
use crate::battle::scene::BattleSession;
use crate::battle::scene::terrain::season_ground;

#[derive(Resource, Debug, Clone, Default)]
pub struct GrassStats {
    pub tufts: usize,
    pub chunks: usize,
}

pub struct GrassPlugin;

impl Plugin for GrassPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, scatter.run_if(resource_added::<BattleSession>))
            .add_systems(OnExit(AppState::Battle), |mut commands: Commands| {
                commands.remove_resource::<GrassStats>();
            });
    }
}

#[derive(Default)]
struct ChunkBuffers {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

impl ChunkBuffers {
    /// Adds a tuft: two crossed cards, `width` wide and `height` tall, foot at `foot`.
    fn tuft(&mut self, foot: Vec3, yaw: f32, width: f32, height: f32, shade: [f32; 3]) {
        for card in 0..2 {
            let a = yaw + card as f32 * std::f32::consts::FRAC_PI_2;
            let along = Vec3::new(a.cos(), 0.0, a.sin()) * width * 0.5;
            let normal = Vec3::new(-a.sin(), 0.0, a.cos());
            let base = self.positions.len() as u32;
            for (corner, uv, lift) in [
                (-along, [0.0, 1.0], 0.0),
                (along, [1.0, 1.0], 0.0),
                (along * 0.75, [0.875, 0.0], height),
                (-along * 0.75, [0.125, 0.0], height),
            ] {
                self.positions
                    .push((foot + corner + Vec3::Y * lift).to_array());
                // Rounded normal: mostly up, so the lighting does not flip between the sides.
                self.normals
                    .push((Vec3::Y * 0.8 + normal * 0.2).normalize().to_array());
                self.uvs.push(uv);
                let dark = if lift == 0.0 { 0.55 } else { 1.0 };
                self.colors
                    .push([shade[0] * dark, shade[1] * dark, shade[2] * dark, 1.0]);
            }
            self.indices
                .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }

    fn into_mesh(self) -> Mesh {
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
        .with_inserted_indices(Indices::U32(self.indices))
    }
}

/// Tuft colour factor of a season and weather (the card is grey, its tint is the ground colour).
fn grass_tint(season: BattleSeason, weather: Weather, gain: f32) -> [f32; 3] {
    let ground = season_ground(season);
    let mut tint = [ground[0] * gain, ground[1] * gain, ground[2] * gain];
    if matches!(season, BattleSeason::Autumn | BattleSeason::Winter) {
        tint = [tint[0] * 1.05, tint[1] * 0.95, tint[2] * 0.85];
    }
    if matches!(weather, Weather::Rain) {
        tint = [tint[0] * 0.85, tint[1] * 0.9, tint[2] * 0.85];
    }
    tint
}

fn scatter(
    mut commands: Commands,
    mut session: ResMut<BattleSession>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<WindMaterial>>,
    assets: Res<AssetServer>,
    look: Res<DecorLook>,
    mut images: ResMut<Assets<Image>>,
) {
    let (spacing, chunk_size) = (look.grass.spacing_m, look.grass.chunk_m);
    let (fade_near, fade_far) = (look.grass.fade_near_m, look.grass.fade_far_m);
    let sim = session.sim_mut();
    let field = sim.field();
    let weather = sim.weather();
    let snowy =
        matches!(weather, Weather::Snow) || matches!(field.ground, sim_battle::Ground::Snowy);
    let tint = grass_tint(field.season, weather, look.grass.tint_gain);
    let flatten =
        super::flatten::FlattenMap::new(field.width as f32, field.depth as f32, &mut images);
    let mut grass_params = WindParams::new(&look.grass.wind, 0.0, 1.0);
    grass_params.flatten_rect = flatten.rect();
    let flatten_image = flatten.image.clone();
    commands.insert_resource(flatten);
    let material = materials.add(WindMaterial {
        base: StandardMaterial {
            base_color: Color::srgb(tint[0], tint[1], tint[2]),
            base_color_texture: Some(assets.load("textures/battle/grass_blades.png")),
            alpha_mode: AlphaMode::Mask(0.4),
            double_sided: true,
            cull_mode: None,
            perceptual_roughness: 0.95,
            reflectance: 0.02,
            ..default()
        },
        extension: WindExtension {
            params: grass_params,
            flatten: Some(flatten_image),
        },
    });
    let columns = (field.width / f64::from(chunk_size)).ceil() as usize;
    let rows = (field.depth / f64::from(chunk_size)).ceil() as usize;
    let mut chunks: Vec<ChunkBuffers> = (0..columns * rows)
        .map(|_| ChunkBuffers::default())
        .collect();
    let mut tufts = 0usize;
    let nx = (field.width as f32 / spacing) as usize;
    let nz = (field.depth as f32 / spacing) as usize;
    for iz in 0..nz {
        for ix in 0..nx {
            let x = (ix as f32 + hash01(ix as f32, iz as f32, 1.0)) * spacing;
            let z = (iz as f32 + hash01(ix as f32, iz as f32, 2.0)) * spacing;
            let splat = splat_at(field, f64::from(x), f64::from(z));
            if splat.grass < 0.55 || hash01(x, z, 3.0) > splat.grass {
                continue;
            }
            let height = field.height(f64::from(x), f64::from(z)) as f32;
            let scale = 0.8 + 0.6 * hash01(x, z, 4.0);
            let variation = 0.8 + 0.35 * hash01(x, z, 5.0);
            let shade = if snowy {
                [variation * 0.7; 3]
            } else {
                [variation, variation, variation * 0.95]
            };
            let chunk = ((z / chunk_size) as usize).min(rows - 1) * columns
                + ((x / chunk_size) as usize).min(columns - 1);
            chunks[chunk].tuft(
                Vec3::new(x, height - 0.03, z),
                hash01(x, z, 6.0) * std::f32::consts::TAU,
                1.1 * scale,
                0.55 * scale,
                shade,
            );
            tufts += 1;
        }
    }
    let scope = DespawnOnExit(AppState::Battle);
    let mut spawned = 0usize;
    for buffers in chunks {
        if buffers.positions.is_empty() {
            continue;
        }
        spawned += 1;
        commands.spawn((
            Name::new("GrassChunk"),
            Mesh3d(meshes.add(buffers.into_mesh())),
            MeshMaterial3d(material.clone()),
            Transform::default(),
            NotShadowCaster,
            VisibilityRange {
                start_margin: (fade_near - 30.0)..fade_near,
                end_margin: fade_far..(fade_far + 30.0),
                use_aabb: true,
            },
            scope.clone(),
        ));
    }
    commands.insert_resource(GrassStats {
        tufts,
        chunks: spawned,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tuft_adds_two_cards() {
        let mut buffers = ChunkBuffers::default();
        buffers.tuft(Vec3::ZERO, 0.3, 1.0, 0.5, [1.0; 3]);
        assert_eq!(buffers.positions.len(), 8);
        assert_eq!(buffers.indices.len(), 12);
        assert!(buffers.positions.iter().any(|p| (p[1] - 0.5).abs() < 1e-6));
    }
}
