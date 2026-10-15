//! Terroir mask of the ground shader (port of `terroir_mask.gd`, CV1): an RGBA8 `SIZE`² image
//! over the map (square: 1024 / larger map side, like `TerroirMask`), R fields of the finage
//! around each settlement, G vine (wine regions, from the land-use raster), B burnt land
//! (devastation), A pasture ring. Read by `terrain.wesl` (`terroir_splat`, `terroir_landuse`,
//! `hb_clearing_splat`, `fp_parcels`). Visual only: population and devastation come from the
//! simulation state.
//!
//! Built once per campaign on a thread when the places (`markers::places::Places`) and the
//! land-use raster (`countryside::landuse::LanduseTexture`) are ready. Godot rebuilds it at
//! every turn (`build_async`); here it is built once (the burn channel, which follows the
//! devastation, is not drawn yet).

use std::sync::{Arc, Mutex};

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use serde_json::{Value, json};

use super::TerrainMaterialHandle;
use super::material::TerrainMaterial;
use crate::campaign::countryside::landuse::LanduseTexture;
use crate::campaign::markers::places::Places;

/// Side of the mask (`TerroirMask.SIZE`).
pub const SIZE: usize = 1024;
/// Population of reference (scale 1 of the radius).
const REFERENCE_POPULATION: f32 = 300_000.0;
/// Devastation (%) from which the land burns.
const BURN_THRESHOLD: f32 = 8.0;
const HAMLET_RADIUS: f32 = 3.2;

fn radius_of(kind: &str) -> f32 {
    match kind {
        "city" => 13.0,
        "town" => 9.0,
        "castle" => 6.0,
        "abbey" => 7.0,
        _ => 6.0,
    }
}

fn intensity_of(kind: &str) -> f32 {
    match kind {
        "city" => 1.0,
        "town" => 0.9,
        "castle" => 0.6,
        "abbey" => 0.8,
        "village" => 0.75,
        _ => 0.7,
    }
}

/// One finage to paint: position (map px), radius (map px), intensity, vine and bocage shares
/// of the land use there, devastation (%).
#[derive(Clone, Copy, Debug)]
pub struct Finage {
    pub px: Vec2,
    pub radius: f32,
    pub intensity: f32,
    pub vine: f32,
    pub bocage: f32,
    pub devastation: f32,
}

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Mask bytes (RGBA8, `SIZE`²): `TerroirMask._compute` / `_paint` (combination by maximum).
pub fn compute(finages: &[Finage], map_size: Vec2) -> Vec<u8> {
    let scale = SIZE as f32 / map_size.x.max(map_size.y).max(1.0);
    let mut data = vec![0u8; SIZE * SIZE * 4];
    for f in finages {
        let center = f.px * scale;
        let outer = f.radius * 1.6 * scale;
        let inner = f.radius * scale;
        let burn = if f.devastation >= BURN_THRESHOLD {
            (f.devastation / 100.0).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let vine_amount = (f.vine * 1.8).clamp(0.0, 1.0);
        let pasture = (0.35 + 0.65 * f.bocage).clamp(0.0, 1.0);
        let x0 = ((center.x - outer) as i32 - 1).max(0);
        let x1 = ((center.x + outer) as i32 + 1).min(SIZE as i32 - 1);
        let y0 = ((center.y - outer) as i32 - 1).max(0);
        let y1 = ((center.y + outer) as i32 + 1).min(SIZE as i32 - 1);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let d = Vec2::new(x as f32 + 0.5, y as f32 + 0.5).distance(center);
                if d > outer {
                    continue;
                }
                let field = f.intensity * (1.0 - smoothstep(inner * 0.55, inner, d));
                let ring = pasture
                    * f.intensity
                    * smoothstep(inner * 0.6, inner, d)
                    * (1.0 - smoothstep(inner * 1.25, outer, d));
                let scorched = burn * (1.0 - smoothstep(inner * 0.8, outer, d));
                let o = (y as usize * SIZE + x as usize) * 4;
                let put = |v: f32| (v * 255.0) as u8;
                data[o] = data[o].max(put(field));
                data[o + 1] = data[o + 1].max(put(field * vine_amount));
                data[o + 2] = data[o + 2].max(put(scorched));
                data[o + 3] = data[o + 3].max(put(ring));
            }
        }
    }
    data
}

/// The mask, bound to the ground material.
#[derive(Resource, Clone)]
pub struct TerroirTexture {
    pub image: Handle<Image>,
    pub finages: usize,
    pub build_ms: f32,
}

type Slot = Arc<Mutex<Option<(Vec<u8>, usize, f32)>>>;

#[derive(Resource)]
pub(super) struct PendingTerroir(Slot);

/// Land-use value (RGBA 0-1) at a map position, from the raster bytes.
fn landuse_at(bytes: &[u8], side: usize, px: Vec2, map_size: Vec2) -> [f32; 4] {
    if bytes.len() < side * side * 4 || side == 0 {
        return [0.0; 4];
    }
    let x = ((px.x / map_size.x * side as f32) as usize).min(side - 1);
    let y = ((px.y / map_size.y * side as f32) as usize).min(side - 1);
    let o = (y * side + x) * 4;
    [0, 1, 2, 3].map(|i| bytes[o + i] as f32 / 255.0)
}

fn read_hamlets() -> Vec<(Vec2, String)> {
    let Ok(root) = crate::paths::find_root() else {
        return Vec::new();
    };
    let path = crate::paths::data_dir(&root).join("map/hamlets.json");
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(Value::Array(entries)) = serde_json::from_str::<Value>(&text) else {
        return Vec::new();
    };
    entries
        .iter()
        .filter_map(|e| {
            let px = e["px"].as_array()?;
            Some((
                Vec2::new(px.first()?.as_f64()? as f32, px.get(1)?.as_f64()? as f32),
                e["province"].as_str().unwrap_or("").to_string(),
            ))
        })
        .collect()
}

/// Starts the build once the places, the land use and the world size are known.
#[allow(clippy::too_many_arguments)]
pub(super) fn start(
    mut commands: Commands,
    existing: Option<Res<TerroirTexture>>,
    pending: Option<Res<PendingTerroir>>,
    places: Option<Res<Places>>,
    landuse: Option<Res<LanduseTexture>>,
    world: Option<Res<super::TerrainWorld>>,
    images: Res<Assets<Image>>,
    sim: Option<Res<crate::sim::Sim>>,
) {
    if existing.is_some() || pending.is_some() {
        return;
    }
    let (Some(places), Some(landuse), Some(world)) = (places, landuse, world) else {
        return;
    };
    if places.places.is_empty() {
        return;
    }
    let Some(lu_image) = images.get(&landuse.image) else {
        return;
    };
    let side = lu_image.width() as usize;
    let lu_bytes = lu_image.data.clone().unwrap_or_default();
    let map_size = Vec2::new(world.heights.width as f32, world.heights.height as f32);
    // Province population / devastation (sim state; reference values without a state).
    let province_values = |province: &str| -> (f32, f32) {
        let state = sim.as_ref().and_then(|s| s.state.as_ref());
        let Some(state) = state else {
            return (REFERENCE_POPULATION, 0.0);
        };
        let Ok(id) = data_model::ProvinceId::new(province) else {
            return (REFERENCE_POPULATION, 0.0);
        };
        state
            .province_state(&id)
            .map_or((REFERENCE_POPULATION, 0.0), |p| {
                (p.population.total() as f32, p.devastation as f32)
            })
    };
    let mut finages = Vec::with_capacity(places.places.len() * 4);
    for place in &places.places {
        let (population, devastation) = province_values(&place.province);
        let growth = (population / REFERENCE_POPULATION).sqrt().clamp(0.6, 1.6);
        let lu = landuse_at(&lu_bytes, side, place.px, map_size);
        finages.push(Finage {
            px: place.px,
            radius: radius_of(&place.kind) * growth,
            intensity: intensity_of(&place.kind) * (0.75 + 0.25 * growth).clamp(0.0, 1.0),
            vine: lu[0],
            bocage: lu[2],
            devastation,
        });
    }
    for (px, province) in read_hamlets() {
        let (_, devastation) = province_values(&province);
        let lu = landuse_at(&lu_bytes, side, px, map_size);
        finages.push(Finage {
            px,
            radius: HAMLET_RADIUS,
            intensity: 0.55,
            vine: lu[0],
            bocage: lu[2],
            devastation,
        });
    }
    let slot: Slot = Arc::new(Mutex::new(None));
    let out = Arc::clone(&slot);
    std::thread::spawn(move || {
        let started = std::time::Instant::now();
        let bytes = compute(&finages, map_size);
        *out.lock().expect("terroir slot") = Some((
            bytes,
            finages.len(),
            started.elapsed().as_secs_f32() * 1000.0,
        ));
    });
    commands.insert_resource(PendingTerroir(slot));
}

/// Installs the computed mask and binds it (with the land use) to the ground material.
pub(super) fn finish(
    mut commands: Commands,
    pending: Option<Res<PendingTerroir>>,
    mut images: ResMut<Assets<Image>>,
    terroir: Option<Res<TerroirTexture>>,
    landuse: Option<Res<LanduseTexture>>,
    handle: Option<Res<TerrainMaterialHandle>>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
) {
    if let Some(pending) = pending {
        let Some((bytes, finages, build_ms)) = pending.0.lock().expect("terroir slot").take()
        else {
            return;
        };
        commands.remove_resource::<PendingTerroir>();
        let mut image = Image::new(
            Extent3d {
                width: SIZE as u32,
                height: SIZE as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            bytes,
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::RENDER_WORLD,
        );
        image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
            address_mode_u: ImageAddressMode::ClampToEdge,
            address_mode_v: ImageAddressMode::ClampToEdge,
            mag_filter: ImageFilterMode::Linear,
            min_filter: ImageFilterMode::Linear,
            ..default()
        });
        commands.insert_resource(TerroirTexture {
            image: images.add(image),
            finages,
            build_ms,
        });
        return;
    }
    // Bind once both textures exist (the material may be recreated on a new campaign).
    let Some(handle) = handle else {
        return;
    };
    let Some(material) = materials.get(&handle.0) else {
        return;
    };
    let want_terroir = terroir.as_ref().map(|t| t.image.clone());
    let want_landuse = landuse.as_ref().map(|l| l.image.clone());
    // `bind_ground` rewrites the ground block: the flags are checked every frame.
    let flags = Vec2::new(
        if want_landuse.is_some() { 1.0 } else { 0.0 },
        if want_terroir.is_some() { 1.0 } else { 0.0 },
    );
    if material.terroir == want_terroir
        && material.landuse == want_landuse
        && material.ground.splat.zw() == flags
    {
        return;
    }
    if let Some(mut material) = materials.get_mut(&handle.0) {
        material.landuse = want_landuse;
        material.terroir = want_terroir;
        material.ground.splat.z = flags.x;
        material.ground.splat.w = flags.y;
    }
}

/// Removes the mask on leaving the campaign (rebuilt from the next state).
pub(super) fn teardown(mut commands: Commands) {
    commands.remove_resource::<TerroirTexture>();
    commands.remove_resource::<PendingTerroir>();
}

pub fn dump(world: &World) -> Value {
    match world.get_resource::<TerroirTexture>() {
        Some(t) => json!({"ready": true, "finages": t.finages, "build_ms": t.build_ms}),
        None => json!({"ready": false, "pending": world.contains_resource::<PendingTerroir>()}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finage_paints_fields_vine_and_a_pasture_ring() {
        let map = Vec2::new(1024.0, 512.0);
        let f = Finage {
            px: Vec2::new(500.0, 200.0),
            radius: 10.0,
            intensity: 1.0,
            vine: 0.5,
            bocage: 1.0,
            devastation: 50.0,
        };
        let data = compute(&[f], map);
        let at = |x: usize, y: usize| {
            let o = (y * SIZE + x) * 4;
            [data[o], data[o + 1], data[o + 2], data[o + 3]]
        };
        // Scale 1 (1024 / 1024): centre full fields, vine 0.9 of it, burnt.
        let c = at(500, 200);
        assert!(c[0] >= 250, "{c:?}");
        assert!(c[1] > 200 && c[1] < 235, "{c:?}");
        assert!(c[2] > 100, "{c:?}");
        // On the ring (radius 11): fields gone, pasture present.
        let r = at(511, 200);
        assert!(r[0] < 10 && r[3] > 100, "{r:?}");
        // Far away: nothing.
        assert_eq!(at(10, 10), [0, 0, 0, 0]);
    }
}
