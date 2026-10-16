//! Living portraits (`living_portrait.gd`, `portrait_frame.gd`, `portrait_loader.gd`).
//!
//! The picture of a character follows his current state: a historical figure keeps the fixed
//! `portraits/<id>.png` while he stays in its age band, then the aged variant
//! (`portraits/aged/<id>_<band>.jpg`); anyone else gets an archetype
//! (`portraits/archetypes/<rank>_<sex>_<band>_<culture>_<face>.jpg`) whose face is picked by the
//! FNV-1a hash of his id. Everything comes from `data/portraits/archetypes.json`; the frame and
//! the marks (mourning, captivity, wound, crown, mitre...) are drawn with UI nodes. Rendering
//! only: age, ruler and traits come from `game_api::court::PortraitContext`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use bevy::asset::{Asset, embedded_asset};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::ui_render::prelude::{UiMaterial, UiMaterialPlugin};
use game_api::court::PortraitContext;
use serde::Deserialize;

use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::{UiTheme, style};

type Kids<'a> = ChildSpawnerCommands<'a>;

/// Year of the start of the campaign: fixed portraits are painted for the age of that year.
const START_YEAR: i32 = 1337;
/// Youngest age shown by a fixed portrait (characters born after 1337 are painted at ~20).
const FIXED_MIN_AGE: i32 = 20;
const BAND_ORDER: [&str; 4] = ["child", "young", "adult", "old"];

#[derive(Debug, Deserialize, Default, Clone)]
pub struct AgeBand {
    pub id: String,
    #[serde(default)]
    pub max_age: Option<i32>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct Culture {
    pub id: String,
    #[serde(default)]
    pub factions: Vec<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct RankEntry {
    pub id: String,
    #[serde(default)]
    pub headwear: Option<String>,
    #[serde(default)]
    pub frame: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct Cell {
    pub rank: String,
    pub sexes: Vec<String>,
    pub bands: Vec<String>,
    pub cultures: Vec<String>,
    pub faces: u64,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct Fallbacks {
    #[serde(default)]
    pub rank: BTreeMap<String, String>,
    #[serde(default)]
    pub band: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct TitleRule {
    pub rank: String,
    pub words: Vec<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct RankRules {
    #[serde(default)]
    pub data_roles: BTreeMap<String, String>,
    #[serde(default)]
    pub title_keywords: Vec<TitleRule>,
    #[serde(default)]
    pub ruler_house_rank: String,
    #[serde(default)]
    pub army_rank: String,
    #[serde(default)]
    pub default_rank: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct AgedVariant {
    pub character: String,
    pub band: String,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct MarksConfig {
    #[serde(default)]
    pub sick_traits: Vec<String>,
    #[serde(default)]
    pub wound_traits: Vec<String>,
    #[serde(default)]
    pub mourning_years: i32,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct PortraitConfig {
    #[serde(default)]
    pub archetype_dir: String,
    #[serde(default)]
    pub aged_dir: String,
    #[serde(default)]
    pub age_bands: Vec<AgeBand>,
    #[serde(default)]
    pub cultures: Vec<Culture>,
    #[serde(default)]
    pub default_culture: String,
    #[serde(default)]
    pub ranks: Vec<RankEntry>,
    #[serde(default)]
    pub cells: Vec<Cell>,
    #[serde(default)]
    pub fallbacks: Fallbacks,
    #[serde(default)]
    pub rank_rules: RankRules,
    #[serde(default)]
    pub aged_variants: Vec<AgedVariant>,
    #[serde(default)]
    pub marks: MarksConfig,
}

/// Parsed portrait bank plus the asset root used to test which pictures exist.
#[derive(Resource, Default, Clone)]
pub struct PortraitBank {
    pub config: PortraitConfig,
    pub asset_root: Option<PathBuf>,
    aged: BTreeMap<String, Vec<String>>,
}

/// What a portrait is drawn for (a sheet or a family node).
#[derive(Debug, Clone, Default)]
pub struct Subject<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub sex: &'a str,
    pub age: i32,
    pub birth_year: i32,
    pub alive: bool,
    pub faction: &'a str,
    pub house: &'a str,
    pub title: &'a str,
    pub spouse: &'a str,
    pub context: Option<&'a PortraitContext>,
}

/// Chosen picture.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Resolved {
    /// Asset path (relative to `game/assets`), empty when no picture exists.
    pub path: String,
    /// `fixed`, `aged`, `archetype` or empty.
    pub kind: String,
    pub rank: String,
    pub band: String,
    pub culture: String,
    pub headwear: String,
    pub mirrored: bool,
}

/// Marks drawn over the picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Marks {
    pub dead: bool,
    pub captive: bool,
    pub sick: bool,
    pub wounded: bool,
    pub mourning: bool,
    pub crown: bool,
    pub mitre: bool,
}

impl Marks {
    /// Shader parameters of `portrait_frame.gd::_marks_material`: `(grey, pallor, dim)`, all zero
    /// when the picture needs no shader.
    pub fn shader_params(&self) -> Vec3 {
        Vec3::new(
            if self.dead { 1.0 } else { 0.0 },
            if self.sick { 0.8 } else { 0.0 },
            if self.captive { 1.0 } else { 0.0 },
        )
    }
}

/// Material of `portrait_marks.gdshader` for a UI picture (grey of the dead, pallor of the sick,
/// shade of the captive, optional horizontal mirror).
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct PortraitMarksMaterial {
    /// `(grey, pallor, dim, flip)`.
    #[uniform(0)]
    pub marks: Vec4,
    #[texture(1)]
    #[sampler(2)]
    pub picture: Handle<Image>,
}

impl UiMaterial for PortraitMarksMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://cent_ans_client/campaign/panels/court/portrait_marks.wesl".into()
    }
}

/// Picture node that wants the marks shader: [`dress_marked_pictures`] swaps its `ImageNode` for
/// a [`PortraitMarksMaterial`] node.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct PortraitMarked {
    pub params: Vec3,
    pub flip: bool,
}

/// Number of pictures drawn with the shader (dump).
#[derive(Resource, Default, Debug)]
pub struct PortraitMarksStats {
    pub dressed: usize,
    pub grey: usize,
    pub pallor: usize,
    pub dim: usize,
}

pub struct PortraitMarksPlugin;

impl Plugin for PortraitMarksPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "portrait_marks.wesl");
        app.add_plugins(UiMaterialPlugin::<PortraitMarksMaterial>::default())
            .init_resource::<PortraitMarksStats>()
            .add_systems(Update, dress_marked_pictures);
    }
}

fn dress_marked_pictures(
    mut commands: Commands,
    mut materials: ResMut<Assets<PortraitMarksMaterial>>,
    mut stats: ResMut<PortraitMarksStats>,
    pictures: Query<(Entity, &PortraitMarked, &ImageNode), Added<PortraitMarked>>,
) {
    for (entity, marked, image) in &pictures {
        let material = materials.add(PortraitMarksMaterial {
            marks: marked.params.extend(if marked.flip { 1.0 } else { 0.0 }),
            picture: image.image.clone(),
        });
        commands
            .entity(entity)
            .remove::<ImageNode>()
            .insert(MaterialNode(material));
        stats.dressed += 1;
        stats.grey += usize::from(marked.params.x > 0.0);
        stats.pallor += usize::from(marked.params.y > 0.0);
        stats.dim += usize::from(marked.params.z > 0.0);
    }
}

/// FNV-1a 32 bits over the UTF-8 of `text` (stable across versions and saves).
pub fn fnv1a(text: &str) -> u64 {
    let mut hash: u64 = 2_166_136_261;
    for byte in text.bytes() {
        hash = ((hash ^ u64::from(byte)).wrapping_mul(16_777_619)) & 0xFFFF_FFFF;
    }
    hash
}

fn band_index(band: &str) -> Option<usize> {
    BAND_ORDER.iter().position(|b| *b == band)
}

fn chain(start: &str, links: &BTreeMap<String, String>) -> Vec<String> {
    let mut out = vec![start.to_owned()];
    let mut current = start.to_owned();
    while let Some(next) = links.get(&current) {
        if out.contains(next) {
            break;
        }
        out.push(next.clone());
        current = next.clone();
    }
    out
}

impl PortraitBank {
    /// Reads `data/portraits/archetypes.json` under the repository root (empty bank when absent).
    pub fn load(root: &Path) -> Self {
        let text = std::fs::read_to_string(root.join("data/portraits/archetypes.json"));
        let config: PortraitConfig = text
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        Self::new(config, Some(root.join("game/assets")))
    }

    pub fn new(config: PortraitConfig, asset_root: Option<PathBuf>) -> Self {
        let mut aged: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for variant in &config.aged_variants {
            aged.entry(variant.character.clone())
                .or_default()
                .push(variant.band.clone());
        }
        Self {
            config,
            asset_root,
            aged,
        }
    }

    fn exists(&self, path: &str) -> bool {
        self.asset_root
            .as_ref()
            .is_some_and(|root| root.join(path).is_file())
    }

    pub fn age_band(&self, age: i32) -> String {
        for band in &self.config.age_bands {
            if band.max_age.is_none_or(|max| age <= max) {
                return band.id.clone();
            }
        }
        "old".to_owned()
    }

    pub fn culture_of(&self, faction: &str) -> String {
        self.config
            .cultures
            .iter()
            .find(|c| c.factions.iter().any(|f| f == faction))
            .map_or_else(
                || {
                    if self.config.default_culture.is_empty() {
                        "france".to_owned()
                    } else {
                        self.config.default_culture.clone()
                    }
                },
                |c| c.id.clone(),
            )
    }

    /// Display rank (key of `ranks`), `living_portrait.gd::rank_of`.
    pub fn rank_of(&self, subject: &Subject) -> String {
        let rules = &self.config.rank_rules;
        let male = subject.sex != "female";
        if self.age_band(subject.age) == "child" {
            return "child".to_owned();
        }
        let default = PortraitContext::default();
        let context = subject.context.unwrap_or(&default);
        let ruler = context.ruler.as_str();
        if !ruler.is_empty() && (subject.id == ruler || subject.spouse == ruler) {
            return "sovereign".to_owned();
        }
        if let Some(rank) = rules.data_roles.get(&context.data_role) {
            return rank.clone();
        }
        let title = format!(" {} ", subject.title.to_lowercase());
        for rule in &rules.title_keywords {
            if rule.words.iter().any(|w| title.contains(w.as_str())) {
                return rule.rank.clone();
            }
        }
        if context.has_army && male && !rules.army_rank.is_empty() {
            return rules.army_rank.clone();
        }
        if !context.ruler_house.is_empty()
            && subject.house == context.ruler_house
            && !rules.ruler_house_rank.is_empty()
        {
            return rules.ruler_house_rank.clone();
        }
        rules
            .default_rank
            .get(if male { "male" } else { "female" })
            .cloned()
            .unwrap_or_else(|| "noble".to_owned())
    }

    fn rank_entry(&self, rank: &str) -> Option<&RankEntry> {
        self.config.ranks.iter().find(|r| r.id == rank)
    }

    /// Frame style of a rank (`sovereign`, `prelate`, `knight`, `burgher`, `noble`...).
    pub fn frame_style(&self, rank: &str) -> String {
        self.rank_entry(rank)
            .and_then(|r| r.frame.clone())
            .unwrap_or_else(|| "noble".to_owned())
    }

    fn headwear(&self, rank: &str) -> String {
        self.rank_entry(rank)
            .and_then(|r| r.headwear.clone())
            .unwrap_or_else(|| "none".to_owned())
    }

    fn faces(&self, rank: &str, sex: &str, band: &str, culture: &str) -> u64 {
        self.config
            .cells
            .iter()
            .find(|c| {
                c.rank == rank
                    && c.sexes.iter().any(|s| s == sex)
                    && c.bands.iter().any(|b| b == band)
                    && c.cultures.iter().any(|x| x == culture)
            })
            .map_or(0, |c| c.faces)
    }

    /// Fixed portrait or aged variant of the current band; empty when none applies.
    fn historical(&self, subject: &Subject, band: &str) -> String {
        if subject.id.is_empty() {
            return String::new();
        }
        let fixed = format!("portraits/{}.png", subject.id);
        let base_band = if subject.birth_year > 0 {
            self.age_band((START_YEAR - subject.birth_year).max(FIXED_MIN_AGE))
        } else {
            band.to_owned()
        };
        let mut candidates: BTreeMap<String, String> = BTreeMap::new();
        if self.exists(&fixed) {
            candidates.insert(base_band, fixed);
        }
        let dir = if self.config.aged_dir.is_empty() {
            "portraits/aged"
        } else {
            self.config.aged_dir.as_str()
        };
        for aged_band in self.aged.get(subject.id).into_iter().flatten() {
            let path = format!("{dir}/{}_{aged_band}.jpg", subject.id);
            if self.exists(&path) {
                candidates.insert(aged_band.clone(), path);
            }
        }
        if let Some(path) = candidates.get(band) {
            return path.clone();
        }
        if !self.aged.contains_key(subject.id) {
            // Secondary figure: archetype as soon as the band changes.
            return String::new();
        }
        // Great figure: the most recent picture whose band does not exceed the current age.
        let current = band_index(band);
        let mut best = (None, String::new());
        for (candidate_band, path) in &candidates {
            let index = band_index(candidate_band);
            if index <= current && index > best.0 {
                best = (index, path.clone());
            }
        }
        best.1
    }

    /// Closest existing archetype: rank then rank fallbacks, band then band fallbacks, culture
    /// then `any`, then the other cultures; face = hash of the id.
    fn archetype(
        &self,
        id: &str,
        sex: &str,
        rank: &str,
        band: &str,
        culture: &str,
    ) -> Option<(String, String, String, String)> {
        let ranks = chain(rank, &self.config.fallbacks.rank);
        let bands = if band == "child" {
            vec![band.to_owned()]
        } else {
            chain(band, &self.config.fallbacks.band)
        };
        let mut cultures = vec![culture.to_owned(), "any".to_owned()];
        for entry in &self.config.cultures {
            if !cultures.contains(&entry.id) {
                cultures.push(entry.id.clone());
            }
        }
        let lineage = fnv1a(id);
        let dir = if self.config.archetype_dir.is_empty() {
            "portraits/archetypes"
        } else {
            self.config.archetype_dir.as_str()
        };
        for culture_id in &cultures {
            for rank_id in &ranks {
                for band_id in &bands {
                    let faces = self.faces(rank_id, sex, band_id, culture_id);
                    for offset in 0..faces {
                        let face = (lineage + offset) % faces;
                        let path =
                            format!("{dir}/{rank_id}_{sex}_{band_id}_{culture_id}_{face}.jpg");
                        if self.exists(&path) {
                            return Some((
                                path,
                                rank_id.clone(),
                                band_id.clone(),
                                culture_id.clone(),
                            ));
                        }
                    }
                }
            }
        }
        None
    }

    /// Picture chosen for the current state of the character (`LivingPortrait.resolve`).
    pub fn resolve(&self, subject: &Subject) -> Resolved {
        let band = self.age_band(subject.age);
        let rank = self.rank_of(subject);
        let culture = self.culture_of(subject.faction);
        let mut out = Resolved {
            rank: rank.clone(),
            band: band.clone(),
            culture: culture.clone(),
            headwear: "none".to_owned(),
            ..default()
        };
        let historical = self.historical(subject, &band);
        if !historical.is_empty() {
            out.kind = if historical.contains("/aged/") {
                "aged"
            } else {
                "fixed"
            }
            .to_owned();
            out.path = historical;
            return out;
        }
        if self.config.cells.is_empty() {
            return out;
        }
        let sex = if subject.sex == "female" {
            "female"
        } else {
            "male"
        };
        let archetype = self.archetype(subject.id, sex, &rank, &band, &culture);
        // Incomplete bank: a historical figure keeps his fixed portrait rather than the dress of
        // another cultural area.
        let fixed = format!("portraits/{}.png", subject.id);
        if !subject.id.is_empty()
            && self.exists(&fixed)
            && archetype
                .as_ref()
                .is_none_or(|(_, _, _, c)| *c != culture && c != "any")
        {
            out.path = fixed;
            out.kind = "fixed".to_owned();
            return out;
        }
        if let Some((path, rank_id, band_id, culture_id)) = archetype {
            out.headwear = self.headwear(&rank_id);
            out.path = path;
            out.rank = rank_id;
            out.band = band_id;
            out.culture = culture_id;
            out.kind = "archetype".to_owned();
            out.mirrored = (fnv1a(subject.id) >> 11) & 1 == 1;
        }
        out
    }

    /// Marks of the character (`LivingPortrait.marks_for`).
    pub fn marks(&self, subject: &Subject, resolved: &Resolved) -> Marks {
        let default = PortraitContext::default();
        let context = subject.context.unwrap_or(&default);
        let has = |list: &[String]| context.trait_ids.iter().any(|t| list.contains(t));
        let dead = !subject.alive;
        let mut marks = Marks {
            dead,
            captive: context.captive,
            sick: has(&self.config.marks.sick_traits),
            wounded: has(&self.config.marks.wound_traits),
            mourning: !dead
                && context
                    .years_since_bereavement
                    .is_some_and(|years| years <= self.config.marks.mourning_years.max(1)),
            ..Marks::default()
        };
        let archetype = resolved.kind == "archetype";
        if resolved.rank == "sovereign" && resolved.headwear != "crown" {
            marks.crown = archetype || !["ruler", "consort"].contains(&context.data_role.as_str());
        }
        if resolved.rank == "prelate" && resolved.headwear != "mitre" {
            marks.mitre = archetype || context.data_role != "prelate";
        }
        marks
    }
}

/// Tooltip text of the marks (« Défunt · Captif… »), empty when none.
pub fn marks_text(marks: &Marks, female: bool) -> String {
    let f = |m: &'static str, fe: &'static str| if female { fe } else { m };
    let mut parts: Vec<&str> = Vec::new();
    if marks.dead {
        parts.push(f("Défunt", "Défunte"));
    }
    if marks.captive {
        parts.push(f("Captif", "Captive"));
    }
    if marks.sick {
        parts.push(f("Maladif", "Maladive"));
    }
    if marks.wounded {
        parts.push(f("Blessé", "Blessée"));
    }
    if marks.mourning {
        parts.push("En deuil");
    }
    parts.join(" · ")
}

/// Frame colors (outer, inner filet) by rank; muted for the dead (`frame_colors`).
pub fn frame_colors(frame: &str, dead: bool) -> (Color, Color) {
    const OR: Color = Color::srgb(0.949, 0.761, 0.188);
    const AZUR: Color = Color::srgb(0.122, 0.227, 0.576);
    const POURPRE: Color = Color::srgb(0.42, 0.165, 0.478);
    if dead {
        return (style::INK_SOFT, Color::srgb(0.55, 0.52, 0.48));
    }
    match frame {
        "sovereign" => (OR, AZUR),
        "prelate" => (OR, POURPRE),
        "knight" => (style::INK, OR),
        "burgher" => (style::INK, style::INK_SOFT),
        _ => (OR, style::INK),
    }
}

/// Spawns the framed living portrait of `subject` at `size` px; falls back to the arms of the
/// house or faction, then the initials. Returns the entity of the frame.
pub fn spawn_portrait(
    parent: &mut Kids,
    theme: &UiTheme,
    assets: &AssetServer,
    bank: &PortraitBank,
    subject: &Subject,
    size: f32,
) -> Entity {
    let resolved = bank.resolve(subject);
    let marks = bank.marks(subject, &resolved);
    let (outer, inner) = frame_colors(&bank.frame_style(&resolved.rank), marks.dead);
    let unit = size.max(24.0) / 64.0;
    let width = (3.0 * unit).round().clamp(1.0, 8.0);
    let female = subject.sex == "female";
    let tooltip = marks_text(&marks, female);
    let arms = format!("heraldry/{}.png", subject.faction);
    let mut frame = parent.spawn((
        Name::new(format!("court:portrait:{}", subject.id)),
        Node {
            width: Val::Px(size),
            height: Val::Px(size),
            flex_shrink: 0.0,
            border: UiRect::all(Val::Px(width)),
            padding: UiRect::all(Val::Px(1.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BorderColor::all(outer),
        BackgroundColor(style::PARCHMENT_DARK),
    ));
    if !tooltip.is_empty() {
        frame.insert(Tooltip::new(&tooltip));
    }
    frame.with_children(|inside| {
        // Inner filet.
        inside
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    border: UiRect::all(Val::Px(1.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    overflow: Overflow::clip(),
                    ..default()
                },
                BorderColor::all(inner),
            ))
            .with_children(|picture| {
                if !resolved.path.is_empty() {
                    // Marks of the picture alone (`portrait_marks.gdshader`); an unmarked picture
                    // stays a plain image.
                    let params = marks.shader_params();
                    let mut node = picture.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        ImageNode {
                            flip_x: resolved.mirrored,
                            ..ImageNode::new(assets.load(resolved.path.clone()))
                        },
                    ));
                    if params != Vec3::ZERO {
                        node.insert(PortraitMarked {
                            params,
                            flip: resolved.mirrored,
                        });
                    }
                } else if bank.exists(&arms) {
                    picture.spawn((
                        Node {
                            width: Val::Percent(60.0),
                            height: Val::Percent(60.0),
                            ..default()
                        },
                        ImageNode::new(assets.load(arms)),
                    ));
                } else {
                    let initials: String = subject
                        .name
                        .split_whitespace()
                        .filter(|w| w.chars().next().is_some_and(char::is_uppercase))
                        .filter_map(|w| w.chars().next())
                        .take(2)
                        .collect();
                    let initials = if initials.is_empty() {
                        "?".to_owned()
                    } else {
                        initials
                    };
                    picture.spawn((
                        Text::new(initials),
                        theme.title_font((size * 0.36).max(10.0)),
                        TextColor(style::INK_SOFT),
                    ));
                }
                spawn_marks(picture, theme, &marks, size);
            });
    });
    frame.id()
}

fn spawn_marks(picture: &mut Kids, theme: &UiTheme, marks: &Marks, size: f32) {
    let sable = Color::srgba(0.1, 0.1, 0.1, 0.78);
    let absolute = |left: Val, top: Val, width: Val, height: Val| Node {
        position_type: PositionType::Absolute,
        left,
        top,
        width,
        height,
        ..default()
    };
    if marks.captive {
        // Iron bars across the picture.
        for i in 0..4 {
            picture.spawn((
                Name::new("court:portrait:bar"),
                absolute(
                    Val::Percent(100.0 * (i as f32 + 0.5) / 4.0),
                    Val::Px(0.0),
                    Val::Px((size * 0.04).max(1.5)),
                    Val::Percent(100.0),
                ),
                BackgroundColor(sable),
            ));
        }
        picture.spawn((
            absolute(
                Val::Px(0.0),
                Val::Percent(30.0),
                Val::Percent(100.0),
                Val::Px((size * 0.028).max(1.0)),
            ),
            BackgroundColor(sable),
        ));
    }
    if marks.wounded {
        picture.spawn((
            Name::new("court:portrait:wound"),
            absolute(
                Val::Percent(10.0),
                Val::Percent(74.0),
                Val::Percent(22.0),
                Val::Px((size * 0.06).max(2.0)),
            ),
            BackgroundColor(Color::srgb(0.69, 0.094, 0.169)),
        ));
    }
    if marks.mourning {
        picture.spawn((
            Name::new("court:portrait:mourning"),
            absolute(
                Val::Px(0.0),
                Val::Px(0.0),
                Val::Percent(26.0),
                Val::Percent(26.0),
            ),
            BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.92)),
        ));
    }
    let glyph = if marks.crown {
        Some(("♛", Color::srgb(0.949, 0.761, 0.188)))
    } else if marks.mitre {
        Some(("♗", Color::srgb(0.96, 0.945, 0.9)))
    } else {
        None
    };
    if let Some((text, color)) = glyph {
        picture.spawn((
            Name::new("court:portrait:headwear"),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            children![(
                Text::new(text),
                theme.text_font(false, false, (size * 0.26).clamp(10.0, 36.0)),
                TextColor(color),
            )],
        ));
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn marks_shader_params_follow_portrait_frame() {
        assert_eq!(Marks::default().shader_params(), Vec3::ZERO);
        let dead = Marks {
            dead: true,
            ..Marks::default()
        };
        assert_eq!(dead.shader_params(), Vec3::new(1.0, 0.0, 0.0));
        let sick_captive = Marks {
            sick: true,
            captive: true,
            ..Marks::default()
        };
        assert_eq!(sick_captive.shader_params(), Vec3::new(0.0, 0.8, 1.0));
    }

    use super::*;

    fn bank() -> PortraitBank {
        let root = crate::paths::find_root().expect("repository root");
        PortraitBank::load(&root)
    }

    fn subject<'a>(id: &'a str, age: i32, context: &'a PortraitContext) -> Subject<'a> {
        Subject {
            id,
            name: "Test",
            sex: "male",
            age,
            birth_year: 1337 - age,
            alive: true,
            faction: "fac_france",
            house: "Valois",
            title: "",
            spouse: "",
            context: Some(context),
        }
    }

    #[test]
    fn fnv_matches_the_godot_hash() {
        // Reference values of FNV-1a 32 bits.
        assert_eq!(fnv1a(""), 2_166_136_261);
        assert_eq!(fnv1a("a"), 0xE40C_292C);
    }

    #[test]
    fn age_bands_follow_the_data() {
        let bank = bank();
        assert_eq!(bank.age_band(8), "child");
        assert_eq!(bank.age_band(22), "young");
        assert_eq!(bank.age_band(40), "adult");
        assert_eq!(bank.age_band(70), "old");
    }

    #[test]
    fn ruler_is_sovereign_and_child_is_child() {
        let bank = bank();
        let context = PortraitContext {
            ruler: "chr_a".to_owned(),
            ..default()
        };
        assert_eq!(bank.rank_of(&subject("chr_a", 40, &context)), "sovereign");
        assert_eq!(bank.rank_of(&subject("chr_b", 9, &context)), "child");
        let mut spouse = subject("chr_c", 35, &context);
        spouse.spouse = "chr_a";
        assert_eq!(bank.rank_of(&spouse), "sovereign");
        let mut bishop = subject("chr_d", 50, &context);
        bishop.title = "Évêque de Paris";
        assert_eq!(bank.rank_of(&bishop), "prelate");
    }

    #[test]
    fn born_in_play_gets_an_archetype_that_exists() {
        let bank = bank();
        let context = PortraitContext::default();
        let subject = subject("chr_born_in_play_42", 30, &context);
        let resolved = bank.resolve(&subject);
        assert_eq!(resolved.kind, "archetype", "{resolved:?}");
        assert!(bank.exists(&resolved.path));
        // Deterministic.
        assert_eq!(resolved, bank.resolve(&subject));
    }

    #[test]
    fn historical_figure_ages_into_his_old_variant() {
        let bank = bank();
        let context = PortraitContext {
            data_role: "ruler".to_owned(),
            ruler: "chr_philippe_vi".to_owned(),
            ..default()
        };
        let mut philip = subject("chr_philippe_vi", 60, &context);
        philip.birth_year = 1293;
        let resolved = bank.resolve(&philip);
        assert_eq!(resolved.kind, "aged", "{resolved:?}");
        assert!(resolved.path.contains("chr_philippe_vi_old"));
        philip.age = 44;
        let resolved = bank.resolve(&philip);
        assert_eq!(resolved.kind, "fixed", "{resolved:?}");
    }

    #[test]
    fn marks_come_from_traits_and_context() {
        let bank = bank();
        let context = PortraitContext {
            captive: true,
            trait_ids: vec!["trait_sickly".to_owned(), "trait_maimed".to_owned()],
            years_since_bereavement: Some(0),
            ..default()
        };
        let subject = subject("chr_x", 30, &context);
        let resolved = bank.resolve(&subject);
        let marks = bank.marks(&subject, &resolved);
        assert!(marks.captive && marks.sick && marks.wounded && marks.mourning);
        assert!(!marks.dead && !marks.crown);
        assert_eq!(
            marks_text(&marks, false),
            "Captif · Maladif · Blessé · En deuil"
        );
    }

    #[test]
    fn crown_is_drawn_only_when_the_picture_has_none() {
        let bank = bank();
        let context = PortraitContext {
            ruler: "chr_x".to_owned(),
            ..default()
        };
        let subject = subject("chr_x", 30, &context);
        let resolved = bank.resolve(&subject);
        let marks = bank.marks(&subject, &resolved);
        assert_eq!(resolved.rank, "sovereign");
        if resolved.kind == "archetype" {
            assert_eq!(marks.crown, resolved.headwear != "crown");
        }
        // A fixed portrait of someone who is not a data ruler gets the crown drawn.
        let mut fixed = resolved.clone();
        fixed.kind = "fixed".to_owned();
        fixed.headwear = "none".to_owned();
        assert!(bank.marks(&subject, &fixed).crown);
    }
}
