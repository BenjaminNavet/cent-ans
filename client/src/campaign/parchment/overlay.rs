//! 2D layer of the parchment view (port of `ParchmentOverlay`): drawn over the map at the camera's
//! projection, opacity = parchment weight. Four stacked layers: sea ornaments (painted sirens,
//! drawn ships), ink town vignettes with the owner's pennant (and the occupant's below it), realm
//! names in spaced arched capitals and italic province names (from mid-fade, replacing the 3D
//! labels), heraldic army tokens. Every node ignores the pointer: selection stays the 3D one.

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use bevy::ui::UiTransform;

use super::decor::{Decor, SeaKind};
use super::glyphs::{self, GlyphSpec};
use super::realms::{self, ProvinceIn, Realm};
use crate::app::AppState;
use crate::campaign::armies::ArmyViews;
use crate::campaign::armies::SelectedArmy;
use crate::campaign::camera::zoom::ZoomView;
use crate::campaign::provinces::data::ProvinceMap;
use crate::campaign::provinces::overlay::OverlayState;
use crate::campaign::terrain::surface::GroundHeight;
use crate::sim::Sim;
use crate::ui::kit::UiCamera;

/// Camera distance of scale 1 for vignettes and tokens.
pub const REF_DISTANCE: f32 = 1400.0;
/// Town cell (screen px at scale 1): one vignette per cell, capitals first.
pub const TOWN_CELL: f32 = 30.0;

/// View scale of vignettes and tokens (`clamp(REF / distance, 0.7, 1.6)`).
pub fn view_scale(distance: f32) -> f32 {
    (REF_DISTANCE / distance.max(1.0)).clamp(0.7, 1.6)
}

/// Opacity of texts (from mid-fade) and of tokens, from the parchment weight.
pub fn text_alpha(weight: f32) -> f32 {
    smoothstep(0.45, 0.85, weight)
}

pub fn token_alpha(weight: f32) -> f32 {
    smoothstep(0.4, 0.75, weight)
}

pub fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Logical pixels of a `FontSize::Px` (0 for the viewport-relative variants, unused here).
trait FontPx {
    fn px(self) -> f32;
}

impl FontPx for bevy::text::FontSize {
    fn px(self) -> f32 {
        match self {
            bevy::text::FontSize::Px(v) => v,
            _ => 0.0,
        }
    }
}

fn rgb(c: [f32; 3], a: f32) -> Color {
    Color::srgba(c[0], c[1], c[2], a)
}

/// Painted ornament of the catalogue (`display` of `parchment_ornaments.json`).
#[derive(Clone)]
pub struct PaintedOrnament {
    pub image: Handle<Image>,
    pub size: Vec2,
    pub anchor: Vec2,
    pub height: f32,
    pub faces_left: bool,
}

/// Glyph textures and the placed decor, built once.
#[derive(Resource, Clone)]
pub struct OverlayAssets {
    pub glyphs: HashMap<&'static str, (GlyphSpec, Handle<Image>)>,
    pub monsters: Vec<PaintedOrnament>,
    pub ships: Vec<PaintedOrnament>,
    pub decor: Decor,
    pub pennant_units: f32,
    /// Stance-cue colours (`stance_cues.json`): `self`, `enemy`, `friend` borders, army rings.
    pub cue_border: HashMap<String, Color>,
    pub cue_ring: HashMap<String, Color>,
}

/// Category of a stance key (`stance_cues.json` `categories`).
pub fn cue_category(stance: &str, is_player: bool) -> &'static str {
    if is_player {
        return "self";
    }
    match stance {
        "war" => "enemy",
        "ally" | "vassal" => "friend",
        _ => "other",
    }
}

/// Counters for the dump.
#[derive(Resource, Default, Debug, Clone)]
pub struct OverlayStats {
    pub weight: f32,
    pub scale: f32,
    /// Realm names drawn this frame (all realms: `realms_total`).
    pub realms: Vec<String>,
    pub realms_total: usize,
    pub sea_shown: usize,
    pub towns: usize,
    pub towns_shown: usize,
    pub towns_occupied: usize,
    pub provinces_shown: usize,
    pub letters: usize,
    pub tokens_shown: usize,
    pub rebuilds: u32,
}

#[derive(Component)]
pub struct OverlayRoot;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    Sea,
    Towns,
    Names,
    Tokens,
}

#[derive(Component)]
pub struct SeaNode(pub usize);

#[derive(Component)]
pub struct TownNode {
    pub px: Vec2,
    pub capital: bool,
}

#[derive(Component)]
pub struct TownPart(pub TownPartKind);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TownPartKind {
    Glyph,
    Pole,
    Pennant,
    Edge,
    OccupantPole,
    OccupantPennant,
    OccupantEdge,
}

#[derive(Component)]
pub struct RealmLetter {
    pub realm: usize,
    pub index: usize,
}

#[derive(Component)]
pub struct ProvinceName {
    pub px: Vec2,
}

#[derive(Component)]
pub struct Token(pub String);

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum TokenPart {
    Disc,
    Ring,
    Pulse,
    Arms,
    Men,
}

/// Names and towns built from the simulation; rebuilt when owners or places change.
#[derive(Resource, Default)]
pub struct OverlayContent {
    pub key: Vec<String>,
    pub realms: Vec<Realm>,
}

/// Spawns the root and its four layers.
pub fn spawn_root(mut commands: Commands) {
    commands
        .spawn((
            Name::new("ParchmentOverlay"),
            OverlayRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            GlobalZIndex(-2),
            Visibility::Hidden,
            Pickable::IGNORE,
            DespawnOnExit(AppState::Campaign),
        ))
        .with_children(|root| {
            for layer in [Layer::Sea, Layer::Towns, Layer::Names, Layer::Tokens] {
                root.spawn((
                    layer,
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    Pickable::IGNORE,
                ));
            }
        });
}

fn absolute() -> Node {
    Node {
        position_type: PositionType::Absolute,
        ..default()
    }
}

/// Sea ornaments, once the decor is ready.
pub fn spawn_sea(
    mut commands: Commands,
    assets: Option<Res<OverlayAssets>>,
    layers: Query<(Entity, &Layer)>,
    existing: Query<(), With<SeaNode>>,
) {
    let Some(assets) = assets else {
        return;
    };
    if !existing.is_empty() {
        return;
    }
    let Some((layer, _)) = layers.iter().find(|(_, l)| **l == Layer::Sea) else {
        return;
    };
    for (i, item) in assets.decor.items.iter().enumerate() {
        let image = match (item.kind, item.painted) {
            (SeaKind::Ship, Some(k)) => assets.ships.get(k).map(|o| o.image.clone()),
            (_, Some(k)) => assets.monsters.get(k).map(|o| o.image.clone()),
            (kind, None) => assets.glyphs.get(kind.name()).map(|g| g.1.clone()),
        };
        let Some(image) = image else {
            continue;
        };
        commands.entity(layer).with_child((
            Name::new(format!("parchment:sea:{}", item.kind.name())),
            SeaNode(i),
            absolute(),
            ImageNode::new(image),
            UiTransform::default(),
            Visibility::Hidden,
            Pickable::IGNORE,
        ));
    }
}

type ContentFilter = Or<(With<TownNode>, With<RealmLetter>, With<ProvinceName>)>;

/// Rebuilds realm names, province names and towns when the owners or places change.
#[allow(clippy::too_many_arguments)]
pub fn rebuild_content(
    mut commands: Commands,
    sim: Res<Sim>,
    map: Option<Res<ProvinceMap>>,
    overlay: Option<Res<OverlayState>>,
    places: Option<Res<crate::campaign::markers::places::Places>>,
    assets: Option<Res<OverlayAssets>>,
    theme: Option<Res<crate::ui::kit::UiTheme>>,
    mut content: ResMut<OverlayContent>,
    mut stats: ResMut<OverlayStats>,
    layers: Query<(Entity, &Layer)>,
    old: Query<Entity, ContentFilter>,
) {
    let (Some(map), Some(overlay), Some(assets), Some(theme)) = (map, overlay, assets, theme)
    else {
        return;
    };
    let places = places.filter(|p| !p.places.is_empty());
    let mut key = overlay.view.owners.clone();
    key.extend(overlay.view.controllers.iter().cloned());
    if let Some(places) = &places {
        key.extend(places.owners.iter().cloned());
        key.extend(places.places.iter().map(|p| p.controller.clone()));
    }
    key.push(overlay.view.viewer.clone());
    if content.key == key && !content.key.is_empty() {
        return;
    }
    content.key = key;
    stats.rebuilds += 1;
    for entity in &old {
        commands.entity(entity).despawn();
    }
    let layer_of = |want: Layer| layers.iter().find(|(_, l)| **l == want).map(|(e, _)| e);
    let (Some(towns_layer), Some(names_layer)) = (layer_of(Layer::Towns), layer_of(Layer::Names))
    else {
        return;
    };
    // Realms and province names.
    let provinces: Vec<ProvinceIn> = map
        .provinces()
        .iter()
        .map(|p| ProvinceIn {
            id: p.id.as_str(),
            centroid: Vec2::from(p.centroid),
            area: p.area_px as f32,
            neighbors: &p.neighbors,
        })
        .collect();
    let owner_of: HashMap<&str, &str> = map
        .provinces()
        .iter()
        .map(|p| (p.id.as_str(), overlay.view.owner_of(p.index as u16)))
        .collect();
    let data = sim.data.clone();
    let name_of = |faction: &str| -> String {
        data.as_ref()
            .and_then(|d| d.factions.get(faction))
            // Full title as Godot (`faction_info.name`): « ROYAUME DE FRANCE ».
            .map_or_else(|| faction.to_string(), |f| f.name.display.clone())
    };
    content.realms = realms::build_realms(&provinces, &owner_of, name_of);
    let roman = theme.font_title.clone();
    let italic = theme.font_title_italic.clone();
    let shadow = |a: f32| TextShadow {
        offset: Vec2::new(1.0, 1.0),
        color: rgb(glyphs::PAPER, a),
    };
    for (r, realm) in content.realms.iter().enumerate() {
        for (i, ch) in realm.name.chars().enumerate() {
            commands.entity(names_layer).with_child((
                RealmLetter { realm: r, index: i },
                absolute(),
                Text::new(ch.to_string()),
                TextFont {
                    font: roman.clone(),
                    font_size: realm.size.into(),
                    ..default()
                },
                TextColor(Color::NONE),
                shadow(0.0),
                UiTransform::default(),
                Visibility::Hidden,
                Pickable::IGNORE,
            ));
        }
    }
    let mut by_area: Vec<_> = map.provinces().iter().collect();
    by_area.sort_by(|a, b| b.area_px.total_cmp(&a.area_px).then(a.index.cmp(&b.index)));
    for province in by_area {
        commands.entity(names_layer).with_child((
            ProvinceName {
                px: Vec2::from(province.centroid),
            },
            absolute(),
            Text::new(realms::short_name(&province.name).to_string()),
            TextFont {
                font: italic.clone(),
                font_size: 15.0.into(),
                ..default()
            },
            TextColor(Color::NONE),
            shadow(0.0),
            Visibility::Hidden,
            Pickable::IGNORE,
        ));
    }
    // Towns: cities only, capitals first.
    let Some(places) = places else {
        stats.towns = 0;
        return;
    };
    let Some((town_spec, town_image)) = assets.glyphs.get("town").cloned() else {
        return;
    };
    let pennant = assets.glyphs.get("pennant").map(|g| g.1.clone());
    let edge = assets.glyphs.get("pennant_edge").map(|g| g.1.clone());
    let viewer = overlay.view.viewer.clone();
    let heraldic = |faction: &str| {
        overlay
            .heraldic
            .get(faction)
            .map_or(rgb(glyphs::INK, 1.0), |c| Color::Srgba(c.with_alpha(1.0)))
    };
    let edge_of = |faction: &str| -> Option<Color> {
        if viewer.is_empty() || faction.is_empty() {
            return None;
        }
        let cue = cue_category(overlay.view.stance_of(faction), faction == viewer);
        assets.cue_border.get(cue).copied()
    };
    let mut cities: Vec<(usize, bool)> = places
        .places
        .iter()
        .enumerate()
        .filter(|(_, p)| p.kind == "city")
        .map(|(i, p)| {
            let index = map.index_of_id(&p.province);
            let capital = map
                .province(index)
                .is_some_and(|f| Vec2::from(f.capital_px).distance(p.px) < 6.0);
            (i, capital)
        })
        .collect();
    cities.sort_by_key(|(_, capital)| !*capital);
    stats.towns = cities.len();
    stats.towns_occupied = 0;
    for (i, capital) in cities {
        let place = &places.places[i];
        let owner = places.owners.get(i).cloned().unwrap_or_default();
        let controller = place.controller.clone();
        let occupied = !controller.is_empty() && controller != owner;
        stats.towns_occupied += usize::from(occupied);
        let owner_color = if owner.is_empty() {
            rgb(glyphs::INK, 1.0)
        } else {
            heraldic(&owner)
        };
        let mut town = commands.spawn((
            Name::new(format!("parchment:town:{}", place.id)),
            TownNode {
                px: place.px,
                capital,
            },
            absolute(),
            Visibility::Hidden,
            Pickable::IGNORE,
        ));
        town.with_children(|t| {
            t.spawn((
                TownPart(TownPartKind::Glyph),
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                ImageNode::new(town_image.clone()),
                Pickable::IGNORE,
            ));
            t.spawn((
                TownPart(TownPartKind::Pole),
                absolute(),
                BackgroundColor(rgb(glyphs::INK, 1.0)),
                Pickable::IGNORE,
            ));
            if let Some(pennant) = &pennant {
                t.spawn((
                    TownPart(TownPartKind::Pennant),
                    absolute(),
                    ImageNode::new(pennant.clone()).with_color(owner_color),
                    Pickable::IGNORE,
                ));
            }
            if let (Some(edge), Some(color)) = (&edge, edge_of(&owner)) {
                t.spawn((
                    TownPart(TownPartKind::Edge),
                    absolute(),
                    ImageNode::new(edge.clone()).with_color(color),
                    Pickable::IGNORE,
                ));
            }
            if occupied {
                t.spawn((
                    TownPart(TownPartKind::OccupantPole),
                    absolute(),
                    BackgroundColor(rgb(glyphs::INK, 1.0)),
                    Pickable::IGNORE,
                ));
                if let Some(pennant) = &pennant {
                    t.spawn((
                        TownPart(TownPartKind::OccupantPennant),
                        absolute(),
                        ImageNode::new(pennant.clone()).with_color(heraldic(&controller)),
                        Pickable::IGNORE,
                    ));
                }
                if let (Some(edge), Some(color)) = (&edge, edge_of(&controller)) {
                    t.spawn((
                        TownPart(TownPartKind::OccupantEdge),
                        absolute(),
                        ImageNode::new(edge.clone()).with_color(color),
                        Pickable::IGNORE,
                    ));
                }
            }
        });
        let id = town.id();
        commands.entity(towns_layer).add_child(id);
    }
    let _ = town_spec;
}

/// Keeps one token per army shown on the map.
pub fn sync_tokens(
    mut commands: Commands,
    views: Option<Res<ArmyViews>>,
    theme: Option<Res<crate::ui::kit::UiTheme>>,
    asset_server: Res<AssetServer>,
    layers: Query<(Entity, &Layer)>,
    tokens: Query<(Entity, &Token)>,
) {
    let (Some(views), Some(theme)) = (views, theme) else {
        return;
    };
    if !views.is_changed() && !tokens.is_empty() {
        return;
    }
    let Some((layer, _)) = layers.iter().find(|(_, l)| **l == Layer::Tokens) else {
        return;
    };
    let wanted: HashSet<String> = views.0.iter().map(|v| v.id.to_string()).collect();
    let mut have = HashSet::new();
    for (entity, token) in &tokens {
        if wanted.contains(&token.0) {
            have.insert(token.0.clone());
        } else {
            commands.entity(entity).despawn();
        }
    }
    let root = crate::paths::find_root().ok();
    for view in &views.0 {
        let id = view.id.to_string();
        if have.contains(&id) {
            continue;
        }
        let faction = view.faction.to_string();
        let arms = root
            .as_ref()
            .map(|r| crate::paths::asset_root(r).join(format!("heraldry/{faction}.png")))
            .filter(|p| p.is_file())
            .map(|_| asset_server.load::<Image>(format!("heraldry/{faction}.png")));
        let token = commands
            .spawn((
                Name::new(format!("parchment:token:{id}")),
                Token(id),
                absolute(),
                Visibility::Hidden,
                Pickable::IGNORE,
            ))
            .with_children(|t| {
                for part in [TokenPart::Pulse, TokenPart::Ring, TokenPart::Disc] {
                    t.spawn((
                        part,
                        Node {
                            position_type: PositionType::Absolute,
                            border_radius: BorderRadius::MAX,
                            ..default()
                        },
                        BorderColor::all(Color::NONE),
                        BackgroundColor(Color::NONE),
                        Pickable::IGNORE,
                    ));
                }
                if let Some(arms) = arms {
                    t.spawn((
                        TokenPart::Arms,
                        absolute(),
                        ImageNode::new(arms),
                        Pickable::IGNORE,
                    ));
                }
                t.spawn((
                    TokenPart::Men,
                    absolute(),
                    Text::new(""),
                    TextFont {
                        font: theme.font_title.clone(),
                        font_size: 13.0.into(),
                        ..default()
                    },
                    TextColor(Color::NONE),
                    TextShadow {
                        offset: Vec2::new(1.0, 1.0),
                        color: Color::NONE,
                    },
                    Pickable::IGNORE,
                ));
            })
            .id();
        commands.entity(layer).add_child(token);
    }
}

/// Men count as on the army plates (`ArmyPlate.format_men`: thousands separated by a space).
pub fn format_men(men: u32) -> String {
    if men >= 1000 {
        format!("{} {:03}", men / 1000, men % 1000)
    } else {
        men.to_string()
    }
}

fn set_node(node: &mut Mut<Node>, pos: Vec2, size: Option<Vec2>) {
    let left = Val::Px(pos.x);
    let top = Val::Px(pos.y);
    if node.left != left {
        node.left = left;
    }
    if node.top != top {
        node.top = top;
    }
    if let Some(size) = size {
        let (w, h) = (Val::Px(size.x), Val::Px(size.y));
        if node.width != w {
            node.width = w;
        }
        if node.height != h {
            node.height = h;
        }
    }
}

fn show(visibility: &mut Mut<Visibility>, on: bool) {
    let want = if on {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if **visibility != want {
        **visibility = want;
    }
}

/// Screen projection helper (UI logical px; `None` behind the camera).
struct Projector<'a> {
    camera: &'a Camera,
    transform: &'a GlobalTransform,
    ui_scale: f32,
}

impl Projector<'_> {
    fn at(&self, world: Vec3) -> Option<Vec2> {
        self.camera
            .world_to_viewport(self.transform, world)
            .ok()
            .map(|p| p / self.ui_scale)
    }
}

/// Places every node of the layer for the current view.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn update_overlay(
    zoom: Res<ZoomView>,
    assets: Option<Res<OverlayAssets>>,
    content: Res<OverlayContent>,
    views: Option<Res<ArmyViews>>,
    selected: Option<Res<SelectedArmy>>,
    ui_scale: Res<UiScale>,
    time: Res<Time>,
    ground: GroundHeight,
    cameras: Query<(&Camera, &GlobalTransform), (With<Camera3d>, Without<UiCamera>)>,
    mut stats: ResMut<OverlayStats>,
    mut root: Query<&mut Visibility, With<OverlayRoot>>,
    mut nodes: ParamSet<(
        Query<
            (
                &SeaNode,
                &mut Node,
                &mut ImageNode,
                &mut UiTransform,
                &mut Visibility,
            ),
            Without<OverlayRoot>,
        >,
        Query<(&TownNode, &mut Node, &mut Visibility, &Children), Without<OverlayRoot>>,
        Query<(
            &TownPart,
            &mut Node,
            Option<&mut ImageNode>,
            Option<&mut BackgroundColor>,
        )>,
        Query<
            (
                &RealmLetter,
                &mut Node,
                &mut TextFont,
                &mut TextColor,
                &mut TextShadow,
                &mut UiTransform,
                &mut Visibility,
                &ComputedNode,
            ),
            Without<OverlayRoot>,
        >,
        Query<
            (
                &ProvinceName,
                &mut Node,
                &mut TextFont,
                &mut TextColor,
                &mut TextShadow,
                &mut Visibility,
                &ComputedNode,
            ),
            Without<OverlayRoot>,
        >,
        Query<(&Token, &mut Node, &mut Visibility, &Children), Without<OverlayRoot>>,
        Query<(
            &TokenPart,
            &mut Node,
            Option<&mut BackgroundColor>,
            Option<&mut BorderColor>,
            Option<&mut ImageNode>,
            Option<&mut Text>,
            Option<&mut TextColor>,
            Option<&mut TextShadow>,
            Option<&mut TextFont>,
        )>,
    )>,
) {
    let weight = zoom.strategic;
    stats.weight = weight;
    let Ok(mut root_visibility) = root.single_mut() else {
        return;
    };
    let camera = cameras.iter().find(|(c, _)| c.is_active);
    let (Some(assets), Some((camera, transform))) = (assets, camera) else {
        show(&mut root_visibility, false);
        return;
    };
    if weight <= 0.01 {
        show(&mut root_visibility, false);
        return;
    }
    show(&mut root_visibility, true);
    let Some(viewport) = camera.logical_viewport_size() else {
        return;
    };
    let proj = Projector {
        camera,
        transform,
        ui_scale: ui_scale.0.max(0.1),
    };
    let viewport = viewport / proj.ui_scale;
    let view = Rect::from_corners(Vec2::splat(-60.0), viewport + 60.0);
    let s = view_scale(zoom.distance);
    stats.scale = s;
    let world = |px: Vec2| Vec3::new(px.x, ground.at(px.x, px.y).unwrap_or(0.0), px.y);
    let a = weight;

    // Sea ornaments.
    let mut sea_shown = 0;
    for (sea, mut node, mut image, mut ui_transform, mut visibility) in nodes.p0().iter_mut() {
        let Some(item) = assets.decor.items.get(sea.0) else {
            continue;
        };
        let Some(p) = proj
            .at(Vec3::new(item.px.x, 0.0, item.px.y))
            .filter(|p| view.contains(*p))
        else {
            show(&mut visibility, false);
            continue;
        };
        let u = item.unit * s;
        let painted = match (item.kind, item.painted) {
            (SeaKind::Ship, Some(k)) => assets.ships.get(k),
            (_, Some(k)) => assets.monsters.get(k),
            _ => None,
        };
        let (size, anchor, mirrored) = match painted {
            Some(o) => {
                let height = o.height * u;
                let size = Vec2::new(height * o.size.x / o.size.y.max(1.0), height);
                let mirrored = item.heading_right == o.faces_left;
                let mut anchor = o.anchor;
                if mirrored {
                    anchor.x = 1.0 - anchor.x;
                }
                (size, anchor * size, mirrored)
            }
            None => {
                let Some((spec, _)) = assets.glyphs.get(item.kind.name()) else {
                    continue;
                };
                let k = u / spec.unit;
                let anchor = if item.mirror {
                    Vec2::new(spec.size.x - spec.anchor.x, spec.anchor.y)
                } else {
                    spec.anchor
                };
                (spec.size * k, anchor * k, item.mirror)
            }
        };
        set_node(&mut node, p - anchor, Some(size));
        let scale = Vec2::new(if mirrored { -1.0 } else { 1.0 }, 1.0);
        if ui_transform.scale != scale {
            ui_transform.scale = scale;
        }
        let color = Color::srgba(1.0, 1.0, 1.0, a);
        if image.color != color {
            image.color = color;
        }
        show(&mut visibility, true);
        sea_shown += 1;
    }
    stats.sea_shown = sea_shown;

    // Towns: one vignette per screen cell, capitals first (spawn order).
    let cell = TOWN_CELL * s;
    let mut occupied_cells: HashSet<(i32, i32)> = HashSet::new();
    let mut town_layout: HashMap<Entity, (f32, bool)> = HashMap::new();
    let mut towns_shown = 0;
    {
        let mut towns = nodes.p1();
        let mut list: Vec<_> = towns.iter_mut().collect();
        list.sort_by_key(|(t, ..)| !t.capital);
        for (town, mut node, mut visibility, children) in list {
            let p = proj.at(world(town.px)).filter(|p| view.contains(*p));
            let key = p.map(|p| ((p.x / cell) as i32, (p.y / cell) as i32));
            let Some((p, key)) = p.zip(key).filter(|(_, k)| !occupied_cells.contains(k)) else {
                show(&mut visibility, false);
                continue;
            };
            occupied_cells.insert(key);
            let u = if town.capital { 11.0 } else { 7.5 } * s;
            let k = u / glyphs::TOWN.unit;
            set_node(
                &mut node,
                p - glyphs::TOWN.anchor * k,
                Some(glyphs::TOWN.size * k),
            );
            show(&mut visibility, true);
            towns_shown += 1;
            for child in children.iter() {
                town_layout.insert(child, (u, true));
            }
        }
    }
    stats.towns_shown = towns_shown;
    {
        let length = assets.pennant_units;
        let mut parts = nodes.p2();
        for (entity, (u, _)) in &town_layout {
            let Ok((part, mut node, image, background)) = parts.get_mut(*entity) else {
                continue;
            };
            let u = *u;
            let k = u / glyphs::TOWN.unit;
            // Offsets relative to the vignette's top-left corner, from the anchor.
            let base = glyphs::TOWN.anchor * k;
            let x = base.x + 0.15 * u;
            let top = base.y - 2.3 * u;
            let len = length * u;
            let pole_bottom = base.y - 1.8 * u;
            let pennant_size = |l: f32| Vec2::new(l, l * 0.5);
            let (pos, size) = match part.0 {
                TownPartKind::Glyph => {
                    if let Some(mut image) = image {
                        let c = Color::srgba(1.0, 1.0, 1.0, a);
                        if image.color != c {
                            image.color = c;
                        }
                    }
                    continue;
                }
                TownPartKind::Pole => (Vec2::new(x - 0.6, top), Vec2::new(1.2, pole_bottom - top)),
                TownPartKind::OccupantPole => (
                    Vec2::new(x - 0.6, top),
                    Vec2::new(1.2, (top + len * 0.95 - top).max(pole_bottom - top)),
                ),
                TownPartKind::Pennant | TownPartKind::Edge => {
                    (Vec2::new(x, top), pennant_size(len))
                }
                TownPartKind::OccupantPennant | TownPartKind::OccupantEdge => {
                    (Vec2::new(x, top + len * 0.5), pennant_size(len * 0.8))
                }
            };
            set_node(&mut node, pos, Some(size));
            if let Some(mut image) = image {
                let c = image.color.with_alpha(a);
                if image.color != c {
                    image.color = c;
                }
            }
            if let Some(mut background) = background {
                let c = rgb(glyphs::INK, a);
                if background.0 != c {
                    background.0 = c;
                }
            }
        }
    }

    // Names (from mid-fade).
    let ta = text_alpha(a);
    let mut placed: Vec<Rect> = Vec::new();
    let mut letters = 0;
    {
        let mut query = nodes.p3();
        // Per realm: measured widths (previous frame's layout) then the arch.
        let mut by_realm: HashMap<usize, Vec<(usize, f32)>> = HashMap::new();
        for (letter, _, font, .., computed) in query.iter() {
            let w = computed.size().x * computed.inverse_scale_factor();
            let w = if w > 0.0 {
                w
            } else {
                font.font_size.px() * 0.62
            };
            by_realm
                .entry(letter.realm)
                .or_default()
                .push((letter.index, w));
        }
        let mut layout: HashMap<(usize, usize), (Vec2, f32, f32)> = HashMap::new();
        // Largest realms first (build order); a name that would cross an earlier one is left out
        // (Godot drew them all, unreadable with ~150 factions on the map).
        let mut by_realm: Vec<(usize, Vec<(usize, f32)>)> = by_realm.into_iter().collect();
        by_realm.sort_by_key(|(r, _)| *r);
        for (r, mut list) in by_realm {
            let Some(realm) = content.realms.get(r) else {
                continue;
            };
            list.sort_by_key(|(i, _)| *i);
            let size = (realm.size * s).floor().max(8.0);
            let (Some(pa), Some(pb), Some(pc)) = (
                proj.at(world(realm.a)),
                proj.at(world(realm.b)),
                proj.at(world(realm.c)),
            ) else {
                continue;
            };
            if ta <= 0.01 {
                continue;
            }
            let widths: Vec<f32> = list.iter().map(|(_, w)| *w).collect();
            let arch = realms::arch_layout(pa, pb, pc, &widths, size);
            let mut box_rect = Rect::from_center_size(pc, Vec2::ZERO);
            for (pos, _) in &arch {
                box_rect = box_rect.union_point(*pos);
            }
            let box_rect = Rect {
                min: box_rect.min - size * 0.7,
                max: box_rect.max + size * 0.7,
            };
            if view.intersect(box_rect).is_empty()
                || placed.iter().any(|r| !r.intersect(box_rect).is_empty())
            {
                continue;
            }
            for ((i, w), (pos, angle)) in list.iter().zip(arch) {
                layout.insert((r, *i), (pos, angle, *w));
            }
            placed.push(box_rect);
        }
        for (
            letter,
            mut node,
            mut font,
            mut color,
            mut shadow,
            mut ui_transform,
            mut visibility,
            computed,
        ) in query.iter_mut()
        {
            let Some((pos, angle, w)) = layout.get(&(letter.realm, letter.index)).copied() else {
                show(&mut visibility, false);
                continue;
            };
            let Some(realm) = content.realms.get(letter.realm) else {
                continue;
            };
            let size = (realm.size * s).floor().max(8.0);
            if font.font_size.px() != size {
                font.font_size = size.into();
            }
            let h = computed.size().y * computed.inverse_scale_factor();
            let h = if h > 0.0 { h } else { size * 1.2 };
            set_node(&mut node, pos - Vec2::new(w, h) * 0.5, None);
            let rotation = Rot2::radians(angle);
            if ui_transform.rotation != rotation {
                ui_transform.rotation = rotation;
            }
            let c = Color::srgba(0.62 * 0.75, 0.14 * 0.75, 0.09 * 0.75, 0.95 * ta);
            if color.0 != c {
                color.0 = c;
            }
            let sc = rgb([1.0, 0.95, 0.85], 0.8 * ta);
            if shadow.color != sc {
                shadow.color = sc;
            }
            show(&mut visibility, true);
            letters += 1;
        }
    }
    stats.letters = letters;
    let mut shown: Vec<usize> = Vec::new();
    {
        let query = nodes.p3();
        for (letter, .., visibility, _) in query.iter() {
            if *visibility != Visibility::Hidden && !shown.contains(&letter.realm) {
                shown.push(letter.realm);
            }
        }
    }
    shown.sort_unstable();
    stats.realms_total = content.realms.len();
    stats.realms = shown
        .iter()
        .filter_map(|r| content.realms.get(*r).map(|x| x.name.clone()))
        .collect();
    let mut provinces_shown = 0;
    {
        let size = (15.0 * s).clamp(12.0, 22.0).floor();
        let mut query = nodes.p4();
        for (name, mut node, mut font, mut color, mut shadow, mut visibility, computed) in
            query.iter_mut()
        {
            if ta <= 0.01 {
                show(&mut visibility, false);
                continue;
            }
            let Some(p) = proj.at(world(name.px)).filter(|p| view.contains(*p)) else {
                show(&mut visibility, false);
                continue;
            };
            if font.font_size.px() != size {
                font.font_size = size.into();
            }
            let measured = computed.size() * computed.inverse_scale_factor();
            let w = if measured.x > 0.0 {
                measured.x
            } else {
                size * 6.0
            };
            let rect = Rect::from_corners(
                p - Vec2::new(w * 0.5, size * 0.9),
                p + Vec2::new(w * 0.5, size * 0.3),
            );
            if placed.iter().any(|r| !r.intersect(rect).is_empty()) {
                show(&mut visibility, false);
                continue;
            }
            placed.push(Rect {
                min: rect.min - 3.0,
                max: rect.max + 3.0,
            });
            set_node(&mut node, Vec2::new(rect.min.x, p.y - size * 0.9), None);
            let c = rgb(glyphs::INK, 0.9 * ta);
            if color.0 != c {
                color.0 = c;
            }
            let sc = rgb(glyphs::PAPER, 0.6 * ta);
            if shadow.color != sc {
                shadow.color = sc;
            }
            show(&mut visibility, true);
            provinces_shown += 1;
        }
    }
    stats.provinces_shown = provinces_shown;

    // Army tokens.
    let tka = token_alpha(a);
    let mut tokens_shown = 0;

    let selected_id = selected.and_then(|s| s.0.as_ref().map(|id| id.to_string()));
    let pulse = 0.5 + 0.5 * (time.elapsed_secs() * 4.0).sin();
    let r = 11.0 * s;
    let mut token_parts: HashMap<Entity, (String, bool)> = HashMap::new();
    {
        let mut tokens = nodes.p5();
        for (token, mut node, mut visibility, children) in tokens.iter_mut() {
            let view_entry = views
                .as_ref()
                .and_then(|v| v.0.iter().find(|v| v.id.to_string() == token.0));
            let Some(entry) = view_entry else {
                show(&mut visibility, false);
                continue;
            };
            let px = Vec2::from(entry.point);
            let Some(p) = proj
                .at(world(px))
                .filter(|p| view.contains(*p) && tka > 0.01)
            else {
                show(&mut visibility, false);
                continue;
            };
            set_node(&mut node, p, Some(Vec2::ZERO));
            show(&mut visibility, true);
            tokens_shown += 1;
            for child in children.iter() {
                token_parts.insert(
                    child,
                    (
                        token.0.clone(),
                        selected_id.as_deref() == Some(token.0.as_str()),
                    ),
                );
            }
        }
    }
    stats.tokens_shown = tokens_shown;
    let mut parts = nodes.p6();
    for (entity, (id, selected)) in token_parts {
        let Some(entry) = views
            .as_ref()
            .and_then(|v| v.0.iter().find(|v| v.id.to_string() == id))
        else {
            continue;
        };
        let Ok((part, mut node, background, border, image, text, text_color, shadow, font)) =
            parts.get_mut(entity)
        else {
            continue;
        };
        let faction_color = Color::srgba(entry.color[0], entry.color[1], entry.color[2], tka);
        let cue = if entry.player {
            "self"
        } else if entry.hostile {
            "enemy"
        } else {
            "other"
        };
        let ring = assets.cue_ring.get(cue).copied();
        match part {
            TokenPart::Disc => {
                set_node(&mut node, Vec2::splat(-r), Some(Vec2::splat(2.0 * r)));
                node.border = UiRect::all(Val::Px(1.6));
                if let Some(mut b) = background {
                    b.0 = faction_color;
                }
                if let Some(mut b) = border {
                    *b = BorderColor::all(rgb(glyphs::INK, tka));
                }
            }
            TokenPart::Ring => {
                let (width, color) = match ring {
                    Some(c) => (2.6, c.with_alpha(c.alpha() * tka)),
                    None if entry.player => (1.4, rgb(glyphs::GOLD, tka)),
                    None => (1.4, rgb(glyphs::PAPER, tka)),
                };
                let rr = r + 2.0 + width * 0.5;
                set_node(&mut node, Vec2::splat(-rr), Some(Vec2::splat(2.0 * rr)));
                node.border = UiRect::all(Val::Px(width));
                if let Some(mut b) = border {
                    *b = BorderColor::all(color);
                }
            }
            TokenPart::Pulse => {
                let rr = r + 5.0 + pulse * 2.0 + 1.0;
                set_node(&mut node, Vec2::splat(-rr), Some(Vec2::splat(2.0 * rr)));
                node.border = UiRect::all(Val::Px(2.0));
                if let Some(mut b) = border {
                    *b = BorderColor::all(if selected {
                        rgb(glyphs::GOLD, tka)
                    } else {
                        Color::NONE
                    });
                }
            }
            TokenPart::Arms => {
                let side = r * 1.25;
                set_node(&mut node, Vec2::splat(-side * 0.5), Some(Vec2::splat(side)));
                if let Some(mut image) = image {
                    image.color = Color::srgba(1.0, 1.0, 1.0, tka);
                }
            }
            TokenPart::Men => {
                let fs = (13.0 * s).clamp(12.0, 17.0).floor();
                if let Some(mut font) = font
                    && font.font_size.px() != fs
                {
                    font.font_size = fs.into();
                }
                let men = format_men(entry.men);
                if let Some(mut text) = text
                    && text.0 != men
                {
                    text.0 = men.clone();
                }
                let w = men.chars().count() as f32 * fs * 0.5;
                set_node(&mut node, Vec2::new(-w * 0.5, r + 3.0), None);
                if let Some(mut c) = text_color {
                    c.0 = rgb(glyphs::INK, tka);
                }
                if let Some(mut sh) = shadow {
                    sh.color = rgb(glyphs::PAPER, tka);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scales_and_fades_follow_the_godot_layer() {
        assert_eq!(view_scale(1400.0), 1.0);
        assert_eq!(view_scale(4000.0), 0.7);
        assert_eq!(view_scale(200.0), 1.6);
        assert_eq!(text_alpha(0.4), 0.0);
        assert_eq!(text_alpha(0.9), 1.0);
        assert!(token_alpha(0.6) > 0.0 && token_alpha(0.6) < 1.0);
    }

    #[test]
    fn men_and_cues() {
        assert_eq!(format_men(620), "620");
        assert_eq!(format_men(1234), "1 234");
        assert_eq!(format_men(12_300), "12 300");
        assert_eq!(cue_category("war", false), "enemy");
        assert_eq!(cue_category("vassal", false), "friend");
        assert_eq!(cue_category("war", true), "self");
        assert_eq!(cue_category("neutral", false), "other");
    }
}
