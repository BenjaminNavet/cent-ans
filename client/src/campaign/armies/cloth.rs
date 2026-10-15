//! Cloth of the campaign army standards: which banner an army flies, the cloth mesh and the wave
//! material. Port of `ArmyMarker.standard_for` / `_apply_standard` (army_marker.gd) and of
//! `map_banner.gdshader`.
//!
//! Lookup order (first existing file wins): royal banner of the king's own army
//! (`banners/oriflamme.png` France, `banners/st_george.png` England; the commander is the ruler of
//! the faction), house banner of the commander (`banners/houses/<house>_banner.png`, only for a
//! house that belongs to or is vassal of the army's faction: a new step over Godot, which flies
//! the faction banner), faction banner (`banners/<faction>_banner.png`), shield of the faction
//! (`heraldry/<faction>.png`, cropped), plain faction colour. A texture wider than tall is a
//! pennant (mode 1), otherwise a hung banner (mode 2), as in Godot.

use std::path::Path;

use bevy::asset::uuid_handle;
use bevy::mesh::Indices;
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{AsBindGroup, PrimitiveTopology, ShaderType};
use bevy::shader::{Shader, ShaderRef};

const FLAG_SHADER: Handle<Shader> = uuid_handle!("6f0b1a52-3c1e-4d0a-9d31-0a5f7c1e00f1");

pub const BANNERS_DIR: &str = "heraldry/banners/";
/// Height of the pole carrying a hung banner (`POLE_TALL`).
pub const POLE_TALL: f32 = 11.0;

/// How the cloth is drawn (`mode` of `map_banner.gdshader`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClothMode {
    /// Flag with the shield cropped from `heraldry/<faction>.png`, or the plain colour.
    Flag,
    /// Pennant, whole texture.
    Pennant,
    /// Vertical banner hung from a crossbar, whole texture.
    Banner,
}

impl ClothMode {
    pub fn shader_value(self) -> f32 {
        match self {
            ClothMode::Flag => 0.0,
            ClothMode::Pennant => 1.0,
            ClothMode::Banner => 2.0,
        }
    }
}

/// Where the choice came from, for the dump.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClothSource {
    Royal,
    House,
    FactionBanner,
    Shield,
    PlainColour,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClothChoice {
    pub mode: ClothMode,
    /// Texture file relative to the asset root, `None` for the plain colour.
    pub file: Option<String>,
    pub source: ClothSource,
    /// Height over width of the texture (pennant mesh).
    pub aspect: f32,
}

impl ClothChoice {
    /// Banner id shown by the dump: the file stem, `"plain"` without texture.
    pub fn id(&self) -> String {
        self.file.as_deref().map_or("plain".to_owned(), |f| {
            Path::new(f)
                .file_stem()
                .map_or(f.to_owned(), |s| s.to_string_lossy().into_owned())
        })
    }
}

/// What the choice depends on.
#[derive(Debug, Clone, Copy)]
pub struct ClothQuery<'a> {
    pub faction: &'a str,
    /// The commander is the ruler of the faction.
    pub royal: bool,
    /// House id of the commander when it belongs to / is vassal of the faction.
    pub house: Option<&'a str>,
}

/// Royal banner file of a faction (`ROYAL_STANDARDS`).
pub fn royal_banner(faction: &str) -> Option<&'static str> {
    match faction {
        "fac_france" => Some("oriflamme"),
        "fac_england" => Some("st_george"),
        _ => None,
    }
}

/// Candidate banner files in order, with their source.
pub fn candidates(query: &ClothQuery) -> Vec<(String, ClothSource)> {
    let mut out = Vec::new();
    if query.royal
        && let Some(special) = royal_banner(query.faction)
    {
        out.push((format!("{BANNERS_DIR}{special}.png"), ClothSource::Royal));
    }
    if let Some(house) = query.house.filter(|h| !h.is_empty()) {
        out.push((
            format!("{BANNERS_DIR}houses/{house}_banner.png"),
            ClothSource::House,
        ));
    }
    if !query.faction.is_empty() {
        out.push((
            format!("{BANNERS_DIR}{}_banner.png", query.faction),
            ClothSource::FactionBanner,
        ));
    }
    out
}

/// The cloth of an army. `size` gives the pixel size of an existing file (`None` when absent).
pub fn choose(query: &ClothQuery, size: impl Fn(&str) -> Option<(u32, u32)>) -> ClothChoice {
    for (file, source) in candidates(query) {
        if let Some((w, h)) = size(&file) {
            let wide = w as f32 > h as f32 * 1.2;
            return ClothChoice {
                mode: if wide {
                    ClothMode::Pennant
                } else {
                    ClothMode::Banner
                },
                file: Some(file),
                source,
                aspect: h as f32 / (w.max(1) as f32),
            };
        }
    }
    let shield = format!("heraldry/{}.png", query.faction);
    if !query.faction.is_empty() && size(&shield).is_some() {
        return ClothChoice {
            mode: ClothMode::Flag,
            file: Some(shield),
            source: ClothSource::Shield,
            aspect: 1.0,
        };
    }
    ClothChoice {
        mode: ClothMode::Flag,
        file: None,
        source: ClothSource::PlainColour,
        aspect: 1.0,
    }
}

/// Houses of `data/heraldry/houses.json` and the house of commanders (`characters/<id>.json`).
#[derive(Debug, Default)]
pub struct HouseBook {
    data_dir: std::path::PathBuf,
    /// id, name, factions and vassal_of of each house.
    houses: Vec<(String, String, Vec<String>)>,
    /// Character id to house name, read once.
    characters: std::collections::HashMap<String, Option<String>>,
}

impl HouseBook {
    pub fn load(data_dir: &Path) -> Self {
        let value = std::fs::read_to_string(data_dir.join("heraldry/houses.json"))
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok());
        let strings = |v: &serde_json::Value, key: &str| -> Vec<String> {
            v[key]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|s| s.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default()
        };
        let houses = value
            .as_ref()
            .and_then(|v| v["houses"].as_array())
            .map(|list| {
                list.iter()
                    .filter_map(|h| {
                        let mut factions = strings(h, "factions");
                        factions.extend(strings(h, "vassal_of"));
                        Some((
                            h["id"].as_str()?.to_owned(),
                            h["name"].as_str()?.to_owned(),
                            factions,
                        ))
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self {
            data_dir: data_dir.to_path_buf(),
            houses,
            characters: Default::default(),
        }
    }

    /// House id of `character` when that house belongs to or is vassal of `faction`.
    pub fn house_of(&mut self, character: &str, faction: &str) -> Option<String> {
        let dir = &self.data_dir;
        let name = self
            .characters
            .entry(character.to_owned())
            .or_insert_with(|| {
                std::fs::read_to_string(dir.join(format!("characters/{character}.json")))
                    .ok()
                    .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
                    .and_then(|v| v["house"].as_str().map(str::to_owned))
            })
            .clone()?;
        self.houses
            .iter()
            .find(|(id, house_name, _)| *id == name || *house_name == name)
            .filter(|(_, _, factions)| factions.iter().any(|f| f == faction))
            .map(|(id, _, _)| id.clone())
    }
}

/// Pixel size of a PNG from its header, `None` if the file is missing or not a PNG.
pub fn png_size(path: &Path) -> Option<(u32, u32)> {
    use std::io::Read;
    let mut header = [0u8; 24];
    std::fs::File::open(path)
        .ok()?
        .read_exact(&mut header)
        .ok()?;
    if &header[1..4] != b"PNG" {
        return None;
    }
    let be =
        |i: usize| u32::from_be_bytes([header[i], header[i + 1], header[i + 2], header[i + 3]]);
    Some((be(16), be(20)))
}

/// Cloth rest geometry (`_apply_standard`): size, grid, offset of the quad from the pole.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClothShape {
    pub size: Vec2,
    pub segments: UVec2,
    pub offset: Vec2,
}

pub fn shape(mode: ClothMode, aspect: f32) -> ClothShape {
    match mode {
        ClothMode::Banner => ClothShape {
            size: Vec2::new(3.0, 6.0),
            segments: UVec2::new(4, 11),
            offset: Vec2::new(0.0, -3.0),
        },
        ClothMode::Pennant => ClothShape {
            size: Vec2::new(4.2, 4.2 * aspect),
            segments: UVec2::new(13, 2),
            offset: Vec2::new(2.1, -2.1 * aspect),
        },
        ClothMode::Flag => ClothShape {
            size: Vec2::new(3.4, 2.3),
            segments: UVec2::new(11, 3),
            offset: Vec2::new(1.7, -1.15),
        },
    }
}

/// Subdivided quad, UV (0,0) top left, centred on `offset` (a Godot `QuadMesh` with `center_offset`).
pub fn cloth_mesh(shape: &ClothShape) -> Mesh {
    let (nx, ny) = (shape.segments.x as usize, shape.segments.y as usize);
    let mut positions = Vec::with_capacity((nx + 1) * (ny + 1));
    let mut uvs = Vec::with_capacity(positions.capacity());
    for j in 0..=ny {
        for i in 0..=nx {
            let u = i as f32 / nx as f32;
            let v = j as f32 / ny as f32;
            positions.push([
                shape.offset.x + (u - 0.5) * shape.size.x,
                shape.offset.y + (0.5 - v) * shape.size.y,
                0.0,
            ]);
            uvs.push([u, v]);
        }
    }
    let mut indices = Vec::with_capacity(nx * ny * 6);
    let row = (nx + 1) as u32;
    for j in 0..ny as u32 {
        for i in 0..nx as u32 {
            let a = j * row + i;
            indices.extend_from_slice(&[a, a + row, a + 1, a + 1, a + row, a + row + 1]);
        }
    }
    let normals = vec![[0.0, 0.0, 1.0]; positions.len()];
    Mesh::new(
        PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}

#[derive(ShaderType, Debug, Clone, Copy)]
pub struct FlagParams {
    pub faction_color: Vec4,
    pub crop: Vec4,
    pub pole_gust: Vec4,
    pub wave: Vec4,
    pub flags: Vec4,
}

/// Wave material of a standard (one per army: pole direction and gust change every frame).
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct FlagMaterial {
    #[uniform(0)]
    pub params: FlagParams,
    #[texture(1)]
    #[sampler(2)]
    pub heraldry: Option<Handle<Image>>,
}

impl FlagMaterial {
    pub fn new(color: [f32; 3], choice: &ClothChoice, heraldry: Option<Handle<Image>>) -> Self {
        let linear = Color::srgb(color[0], color[1], color[2]).to_linear();
        Self {
            params: FlagParams {
                faction_color: Vec4::new(linear.red, linear.green, linear.blue, 1.0),
                crop: Vec4::new(0.17, 0.12, 0.66, 0.5),
                pole_gust: Vec4::new(0.0, 1.0, 0.0, 0.0),
                wave: Vec4::new(0.22, 3.2, 0.18, 0.0),
                flags: Vec4::new(
                    choice.mode.shader_value(),
                    if heraldry.is_some() { 1.0 } else { 0.0 },
                    0.0,
                    0.0,
                ),
            },
            heraldry,
        }
    }
}

impl Material for FlagMaterial {
    fn vertex_shader() -> ShaderRef {
        FLAG_SHADER.into()
    }
    fn fragment_shader() -> ShaderRef {
        FLAG_SHADER.into()
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

/// Registers the embedded shader (no-op without the render plugin, e.g. in tests).
pub fn register_shader(app: &mut App) {
    let Some(mut shaders) = app.world_mut().get_resource_mut::<Assets<Shader>>() else {
        return;
    };
    if let Err(error) = shaders.insert(
        FLAG_SHADER.id(),
        Shader::from_wesl(include_str!("shaders/flag.wesl"), "armies/flag.wesl"),
    ) {
        error!("armies: flag shader: {error:?}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query<'a>(faction: &'a str, royal: bool, house: Option<&'a str>) -> ClothQuery<'a> {
        ClothQuery {
            faction,
            royal,
            house,
        }
    }

    fn with(files: &'static [(&'static str, (u32, u32))]) -> impl Fn(&str) -> Option<(u32, u32)> {
        move |f| files.iter().find(|(n, _)| *n == f).map(|(_, s)| *s)
    }

    #[test]
    fn royal_army_flies_the_royal_banner_first() {
        let files = with(&[
            ("heraldry/banners/oriflamme.png", (256, 512)),
            ("heraldry/banners/fac_france_banner.png", (256, 512)),
        ]);
        let c = choose(&query("fac_france", true, None), &files);
        assert_eq!(c.source, ClothSource::Royal);
        assert_eq!(c.id(), "oriflamme");
        // A lord of the same faction flies the faction banner.
        let c = choose(&query("fac_france", false, None), &files);
        assert_eq!(c.source, ClothSource::FactionBanner);
        assert_eq!(c.mode, ClothMode::Banner);
    }

    #[test]
    fn house_banner_comes_before_the_faction_one() {
        let files = with(&[
            ("heraldry/banners/houses/armagnac_banner.png", (256, 512)),
            ("heraldry/banners/fac_france_banner.png", (256, 512)),
        ]);
        let c = choose(&query("fac_france", false, Some("armagnac")), &files);
        assert_eq!(c.source, ClothSource::House);
        assert_eq!(c.id(), "armagnac_banner");
        // A house without a file falls back to the faction.
        let c = choose(&query("fac_france", false, Some("nobody")), &files);
        assert_eq!(c.source, ClothSource::FactionBanner);
        // The king of England has no oriflamme: royal only for the listed factions.
        assert!(royal_banner("fac_burgundy").is_none());
    }

    #[test]
    fn shield_then_plain_colour_as_fallbacks() {
        let files = with(&[("heraldry/fac_x.png", (512, 512))]);
        let c = choose(&query("fac_x", false, None), &files);
        assert_eq!((c.source, c.mode), (ClothSource::Shield, ClothMode::Flag));
        let c = choose(&query("fac_y", false, None), &files);
        assert_eq!(c.source, ClothSource::PlainColour);
        assert!(c.file.is_none());
        assert_eq!(c.id(), "plain");
    }

    #[test]
    fn wide_texture_is_a_pennant() {
        let files = with(&[("heraldry/banners/fac_x_banner.png", (512, 128))]);
        let c = choose(&query("fac_x", false, None), &files);
        assert_eq!(c.mode, ClothMode::Pennant);
        assert!((c.aspect - 0.25).abs() < 1e-6);
        let shape = shape(c.mode, c.aspect);
        assert!((shape.size.y - 1.05).abs() < 1e-5);
    }

    #[test]
    fn mesh_has_the_grid_and_the_uv_corners() {
        let s = shape(ClothMode::Banner, 2.0);
        let mesh = cloth_mesh(&s);
        assert_eq!(mesh.count_vertices(), 5 * 12);
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(p)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("positions");
        };
        // Top left of the banner: half a cloth left of the pole, top edge on the crossbar (y 0).
        assert!((p[0][0] + 1.5).abs() < 1e-5 && p[0][1].abs() < 1e-5);
    }

    #[test]
    fn houses_follow_the_commander_and_his_faction() {
        let root = crate::paths::find_root().expect("root");
        let mut book = HouseBook::load(&crate::paths::data_dir(&root));
        // The ruler of France is a Valois.
        assert_eq!(
            book.house_of("chr_philippe_vi", "fac_france").as_deref(),
            Some("valois")
        );
        // Another faction does not fly that house.
        assert_eq!(book.house_of("chr_philippe_vi", "fac_england"), None);
        assert_eq!(book.house_of("chr_nobody", "fac_france"), None);
    }

    #[test]
    fn shipped_assets_resolve() {
        let root = crate::paths::find_root().expect("root");
        let assets = crate::paths::asset_root(&root);
        let size = |f: &str| png_size(&assets.join(f));
        let c = choose(&query("fac_france", true, None), size);
        assert_eq!(c.id(), "oriflamme", "{c:?}");
        let c = choose(&query("fac_france", false, None), size);
        assert_eq!(c.id(), "fac_france_banner");
        assert_eq!(c.mode, ClothMode::Banner);
    }
}
