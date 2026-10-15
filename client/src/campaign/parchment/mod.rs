//! Decoration of the strategic parchment view (camera > 1200, ADR 0124): portulan sea (wash,
//! coast water lines, rhumb lines, painted wind roses, double frame, burnt margins), sea
//! ornaments (painted sirens of the 1375 Catalan Atlas, drawn ships), ink town vignettes with
//! owner pennants, arched realm names, italic province names and heraldic army tokens. Fades
//! in and out with `ZoomView::strategic` (the only source of the two views).
//!
//! Replaces game/scripts/map/: parchment_decor.gd (ornament placement), parchment_overlay.gd
//! (2D layer), strategic_view.gd (weight, roses on the sea), map_fade.gd (fades; here every layer
//! reads `ZoomView`), and game/shaders/parchment_sea.gdshaderinc + parchment_common.gdshaderinc
//! (sea part). The land part of the parchment is `campaign/terrain` (`terrain.wesl`).
//!
//! Data: `data/map/coast_dist.png` (signed coast distance, L8, scale 2), `data/map/map.json`
//! (`size_px`), `data/map/parchment_ornaments.json` (catalogue, `use` per kind),
//! `data/map/settlement_markers.json` (`banner.parchment_pennant_units`),
//! `data/map/stance_cues.json` (pennant outlines, army rings), `game/assets/textures/parchment/`.

pub mod decor;
pub mod glyphs;
pub mod overlay;
pub mod realms;
pub mod sea;

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use serde_json::{Value, json};

use self::decor::{CoastRaster, Decor, Painted};
use self::overlay::{OverlayAssets, OverlayContent, OverlayStats, PaintedOrnament};
use self::sea::{ParchmentSea, ParchmentSeaMaterial};
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::camera::zoom::ZoomView;

/// Plugin of the parchment decoration.
pub struct CampaignParchmentPlugin;

impl Plugin for CampaignParchmentPlugin {
    fn build(&self, app: &mut App) {
        sea::register_shader(app);
        app.add_plugins(MaterialPlugin::<ParchmentSeaMaterial>::default())
            .init_resource::<ParchmentLoad>()
            .init_resource::<OverlayContent>()
            .init_resource::<OverlayStats>()
            .init_resource::<ZoomView>()
            .add_systems(
                OnEnter(AppState::Campaign),
                (start_load, reset_content, overlay::spawn_root),
            )
            .add_systems(
                Update,
                (
                    finish_load,
                    ensure_sea_plane,
                    update_sea,
                    overlay::spawn_sea,
                    overlay::rebuild_content,
                    overlay::sync_tokens,
                    overlay::update_overlay,
                )
                    .chain()
                    .run_if(in_state(AppState::Campaign)),
            );
        app.register_dump(dump_parchment);
    }
}

/// A painted ornament decoded off the main thread: image, pixel size, anchor, height, faces left.
pub type LoadedOrnament = (Image, Vec2, Vec2, f32, bool);

/// Everything decoded off the main thread.
pub struct Loaded {
    pub map_size: Vec2,
    pub coast: Option<CoastRaster>,
    pub decor: Decor,
    pub rose: Option<Image>,
    pub monsters: Vec<LoadedOrnament>,
    pub ships: Vec<LoadedOrnament>,
    pub glyphs: Vec<(&'static str, glyphs::GlyphSpec, Image)>,
    pub pennant_units: f32,
    pub cue_border: HashMap<String, Color>,
    pub cue_ring: HashMap<String, Color>,
    pub errors: Vec<String>,
}

type Pending = Arc<Mutex<Option<Loaded>>>;

/// Loading state: started once, kept across campaigns.
#[derive(Resource, Default)]
pub struct ParchmentLoad {
    pending: Option<Pending>,
    pub ready: bool,
    pub map_size: Vec2,
    pub roses: Vec<(Vec2, f32)>,
    pub painted_rose: bool,
    pub coast: Option<Handle<Image>>,
    pub rose: Option<Handle<Image>>,
    pub coast_scale: f32,
    pub errors: Vec<String>,
    pub material: Option<Handle<ParchmentSeaMaterial>>,
}

fn start_load(mut load: ResMut<ParchmentLoad>) {
    if load.ready || load.pending.is_some() {
        return;
    }
    let slot: Pending = Arc::new(Mutex::new(None));
    load.pending = Some(slot.clone());
    std::thread::spawn(move || {
        let loaded = match crate::paths::find_root() {
            Ok(root) => load_all(&root),
            Err(error) => Loaded::empty(vec![error]),
        };
        if let Ok(mut guard) = slot.lock() {
            *guard = Some(loaded);
        }
    });
}

fn reset_content(mut content: ResMut<OverlayContent>, mut load: ResMut<ParchmentLoad>) {
    content.key.clear();
    content.realms.clear();
    load.material = None;
}

impl Loaded {
    fn empty(errors: Vec<String>) -> Self {
        Self {
            map_size: Vec2::new(7080.0, 6104.0),
            coast: None,
            decor: Decor::default(),
            rose: None,
            monsters: Vec::new(),
            ships: Vec::new(),
            glyphs: Vec::new(),
            pennant_units: 0.55,
            cue_border: HashMap::new(),
            cue_ring: HashMap::new(),
            errors,
        }
    }
}

fn read_json(path: &Path) -> Option<Value> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

/// RGBA PNG to an sRGB image with mips.
fn load_rgba(path: &Path) -> Result<Image, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; reader.output_buffer_size().ok_or("png: size overflow")?];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    let channels = info.color_type.samples();
    let rgba: Vec<u8> = buf[..info.buffer_size()]
        .chunks_exact(channels)
        .flat_map(|c| match channels {
            4 => [c[0], c[1], c[2], c[3]],
            3 => [c[0], c[1], c[2], 255],
            2 => [c[0], c[0], c[0], c[1]],
            _ => [c[0], c[0], c[0], 255],
        })
        .collect();
    Ok(glyphs::rgba_image_with_mips(info.width, info.height, rgba))
}

fn hex_color(text: &str, alpha: f32) -> Option<Color> {
    let srgba = Srgba::hex(text.trim_start_matches('#')).ok()?;
    Some(Color::Srgba(srgba.with_alpha(alpha)))
}

/// Reads and decodes every input (runs on a worker thread).
pub fn load_all(root: &Path) -> Loaded {
    let data = crate::paths::data_dir(root);
    let assets = crate::paths::asset_root(root);
    let mut loaded = Loaded::empty(Vec::new());
    if let Some(size) = read_json(&data.join("map/map.json"))
        .and_then(|m| m.get("size_px").cloned())
        .and_then(|s| {
            Some(Vec2::new(
                s.get(0)?.as_f64()? as f32,
                s.get(1)?.as_f64()? as f32,
            ))
        })
    {
        loaded.map_size = size;
    }
    // Catalogue of painted ornaments.
    let mut rose_file = None;
    if let Some(catalogue) = read_json(&data.join("map/parchment_ornaments.json")) {
        let dir = assets.join("textures/parchment");
        for entry in catalogue["ornaments"].as_array().into_iter().flatten() {
            let kind = entry["kind"].as_str().unwrap_or("");
            if catalogue["use"][kind].as_str().unwrap_or("real") != "real" {
                continue;
            }
            let path = dir.join(entry["file"].as_str().unwrap_or(""));
            if !path.is_file() {
                continue;
            }
            let display = &entry["display"];
            let anchor = Vec2::new(
                display["anchor"][0].as_f64().unwrap_or(0.5) as f32,
                display["anchor"][1].as_f64().unwrap_or(0.5) as f32,
            );
            let height = display["height"].as_f64().unwrap_or(3.0) as f32;
            let faces_left = display["faces"].as_str() == Some("left");
            if kind == "rose" {
                rose_file = Some(path);
                continue;
            }
            match load_rgba(&path) {
                Ok(image) => {
                    let size = Vec2::new(image.width() as f32, image.height() as f32);
                    let item = (image, size, anchor, height, faces_left);
                    match kind {
                        "ship" => loaded.ships.push(item),
                        "monster" => loaded.monsters.push(item),
                        _ => {}
                    }
                }
                Err(error) => loaded.errors.push(error),
            }
        }
    } else {
        loaded
            .errors
            .push("parchment_ornaments.json unreadable".into());
    }
    if let Some(path) = rose_file {
        match load_rgba(&path) {
            Ok(image) => loaded.rose = Some(image),
            Err(error) => loaded.errors.push(error),
        }
    }
    match std::fs::read(data.join("map/coast_dist.png"))
        .map_err(|e| e.to_string())
        .and_then(|bytes| CoastRaster::from_png(&bytes, loaded.map_size, 2.0))
    {
        Ok(coast) => {
            loaded.decor = decor::place(
                &coast,
                Painted {
                    ships: loaded.ships.len(),
                    monsters: loaded.monsters.len(),
                },
            );
            loaded.coast = Some(coast);
        }
        Err(error) => loaded.errors.push(format!("coast_dist.png: {error}")),
    }
    for name in [
        "town",
        "ship",
        "serpent",
        "whale",
        "pennant",
        "pennant_edge",
    ] {
        if let Some((spec, image)) = glyphs::paint(name) {
            loaded.glyphs.push((name, spec, image));
        }
    }
    if let Some(units) = read_json(&data.join("map/settlement_markers.json"))
        .and_then(|m| m["banner"]["parchment_pennant_units"].as_f64())
    {
        loaded.pennant_units = units as f32;
    }
    if let Some(cues) = read_json(&data.join("map/stance_cues.json")) {
        for cue in ["self", "enemy", "friend"] {
            if let Some(c) = cues["border"][cue].as_str().and_then(|t| hex_color(t, 1.0)) {
                loaded.cue_border.insert(cue.into(), c);
            }
            let ring = &cues["army"]["ring"][cue];
            if let Some(c) = ring["color"]
                .as_str()
                .and_then(|t| hex_color(t, ring["alpha"].as_f64().unwrap_or(1.0) as f32))
            {
                loaded.cue_ring.insert(cue.into(), c);
            }
        }
    }
    loaded
}

fn painted(images: &mut Assets<Image>, ornament: LoadedOrnament) -> PaintedOrnament {
    let (image, size, anchor, height, faces_left) = ornament;
    PaintedOrnament {
        image: images.add(image),
        size,
        anchor,
        height,
        faces_left,
    }
}

fn finish_load(
    mut commands: Commands,
    mut load: ResMut<ParchmentLoad>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(slot) = load.pending.clone() else {
        return;
    };
    let Some(loaded) = slot.lock().ok().and_then(|mut g| g.take()) else {
        return;
    };
    load.pending = None;
    load.ready = true;
    load.map_size = loaded.map_size;
    load.roses = loaded.decor.roses.clone();
    load.errors = loaded.errors;
    for error in &load.errors {
        warn!("parchment: {error}");
    }
    load.coast_scale = loaded.coast.as_ref().map_or(2.0, |c| c.scale);
    load.coast = loaded
        .coast
        .as_ref()
        .map(|c| images.add(sea::coast_image(c)));
    load.painted_rose = loaded.rose.is_some();
    load.rose = loaded.rose.map(|i| images.add(i));
    let monsters = loaded
        .monsters
        .into_iter()
        .map(|o| painted(&mut images, o))
        .collect();
    let ships = loaded
        .ships
        .into_iter()
        .map(|o| painted(&mut images, o))
        .collect();
    let glyphs = loaded
        .glyphs
        .into_iter()
        .map(|(name, spec, image)| (name, (spec, images.add(image))))
        .collect();
    commands.insert_resource(OverlayAssets {
        glyphs,
        monsters,
        ships,
        decor: loaded.decor,
        pennant_units: loaded.pennant_units,
        cue_border: loaded.cue_border,
        cue_ring: loaded.cue_ring,
    });
}

fn ensure_sea_plane(
    mut commands: Commands,
    mut load: ResMut<ParchmentLoad>,
    existing: Query<(), With<ParchmentSea>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ParchmentSeaMaterial>>,
) {
    if !load.ready || !existing.is_empty() || load.material.is_some() {
        return;
    }
    let material = ParchmentSeaMaterial {
        params: sea::params(
            load.map_size,
            &load.roses,
            0.0,
            load.painted_rose,
            load.coast.is_some(),
            load.coast_scale,
        ),
        coast_dist: load.coast.clone(),
        rose: load.rose.clone(),
    };
    let size = load.map_size;
    load.material = Some(sea::spawn_plane(
        &mut commands,
        &mut meshes,
        &mut materials,
        size,
        material,
    ));
}

fn update_sea(
    zoom: Res<ZoomView>,
    load: Res<ParchmentLoad>,
    mut materials: ResMut<Assets<ParchmentSeaMaterial>>,
    mut planes: Query<&mut Visibility, With<ParchmentSea>>,
) {
    let weight = zoom.strategic;
    for mut visibility in &mut planes {
        let want = if weight > 0.001 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != want {
            *visibility = want;
        }
    }
    if let Some(handle) = &load.material
        && materials
            .get(handle)
            .is_some_and(|m| m.params.map.z != weight)
        && let Some(mut material) = materials.get_mut(handle)
    {
        material.params.map.z = weight;
    }
}

fn dump_parchment(world: &World) -> (String, Value) {
    let load = world.resource::<ParchmentLoad>();
    let stats = world.resource::<OverlayStats>();
    let assets = world.get_resource::<OverlayAssets>();
    let items: Vec<Value> = assets
        .map(|a| {
            a.decor
                .items
                .iter()
                .map(|i| {
                    json!({"kind": i.kind.name(), "px": [i.px.x, i.px.y],
                           "painted": i.painted.is_some(), "heading_right": i.heading_right})
                })
                .collect()
        })
        .unwrap_or_default();
    (
        "campaign_parchment".into(),
        json!({
            "ready": load.ready,
            "errors": load.errors,
            "map_size": [load.map_size.x, load.map_size.y],
            "roses": load.roses.iter().map(|(p, r)| json!([p.x, p.y, r])).collect::<Vec<_>>(),
            "painted_rose": load.painted_rose,
            "coast_raster": load.coast.is_some(),
            "sea_plane": load.material.is_some(),
            "sea_items": items,
            "glyphs": assets.map(|a| a.glyphs.len()).unwrap_or(0),
            "painted_monsters": assets.map(|a| a.monsters.len()).unwrap_or(0),
            "overlay": {
                "weight": stats.weight,
                "scale": stats.scale,
                "realms": stats.realms,
                "realms_total": stats.realms_total,
                "letters": stats.letters,
                "sea_shown": stats.sea_shown,
                "towns": stats.towns,
                "towns_shown": stats.towns_shown,
                "towns_occupied": stats.towns_occupied,
                "provinces_shown": stats.provinces_shown,
                "tokens_shown": stats.tokens_shown,
                "rebuilds": stats.rebuilds,
            },
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_data_places_roses_and_sirens() {
        let Ok(root) = crate::paths::find_root() else {
            return;
        };
        let loaded = load_all(&root);
        assert!(loaded.errors.is_empty(), "{:?}", loaded.errors);
        assert!(!loaded.decor.roses.is_empty() && loaded.decor.roses.len() <= 4);
        assert!(loaded.rose.is_some(), "painted rose");
        assert_eq!(loaded.monsters.len(), 1, "one siren in the catalogue");
        assert!(
            loaded.ships.is_empty(),
            "ships are drawn (`use.ship = drawn`)"
        );
        let ships = loaded
            .decor
            .items
            .iter()
            .filter(|i| i.kind == decor::SeaKind::Ship)
            .count();
        assert!(ships > 0 && ships <= 9);
        assert!((loaded.pennant_units - 1.1).abs() < 1e-6);
        assert!(loaded.cue_border.contains_key("enemy"));
    }
}
