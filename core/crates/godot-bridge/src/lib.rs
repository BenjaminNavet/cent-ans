//! GDExtension entry point: exposes the Rust simulation to Godot.
//!
//! Only plain values cross the boundary (integers, strings, packed arrays);
//! Godot never holds a pointer into the simulation.

use std::path::PathBuf;

use data_model::{GameData, HistoricalDate};
use godot::classes::RefCounted;
use godot::prelude::*;
use sim_campaign::CampaignState;

struct CentAnsExtension;

#[gdextension]
unsafe impl ExtensionLibrary for CentAnsExtension {}

/// Godot-facing handle on a campaign simulation.
#[derive(GodotClass)]
#[class(base = RefCounted)]
pub struct CampaignSim {
    state: Option<CampaignState>,
    base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for CampaignSim {
    fn init(base: Base<RefCounted>) -> Self {
        CampaignSim { state: None, base }
    }
}

#[godot_api]
impl CampaignSim {
    /// Starts a fresh campaign in spring 1337 with the given RNG seed.
    #[func]
    fn new_campaign(&mut self, seed: i64) {
        self.state = Some(CampaignState::new(seed as u64));
    }

    /// Advances the campaign by one season. Does nothing before `new_campaign`.
    #[func]
    fn end_turn(&mut self) {
        match self.state.as_mut() {
            Some(state) => state.end_turn(),
            None => godot_warn!("CampaignSim.end_turn called before new_campaign"),
        }
    }

    /// Zero-based turn counter (-1 before `new_campaign`).
    #[func]
    fn get_turn(&self) -> i64 {
        self.state
            .as_ref()
            .map_or(-1, |state| i64::from(state.turn))
    }

    /// Current campaign year (0 before `new_campaign`).
    #[func]
    fn get_year(&self) -> i64 {
        self.state.as_ref().map_or(0, |state| i64::from(state.year))
    }

    /// French date label, e.g. `"Printemps 1337"` (empty before `new_campaign`).
    #[func]
    fn get_date_label(&self) -> GString {
        self.state
            .as_ref()
            .map_or_else(GString::new, |state| GString::from(&state.date_label()))
    }
}

/// Godot-facing read-only view of the game data loaded from `data/`.
///
/// Every accessor returns plain Godot values (`Dictionary`, packed arrays);
/// unknown ids give an empty `Dictionary`.
#[derive(GodotClass)]
#[class(base = RefCounted)]
pub struct GameDataStore {
    data: Option<GameData>,
    warnings: Vec<String>,
    base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for GameDataStore {
    fn init(base: Base<RefCounted>) -> Self {
        GameDataStore {
            data: None,
            warnings: Vec::new(),
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
        match GameData::load(&root) {
            Ok((data, warnings)) => {
                self.warnings = warnings.iter().map(ToString::to_string).collect();
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
            "neighbors" => &ids_of(province.neighbors.iter()),
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
