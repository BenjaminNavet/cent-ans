//! Arms on the battle figures (DA1): one texture array per battle with the arms of each side's
//! faction, of its general's house and of the vassal houses whose bannerets follow the army
//! (`vassal_of` of `data/heraldry/houses.json`), plus the livery cross of the commons (`badges`).
//! Ports `_build_arms_atlas` / `_apply_arms` of game/scripts/battle/battle_soldiers.gd,
//! `chest_box` of battle_skinned.gd and game/scripts/ui/house_arms.gd (vassals, badge,
//! tinctures). The shader side (chest of the surcoat, shield faces) is in `skinned.wesl`.
//!
//! Atlas: `ARMS_SIZE`² RGBA8 sRGB layers with mipmaps; layer files are the 128² shields of
//! `assets/heraldry/<faction>.png` and `assets/heraldry/houses/<house>.png`, as Godot loads them.

use std::path::Path;

use bevy::{
    asset::RenderAssetUsages,
    image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    render::{
        RenderApp,
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
};
use sim_battle::{BattleSim, SideId};

use serde_json::json;

use super::figures::FigureArrays;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::battle::decor::ground::{mip_chain, mip_count};

/// Side of one layer (pixels), `create_from_images` of Godot at 128².
pub const ARMS_SIZE: u32 = 128;
/// Share of the noble figures that wear a banneret's arms instead of the lord's (shader default).
pub const VASSAL_SHARE: f32 = 0.3;

/// Livery cross of the commons of a faction (linear RGB).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Badge {
    pub cross: [f32; 3],
    pub patch: Option<[f32; 3]>,
}

/// Layers of one side (`-1` = none).
#[derive(Debug, Clone, Default)]
pub struct SideArms {
    pub faction: String,
    pub faction_layer: i32,
    pub lord: Option<String>,
    pub lord_layer: i32,
    pub vassal_layers: Vec<i32>,
    pub badge: Option<Badge>,
}

/// Arms atlas of the current battle.
#[derive(Resource, Default)]
pub struct BattleHeraldry {
    pub built: bool,
    pub image: Option<Handle<Image>>,
    /// Layer → faction id or house id.
    pub layer_ids: Vec<String>,
    /// Attacker, defender.
    pub sides: [SideArms; 2],
    /// Regiments given arms layers, chest arms (nobles) and a livery cross (commons).
    pub regiments: usize,
    pub chest_arms: usize,
    pub badges: usize,
    /// Arms given to each regiment, by unit id (dump).
    pub assigned: Vec<(u32, RegimentArms)>,
}

/// Atlas handle seen by the render world (bound by `skinned.rs`).
#[derive(Resource, Default, Clone)]
pub struct ArmsAtlasHandle(pub Option<Handle<Image>>);

impl ExtractResource<RenderApp> for ArmsAtlasHandle {
    type Source = Self;

    fn extract_resource(source: &Self) -> Self {
        source.clone()
    }
}

/// Bust of the rest pose where the surcoat carries the arms: centre, width/height, z range.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChestBox {
    pub center: [f32; 2],
    pub size: [f32; 2],
    pub depth: [f32; 2],
}

/// Arms uniforms of one regiment (`arms_layers`, `chest_*`, `badge_*` of the Godot shader).
#[derive(Debug, Clone, PartialEq)]
pub struct RegimentArms {
    /// x = lord (or faction), y/z/w = bannerets; `-1` = none (x = -1: no atlas).
    pub layers: [i32; 4],
    pub chest: Option<ChestBox>,
    pub chest_arms: bool,
    pub badge: Option<Badge>,
}

pub struct HeraldryPlugin;

impl Plugin for HeraldryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BattleHeraldry>()
            .init_resource::<ArmsAtlasHandle>()
            .add_plugins(ExtractResourcePlugin::<ArmsAtlasHandle>::default())
            .register_dump(dump_heraldry)
            .add_systems(OnExit(AppState::Battle), clear_heraldry);
    }
}

fn dump_heraldry(world: &World) -> (String, serde_json::Value) {
    let Some(h) = world.get_resource::<BattleHeraldry>() else {
        return ("battle_heraldry".into(), serde_json::Value::Null);
    };
    let sides: Vec<serde_json::Value> = h
        .sides
        .iter()
        .map(|s| {
            json!({
                "faction": s.faction, "faction_layer": s.faction_layer, "lord": s.lord,
                "lord_layer": s.lord_layer, "vassal_layers": s.vassal_layers,
                "badge": s.badge.map(|b| json!({"cross": b.cross, "patch": b.patch})),
            })
        })
        .collect();
    let regiments: Vec<serde_json::Value> = h
        .assigned
        .iter()
        .map(|(id, a)| {
            json!({
                "unit": id, "layers": a.layers, "chest_arms": a.chest_arms,
                "badge": a.badge.is_some(), "chest_box": a.chest.map(|c| json!({
                    "center": c.center, "size": c.size, "depth": c.depth})),
            })
        })
        .collect();
    (
        "battle_heraldry".into(),
        json!({
            "atlas": h.image.is_some().then(|| json!({
                "layers": h.layer_ids.len(), "size": [ARMS_SIZE, ARMS_SIZE],
                "mips": mip_count(ARMS_SIZE), "format": "rgba8_srgb_array"})),
            "layer_ids": h.layer_ids,
            "sides": sides,
            "regiments_with_arms": h.regiments,
            "chest_arms": h.chest_arms,
            "livery_crosses": h.badges,
            "regiments": regiments,
        }),
    )
}

fn clear_heraldry(mut heraldry: ResMut<BattleHeraldry>, mut handle: ResMut<ArmsAtlasHandle>) {
    *heraldry = BattleHeraldry::default();
    handle.0 = None;
}

/// One entry of `houses.json`.
#[derive(Debug, Clone)]
pub struct House {
    pub id: String,
    pub name: String,
    pub vassal_of: Vec<String>,
}

/// `houses.json`: houses sorted by id and the badges of the factions (tincture names).
#[derive(Debug, Clone, Default)]
pub struct HousesData {
    pub houses: Vec<House>,
    pub badges: std::collections::HashMap<String, (String, Option<String>)>,
}

impl HousesData {
    pub fn load(data_dir: &Path) -> Self {
        std::fs::read_to_string(data_dir.join("heraldry/houses.json"))
            .ok()
            .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
            .map(|value| Self::parse(&value))
            .unwrap_or_default()
    }

    pub fn parse(value: &serde_json::Value) -> Self {
        let text = |v: &serde_json::Value, key: &str| {
            v.get(key).and_then(|x| x.as_str()).map(str::to_owned)
        };
        let mut houses: Vec<House> = value
            .get("houses")
            .and_then(|h| h.as_array())
            .map(|list| {
                list.iter()
                    .filter_map(|h| {
                        Some(House {
                            id: text(h, "id")?,
                            name: text(h, "name").unwrap_or_default(),
                            vassal_of: h
                                .get("vassal_of")
                                .and_then(|v| v.as_array())
                                .map(|a| {
                                    a.iter()
                                        .filter_map(|f| f.as_str().map(str::to_owned))
                                        .collect()
                                })
                                .unwrap_or_default(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        houses.sort_by(|a, b| a.id.cmp(&b.id));
        let badges = value
            .get("badges")
            .and_then(|b| b.as_object())
            .map(|map| {
                map.iter()
                    .filter_map(|(faction, b)| {
                        Some((faction.clone(), (text(b, "cross")?, text(b, "patch"))))
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self { houses, badges }
    }

    /// House id of a house given by name or id (`HouseArms.id_of`).
    pub fn id_of(&self, house: &str) -> Option<&str> {
        self.houses
            .iter()
            .find(|h| h.id == house || h.name == house)
            .map(|h| h.id.as_str())
    }

    /// Houses whose bannerets serve in the armies of `faction`, except `exclude`
    /// (`HouseArms.vassals`), sorted by id.
    pub fn vassals(&self, faction: &str, exclude: Option<&str>) -> Vec<&str> {
        self.houses
            .iter()
            .filter(|h| Some(h.id.as_str()) != exclude && h.vassal_of.iter().any(|f| f == faction))
            .map(|h| h.id.as_str())
            .collect()
    }

    /// Livery cross of the commons of `faction` (`HouseArms.badge`), linear RGB.
    pub fn badge(&self, faction: &str) -> Option<Badge> {
        let (cross, patch) = self.badges.get(faction)?;
        Some(Badge {
            cross: srgb_to_linear(tincture(cross)),
            patch: patch.as_deref().map(|p| srgb_to_linear(tincture(p))),
        })
    }
}

/// Canonical tinctures (DA bible § 3.1, `HouseArms.tincture`), sRGB 0-1.
pub fn tincture(name: &str) -> [f32; 3] {
    let rgb: [u8; 3] = match name {
        "or" => [242, 194, 48],
        "argent" => [245, 241, 230],
        "gueules" => [176, 24, 43],
        "azur" => [31, 58, 147],
        "sable" => [26, 26, 26],
        "sinople" => [30, 120, 60],
        "pourpre" => [107, 42, 122],
        _ => [255, 255, 255],
    };
    rgb.map(|c| f32::from(c) / 255.0)
}

fn srgb_to_linear(rgb: [f32; 3]) -> [f32; 3] {
    let c = Color::srgb(rgb[0], rgb[1], rgb[2]).to_linear();
    [c.red, c.green, c.blue]
}

/// RGBA8 pixels of a shield file, resized to `ARMS_SIZE`² (nearest) when needed.
pub fn load_layer(path: &Path) -> Option<Vec<u8>> {
    let file = std::fs::File::open(path).ok()?;
    let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width as usize, info.height as usize);
    let channels = info.color_type.samples();
    let rgba: Vec<u8> = (0..w * h)
        .flat_map(|i| {
            let p = &buf[i * channels..(i + 1) * channels];
            match channels {
                1 => [p[0], p[0], p[0], 255],
                2 => [p[0], p[0], p[0], p[1]],
                3 => [p[0], p[1], p[2], 255],
                _ => [p[0], p[1], p[2], p[3]],
            }
        })
        .collect();
    let size = ARMS_SIZE as usize;
    if w == size && h == size {
        return Some(rgba);
    }
    if w == 0 || h == 0 {
        return None;
    }
    let mut out = vec![0; size * size * 4];
    for y in 0..size {
        for x in 0..size {
            let src = ((y * h / size) * w + x * w / size) * 4;
            out[(y * size + x) * 4..(y * size + x) * 4 + 4].copy_from_slice(&rgba[src..src + 4]);
        }
    }
    Some(out)
}

/// Layers of the battle: `(ids, pixels, sides)` for the two factions and generals' houses.
/// `load` returns the pixels of a faction (`fac_*`) or house id, `None` when it has no shield.
pub fn build_layers(
    houses: &HousesData,
    factions: [&str; 2],
    lords: [Option<&str>; 2],
    mut load: impl FnMut(&str) -> Option<Vec<u8>>,
) -> (Vec<String>, Vec<Vec<u8>>, [SideArms; 2]) {
    let mut ids: Vec<String> = Vec::new();
    let mut pixels: Vec<Vec<u8>> = Vec::new();
    let mut add = |key: &str, pixels: &mut Vec<Vec<u8>>| -> i32 {
        if let Some(i) = ids.iter().position(|k| k == key) {
            return i as i32;
        }
        match load(key) {
            Some(layer) => {
                ids.push(key.to_owned());
                pixels.push(layer);
                ids.len() as i32 - 1
            }
            None => -1,
        }
    };
    let mut sides: [SideArms; 2] = Default::default();
    for (index, side) in sides.iter_mut().enumerate() {
        let faction = factions[index];
        let faction_layer = add(faction, &mut pixels);
        let lord = lords[index]
            .and_then(|h| houses.id_of(h))
            .map(str::to_owned);
        let lord_layer = lord.as_deref().map_or(-1, |l| add(l, &mut pixels));
        let lord_layer = if lord_layer >= 0 {
            lord_layer
        } else {
            faction_layer
        };
        let mut vassal_layers = Vec::new();
        for house in houses.vassals(faction, lord.as_deref()) {
            let layer = add(house, &mut pixels);
            if layer >= 0 && layer != lord_layer {
                vassal_layers.push(layer);
            }
        }
        *side = SideArms {
            faction: faction.to_owned(),
            faction_layer,
            lord,
            lord_layer,
            vassal_layers,
            badge: houses.badge(faction),
        };
    }
    (ids, pixels, sides)
}

/// Texture array of `layers` (`ARMS_SIZE`² RGBA8 each) with box-filtered mipmaps.
pub fn atlas_image(layers: &[Vec<u8>]) -> Option<Image> {
    if layers.is_empty() {
        return None;
    }
    let size = ARMS_SIZE as usize;
    let mut data = Vec::with_capacity(layers.len() * size * size * 4 * 4 / 3 + 64);
    for layer in layers {
        data.extend_from_slice(&mip_chain(layer, size));
    }
    let mut image = Image::new(
        Extent3d {
            width: ARMS_SIZE,
            height: ARMS_SIZE,
            depth_or_array_layers: layers.len() as u32,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.mip_level_count = mip_count(ARMS_SIZE);
    image.texture_view_descriptor = Some(bevy::render::render_resource::TextureViewDescriptor {
        dimension: Some(bevy::render::render_resource::TextureViewDimension::D2Array),
        ..default()
    });
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        ..default()
    });
    Some(image)
}

impl BattleHeraldry {
    /// Builds the atlas of the battle once (factions and generals' houses of both sides).
    pub fn ensure_built(
        &mut self,
        sim: &BattleSim,
        images: &mut Assets<Image>,
        handle: &mut ArmsAtlasHandle,
    ) {
        if self.built {
            return;
        }
        self.built = true;
        let Ok(root) = crate::paths::find_root() else {
            return;
        };
        let data_dir = crate::paths::data_dir(&root);
        let asset_root = crate::paths::asset_root(&root);
        let houses = HousesData::load(&data_dir);
        let factions =
            [SideId::Attacker, SideId::Defender].map(|s| sim.setup().side(s).faction.clone());
        let lords = [SideId::Attacker, SideId::Defender]
            .map(|s| super::banners::general_house(sim, &data_dir, s));
        let (ids, pixels, sides) = build_layers(
            &houses,
            [&factions[0], &factions[1]],
            [lords[0].as_deref(), lords[1].as_deref()],
            |key| {
                let file = if key.starts_with("fac_") {
                    asset_root.join(format!("heraldry/{key}.png"))
                } else {
                    asset_root.join(format!("heraldry/houses/{key}.png"))
                };
                load_layer(&file)
            },
        );
        self.layer_ids = ids;
        self.sides = sides;
        if let Some(image) = atlas_image(&pixels) {
            let image = images.add(image);
            handle.0 = Some(image.clone());
            self.image = Some(image);
        }
    }

    /// Arms of a regiment of `side` (`_apply_arms`): nobles wear the lord's arms (three
    /// bannerets drawn per regiment from the vassal pool), commons the faction's arms on their
    /// shields and the livery cross on the chest.
    pub fn regiment(
        &mut self,
        side_index: usize,
        unit_id: u32,
        noble: bool,
        chest: Option<ChestBox>,
    ) -> RegimentArms {
        let side = &self.sides[side_index.min(1)];
        if self.image.is_none() || side.faction_layer < 0 {
            return RegimentArms {
                layers: [-1; 4],
                chest: None,
                chest_arms: false,
                badge: None,
            };
        }
        self.regiments += 1;
        let mut layers = [side.faction_layer, -1, -1, -1];
        let mut badge = None;
        let chest_arms = noble && chest.is_some();
        if noble {
            layers[0] = side.lord_layer;
            let pool = shuffled(&side.vassal_layers, unit_id);
            for (slot, layer) in pool.iter().take(3).enumerate() {
                layers[slot + 1] = *layer;
            }
            if chest_arms {
                self.chest_arms += 1;
            }
        } else if chest.is_some() {
            badge = side.badge;
            if badge.is_some() {
                self.badges += 1;
            }
        }
        let arms = RegimentArms {
            layers,
            chest,
            chest_arms,
            badge,
        };
        self.assigned.retain(|(id, _)| *id != unit_id);
        self.assigned.push((unit_id, arms.clone()));
        arms
    }
}

/// Fisher-Yates shuffle seeded by the regiment (`hash("da1/%d")` in Godot; the draw differs,
/// the rule is the same: three bannerets per regiment when the pool has more than three).
fn shuffled(pool: &[i32], unit_id: u32) -> Vec<i32> {
    let mut out = pool.to_vec();
    if out.len() <= 3 {
        return out;
    }
    let mut state = u64::from(unit_id).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xDA1;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for i in (1..out.len()).rev() {
        let j = (next() % (i as u64 + 1)) as usize;
        out.swap(i, j);
    }
    out
}

/// Bust box of a figure (`BattleSkinned.chest_box`): livery vertices (code 0) whose dominant
/// bone is `Torso` or `Chest`; `None` when the bust has no livery (full plate).
pub fn chest_box(arrays: &FigureArrays, bones: &[String]) -> Option<ChestBox> {
    let torso: Vec<usize> = bones
        .iter()
        .enumerate()
        .filter(|(_, name)| {
            let name = name.trim_start_matches("R:");
            name == "Torso" || name == "Chest"
        })
        .map(|(i, _)| i)
        .collect();
    if torso.is_empty() {
        return None;
    }
    let mut low = [f32::INFINITY; 3];
    let mut high = [f32::NEG_INFINITY; 3];
    for (i, p) in arrays.positions.iter().enumerate() {
        if (arrays.colors[i][3] + 0.5) as i32 != 0 {
            continue;
        }
        let w = arrays.weights[i];
        let mut best = 0;
        for k in 1..4 {
            if w[k] > w[best] {
                best = k;
            }
        }
        if !torso.contains(&((arrays.bones[i][best] + 0.5) as usize)) {
            continue;
        }
        for axis in 0..3 {
            low[axis] = low[axis].min(p[axis]);
            high[axis] = high[axis].max(p[axis]);
        }
    }
    if low[0] >= high[0] {
        return None;
    }
    let width = (high[0] - low[0]) + 0.06;
    Some(ChestBox {
        center: [(low[0] + high[0]) * 0.5, (low[1] + high[1]) * 0.5],
        size: [width, width * 1.25],
        depth: [low[2] - 0.06, high[2] + 0.06],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn houses() -> HousesData {
        HousesData::parse(&serde_json::json!({
            "badges": {"fac_france": {"cross": "argent", "patch": null},
                       "fac_england": {"cross": "gueules", "patch": "argent"}},
            "houses": [
                {"id": "valois", "name": "Valois", "vassal_of": []},
                {"id": "b", "name": "B", "vassal_of": ["fac_france"]},
                {"id": "a", "name": "A", "vassal_of": ["fac_france"]},
                {"id": "c", "name": "C", "vassal_of": ["fac_france", "fac_england"]},
                {"id": "d", "name": "D", "vassal_of": ["fac_france"]},
                {"id": "nopng", "name": "N", "vassal_of": ["fac_france"]},
            ]
        }))
    }

    #[test]
    fn layers_follow_the_godot_order() {
        let data = houses();
        let (ids, pixels, sides) = build_layers(
            &data,
            ["fac_france", "fac_england"],
            [Some("Valois"), None],
            |key| (key != "nopng").then(|| vec![0; 4]),
        );
        assert_eq!(
            ids,
            ["fac_france", "valois", "a", "b", "c", "d", "fac_england"]
        );
        assert_eq!(pixels.len(), ids.len());
        assert_eq!(sides[0].faction_layer, 0);
        assert_eq!(sides[0].lord_layer, 1);
        assert_eq!(sides[0].vassal_layers, [2, 3, 4, 5]);
        // No general's house: the lord wears the faction arms; vassal `c` already in the atlas.
        assert_eq!(sides[1].lord_layer, sides[1].faction_layer);
        assert_eq!(sides[1].vassal_layers, [4]);
        assert_eq!(sides[0].badge.unwrap().patch, None);
        assert!(sides[1].badge.unwrap().patch.is_some());
    }

    #[test]
    fn nobles_take_three_bannerets_commons_the_cross() {
        let data = houses();
        let (ids, pixels, sides) = build_layers(
            &data,
            ["fac_france", "fac_england"],
            [Some("valois"), None],
            |_| Some(vec![0; 4]),
        );
        let mut heraldry = BattleHeraldry {
            built: true,
            image: Some(Handle::default()),
            layer_ids: ids,
            sides,
            ..default()
        };
        assert_eq!(pixels.len(), 7);
        let chest = Some(ChestBox {
            center: [0.0, 1.2],
            size: [0.3, 0.375],
            depth: [-0.1, 0.2],
        });
        let noble = heraldry.regiment(0, 7, true, chest);
        assert_eq!(noble.layers[0], 1);
        assert!(noble.layers[1..].iter().all(|l| (2..=6).contains(l)));
        assert!(noble.chest_arms && noble.badge.is_none());
        let again = heraldry.regiment(0, 7, true, chest);
        assert_eq!(
            again.layers, noble.layers,
            "same draw for the same regiment"
        );
        let common = heraldry.regiment(0, 8, false, chest);
        assert_eq!(common.layers, [0, -1, -1, -1]);
        assert!(!common.chest_arms && common.badge.is_some());
        let plate = heraldry.regiment(0, 9, true, None);
        assert!(!plate.chest_arms);
        assert_eq!(
            (heraldry.regiments, heraldry.chest_arms, heraldry.badges),
            (4, 2, 1)
        );
    }

    #[test]
    fn tinctures_match_house_arms() {
        assert_eq!(
            tincture("gueules"),
            [176.0 / 255.0, 24.0 / 255.0, 43.0 / 255.0]
        );
        assert_eq!(tincture("?"), [1.0; 3]);
    }

    #[test]
    fn bundled_shields_load_at_128() {
        let root = crate::paths::find_root().expect("root");
        let assets = crate::paths::asset_root(&root);
        let layer = load_layer(&assets.join("heraldry/fac_france.png")).expect("france");
        assert_eq!(layer.len(), (ARMS_SIZE * ARMS_SIZE * 4) as usize);
        let data = HousesData::load(&crate::paths::data_dir(&root));
        assert!(data.houses.len() > 100, "{}", data.houses.len());
        assert!(data.badge("fac_france").is_some());
        let image = atlas_image(&[layer.clone(), layer]).expect("atlas");
        assert_eq!(image.texture_descriptor.size.depth_or_array_layers, 2);
        assert_eq!(image.texture_descriptor.mip_level_count, 8);
    }
}
