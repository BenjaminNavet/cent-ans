//! GDExtension entry point: exposes the Rust simulation to Godot.
//!
//! Only plain values cross the boundary (integers, strings, packed arrays);
//! Godot never holds a pointer into the simulation.

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use data_model::{GameData, HistoricalDate};
use godot::classes::RefCounted;
use godot::prelude::*;

mod battle_replay;
mod battle_sim;
mod campaign_sim;
mod campaign_sim_agents;
mod campaign_sim_ai_replay;
mod campaign_sim_difficulty;
mod campaign_sim_diplomacy;
mod campaign_sim_dp2;
mod campaign_sim_edicts;
mod campaign_sim_events;
mod campaign_sim_family;
mod campaign_sim_h5h6;
mod campaign_sim_map_lens;
mod campaign_sim_movement;
mod campaign_sim_provinces;
mod campaign_sim_retinue;
mod campaign_sim_settlements;
mod campaign_sim_siege;
mod campaign_sim_table;
mod campaign_sim_tech;
mod campaign_sim_trade;
mod campaign_sim_treaty;
mod campaign_sim_turn;
mod campaign_sim_victory;
mod campaign_sim_vision;
mod campaign_sim_weather;
mod convert;
mod data_store_rules;
mod historical_battles;
mod naval_sim;
mod relief_decoder;
mod relief_lod_bridge;
mod turn_job;
mod vegetation_scatter;

pub use battle_sim::BattleSim;
pub use campaign_sim::CampaignSim;
pub use naval_sim::NavalBattleSim;
pub use relief_decoder::ReliefDecoder;
pub use relief_lod_bridge::ReliefLod;
pub use vegetation_scatter::VegetationScatter;

struct CentAnsExtension;

#[gdextension]
unsafe impl ExtensionLibrary for CentAnsExtension {}

/// Godot-facing read-only view of the game data loaded from `data/`.
///
/// Every accessor returns plain Godot values (`Dictionary`, packed arrays);
/// unknown ids give an empty `Dictionary`.
#[derive(GodotClass)]
#[class(base = RefCounted)]
pub struct GameDataStore {
    /// Shared with `CampaignSim` (one load per data folder).
    data: Option<std::sync::Arc<GameData>>,
    warnings: Vec<String>,
    last_image_size: Vector2i,
    base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for GameDataStore {
    fn init(base: Base<RefCounted>) -> Self {
        GameDataStore {
            data: None,
            warnings: Vec::new(),
            last_image_size: Vector2i::ZERO,
            base,
        }
    }
}

#[godot_api]
impl GameDataStore {
    /// Loads every entity folder under `data_dir`. Returns `false` (and logs the
    /// error) if any file is invalid or a reference is dangling; warnings about
    /// provinces still missing from the map are kept for `get_warnings`.
    #[func]
    fn load(&mut self, data_dir: GString) -> bool {
        let root = PathBuf::from(data_dir.to_string());
        match campaign_sim::load_shared_data(&root) {
            Ok((data, warnings)) => {
                self.warnings = warnings.to_vec();
                self.data = Some(data);
                true
            }
            Err(error) => {
                godot_error!("GameDataStore.load({}) failed: {error}", root.display());
                self.data = None;
                self.warnings.clear();
                false
            }
        }
    }

    /// Whether `load` succeeded.
    #[func]
    fn is_loaded(&self) -> bool {
        self.data.is_some()
    }

    /// Non-fatal problems reported by the last successful `load`.
    #[func]
    fn get_warnings(&self) -> PackedStringArray {
        self.warnings.iter().map(GString::from).collect()
    }

    /// Sorted province ids.
    #[func]
    fn get_province_ids(&self) -> PackedStringArray {
        self.data
            .as_ref()
            .map(|data| ids_of(data.provinces.keys()))
            .unwrap_or_default()
    }

    /// Province summary, or an empty dictionary for an unknown id.
    #[func]
    fn get_province(&self, id: GString) -> VarDictionary {
        let Some(data) = &self.data else {
            return VarDictionary::new();
        };
        let Some(province) = data.provinces.get(id.to_string().as_str()) else {
            return VarDictionary::new();
        };
        let owner = data.factions.get(&province.owner);
        let mut population = VarDictionary::new();
        for (class, entry) in province.population.classes.iter() {
            population.set(class.key(), entry.count as i64);
        }
        let mut dict = vdict! {
            "id" => province.id.as_str(),
            "display_name" => province.name.display.as_str(),
            "local_name" => province.name.local.as_deref().unwrap_or(&province.name.display),
            "region" => province.region.as_str(),
            "terrain" => province.terrain.key(),
            "coastal" => province.coastal,
            "port" => province.has_port(),
            "capital" => province.capital_city.name.display.as_str(),
            "owner" => province.owner.as_str(),
            "owner_display_name" => owner.map_or("", |faction| faction.short_or_display_name()),
            "overlord" => province.overlord.as_ref().map_or("", |id| id.as_str()),
            "holder" => province.holder.as_ref().map_or("", |id| id.as_str()),
            "population" => &population,
            "population_total" => province.population.classes.total() as i64,
            "neighbors" =>&ids_of(sim_campaign::movement::land_neighbors(data, &province.id).iter()),
        };
        if let Some(geometry) = data.province_geometry.get(&province.id) {
            dict.set("centroid", vector2(geometry.centroid));
            dict.set("capital_px", vector2(geometry.capital_px));
        }
        dict
    }

    /// Sorted faction ids.
    #[func]
    fn get_faction_ids(&self) -> PackedStringArray {
        self.data
            .as_ref()
            .map(|data| ids_of(data.factions.keys()))
            .unwrap_or_default()
    }

    /// Faction summary, or an empty dictionary for an unknown id.
    #[func]
    fn get_faction(&self, id: GString) -> VarDictionary {
        let Some(faction) = self
            .data
            .as_ref()
            .and_then(|data| data.factions.get(id.to_string().as_str()))
        else {
            return VarDictionary::new();
        };
        let heraldry = &faction.heraldry;
        vdict! {
            "id" => faction.id.as_str(),
            "name" => faction.name.display.as_str(),
            "short_name" => faction.short_or_display_name(),
            "playable" => faction.playable,
            "color" => html_color(&heraldry.primary_color),
            "secondary_color" => heraldry
                .secondary_color
                .as_deref()
                .map_or(Color::WHITE, html_color),
            "blazon" => heraldry.blazon.as_str(),
            "capital" => faction.capital.as_str(),
            "ruler" => faction.ruler.as_ref().map_or("", |id| id.as_str()),
            "victory_summary" => faction.victory.as_ref().and_then(|v| v.summary.as_deref()).unwrap_or(""),
            "victory_end_year" => faction.victory.as_ref().map_or(0, |v| i64::from(v.end_year)),
        }
    }

    /// Sorted character ids.
    #[func]
    fn get_character_ids(&self) -> PackedStringArray {
        self.data
            .as_ref()
            .map(|data| ids_of(data.characters.keys()))
            .unwrap_or_default()
    }

    /// Character summary, or an empty dictionary for an unknown id.
    #[func]
    fn get_character(&self, id: GString) -> VarDictionary {
        let Some(character) = self
            .data
            .as_ref()
            .and_then(|data| data.characters.get(id.to_string().as_str()))
        else {
            return VarDictionary::new();
        };
        let titles: PackedStringArray = character
            .titles
            .iter()
            .map(|title| GString::from(&title.title))
            .collect();
        vdict! {
            "id" => character.id.as_str(),
            "name" => character.name.display.as_str(),
            "epithet" => character.epithet.as_deref().unwrap_or(""),
            "birth" => &date_dict(Some(&character.birth)),
            "death" => &date_dict(character.death.as_ref()),
            "faction" => character.faction.as_str(),
            "role" => format!("{:?}", character.role).to_lowercase(),
            "titles" => &titles,
            "skills" => &vdict! {
                "command" => i64::from(character.skills.command),
                "governance" => i64::from(character.skills.governance),
                "court" => i64::from(character.skills.court),
            },
            "starting_location" => character
                .starting_location
                .as_ref()
                .map_or("", |id| id.as_str()),
        }
    }

    /// Trait definition `{id, name, category, description, opposites[]}`
    /// (spec M4 § 3), or an empty dictionary for an unknown id.
    #[func]
    fn get_trait(&self, id: GString) -> VarDictionary {
        let Some(definition) = self
            .data
            .as_ref()
            .and_then(|data| data.traits.get(id.to_string().as_str()))
        else {
            return VarDictionary::new();
        };
        vdict! {
            "id" => definition.id.as_str(),
            "name" => definition.name.display.as_str(),
            "category" => format!("{:?}", definition.category).to_lowercase(),
            "description" => definition.description.as_str(),
            "opposites" => &ids_of(definition.opposites.iter()),
        }
    }

    /// Skill definition `{id, name, branch, tier, prerequisites[], cost,
    /// description}` (spec M4 § 3), or an empty dictionary for an unknown id.
    #[func]
    fn get_skill(&self, id: GString) -> VarDictionary {
        let Some(skill) = self
            .data
            .as_ref()
            .and_then(|data| data.skills.get(id.to_string().as_str()))
        else {
            return VarDictionary::new();
        };
        vdict! {
            "id" => skill.id.as_str(),
            "name" => skill.name.display.as_str(),
            "branch" => format!("{:?}", skill.branch).to_lowercase(),
            "tier" => i64::from(skill.tier),
            "prerequisites" => &ids_of(skill.prerequisites.iter()),
            "cost" => i64::from(skill.cost),
            "description" => skill.description.as_str(),
        }
    }

    /// Primary heraldry colour of each province's owner, in the order of
    /// `province_ids`. Unknown provinces or factions give magenta so that a
    /// missing colour is visible on the map.
    #[func]
    fn get_province_owner_colors(&self, province_ids: PackedStringArray) -> PackedColorArray {
        let Some(data) = &self.data else {
            return PackedColorArray::new();
        };
        province_ids
            .as_slice()
            .iter()
            .map(|id| {
                data.provinces
                    .get(id.to_string().as_str())
                    .and_then(|province| data.factions.get(&province.owner))
                    .map_or(Color::MAGENTA, |faction| {
                        html_color(&faction.heraldry.primary_color)
                    })
            })
            .collect()
    }

    /// Decodes a 16-bit grayscale PNG into little-endian `u16` samples
    /// (row-major, `width * height * 2` bytes). Empty array (and an error
    /// message) if the file is missing or not 16-bit grayscale. The image
    /// size is available afterwards through `get_last_image_size`.
    #[func]
    fn load_heightmap_u16(&mut self, path: GString) -> PackedByteArray {
        self.decode_png(&path, png::ColorType::Grayscale, png::BitDepth::Sixteen)
    }

    /// Decodes an 8-bit grayscale PNG into one byte per pixel (row-major).
    #[func]
    fn load_mask_u8(&mut self, path: GString) -> PackedByteArray {
        self.decode_png(&path, png::ColorType::Grayscale, png::BitDepth::Eight)
    }

    /// Decodes an 8-bit RGB PNG into three bytes per pixel (row-major, RGB).
    #[func]
    fn load_rgb8(&mut self, path: GString) -> PackedByteArray {
        self.decode_png(&path, png::ColorType::Rgb, png::BitDepth::Eight)
    }

    /// Size in pixels of the last image decoded by `load_heightmap_u16`,
    /// `load_mask_u8` or `load_rgb8` (zero if the last decode failed).
    #[func]
    fn get_last_image_size(&self) -> Vector2i {
        self.last_image_size
    }
}

impl GameDataStore {
    /// Decodes `path` and checks that it has exactly the expected colour type
    /// and bit depth. 16-bit samples are converted from PNG's big-endian to
    /// little-endian so that GDScript can read them with `decode_u16`.
    fn decode_png(
        &mut self,
        path: &GString,
        color_type: png::ColorType,
        bit_depth: png::BitDepth,
    ) -> PackedByteArray {
        self.last_image_size = Vector2i::ZERO;
        match decode_png_file(Path::new(&path.to_string()), color_type, bit_depth) {
            Ok((bytes, width, height)) => {
                self.last_image_size = Vector2i::new(width as i32, height as i32);
                PackedByteArray::from(bytes)
            }
            Err(error) => {
                godot_error!("GameDataStore: cannot decode {path}: {error}");
                PackedByteArray::new()
            }
        }
    }
}

/// Reads a PNG with exactly `color_type` / `bit_depth`; returns raw samples
/// (little-endian for 16-bit) plus the image size.
fn decode_png_file(
    path: &Path,
    color_type: png::ColorType,
    bit_depth: png::BitDepth,
) -> Result<(Vec<u8>, u32, u32), String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut decoder = png::Decoder::new(BufReader::new(file));
    // Keep the raw 16-bit samples instead of letting the decoder truncate them.
    decoder.set_transformations(png::Transformations::IDENTITY);
    let mut reader = decoder.read_info().map_err(|error| error.to_string())?;
    let info = reader.info();
    if info.color_type != color_type || info.bit_depth != bit_depth {
        return Err(format!(
            "expected {color_type:?}/{bit_depth:?}, got {:?}/{:?}",
            info.color_type, info.bit_depth
        ));
    }
    let (width, height) = (info.width, info.height);
    let mut bytes = vec![0u8; reader.output_buffer_size()];
    let frame = reader
        .next_frame(&mut bytes)
        .map_err(|error| error.to_string())?;
    bytes.truncate(frame.buffer_size());
    if bit_depth == png::BitDepth::Sixteen {
        swap_u16_endianness(&mut bytes);
    }
    Ok((bytes, width, height))
}

/// Swaps the two bytes of every 16-bit sample in place. Works on eight bytes
/// at a time so that it stays fast in unoptimised builds (a 4096² heightmap is
/// 16M samples).
fn swap_u16_endianness(bytes: &mut [u8]) {
    const LOW_BYTES: u64 = 0x00ff_00ff_00ff_00ff;
    let (words, rest) = bytes.as_chunks_mut::<8>();
    for word in words {
        let value = u64::from_ne_bytes(*word);
        let swapped = ((value & LOW_BYTES) << 8) | ((value >> 8) & LOW_BYTES);
        *word = swapped.to_ne_bytes();
    }
    for sample in rest.as_chunks_mut::<2>().0 {
        sample.swap(0, 1);
    }
}

/// Collects ids into a `PackedStringArray`.
fn ids_of<'a>(ids: impl Iterator<Item = &'a (impl AsRef<str> + 'a)>) -> PackedStringArray {
    ids.map(|id| GString::from(id.as_ref())).collect()
}

/// `{value, uncertain, year}` for a historical date; empty when unknown.
fn date_dict(date: Option<&HistoricalDate>) -> VarDictionary {
    match date {
        Some(date) => vdict! {
            "value" => date.value.as_str(),
            "uncertain" => date.uncertain,
            "year" => date.year().map_or(0, i64::from),
        },
        None => VarDictionary::new(),
    }
}

/// Parses `#RRGGBB`; falls back to magenta so a bad colour is visible.
fn html_color(html: &str) -> Color {
    Color::from_html(html).unwrap_or(Color::MAGENTA)
}

fn vector2(point: [f64; 2]) -> Vector2 {
    Vector2::new(point[0] as f32, point[1] as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swaps_every_sample_including_tail() {
        let mut bytes: Vec<u8> = (0u8..18).collect();
        swap_u16_endianness(&mut bytes);
        let expected: Vec<u8> = (0u8..18).map(|i| i ^ 1).collect();
        assert_eq!(bytes, expected);
    }

    #[test]
    fn real_heightmap_decodes_to_little_endian_u16() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data/map/heightmap.png");
        if !path.exists() {
            return;
        }
        let (bytes, width, height) =
            decode_png_file(&path, png::ColorType::Grayscale, png::BitDepth::Sixteen).unwrap();
        assert_eq!(bytes.len(), (width * height * 2) as usize);
        let centre = ((height / 2) * width + width / 2) as usize * 2;
        let sample = u16::from_le_bytes([bytes[centre], bytes[centre + 1]]);
        assert!(sample > 0, "centre of the map should be land");
    }

    #[test]
    fn wrong_bit_depth_is_refused() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data/map/land_mask.png");
        if !path.exists() {
            return;
        }
        let error =
            decode_png_file(&path, png::ColorType::Grayscale, png::BitDepth::Sixteen).unwrap_err();
        assert!(error.contains("expected"), "{error}");
    }
}
