//! Map modes (filters) and the colour of every province. Ports game/scripts/map/
//! map_mode_controller.gd (political, diplomacy, ramps from the core's map lens),
//! stance_fill.gd + stance_cues.gd (ADR 0155 : red = enemies only, ADR 0175 : stance wash).
//! Pure colouring: the values come from the core (`sim_campaign::stance`, `map_lens`).
//!
//! The five lens modes of the Godot filter menu (religion, loyalty, supply, claims, feudal) read
//! `game_api::lens_view` (`map_mode_controller.gd`, `feudal_map_lens.gd`).

use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::*;
use data_model::{FactionId, GameData, ProvinceId};
use game_api::{LensCell, LensWanted, lens_view};
use serde_json::Value;
use sim_campaign::CampaignState;
use sim_campaign::plan_cache::PlanCache;
use sim_campaign::stance::diplomatic_stance_cached;

use super::data::ProvinceMap;

/// Rebels are always at war with everybody (as in `get_faction_stances_for`).
pub const REBELS_FACTION: &str = "fac_rebels";

/// Active map mode, plus the faction the Diplomacy mode looks from (`None` = the player).
#[derive(Resource, Default, Debug, Clone, PartialEq, Eq)]
pub struct MapModeState {
    pub mode: MapMode,
    pub focus_faction: Option<String>,
}

/// Request to change the map mode (from the HUD filter menu, keys or scripts).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetMapMode(pub MapMode);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MapMode {
    /// Realms: heraldic tint plus the stance wash.
    #[default]
    Political,
    /// Bare terrain: no tint, borders at rest.
    Terrain,
    /// Stance of each controller towards the viewed faction.
    Diplomacy,
    /// Obedience of Avignon / Rome, other faiths, heresy hot spots.
    Religion,
    Unrest,
    Wealth,
    Population,
    /// Loyalty of each vassal towards its suzerain.
    Loyalty,
    /// Supply gained or lost per season by an army without general.
    Supply,
    /// Provinces claimed by the viewer or against the viewer.
    Claims,
    /// Sovereign background, holder hatching, party shield of double allegiances.
    Feudal,
}

impl MapMode {
    pub const ALL: [MapMode; 11] = [
        MapMode::Political,
        MapMode::Terrain,
        MapMode::Diplomacy,
        MapMode::Religion,
        MapMode::Unrest,
        MapMode::Wealth,
        MapMode::Population,
        MapMode::Loyalty,
        MapMode::Supply,
        MapMode::Claims,
        MapMode::Feudal,
    ];

    /// Core values this mode reads from `game_api::lens_view` (`None`: none).
    pub fn lens_wanted(self) -> Option<LensWanted> {
        match self {
            MapMode::Religion => Some(LensWanted {
                religion: true,
                feudal: false,
            }),
            MapMode::Feudal => Some(LensWanted {
                religion: false,
                feudal: true,
            }),
            MapMode::Unrest
            | MapMode::Wealth
            | MapMode::Population
            | MapMode::Loyalty
            | MapMode::Supply
            | MapMode::Claims => Some(LensWanted::default()),
            _ => None,
        }
    }

    /// Key used in `faction_borders.json` `modes` and in dumps.
    pub fn key(self) -> &'static str {
        match self {
            MapMode::Political => "political",
            MapMode::Terrain => "terrain",
            MapMode::Diplomacy => "diplomacy",
            MapMode::Religion => "religion",
            MapMode::Unrest => "unrest",
            MapMode::Wealth => "wealth",
            MapMode::Population => "population",
            MapMode::Loyalty => "loyalty",
            MapMode::Supply => "supply",
            MapMode::Claims => "claims",
            MapMode::Feudal => "feudal",
        }
    }

    pub fn from_key(key: &str) -> Option<MapMode> {
        Self::ALL.into_iter().find(|m| m.key() == key)
    }

    pub fn label_fr(self) -> &'static str {
        match self {
            MapMode::Political => "Politique",
            MapMode::Terrain => "Terrain",
            MapMode::Diplomacy => "Diplomatie",
            MapMode::Religion => "Religion",
            MapMode::Unrest => "Mécontentement",
            MapMode::Wealth => "Richesse",
            MapMode::Population => "Population",
            MapMode::Loyalty => "Loyauté des vassaux",
            MapMode::Supply => "Ravitaillement",
            MapMode::Claims => "Revendications",
            MapMode::Feudal => "Féodalité",
        }
    }

    pub fn tooltip_fr(self) -> &'static str {
        match self {
            MapMode::Political => "Couleurs des royaumes",
            MapMode::Terrain => "Terrain nu, sans teinte",
            MapMode::Diplomacy => {
                "Vert allié, bleu accord, jaune neutre, orange tension, rouge guerre, gris vassal"
            }
            MapMode::Religion => "Obédience d’Avignon ou de Rome, foyers d’hérésie",
            MapMode::Unrest => "Mécontentement moyen de la population (ordre public)",
            MapMode::Wealth => "Impôt de base par saison : population, aisance, dévastation",
            MapMode::Population => "Nombre d’habitants de chaque province",
            MapMode::Loyalty => "Loyauté de chaque vassal envers son suzerain",
            MapMode::Supply => {
                "Ravitaillement gagné ou perdu par saison par une armée sans général"
            }
            MapMode::Claims => "Provinces que vous revendiquez, ou que d’autres vous disputent",
            MapMode::Feudal => {
                "Royaumes, grands vassaux (hachures) et doubles allégeances (écu parti)"
            }
        }
    }
}

// ---- tuning (data/map/*.json) --------------------------------------------------------------

/// `faction_borders.json`, `stance_cues.json`, `stance_fill.json` (empty objects if missing).
#[derive(Resource, Clone, Default)]
pub struct Tuning {
    pub borders: Value,
    pub cues: Value,
    pub fill: Value,
}

impl Tuning {
    pub fn load(map_dir: &Path) -> Self {
        let read = |name: &str| {
            std::fs::read(map_dir.join(name))
                .ok()
                .and_then(|b| serde_json::from_slice(&b).ok())
                .unwrap_or(Value::Null)
        };
        Self {
            borders: read("faction_borders.json"),
            cues: read("stance_cues.json"),
            fill: read("stance_fill.json"),
        }
    }
}

/// Number at `path` of a JSON tree, or `default`.
pub fn num(tree: &Value, path: &[&str], default: f32) -> f32 {
    path.iter()
        .try_fold(tree, |node, key| node.get(key))
        .and_then(Value::as_f64)
        .map_or(default, |v| v as f32)
}

pub fn text<'a>(tree: &'a Value, path: &[&str]) -> Option<&'a str> {
    path.iter()
        .try_fold(tree, |node, key| node.get(key))
        .and_then(Value::as_str)
}

pub fn hex(code: &str) -> Option<Srgba> {
    Srgba::hex(code).ok()
}

// ---- stance cues (ADR 0155) ------------------------------------------------------------------

/// Sign category of a faction seen by the viewer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cue {
    SelfFaction,
    Enemy,
    Friend,
    Other,
}

impl Cue {
    pub fn key(self) -> &'static str {
        match self {
            Cue::SelfFaction => "self",
            Cue::Enemy => "enemy",
            Cue::Friend => "friend",
            Cue::Other => "other",
        }
    }
}

/// Category of a diplomatic stance key (`war` -> enemy ...); unknown: other.
pub fn cue_of_stance(stance_key: &str, cues: &Value) -> Cue {
    match text(cues, &["categories", stance_key]) {
        Some("self") => Cue::SelfFaction,
        Some("enemy") => Cue::Enemy,
        Some("friend") => Cue::Friend,
        _ => Cue::Other,
    }
}

/// Border colour of a faction: fixed for us / enemies / friends, muted heraldic otherwise.
pub fn border_color(heraldic: Srgba, cue: Cue, cues: &Value) -> Srgba {
    if cues.get("border").is_none() {
        return heraldic;
    }
    if cue != Cue::Other
        && let Some(color) = text(cues, &["border", cue.key()]).and_then(hex)
    {
        return color;
    }
    let hsv = Hsva::from(heraldic);
    let saturation = (hsv.saturation * num(cues, &["border", "other", "saturation_scale"], 1.0))
        .min(num(cues, &["border", "other", "saturation_max"], 1.0));
    let value = hsv.value.clamp(
        num(cues, &["border", "other", "value_min"], 0.0),
        num(cues, &["border", "other", "value_max"], 1.0),
    );
    Srgba::from(Hsva::new(hsv.hue, saturation, value, 1.0))
}

/// Stance wash colour of a category (alpha = opacity of the category), `stance_fill.json`.
pub fn fill_color(cue: Cue, fill: &Value, cues: &Value) -> Srgba {
    let alpha = num(fill, &["alpha", cue.key()], 0.0);
    if alpha <= 0.0 {
        return Srgba::NONE;
    }
    let code = if cue == Cue::Other {
        text(fill, &["colors", "other"])
    } else {
        text(cues, &["border", cue.key()])
    };
    code.and_then(hex)
        .map_or(Srgba::NONE, |c| c.with_alpha(alpha))
}

/// Diplomacy-mode colours by stance key (`DiplomaticStances.COLORS`).
pub fn diplomacy_color(stance_key: &str) -> Option<Srgba> {
    let (r, g, b) = match stance_key {
        "self" => (0.96, 0.93, 0.84),
        "ally" => (0.22, 0.60, 0.26),
        "agreement" => (0.24, 0.46, 0.82),
        "neutral" => (0.88, 0.78, 0.26),
        "tension" => (0.93, 0.50, 0.12),
        "war" => (0.78, 0.12, 0.10),
        "vassal" => (0.56, 0.56, 0.56),
        _ => return None,
    };
    Some(Srgba::new(r, g, b, 1.0))
}

// ---- simulation snapshot -------------------------------------------------------------------

/// Per-province values read from the simulation (tables indexed by raster index, 0 unused).
#[derive(Clone, Default, Debug, PartialEq)]
pub struct ProvinceView {
    pub owners: Vec<String>,
    pub controllers: Vec<String>,
    /// Stance key of each faction towards `viewer`.
    pub stances: HashMap<String, &'static str>,
    pub viewer: String,
    pub turn: u32,
    /// `(income, population, unrest)` per province, filled for the ramp modes only.
    pub lens: Vec<Option<(f32, f32, f32)>>,
    /// Full lens cell per province (lens modes only), same indexing.
    pub cells: Vec<Option<LensCell>>,
    /// Short names of the factions the cells mention (suzerain, holder, second lord).
    pub names: HashMap<String, String>,
    /// Supply lost per winter outside friendly land (`economy_rules.supply_loss_winter`).
    pub supply_loss_winter: u8,
}

impl ProvinceView {
    pub fn owner_of(&self, index: u16) -> &str {
        self.owners
            .get(usize::from(index))
            .map_or("", String::as_str)
    }

    pub fn controller_of(&self, index: u16) -> &str {
        let controller = self
            .controllers
            .get(usize::from(index))
            .map_or("", String::as_str);
        if controller.is_empty() {
            self.owner_of(index)
        } else {
            controller
        }
    }

    /// Short display name of a faction mentioned by a lens cell (its id when unknown).
    pub fn name_of<'a>(&'a self, faction: &'a str) -> &'a str {
        self.names.get(faction).map_or(faction, String::as_str)
    }

    pub fn cell(&self, index: u16) -> Option<&LensCell> {
        self.cells.get(usize::from(index)).and_then(Option::as_ref)
    }

    pub fn stance_of(&self, faction: &str) -> &'static str {
        if faction == self.viewer {
            return "self";
        }
        self.stances.get(faction).copied().unwrap_or("")
    }

    /// Cue of a faction seen from the viewer.
    pub fn cue_of(&self, faction: &str, cues: &Value) -> Cue {
        if faction.is_empty() {
            return Cue::Other;
        }
        cue_of_stance(self.stance_of(faction), cues)
    }
}

/// Reads owners, controllers and the stances of every faction towards `viewer` from the core;
/// `with_lens` also reads every lens value (all modes).
pub fn read_view(
    map: &ProvinceMap,
    data: &GameData,
    state: &CampaignState,
    viewer: &FactionId,
    with_lens: bool,
) -> ProvinceView {
    let wanted = with_lens.then_some(LensWanted {
        religion: true,
        feudal: true,
    });
    read_view_wanted(map, data, state, viewer, wanted)
}

/// [`read_view`] reading only the lens values `wanted` asks for (`None`: no lens).
pub fn read_view_wanted(
    map: &ProvinceMap,
    data: &GameData,
    state: &CampaignState,
    viewer: &FactionId,
    wanted: Option<LensWanted>,
) -> ProvinceView {
    let len = usize::from(map.max_index()) + 1;
    let mut view = ProvinceView {
        owners: vec![String::new(); len],
        controllers: vec![String::new(); len],
        viewer: viewer.as_str().to_owned(),
        turn: state.turn,
        lens: vec![None; len],
        cells: vec![None; len],
        ..Default::default()
    };
    for province in map.provinces() {
        let Ok(id) = ProvinceId::new(province.id.clone()) else {
            continue;
        };
        let slot = province.index as usize;
        view.owners[slot] = state
            .province_owner(&id)
            .map_or_else(|| province.owner.clone(), |f| f.as_str().to_owned());
        view.controllers[slot] = state
            .province_controller(&id)
            .map_or_else(String::new, |f| f.as_str().to_owned());
    }
    let plan = PlanCache::new(state);
    for faction in state.factions.keys() {
        let key = if faction.as_str() == REBELS_FACTION {
            "war"
        } else {
            diplomatic_stance_cached(&plan, data, viewer, faction).key()
        };
        view.stances.insert(faction.as_str().to_owned(), key);
    }
    view.stances
        .entry(REBELS_FACTION.to_owned())
        .or_insert("war");
    if let Some(wanted) = wanted {
        let lens = lens_view(data, state, viewer, wanted);
        view.supply_loss_winter = lens.supply_loss_winter;
        for province in map.provinces() {
            if let Some(cell) = lens.cells.get(&province.id) {
                view.lens[province.index as usize] = Some((
                    cell.income as f32,
                    cell.population as f32,
                    cell.unrest as f32,
                ));
                for faction in lens_factions(cell) {
                    if !view.names.contains_key(faction)
                        && let Ok(id) = FactionId::new(faction.to_owned())
                    {
                        let name = data.faction_name(&id);
                        let short = crate::campaign::parchment::realms::short_name(&name);
                        view.names.insert(faction.to_owned(), short.to_owned());
                    }
                }
                view.cells[province.index as usize] = Some(cell.clone());
            }
        }
    }
    view
}

/// Faction ids a lens cell mentions (suzerain, holder, second lord), empty ones skipped.
fn lens_factions(cell: &LensCell) -> impl Iterator<Item = &str> {
    let feudal = cell.feudal.as_ref();
    [
        Some(cell.suzerain.as_str()),
        feudal.map(|f| f.holder.as_str()),
        feudal.map(|f| f.second_lord.as_str()),
        feudal.map(|f| f.sovereign.as_str()),
    ]
    .into_iter()
    .flatten()
    .filter(|f| !f.is_empty())
}

/// Heraldic colour of every faction (`primary_color`), for the political tint and the borders.
pub fn heraldic_colors(data: &GameData) -> HashMap<String, Srgba> {
    data.factions
        .iter()
        .filter_map(|(id, faction)| {
            Some((
                id.as_str().to_owned(),
                hex(&faction.heraldry.primary_color)?,
            ))
        })
        .collect()
}

// ---- province colours --------------------------------------------------------------------

const GOOD: Srgba = Srgba::new(0.20, 0.55, 0.20, 1.0);
const BAD: Srgba = Srgba::new(0.75, 0.15, 0.10, 1.0);
const WEALTH_LOW: Srgba = Srgba::new(0.93, 0.88, 0.70, 1.0);
const WEALTH_HIGH: Srgba = Srgba::new(0.62, 0.42, 0.05, 1.0);
const POPULATION_LOW: Srgba = Srgba::new(0.90, 0.88, 0.80, 1.0);
const POPULATION_HIGH: Srgba = Srgba::new(0.30, 0.18, 0.45, 1.0);
// Lens modes (map_mode_controller.gd).
const RELIGION_AVIGNON: Srgba = Srgba::new(0.20, 0.34, 0.80, 1.0);
const RELIGION_ROME: Srgba = Srgba::new(0.80, 0.65, 0.20, 1.0);
const RELIGION_OTHER: Srgba = Srgba::new(0.45, 0.45, 0.45, 1.0);
const HERESY: Srgba = Srgba::new(0.58, 0.26, 0.72, 1.0);
/// The great faiths of the East, each its own hue (the others stay grey).
const RELIGION_FAITHS: [(&str, Srgba); 4] = [
    ("rel_orthodox", Srgba::new(0.62, 0.22, 0.38, 1.0)),
    ("rel_islam", Srgba::new(0.12, 0.52, 0.22, 1.0)),
    ("rel_pagan", Srgba::new(0.55, 0.40, 0.22, 1.0)),
    ("rel_armenian", Srgba::new(0.80, 0.42, 0.18, 1.0)),
];
const LOYALTY_LOW: Srgba = Srgba::new(0.75, 0.15, 0.10, 1.0);
const LOYALTY_MID: Srgba = Srgba::new(0.88, 0.62, 0.15, 1.0);
const LOYALTY_HIGH: Srgba = Srgba::new(0.20, 0.40, 0.78, 1.0);
pub const NEUTRAL: Srgba = Srgba::new(0.62, 0.60, 0.56, 1.0);
const SUPPLY_GAIN: Srgba = Srgba::new(0.22, 0.55, 0.25, 1.0);
const SUPPLY_LOSS: Srgba = Srgba::new(0.85, 0.55, 0.15, 1.0);
const SUPPLY_STARVE: Srgba = Srgba::new(0.60, 0.08, 0.06, 1.0);
const CLAIM_OURS: Srgba = Srgba::new(0.85, 0.68, 0.20, 1.0);
const CLAIM_AGAINST_US: Srgba = Srgba::new(0.72, 0.12, 0.10, 1.0);
const CLAIM_CONTESTED: Srgba = Srgba::new(0.50, 0.25, 0.65, 1.0);

/// Colour of a faith in the religion mode: Avignon (the Church during the Schism, blue), Rome
/// (gold), the great eastern faiths, else grey; tinted towards the heresy purple by `heresy` (%).
pub fn religion_color(faith: &str, heresy: u8) -> Srgba {
    let base = match faith {
        "rel_catholic" => RELIGION_AVIGNON,
        "rel_catholic_rome" => RELIGION_ROME,
        other => RELIGION_FAITHS
            .iter()
            .find(|(id, _)| *id == other)
            .map_or(RELIGION_OTHER, |(_, c)| *c),
    };
    mix(base, HERESY, f32::from(heresy) / 100.0 * 1.5)
}

/// Loyalty ramp: red (about to revolt) - amber - blue (faithful); grey for a non-vassal.
pub fn loyalty_color(loyalty: Option<u8>) -> Srgba {
    let Some(loyalty) = loyalty else {
        return NEUTRAL;
    };
    let ratio = (f32::from(loyalty) / 100.0).clamp(0.0, 1.0);
    if ratio < 0.5 {
        mix(LOYALTY_LOW, LOYALTY_MID, ratio * 2.0)
    } else {
        mix(LOYALTY_MID, LOYALTY_HIGH, ratio * 2.0 - 1.0)
    }
}

/// Supply: green when it is regained, amber to dark red as the attrition nears the winter loss.
pub fn supply_color(change: i8, worst_loss: u8) -> Srgba {
    if change >= 0 {
        return SUPPLY_GAIN;
    }
    let t = if worst_loss > 0 {
        f32::from(-i16::from(change)) / f32::from(worst_loss)
    } else {
        1.0
    };
    mix(SUPPLY_LOSS, SUPPLY_STARVE, t)
}

pub fn claim_color(claim: &str) -> Srgba {
    match claim {
        "ours" => CLAIM_OURS,
        "against_us" => CLAIM_AGAINST_US,
        "contested" => CLAIM_CONTESTED,
        _ => NEUTRAL,
    }
}

/// Opacity of the heraldic tint in the political mode and of the ramp / diplomacy fills.
pub const POLITICAL_TINT_ALPHA: f32 = 0.16;
pub const MODE_FILL_ALPHA: f32 = 0.55;

/// Rank of each province (0 = weakest, 1 = strongest) by `value`: a scale by rank, not by value,
/// otherwise Paris crushes every other tint (`_percentiles`).
pub fn percentile_ranks(values: &[(u16, f32)]) -> HashMap<u16, f32> {
    let mut sorted: Vec<(u16, f32)> = values.to_vec();
    sorted.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    let last = (sorted.len().max(2) - 1) as f32;
    sorted
        .iter()
        .enumerate()
        .map(|(rank, (index, _))| (*index, rank as f32 / last))
        .collect()
}

fn mix(a: Srgba, b: Srgba, t: f32) -> Srgba {
    a.mix(&b, t.clamp(0.0, 1.0))
}

/// Source-over blend of `top` on `bottom` (straight alpha).
fn over(bottom: Srgba, top: Srgba) -> Srgba {
    let alpha = top.alpha + bottom.alpha * (1.0 - top.alpha);
    if alpha <= 0.0 {
        return Srgba::NONE;
    }
    let channel = |t: f32, b: f32| (t * top.alpha + b * bottom.alpha * (1.0 - top.alpha)) / alpha;
    Srgba::new(
        channel(top.red, bottom.red),
        channel(top.green, bottom.green),
        channel(top.blue, bottom.blue),
        alpha,
    )
}

/// Overlay colour of every province, indexed by raster index (index 0 = sea, transparent).
pub fn province_colors(
    mode: MapMode,
    max_index: u16,
    view: &ProvinceView,
    heraldic: &HashMap<String, Srgba>,
    tuning: &Tuning,
) -> Vec<Srgba> {
    province_colors_with(mode, max_index, view, heraldic, tuning, true)
}

/// [`province_colors`] with the player option `map/stance_fill`: when `stance_wash` is false the
/// political map keeps the heraldic tint only (no friend / enemy wash).
pub fn province_colors_with(
    mode: MapMode,
    max_index: u16,
    view: &ProvinceView,
    heraldic: &HashMap<String, Srgba>,
    tuning: &Tuning,
    stance_wash: bool,
) -> Vec<Srgba> {
    let mut colors = vec![Srgba::NONE; usize::from(max_index) + 1];
    let ranks = |select: fn(&(f32, f32, f32)) -> f32| {
        let values: Vec<(u16, f32)> = (1..=max_index)
            .filter_map(|i| {
                view.lens
                    .get(usize::from(i))
                    .copied()
                    .flatten()
                    .map(|l| (i, select(&l)))
            })
            .collect();
        percentile_ranks(&values)
    };
    let wealth = (mode == MapMode::Wealth).then(|| ranks(|l| l.0));
    let population = (mode == MapMode::Population).then(|| ranks(|l| l.1));
    let washes: HashMap<Cue, Srgba> = [Cue::SelfFaction, Cue::Enemy, Cue::Friend, Cue::Other]
        .into_iter()
        .map(|cue| (cue, fill_color(cue, &tuning.fill, &tuning.cues)))
        .collect();
    let mode_alpha = num(&tuning.fill, &["modes", "political"], 1.0);
    for index in 1..=max_index {
        let slot = usize::from(index);
        let controller = view.controller_of(index);
        colors[slot] = match mode {
            MapMode::Terrain => Srgba::NONE,
            MapMode::Political => {
                let owner = view.owner_of(index);
                let tint = heraldic
                    .get(owner)
                    .map_or(Srgba::NONE, |c| c.with_alpha(POLITICAL_TINT_ALPHA));
                let mut wash = washes[&view.cue_of(controller, &tuning.cues)];
                wash.alpha *= mode_alpha;
                if controller.is_empty() || !stance_wash {
                    tint
                } else {
                    over(tint, wash)
                }
            }
            MapMode::Diplomacy => {
                let key = view.stance_of(controller);
                diplomacy_color(key).map_or(Srgba::NONE, |c| c.with_alpha(MODE_FILL_ALPHA))
            }
            MapMode::Religion | MapMode::Loyalty | MapMode::Supply | MapMode::Claims => {
                view.cell(index).map_or(Srgba::NONE, |cell| {
                    match mode {
                        MapMode::Religion => cell
                            .religion
                            .as_ref()
                            .map_or(NEUTRAL, |r| religion_color(&r.religion, r.heresy)),
                        MapMode::Loyalty => loyalty_color(cell.vassal_loyalty),
                        MapMode::Supply => {
                            supply_color(cell.supply_change, view.supply_loss_winter)
                        }
                        _ => claim_color(&cell.claim),
                    }
                    .with_alpha(MODE_FILL_ALPHA)
                })
            }
            MapMode::Feudal => view
                .cell(index)
                .and_then(|cell| cell.feudal.as_ref())
                .filter(|feudal| !feudal.sovereign.is_empty())
                .map_or(Srgba::NONE, |feudal| {
                    faction_tint(heraldic, &feudal.sovereign).with_alpha(MODE_FILL_ALPHA)
                }),
            MapMode::Unrest | MapMode::Wealth | MapMode::Population => {
                match view.lens.get(slot).copied().flatten() {
                    None => Srgba::NONE,
                    Some((_, _, unrest)) => match mode {
                        MapMode::Unrest => mix(GOOD, BAD, unrest / 100.0),
                        MapMode::Wealth => mix(
                            WEALTH_LOW,
                            WEALTH_HIGH,
                            wealth.as_ref().map_or(0.0, |r| r[&index]),
                        ),
                        _ => mix(
                            POPULATION_LOW,
                            POPULATION_HIGH,
                            population.as_ref().map_or(0.0, |r| r[&index]),
                        ),
                    }
                    .with_alpha(MODE_FILL_ALPHA),
                }
            }
        };
    }
    colors
}

/// Heraldic colour of a faction, neutral grey when it has none.
fn faction_tint(heraldic: &HashMap<String, Srgba>, faction: &str) -> Srgba {
    heraldic.get(faction).copied().unwrap_or(NEUTRAL)
}

/// Second fill layer of the Feudal mode, by raster index: the colour of the holder of the
/// province's title where it is not the sovereign (the fill shader stripes it over the
/// sovereign's background), transparent elsewhere. Empty outside the Feudal mode.
pub fn province_hatch(
    mode: MapMode,
    max_index: u16,
    view: &ProvinceView,
    heraldic: &HashMap<String, Srgba>,
) -> Vec<Srgba> {
    let mut hatch = vec![Srgba::NONE; usize::from(max_index) + 1];
    if mode != MapMode::Feudal {
        return hatch;
    }
    for index in 1..=max_index {
        if let Some(feudal) = view.cell(index).and_then(|c| c.feudal.as_ref())
            && !feudal.holder.is_empty()
            && feudal.holder != feudal.sovereign
        {
            hatch[usize::from(index)] =
                faction_tint(heraldic, &feudal.holder).with_alpha(MODE_FILL_ALPHA);
        }
    }
    hatch
}

/// Provinces in double allegiance (holder, second lord), by raster index, for the party shields.
pub fn double_allegiances(view: &ProvinceView, max_index: u16) -> Vec<(u16, String, String)> {
    (1..=max_index)
        .filter_map(|index| {
            let feudal = view.cell(index)?.feudal.as_ref()?;
            (!feudal.second_lord.is_empty() && !feudal.holder.is_empty())
                .then(|| (index, feudal.holder.clone(), feudal.second_lord.clone()))
        })
        .collect()
}

/// Opacity of the border layer in `mode` (`faction_borders.json` `modes`); full intensity flag.
pub fn mode_border_style(mode: MapMode, tuning: &Tuning) -> (f32, bool) {
    let style = tuning
        .borders
        .get("modes")
        .and_then(|m| m.get(mode.key()).or_else(|| m.get("political")));
    let alpha = style
        .and_then(|s| s.get("alpha"))
        .and_then(Value::as_f64)
        .unwrap_or(1.0) as f32;
    let full = style
        .and_then(|s| s.get("full"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    (alpha, full)
}

/// Faction set seen by the owners table, for the dump.
pub fn faction_count(view: &ProvinceView) -> usize {
    view.owners
        .iter()
        .filter(|o| !o.is_empty())
        .collect::<std::collections::BTreeSet<_>>()
        .len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tuning() -> Tuning {
        let root = crate::paths::find_root_from(Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap();
        Tuning::load(&root.join("data/map"))
    }

    #[test]
    fn red_is_for_enemies_only() {
        let t = tuning();
        assert_eq!(cue_of_stance("war", &t.cues), Cue::Enemy);
        assert_eq!(cue_of_stance("ally", &t.cues), Cue::Friend);
        assert_eq!(cue_of_stance("vassal", &t.cues), Cue::Friend);
        assert_eq!(cue_of_stance("neutral", &t.cues), Cue::Other);
        // a red heraldic colour at peace is muted: never the enemy red
        let red = Srgba::new(0.9, 0.1, 0.1, 1.0);
        let muted = border_color(red, Cue::Other, &t.cues);
        let enemy = border_color(red, Cue::Enemy, &t.cues);
        assert_ne!(muted, enemy);
        assert!(Hsva::from(muted).saturation <= 0.5 + 1e-3);
        assert_eq!(enemy, hex("#d21f17").unwrap());
    }

    #[test]
    fn stance_wash_uses_cue_colours() {
        let t = tuning();
        let own = fill_color(Cue::SelfFaction, &t.fill, &t.cues);
        assert_eq!(own, hex("#d9a833").unwrap().with_alpha(0.34));
        assert_eq!(
            fill_color(Cue::Other, &t.fill, &t.cues),
            hex("#8a8478").unwrap().with_alpha(0.08)
        );
    }

    #[test]
    fn ranks_are_by_order_not_value() {
        let ranks = percentile_ranks(&[(1, 1.0), (2, 1_000_000.0), (3, 5.0)]);
        assert_eq!((ranks[&1], ranks[&3], ranks[&2]), (0.0, 0.5, 1.0));
    }

    #[test]
    fn political_wash_follows_the_controller() {
        let t = tuning();
        let mut view = ProvinceView {
            owners: vec![String::new(), "fac_france".into(), "fac_france".into()],
            controllers: vec![String::new(), "fac_france".into(), "fac_england".into()],
            viewer: "fac_france".into(),
            ..Default::default()
        };
        view.stances.insert("fac_england".into(), "war");
        view.lens = vec![None; 3];
        let heraldic = HashMap::from([("fac_france".to_owned(), Srgba::new(0.1, 0.2, 0.7, 1.0))]);
        let colors = province_colors(MapMode::Political, 2, &view, &heraldic, &t);
        // occupied province (owner France, controller England at war): the wash is red-ish, so
        // its red channel beats the one of the province we hold (gold wash over blue tint)
        assert!(colors[2].red > 0.0 && colors[1].alpha > 0.3 && colors[2].alpha > 0.3);
        assert!(colors[2].red != colors[1].red);
        assert_eq!(
            province_colors(MapMode::Terrain, 2, &view, &heraldic, &t)[1],
            Srgba::NONE
        );
        let diplomacy = province_colors(MapMode::Diplomacy, 2, &view, &heraldic, &t);
        assert_eq!(
            diplomacy[2],
            diplomacy_color("war").unwrap().with_alpha(MODE_FILL_ALPHA)
        );
    }

    #[test]
    fn stance_fill_off_keeps_the_heraldic_tint_only() {
        let t = tuning();
        let mut view = ProvinceView {
            owners: vec![String::new(), "fac_france".into()],
            controllers: vec![String::new(), "fac_england".into()],
            viewer: "fac_france".into(),
            ..Default::default()
        };
        view.stances.insert("fac_england".into(), "war");
        view.lens = vec![None; 2];
        let heraldic = HashMap::from([("fac_france".to_owned(), Srgba::new(0.1, 0.2, 0.7, 1.0))]);
        let with = province_colors_with(MapMode::Political, 1, &view, &heraldic, &t, true);
        let without = province_colors_with(MapMode::Political, 1, &view, &heraldic, &t, false);
        assert_eq!(
            without[1],
            Srgba::new(0.1, 0.2, 0.7, 1.0).with_alpha(POLITICAL_TINT_ALPHA)
        );
        assert_ne!(with[1], without[1]);
    }

    #[test]
    fn lens_mode_colours_follow_godot() {
        // religion: Avignon blue, Rome gold, the great eastern faiths, grey otherwise; heresy
        // drags the colour towards purple, capped at the purple itself
        assert_eq!(religion_color("rel_catholic", 0), RELIGION_AVIGNON);
        assert_eq!(religion_color("rel_catholic_rome", 0), RELIGION_ROME);
        assert_eq!(religion_color("rel_islam", 0), RELIGION_FAITHS[1].1);
        assert_eq!(religion_color("rel_unknown", 0), RELIGION_OTHER);
        assert_eq!(religion_color("rel_catholic", 100), HERESY);
        assert_ne!(religion_color("rel_catholic", 20), RELIGION_AVIGNON);
        // loyalty: red - amber - blue, grey for no vassal
        assert_eq!(loyalty_color(Some(0)), LOYALTY_LOW);
        assert_eq!(loyalty_color(Some(50)), LOYALTY_MID);
        assert_eq!(loyalty_color(Some(100)), LOYALTY_HIGH);
        assert_eq!(loyalty_color(None), NEUTRAL);
        // supply: gain green, attrition amber up to dark red at the winter loss
        assert_eq!(supply_color(2, 35), SUPPLY_GAIN);
        assert_eq!(supply_color(0, 35), SUPPLY_GAIN);
        assert_eq!(supply_color(-35, 35), SUPPLY_STARVE);
        assert_eq!(supply_color(-5, 0), SUPPLY_STARVE);
        let mid = supply_color(-10, 35);
        assert!(mid.red < SUPPLY_LOSS.red && mid.red > SUPPLY_STARVE.red);
        // claims
        assert_eq!(claim_color("ours"), CLAIM_OURS);
        assert_eq!(claim_color("against_us"), CLAIM_AGAINST_US);
        assert_eq!(claim_color("contested"), CLAIM_CONTESTED);
        assert_eq!(claim_color(""), NEUTRAL);
    }

    #[test]
    fn feudal_background_hatch_and_shields() {
        use game_api::lens::{FeudalLensCell, LensCell};
        let cell = |sovereign: &str, holder: &str, second: &str| LensCell {
            income: 0.0,
            population: 0,
            unrest: 0.0,
            vassal_loyalty: None,
            suzerain: String::new(),
            supply_change: 0,
            claim: String::new(),
            religion: None,
            feudal: Some(FeudalLensCell {
                sovereign: sovereign.into(),
                holder: holder.into(),
                second_lord: second.into(),
            }),
        };
        let view = ProvinceView {
            cells: vec![
                None,
                Some(cell("fac_france", "fac_france", "")),
                Some(cell("fac_france", "fac_burgundy", "")),
                Some(cell("fac_france", "fac_england", "fac_france")),
            ],
            ..Default::default()
        };
        let heraldic = HashMap::from([
            ("fac_france".to_owned(), Srgba::new(0.1, 0.2, 0.7, 1.0)),
            ("fac_burgundy".to_owned(), Srgba::new(0.7, 0.2, 0.1, 1.0)),
        ]);
        let t = tuning();
        let colors = province_colors(MapMode::Feudal, 3, &view, &heraldic, &t);
        // background: the sovereign everywhere
        assert!(colors[1..=3].iter().all(|c| c.red == 0.1 && c.alpha > 0.0));
        // hatching only where the holder is not the sovereign; an unknown holder is grey
        let hatch = province_hatch(MapMode::Feudal, 3, &view, &heraldic);
        assert_eq!(hatch[1], Srgba::NONE);
        assert_eq!(hatch[2].red, 0.7);
        assert_eq!(hatch[3].red, NEUTRAL.red);
        assert!(
            province_hatch(MapMode::Unrest, 3, &view, &heraldic)
                .iter()
                .all(|c| c.alpha == 0.0)
        );
        // a party shield on the double allegiance only
        assert_eq!(
            double_allegiances(&view, 3),
            vec![(3, "fac_england".to_owned(), "fac_france".to_owned())]
        );
    }

    #[test]
    fn modes_have_keys_and_border_styles() {
        let t = tuning();
        for mode in MapMode::ALL {
            assert_eq!(MapMode::from_key(mode.key()), Some(mode));
        }
        assert!(mode_border_style(MapMode::Diplomacy, &t).1);
        assert!(!mode_border_style(MapMode::Political, &t).1);
    }
}

#[cfg(test)]
mod sim_tests {
    use super::*;
    use crate::campaign::provinces::data::tests::real_map;

    #[test]
    fn read_view_from_a_real_campaign() {
        let root = crate::paths::find_root_from(Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap();
        let (data, _) = GameData::load(&crate::paths::data_dir(&root)).unwrap();
        let france = FactionId::new("fac_france").unwrap();
        let state = CampaignState::new_1337(&data, france.clone(), 1337).unwrap();
        let map = real_map();
        let view = read_view(map, &data, &state, &france, true);
        let paris = map.index_of_id("prov_ile_de_france");
        assert_eq!(view.owner_of(paris), "fac_france");
        assert_eq!(view.stance_of("fac_france"), "self");
        assert_eq!(view.stance_of(REBELS_FACTION), "war");
        assert!(view.stances.len() > 50);
        assert!(view.lens[usize::from(paris)].is_some());
        // England and France are at war in 1337: their provinces get the enemy colour
        let england = view.stance_of("fac_england");
        assert!(
            ["war", "tension", "neutral"].contains(&england),
            "{england}"
        );
        let tuning = Tuning::load(&root.join("data/map"));
        let heraldic = heraldic_colors(&data);
        assert!(heraldic.contains_key("fac_france"));
        for mode in MapMode::ALL {
            let colors = province_colors(mode, map.max_index(), &view, &heraldic, &tuning);
            let tinted = colors.iter().filter(|c| c.alpha > 0.0).count();
            if mode == MapMode::Terrain {
                assert_eq!(tinted, 0);
            } else {
                assert!(tinted > 100, "{mode:?}: {tinted} tinted");
            }
        }
        // the Religion mode paints several faiths, the Feudal one names France above Paris
        let distinct = |mode| {
            province_colors(mode, map.max_index(), &view, &heraldic, &tuning)
                .iter()
                .filter(|c| c.alpha > 0.0)
                .map(|c| c.to_u8_array())
                .collect::<std::collections::BTreeSet<_>>()
                .len()
        };
        assert!(distinct(MapMode::Religion) >= 2);
        assert!(distinct(MapMode::Feudal) >= 3);
        let feudal = view.cell(paris).and_then(|c| c.feudal.as_ref()).unwrap();
        assert_eq!(feudal.sovereign, "fac_france");
        assert_eq!(view.name_of("fac_france"), "France");
    }
}
