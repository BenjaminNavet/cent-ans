//! Grass of the battlefield (reduced port of game/scripts/battle/battle_vegetation.gd and
//! battle_grass_groups.gd).
//!
//! The Godot grass is two camera-following MultiMesh grids animated in `battle_grass.gdshader`.
//! Here the tufts (two crossed alpha-masked cards with the `grass_blades.png` fan) are scattered
//! once over the whole field on a jittered grid, merged per 48 m chunk, and each chunk fades out
//! at distance with a visibility range. Tufts keep off water, mud, roads, ploughland, woods and
//! hamlets (the splat of the ground decides). Wind: `wind.wesl` (amplitude, speed and gusts in `battle_decor_look.json`).
//!
//! Biome groups (TX T3, ADR 0241): the cards come from the group of the battle's biome (green,
//! dry, steppe, alpine, arctic; `data/art/tx_battle_grass_pack.json`, `grass_blades_<group>`,
//! `grass_tufts_<group>`, `grass_clump_<group>`): the tufts mix the three cards (60 / 25 / 15 %)
//! and each card's tint is corrected by its mean luminance so the groups keep the ground's
//! brightness. Without the pack the single `grass_blades.png` card stays.
//! Not ported: snow tint shader parameters, FA7 atlas of photographed tufts, near / far layers.

use bevy::asset::RenderAssetUsages;
use bevy::camera::primitives::Aabb;
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
    /// Group of the biome's cards (`green`, `dry`...; empty without the pack) and tufts per card.
    pub group: String,
    pub biome: u32,
    pub tufts_by_card: Vec<usize>,
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

/// A card of the biome group: texture (under the asset root), mean luminance, height / width.
#[derive(Debug, Clone, PartialEq)]
pub struct GrassCard {
    pub role: String,
    pub path: String,
    pub luma: f32,
    pub aspect: f32,
}

/// Cards of the group that serves `biome` (`data/art/tx_battle_grass_pack.json`): the group
/// name and its cards (blades, tufts, clump), the biome's parent table as fallback.
pub fn group_cards(data_dir: &std::path::Path, biome: u32) -> Option<(String, Vec<GrassCard>)> {
    let pack: serde_json::Value = serde_json::from_slice(
        &std::fs::read(data_dir.join("art/tx_battle_grass_pack.json")).ok()?,
    )
    .ok()?;
    let cards = pack["cards"].as_array()?;
    let lists = |card: &serde_json::Value, b: u32| {
        card["biomes"]
            .as_array()
            .is_some_and(|l| l.iter().any(|v| v.as_u64() == Some(u64::from(b))))
    };
    let parents: Vec<u32> = std::fs::read(data_dir.join("map/biome_parents.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .and_then(|v| {
            v["parents"].as_array().map(|a| {
                a.iter()
                    .filter_map(|p| p.as_u64().map(|p| p as u32))
                    .collect()
            })
        })
        .unwrap_or_default();
    // The biome, then its ancestors, until a card lists it.
    let mut resolved = biome;
    for _ in 0..4 {
        if cards.iter().any(|c| lists(c, resolved)) {
            break;
        }
        let parent = parents.get(resolved as usize).copied().unwrap_or(resolved);
        if parent == resolved {
            break;
        }
        resolved = parent;
    }
    let group = cards
        .iter()
        .find(|c| lists(c, resolved))
        .and_then(|c| c["id"].as_str())
        .and_then(|id| id.rsplit('_').next())?
        .to_owned();
    let found: Vec<GrassCard> = ["grass_blades", "grass_tufts", "grass_clump"]
        .iter()
        .filter_map(|role| {
            let card = cards
                .iter()
                .find(|c| c["id"].as_str() == Some(&format!("{role}_{group}")))?;
            Some(GrassCard {
                role: (*role).to_owned(),
                path: card["file"]
                    .as_str()?
                    .trim_start_matches("res://assets/")
                    .to_owned(),
                luma: card["luma"].as_f64()? as f32,
                aspect: card["aspect"].as_f64().unwrap_or(1.0) as f32,
            })
        })
        .collect();
    (!found.is_empty()).then_some((group, found))
}

/// Mean luminance of the `grass_blades.png` card the tint was tuned for (`GRASS_TEX_LUM_DA6`).
const REFERENCE_LUMA: f32 = 0.21;

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
    // Cards of the biome's group (blades, tufts, clump), else the single legacy card.
    let biome = crate::paths::find_root()
        .ok()
        .map(|root| {
            super::ground::biome_for_terrain(&crate::paths::data_dir(&root), field.terrain.key())
        })
        .unwrap_or(2);
    let regional = crate::paths::find_root()
        .ok()
        .and_then(|root| group_cards(&crate::paths::data_dir(&root), biome));
    let (group, cards): (String, Vec<GrassCard>) = regional.unwrap_or_else(|| {
        (
            String::new(),
            vec![GrassCard {
                role: "grass_blades".to_owned(),
                path: "textures/battle/grass_blades.png".to_owned(),
                luma: REFERENCE_LUMA,
                aspect: 0.763,
            }],
        )
    });
    let card_materials: Vec<Handle<WindMaterial>> = cards
        .iter()
        .map(|card| {
            // The card's own brightness is compensated: the tint was tuned on the legacy card.
            let k = (REFERENCE_LUMA / card.luma.max(0.05)).clamp(0.5, 1.6);
            materials.add(WindMaterial {
                base: StandardMaterial {
                    base_color: Color::srgb(
                        (tint[0] * k).min(1.0),
                        (tint[1] * k).min(1.0),
                        (tint[2] * k).min(1.0),
                    ),
                    base_color_texture: Some(assets.load(card.path.clone())),
                    alpha_mode: AlphaMode::Mask(0.4),
                    // Both sides drawn, but never with the normal flipped: a back face lit from below
                    // turned half of every tuft black.
                    double_sided: false,
                    cull_mode: None,
                    perceptual_roughness: 0.95,
                    reflectance: 0.02,
                    ..default()
                },
                extension: WindExtension {
                    params: grass_params,
                    flatten: Some(flatten_image.clone()),
                },
            })
        })
        .collect();
    let columns = (field.width / f64::from(chunk_size)).ceil() as usize;
    let rows = (field.depth / f64::from(chunk_size)).ceil() as usize;
    let mut chunks: Vec<Vec<ChunkBuffers>> = (0..columns * rows)
        .map(|_| (0..cards.len()).map(|_| ChunkBuffers::default()).collect())
        .collect();
    let mut tufts_by_card = vec![0usize; cards.len()];
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
            // Blades 60 %, tufts 25 %, clump 15 % (the single card when the pack is absent).
            let pick = hash01(x, z, 7.0);
            let card_index = if cards.len() < 3 || pick < 0.6 {
                0
            } else if pick < 0.85 {
                1
            } else {
                2
            };
            let (width, tall) = if card_index == 0 {
                (1.1 * scale, 0.55 * scale)
            } else {
                let width = 0.9 * scale;
                (width, width * cards[card_index].aspect * 0.6)
            };
            chunks[chunk][card_index].tuft(
                Vec3::new(x, height - 0.03, z),
                hash01(x, z, 6.0) * std::f32::consts::TAU,
                width,
                tall,
                shade,
            );
            tufts_by_card[card_index] += 1;
            tufts += 1;
        }
    }
    let scope = DespawnOnExit(AppState::Battle);
    let mut spawned = 0usize;
    for (mut buffers, card_index) in chunks
        .into_iter()
        .flat_map(|per_card| per_card.into_iter().enumerate().map(|(i, b)| (b, i)))
    {
        if buffers.positions.is_empty() {
            continue;
        }
        spawned += 1;
        // The chunk's mesh is re-centred on its own centre: the GPU visibility-range fade measures
        // the camera distance to the mesh's *origin* (the transform translation).
        let (mut low, mut high) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for p in &buffers.positions {
            low = low.min(Vec3::from(*p));
            high = high.max(Vec3::from(*p));
        }
        let centre = (low + high) * 0.5;
        for p in &mut buffers.positions {
            *p = (Vec3::from(*p) - centre).to_array();
        }
        commands.spawn((
            Aabb::from_min_max(low - centre, high - centre),
            Name::new("GrassChunk"),
            Mesh3d(meshes.add(buffers.into_mesh())),
            MeshMaterial3d(card_materials[card_index].clone()),
            Transform::from_translation(centre),
            NotShadowCaster,
            VisibilityRange {
                // Visible from the camera out to `fade_near`, then fading away by `fade_far`
                // (the margins are the fade-in / fade-out distance bands of the range).
                start_margin: 0.0..0.0,
                end_margin: fade_near..fade_far,
                use_aabb: true,
            },
            scope.clone(),
        ));
    }
    commands.insert_resource(GrassStats {
        tufts,
        chunks: spawned,
        group,
        biome,
        tufts_by_card,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn biome_groups_resolve_from_the_real_pack() {
        let root = crate::paths::find_root().expect("root");
        let data = crate::paths::data_dir(&root);
        let (group, cards) = group_cards(&data, 2).expect("green");
        assert_eq!(group, "green");
        assert_eq!(cards.len(), 3);
        assert_eq!(group_cards(&data, 3).unwrap().0, "dry");
        assert_eq!(group_cards(&data, 6).unwrap().0, "alpine");
        assert_eq!(group_cards(&data, 9).unwrap().0, "arctic");
        // Desert (8) is listed with the steppe; an unlisted biome falls back on its parent.
        assert_eq!(group_cards(&data, 8).unwrap().0, "steppe");
        for card in &cards {
            assert!(
                crate::paths::asset_root(&root).join(&card.path).exists(),
                "{}",
                card.path
            );
        }
    }

    #[test]
    fn a_tuft_adds_two_cards() {
        let mut buffers = ChunkBuffers::default();
        buffers.tuft(Vec3::ZERO, 0.3, 1.0, 0.5, [1.0; 3]);
        assert_eq!(buffers.positions.len(), 8);
        assert_eq!(buffers.indices.len(), 12);
        assert!(buffers.positions.iter().any(|p| (p[1] - 0.5).abs() < 1e-6));
    }
}
