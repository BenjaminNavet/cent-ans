//! Clickable map of the playable factions (game/scripts/ui/faction_map_picker.gd), the
//! « Toutes les factions » tab of the faction screen.
//!
//! Every province of `data/map/provinces.geojson` is painted in the colours of its 1337 owner into
//! one CPU-rasterised image (the ring geometry is filled by scanline, no 43 M pixel id raster is
//! loaded); the raster doubles as the hit grid, so hovering gives the faction sheet card and a
//! click chooses the faction. Filters by kingdom and by rank veil the other factions. A playable
//! faction without a province (the crusaders, who only hold Limassol) is a banner standing at its
//! colony, clickable like a land. Beside the map, the filtered factions are listed by kingdom
//! (landless first, then the biggest kingdoms); hovering a name highlights its land.
//!
//! Improvement over Godot: the list buttons and the banners are kit buttons, so scripts and the
//! agent can click them by name (`fm:<faction>`, `banner:<faction>`, `fmk:<kingdom>`, `fmr:<rank>`).
//! No rule here: the sheets come from `game_api::start_sheets`.

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::ui::RelativeCursorPosition;
use bevy::ui_widgets::Button as WidgetButton;
use bevy::window::PrimaryWindow;
use data_model::map_geo::ProvinceFeature;
use game_api::start_sheets::StartSheet;
use serde_json::{Value, json};

use super::MenuData;
use super::faction_select::FactionChoice;
use crate::app::AppState;
use crate::sim::Sim;
use crate::ui::kit::style::{self, UiType};
use crate::ui::kit::{ButtonPressed, Toggled, UiTheme, toggle_button, typed};

/// Width in pixels of the map raster.
pub const IMAGE_WIDTH: u32 = 900;
/// Margin around the playable lands (share of their extent).
const FRAME_MARGIN: f32 = 0.04;
/// Radius (UI px) of a landless faction's banner disc.
const BANNER_RADIUS: f32 = 9.0;
const SEA: [u8; 4] = [77, 97, 107, 255];
const VEIL: [f32; 3] = [0.18, 0.16, 0.14];
const OUTLINE: [f32; 3] = [0.10, 0.07, 0.04];
const SELECTED_OUTLINE: [u8; 3] = [250, 214, 92];

/// Rank filters (key, label).
pub const RANK_FILTERS: [(&str, &str); 4] = [
    ("", "Tous les rangs"),
    ("kingdom", "Royaumes"),
    ("duchy", "Duchés"),
    ("county", "Comtés"),
];

/// Header of the landless group in the list.
pub const LANDLESS_HEADER: &str = "Sans terre";

/// The 1337 sheets of the playable factions (sovereigns first).
#[derive(Resource, Default)]
pub struct StartSheets {
    pub list: Vec<StartSheet>,
    /// Asset root, to check that a ruler's portrait exists.
    pub asset_root: Option<std::path::PathBuf>,
}

impl StartSheets {
    pub fn get(&self, id: &str) -> Option<&StartSheet> {
        self.list.iter().find(|sheet| sheet.id == id)
    }

    /// Asset path of the portrait of a ruler (`portraits/<character>.png`), when the file exists.
    pub fn portrait(&self, ruler_id: &str) -> Option<String> {
        if ruler_id.is_empty() {
            return None;
        }
        let path = format!("portraits/{ruler_id}.png");
        self.asset_root
            .as_ref()
            .is_some_and(|root| root.join(&path).is_file())
            .then_some(path)
    }
}

/// Kingdom and rank filters of the map.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Filters {
    pub kingdom: String,
    pub rank: String,
}

/// Does `sheet` pass the filters?
pub fn matches(sheet: &StartSheet, filters: &Filters) -> bool {
    (filters.kingdom.is_empty() || sheet.kingdom == filters.kingdom)
        && (filters.rank.is_empty() || sheet.rank == filters.rank)
}

/// Kingdoms present, `(title id, name)` sorted by name.
pub fn kingdoms(sheets: &[StartSheet]) -> Vec<(String, String)> {
    let mut seen: Vec<(String, String)> = Vec::new();
    for sheet in sheets {
        if !sheet.kingdom.is_empty() && !seen.iter().any(|(id, _)| *id == sheet.kingdom) {
            seen.push((sheet.kingdom.clone(), sheet.kingdom_name.clone()));
        }
    }
    seen.sort_by(|a, b| a.1.cmp(&b.1));
    seen
}

/// Factions passing the filters grouped by kingdom name (`""` = landless): landless first, then
/// the kingdoms with the most factions, then by name. Each group keeps the core's order.
pub fn groups(sheets: &[StartSheet], filters: &Filters) -> Vec<(String, Vec<String>)> {
    let mut groups: Vec<(String, Vec<String>)> = Vec::new();
    for sheet in sheets.iter().filter(|s| matches(s, filters)) {
        match groups
            .iter_mut()
            .find(|(name, _)| *name == sheet.kingdom_name)
        {
            Some((_, ids)) => ids.push(sheet.id.clone()),
            None => groups.push((sheet.kingdom_name.clone(), vec![sheet.id.clone()])),
        }
    }
    groups.sort_by(|a, b| {
        (!a.0.is_empty())
            .cmp(&(!b.0.is_empty()))
            .then(b.1.len().cmp(&a.1.len()))
            .then(a.0.cmp(&b.0))
    });
    groups
}

/// Rasterised map: the slot (1-based index into `slots`, 0 = sea) of every pixel.
pub struct MapGeometry {
    pub width: u32,
    pub height: u32,
    /// Slot of each pixel, row-major.
    pub grid: Vec<u16>,
    /// Owner faction id of each slot (may be a non-playable faction).
    pub slots: Vec<String>,
    /// Landless playable factions and the position of their banner in `[0, 1]²` of the image.
    pub banners: Vec<(String, Vec2)>,
    /// Extent (map px) shown.
    pub bounds: Rect,
}

impl MapGeometry {
    /// Rasterises `features` (owners from the sheets' provinces) over the playable lands.
    pub fn build(
        features: &[ProvinceFeature],
        sheets: &[StartSheet],
        banners_at: &[(String, [f32; 2])],
        width: u32,
    ) -> Self {
        let owner_of: HashMap<&str, &str> = sheets
            .iter()
            .flat_map(|s| s.provinces.iter().map(move |p| (p.as_str(), s.id.as_str())))
            .collect();
        let playable = |feature: &ProvinceFeature| {
            owner_of
                .get(feature.id.as_str())
                .is_some_and(|owner| sheets.iter().any(|s| s.id == *owner))
        };
        let mut bounds: Option<Rect> = None;
        let mut grow = |point: Vec2| {
            bounds = Some(match bounds {
                Some(rect) => Rect {
                    min: rect.min.min(point),
                    max: rect.max.max(point),
                },
                None => Rect {
                    min: point,
                    max: point,
                },
            });
        };
        let any_playable = features.iter().any(&playable);
        for feature in features {
            if any_playable && !playable(feature) {
                continue;
            }
            for ring in &feature.rings {
                for point in ring {
                    grow(Vec2::from(*point));
                }
            }
        }
        for (_, at) in banners_at {
            grow(Vec2::from(*at));
        }
        let bounds = bounds.unwrap_or(Rect {
            min: Vec2::ZERO,
            max: Vec2::ONE,
        });
        let size = bounds.size().max(Vec2::splat(1.0));
        let bounds = Rect {
            min: bounds.min - size * FRAME_MARGIN,
            max: bounds.max + size * FRAME_MARGIN,
        };
        let size = bounds.size();
        let scale = width as f32 / size.x;
        let height = ((size.y * scale).ceil() as u32).max(1);
        let mut grid = vec![0u16; (width * height) as usize];
        let mut slots: Vec<String> = Vec::new();
        for feature in features {
            let owner = owner_of
                .get(feature.id.as_str())
                .map_or_else(|| feature.owner.clone(), |owner| (*owner).to_owned());
            slots.push(owner);
            let slot = slots.len() as u16;
            for ring in &feature.rings {
                fill_ring(&mut grid, width, height, ring, bounds.min, scale, slot);
            }
        }
        let banners = banners_at
            .iter()
            .map(|(id, at)| (id.clone(), (Vec2::from(*at) - bounds.min) / size))
            .collect();
        Self {
            width,
            height,
            grid,
            slots,
            banners,
            bounds,
        }
    }

    /// Aspect ratio (width / height) of the framed map.
    pub fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }

    /// Slot at `uv` (`[0, 1]²` of the image), 0 outside the lands.
    pub fn slot_at(&self, uv: Vec2) -> u16 {
        if uv.x < 0.0 || uv.y < 0.0 || uv.x >= 1.0 || uv.y >= 1.0 {
            return 0;
        }
        let x = (uv.x * self.width as f32) as u32;
        let y = (uv.y * self.height as f32) as u32;
        self.grid[(y * self.width + x) as usize]
    }

    /// Playable faction owning the land at `uv` (`None` at sea or on a foreign land).
    pub fn faction_at<'a>(&'a self, uv: Vec2, sheets: &StartSheets) -> Option<&'a str> {
        let slot = self.slot_at(uv);
        let owner = self.slots.get(usize::from(slot).checked_sub(1)?)?;
        sheets.get(owner).map(|_| owner.as_str())
    }

    /// A pixel inside the lands of `faction` (largest run of its widest row), for tests.
    pub fn center_of(&self, faction: &str) -> Option<Vec2> {
        let mut best: Option<(u32, u32, u32)> = None;
        for y in 0..self.height {
            let mut run_start = 0;
            let mut run_slot = 0u16;
            for x in 0..=self.width {
                let slot = if x < self.width {
                    self.grid[(y * self.width + x) as usize]
                } else {
                    0
                };
                if slot != run_slot {
                    let owned = run_slot > 0
                        && self
                            .slots
                            .get(usize::from(run_slot) - 1)
                            .map(String::as_str)
                            == Some(faction);
                    if owned && best.is_none_or(|(len, _, _)| x - run_start > len) {
                        best = Some((x - run_start, (run_start + x) / 2, y));
                    }
                    run_start = x;
                    run_slot = slot;
                }
            }
        }
        best.map(|(_, x, y)| {
            Vec2::new(
                (x as f32 + 0.5) / self.width as f32,
                (y as f32 + 0.5) / self.height as f32,
            )
        })
    }
}

/// Even-odd scanline fill of one ring into `grid`.
fn fill_ring(
    grid: &mut [u16],
    width: u32,
    height: u32,
    ring: &[[f32; 2]],
    origin: Vec2,
    scale: f32,
    slot: u16,
) {
    if ring.len() < 3 {
        return;
    }
    let to_image = |p: &[f32; 2]| Vec2::new((p[0] - origin.x) * scale, (p[1] - origin.y) * scale);
    let points: Vec<Vec2> = ring.iter().map(to_image).collect();
    let (mut min_y, mut max_y) = (f32::MAX, f32::MIN);
    for p in &points {
        min_y = min_y.min(p.y);
        max_y = max_y.max(p.y);
    }
    let first = (min_y.floor().max(0.0)) as u32;
    let last = (max_y.ceil().min(height as f32 - 1.0)).max(0.0) as u32;
    let mut crossings: Vec<f32> = Vec::new();
    for y in first..=last {
        let scan = y as f32 + 0.5;
        crossings.clear();
        for i in 0..points.len() {
            let (a, b) = (points[i], points[(i + 1) % points.len()]);
            if (a.y <= scan) != (b.y <= scan) {
                crossings.push(a.x + (scan - a.y) / (b.y - a.y) * (b.x - a.x));
            }
        }
        crossings.sort_by(f32::total_cmp);
        for pair in crossings.chunks_exact(2) {
            let from = (pair[0] - 0.5).ceil().max(0.0) as i64;
            let to = ((pair[1] - 0.5).floor() as i64).min(width as i64 - 1);
            for x in from..=to {
                grid[(y * width + x as u32) as usize] = slot;
            }
        }
    }
}

/// Everything the map needs while the faction screen is open.
#[derive(Resource)]
pub struct FactionMapState {
    pub geometry: Option<MapGeometry>,
    pub image: Handle<Image>,
    pub filters: Filters,
    /// Faction under the cursor on the map (or highlighted from the list).
    pub hovered: String,
    /// What the raster currently shows, to repaint only on change.
    painted: Option<(Filters, String, String)>,
    /// Counters for the dump.
    pub repaints: u32,
    pub clicks: u32,
    /// The list was built for these filters.
    listed: Option<Filters>,
}

#[derive(Component)]
pub struct MapNode;

#[derive(Component)]
pub struct FactionList;

#[derive(Component)]
pub struct HoverCard;

#[derive(Component)]
pub struct HoverCardBody;

#[derive(Component)]
pub struct ListButton(pub String);

#[derive(Component)]
pub struct BannerButton(pub String);

#[derive(Component)]
pub struct FilterButton {
    pub kingdom: bool,
    pub value: String,
}

/// Faction colour (`heraldry.primary_color`).
fn faction_rgb(sim: &Sim, id: &str) -> [f32; 3] {
    sim.data
        .as_ref()
        .zip(data_model::FactionId::new(id).ok())
        .and_then(|(data, id)| data.factions.get(&id))
        .and_then(|f| Srgba::hex(f.heraldry.primary_color.as_str()).ok())
        .map_or([0.5, 0.5, 0.5], |c| [c.red, c.green, c.blue])
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn byte(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// RGBA pixels of the map for the current filters, hover and selection.
pub fn paint(
    geometry: &MapGeometry,
    sheets: &StartSheets,
    sim: &Sim,
    filters: &Filters,
    hovered: &str,
    selected: &str,
) -> Vec<u8> {
    let slot_color: Vec<[f32; 3]> = geometry
        .slots
        .iter()
        .map(|owner| {
            let mut color = match sheets.get(owner) {
                Some(sheet) => {
                    let mut color = faction_rgb(sim, owner);
                    if !matches(sheet, filters) {
                        color = mix(color, VEIL, 0.6);
                    }
                    color
                }
                None if owner.is_empty() => VEIL,
                None => mix(faction_rgb(sim, owner), VEIL, 0.7),
            };
            if !owner.is_empty() && owner == hovered {
                color = mix(color, [1.0, 1.0, 1.0], 0.25);
            }
            color
        })
        .collect();
    let (w, h) = (geometry.width as usize, geometry.height as usize);
    let mut pixels = vec![0u8; w * h * 4];
    let slot_at = |x: usize, y: usize| geometry.grid[y * w + x];
    for y in 0..h {
        for x in 0..w {
            let slot = slot_at(x, y);
            let offset = (y * w + x) * 4;
            if slot == 0 {
                pixels[offset..offset + 4].copy_from_slice(&SEA);
                continue;
            }
            let mut rgb = slot_color[usize::from(slot) - 1];
            let edge = (x + 1 < w && slot_at(x + 1, y) != slot)
                || (y + 1 < h && slot_at(x, y + 1) != slot)
                || x == 0
                || y == 0;
            if edge {
                let chosen =
                    !selected.is_empty() && geometry.slots[usize::from(slot) - 1] == selected;
                if chosen {
                    pixels[offset..offset + 4].copy_from_slice(&[
                        SELECTED_OUTLINE[0],
                        SELECTED_OUTLINE[1],
                        SELECTED_OUTLINE[2],
                        255,
                    ]);
                    continue;
                }
                rgb = mix(rgb, OUTLINE, 0.55);
            }
            pixels[offset..offset + 4].copy_from_slice(&[
                byte(rgb[0]),
                byte(rgb[1]),
                byte(rgb[2]),
                255,
            ]);
        }
    }
    pixels
}

fn read_json(path: &std::path::Path) -> Option<Value> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

/// Where to stand the banner of a landless faction: the colony it holds in the province of its
/// capital (`data/settlements/<capital>.json`, position from `map/settlements_px.json`), else the
/// seat of that province.
fn home_position(
    data_dir: &std::path::Path,
    capital: &str,
    faction: &str,
    features: &[ProvinceFeature],
    positions: Option<&Value>,
) -> Option<[f32; 2]> {
    let listed = read_json(&data_dir.join(format!("settlements/{capital}.json")));
    if let Some(Value::Array(settlements)) = listed {
        for settlement in settlements {
            if settlement.get("owner").and_then(Value::as_str) != Some(faction) {
                continue;
            }
            let px = settlement
                .get("id")
                .and_then(Value::as_str)
                .and_then(|id| positions?.get(id))
                .and_then(Value::as_array);
            if let Some([x, y, ..]) = px.map(Vec::as_slice) {
                return Some([x.as_f64()? as f32, y.as_f64()? as f32]);
            }
        }
    }
    features
        .iter()
        .find(|f| f.id == capital && f.has_seat)
        .map(|f| f.capital_px)
}

/// Loads the sheets and builds the geometry and the image when the screen opens.
pub fn prepare(
    mut commands: Commands,
    sim: Res<Sim>,
    mut images: ResMut<Assets<Image>>,
    existing: Option<Res<StartSheets>>,
) {
    let sheets = match existing {
        Some(sheets) if !sheets.list.is_empty() => StartSheets {
            list: sheets.list.clone(),
            asset_root: sheets.asset_root.clone(),
        },
        _ => StartSheets {
            list: sim
                .data
                .as_ref()
                .map(|data| game_api::start_sheets::feudal_start_sheets(data))
                .unwrap_or_default(),
            asset_root: crate::paths::find_root()
                .ok()
                .map(|root| crate::paths::asset_root(&root)),
        },
    };
    let geometry = crate::paths::find_root().ok().and_then(|root| {
        let data_dir = crate::paths::data_dir(&root);
        let bytes = std::fs::read(data_dir.join("map/provinces.geojson")).ok()?;
        let features = data_model::map_geo::parse_provinces(&bytes).ok()?;
        let positions = read_json(&data_dir.join("map/settlements_px.json"));
        let data = sim.data.as_ref()?;
        let banners: Vec<(String, [f32; 2])> = sheets
            .list
            .iter()
            .filter(|sheet| sheet.provinces.is_empty())
            .filter_map(|sheet| {
                let capital = data_model::FactionId::new(&sheet.id)
                    .ok()
                    .and_then(|id| data.factions.get(&id))
                    .map(|f| f.capital.as_str().to_owned())?;
                let at = home_position(
                    &data_dir,
                    &capital,
                    &sheet.id,
                    &features,
                    positions.as_ref(),
                )?;
                Some((sheet.id.clone(), at))
            })
            .collect();
        Some(MapGeometry::build(
            &features,
            &sheets.list,
            &banners,
            IMAGE_WIDTH,
        ))
    });
    let (width, height) = geometry.as_ref().map_or((1, 1), |g| (g.width, g.height));
    let image = Image::new_fill(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &SEA,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    commands.insert_resource(FactionMapState {
        geometry,
        image: images.add(image),
        filters: Filters::default(),
        hovered: String::new(),
        painted: None,
        repaints: 0,
        clicks: 0,
        listed: None,
    });
    commands.insert_resource(sheets);
}

/// Spawns the tab body under `pane`: filters, hint, map and list. `root` receives the hover card.
pub fn spawn_tab(
    commands: &mut Commands,
    theme: &UiTheme,
    state: &FactionMapState,
    sheets: &StartSheets,
    pane: Entity,
    root: Entity,
) {
    let filters_row = commands
        .spawn((
            Name::new("MapFilters"),
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                flex_shrink: 0.0,
                ..default()
            },
            ChildOf(pane),
        ))
        .id();
    let kingdom_row = filter_row(commands, theme, filters_row, "Royaume");
    commands.spawn((
        toggle_button(theme, "fmk:", "Tous les royaumes", true),
        FilterButton {
            kingdom: true,
            value: String::new(),
        },
        ChildOf(kingdom_row),
    ));
    for (id, name) in kingdoms(&sheets.list) {
        commands.spawn((
            toggle_button(theme, &format!("fmk:{id}"), &name, false),
            FilterButton {
                kingdom: true,
                value: id,
            },
            ChildOf(kingdom_row),
        ));
    }
    let rank_row = filter_row(commands, theme, filters_row, "Rang");
    for (key, label) in RANK_FILTERS {
        commands.spawn((
            toggle_button(theme, &format!("fmr:{key}"), label, key.is_empty()),
            FilterButton {
                kingdom: false,
                value: key.to_owned(),
            },
            ChildOf(rank_row),
        ));
    }
    commands.spawn((
        typed(
            theme,
            "Survolez une terre pour la fiche de son seigneur, cliquez pour le choisir.",
            UiType::Caption,
        ),
        ChildOf(filters_row),
    ));
    let body = commands
        .spawn((
            Name::new("MapBody"),
            Node {
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                column_gap: Val::Px(10.0),
                ..default()
            },
            ChildOf(pane),
        ))
        .id();
    let aspect = state
        .geometry
        .as_ref()
        .map_or(1.0, MapGeometry::aspect_ratio);
    let map = commands
        .spawn((
            Name::new("FactionMap"),
            MapNode,
            RelativeCursorPosition::default(),
            Node {
                height: Val::Percent(100.0),
                aspect_ratio: Some(aspect),
                flex_shrink: 1.0,
                min_width: Val::Px(0.0),
                ..default()
            },
            ImageNode::new(state.image.clone()),
            ChildOf(body),
        ))
        .id();
    if let Some(geometry) = &state.geometry {
        for (id, at) in &geometry.banners {
            commands.spawn((
                Name::new(format!("banner:{id}")),
                BannerButton(id.clone()),
                WidgetButton,
                Hovered::default(),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(at.x * 100.0),
                    top: Val::Percent(at.y * 100.0),
                    width: Val::Px(BANNER_RADIUS * 2.0),
                    height: Val::Px(BANNER_RADIUS * 2.0),
                    margin: UiRect::all(Val::Px(-BANNER_RADIUS)),
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::MAX,
                    ..default()
                },
                BackgroundColor(style::FE_VELLUM),
                BorderColor::all(style::FE_GOLD),
                ChildOf(map),
            ));
        }
    }
    let scroll = commands
        .spawn((
            crate::ui::kit::scroll_list(theme, "fm:list", 500.0),
            Node {
                flex_grow: 1.0,
                min_width: Val::Px(120.0),
                ..default()
            },
            ChildOf(body),
        ))
        .id();
    commands.spawn((
        Name::new("FactionList"),
        FactionList,
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            flex_shrink: 0.0,
            ..default()
        },
        ChildOf(scroll),
    ));
    // Hover card (hidden until a land is hovered).
    let card = commands
        .spawn((
            Name::new("FactionHoverCard"),
            HoverCard,
            Node {
                position_type: PositionType::Absolute,
                min_width: Val::Px(300.0),
                max_width: Val::Px(380.0),
                padding: UiRect::all(Val::Px(10.0)),
                border: UiRect::all(Val::Px(2.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                ..default()
            },
            BackgroundColor(style::FE_VELLUM),
            BorderColor::all(style::FE_GOLD_DARK),
            GlobalZIndex(60),
            Visibility::Hidden,
            ChildOf(root),
        ))
        .id();
    commands.spawn((HoverCardBody, Node::default(), ChildOf(card)));
}

fn filter_row(commands: &mut Commands, theme: &UiTheme, parent: Entity, label: &str) -> Entity {
    let line = commands
        .spawn((
            Node {
                column_gap: Val::Px(6.0),
                align_items: AlignItems::Center,
                flex_wrap: FlexWrap::Wrap,
                row_gap: Val::Px(4.0),
                ..default()
            },
            ChildOf(parent),
        ))
        .id();
    commands.spawn((typed(theme, label, UiType::Body), ChildOf(line)));
    line
}

/// Hover card text lines (title first): ruler, titles, suzerain, kingdom, difficulty, objectives.
pub fn card_lines(sheet: &StartSheet, difficulty: Option<&str>) -> Vec<String> {
    let mut lines = vec![sheet.name.clone()];
    if !sheet.ruler.is_empty() {
        lines.push(format!("Souverain : {}", sheet.ruler));
    }
    lines.push(if sheet.titles.is_empty() {
        "Titres : aucun (ost sans terre)".to_owned()
    } else {
        format!("Titres : {}", sheet.titles.join(", "))
    });
    lines.push(if sheet.liege_name.is_empty() {
        "Suzerain : aucun (souverain)".to_owned()
    } else {
        format!("Suzerain : {}", sheet.liege_name)
    });
    if !sheet.kingdom_name.is_empty() {
        lines.push(format!("Royaume : {}", sheet.kingdom_name));
    }
    if let Some(label) = difficulty {
        lines.push(format!("Difficulté estimée : {label}"));
    }
    if !sheet.objectives.is_empty() {
        let titles: Vec<&str> = sheet.objectives.iter().map(|o| o.title.as_str()).collect();
        lines.push(format!("Objectifs : {}", titles.join(", ")));
    }
    lines
}

/// Pointer on the map: hover from the raster, banners and list; click chooses.
#[allow(clippy::too_many_arguments)]
pub fn map_pointer(
    nodes: Query<&RelativeCursorPosition, With<MapNode>>,
    banners: Query<(&BannerButton, &Hovered)>,
    list: Query<(&ListButton, &Hovered)>,
    buttons: Res<ButtonInput<MouseButton>>,
    sheets: Res<StartSheets>,
    mut state: ResMut<FactionMapState>,
    mut choice: ResMut<FactionChoice>,
) {
    let mut hovered = String::new();
    if let Ok(relative) = nodes.single()
        && relative.cursor_over
        && let Some(normalized) = relative.normalized
        && let Some(geometry) = &state.geometry
    {
        let uv = normalized + Vec2::splat(0.5);
        if let Some(id) = geometry.faction_at(uv, &sheets) {
            hovered = id.to_owned();
            if buttons.just_pressed(MouseButton::Left) {
                choice.faction = id.to_owned();
                state.clicks += 1;
            }
        }
    }
    if hovered.is_empty() {
        let from_ui = banners
            .iter()
            .find(|(_, h)| h.get())
            .map(|(b, _)| b.0.clone())
            .or_else(|| list.iter().find(|(_, h)| h.get()).map(|(b, _)| b.0.clone()));
        hovered = from_ui.unwrap_or_default();
    }
    if state.hovered != hovered {
        state.hovered = hovered;
    }
}

/// Filter, list and banner presses.
pub fn map_buttons(
    mut pressed: MessageReader<ButtonPressed>,
    mut state: ResMut<FactionMapState>,
    mut choice: ResMut<FactionChoice>,
    mut filter_buttons: Query<(&FilterButton, &mut Toggled)>,
) {
    for press in pressed.read() {
        let name = press.name.as_str();
        if let Some(id) = name.strip_prefix("fm:") {
            choice.faction = id.to_owned();
        } else if let Some(id) = name.strip_prefix("banner:") {
            choice.faction = id.to_owned();
            state.clicks += 1;
        } else if let Some(value) = name.strip_prefix("fmk:") {
            state.filters.kingdom = value.to_owned();
            for (button, mut toggled) in &mut filter_buttons {
                if button.kingdom {
                    toggled.0 = button.value == value;
                }
            }
        } else if let Some(value) = name.strip_prefix("fmr:") {
            state.filters.rank = value.to_owned();
            for (button, mut toggled) in &mut filter_buttons {
                if !button.kingdom {
                    toggled.0 = button.value == value;
                }
            }
        }
    }
}

/// Rebuilds the grouped list when the filters change; keeps the selection marks up to date.
pub fn refresh_list(
    mut commands: Commands,
    theme: Res<UiTheme>,
    sheets: Res<StartSheets>,
    mut state: ResMut<FactionMapState>,
    choice: Res<FactionChoice>,
    lists: Query<Entity, With<FactionList>>,
    mut buttons: Query<(&ListButton, &mut Toggled)>,
) {
    let Ok(list) = lists.single() else { return };
    if state.listed.as_ref() != Some(&state.filters) {
        state.listed = Some(state.filters.clone());
        commands.entity(list).despawn_related::<Children>();
        let groups = groups(&sheets.list, &state.filters);
        if groups.is_empty() {
            commands.spawn((
                typed(
                    &theme,
                    "Aucune faction ne correspond à ces filtres.",
                    UiType::Caption,
                ),
                ChildOf(list),
            ));
        }
        for (kingdom, ids) in groups {
            let header = if kingdom.is_empty() {
                LANDLESS_HEADER.to_owned()
            } else {
                kingdom
            };
            commands.spawn((
                Text::new(header),
                theme.title_font(UiType::Body.size()),
                TextColor(style::FE_GULES),
                ChildOf(list),
            ));
            let flow = commands
                .spawn((
                    Node {
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: Val::Px(6.0),
                        row_gap: Val::Px(4.0),
                        ..default()
                    },
                    ChildOf(list),
                ))
                .id();
            for id in ids {
                let name = sheets.get(&id).map_or(id.clone(), |s| s.name.clone());
                commands.spawn((
                    toggle_button(&theme, &format!("fm:{id}"), &name, id == choice.faction),
                    ListButton(id),
                    Hovered::default(),
                    ChildOf(flow),
                ));
            }
        }
        return;
    }
    for (button, mut toggled) in &mut buttons {
        let on = button.0 == choice.faction;
        if toggled.0 != on {
            toggled.0 = on;
        }
    }
}

/// Repaints the raster when the filters, the hover or the selection changed.
pub fn repaint(
    sheets: Res<StartSheets>,
    sim: Res<Sim>,
    choice: Res<FactionChoice>,
    mut state: ResMut<FactionMapState>,
    mut images: ResMut<Assets<Image>>,
) {
    let key = (
        state.filters.clone(),
        state.hovered.clone(),
        choice.faction.clone(),
    );
    if state.painted.as_ref() == Some(&key) {
        return;
    }
    let Some(geometry) = &state.geometry else {
        return;
    };
    let pixels = paint(geometry, &sheets, &sim, &key.0, &key.1, &key.2);
    if let Some(mut image) = images.get_mut(&state.image) {
        image.data = Some(pixels);
    }
    state.painted = Some(key);
    state.repaints += 1;
}

/// Shows the faction card next to the cursor while a land is hovered.
#[allow(clippy::too_many_arguments)]
pub fn hover_card(
    mut commands: Commands,
    theme: Res<UiTheme>,
    state: Res<FactionMapState>,
    sheets: Res<StartSheets>,
    menu: Res<MenuData>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut cards: Query<(&mut Node, &mut Visibility), With<HoverCard>>,
    bodies: Query<Entity, With<HoverCardBody>>,
    mut shown: Local<String>,
) {
    let Ok((mut node, mut visibility)) = cards.single_mut() else {
        return;
    };
    let Some(sheet) = sheets.get(&state.hovered) else {
        *visibility = Visibility::Hidden;
        shown.clear();
        return;
    };
    if let Ok(window) = windows.single() {
        let cursor = window.cursor_position().unwrap_or(Vec2::new(420.0, 140.0));
        let size = window.size();
        node.left = Val::Px((cursor.x + 18.0).min(size.x - 390.0).max(4.0));
        node.top = Val::Px((cursor.y + 18.0).min(size.y - 220.0).max(4.0));
    }
    *visibility = Visibility::Visible;
    if *shown == sheet.id {
        return;
    }
    shown.clone_from(&sheet.id);
    let difficulty = menu
        .front
        .pitch(&sheet.id)
        .map(|pitch| menu.front.difficulty_label(pitch.difficulty));
    let lines = card_lines(sheet, difficulty.as_deref());
    let Ok(body) = bodies.single() else { return };
    commands.entity(body).despawn_related::<Children>();
    commands.entity(body).insert(Node {
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(2.0),
        ..default()
    });
    for (index, line) in lines.into_iter().enumerate() {
        if index == 0 {
            commands.spawn((
                Text::new(line),
                theme.title_font(UiType::Heading.size()),
                TextColor(style::INK_TITLE),
                ChildOf(body),
            ));
        } else {
            commands.spawn((typed(&theme, &line, UiType::Body), ChildOf(body)));
        }
    }
}

/// Section `menu.faction_map`.
pub fn dump(world: &World) -> Value {
    let (Some(state), Some(sheets)) = (
        world.get_resource::<FactionMapState>(),
        world.get_resource::<StartSheets>(),
    ) else {
        return json!({ "loaded": false });
    };
    let in_screen = world
        .get_resource::<State<AppState>>()
        .is_some_and(|s| *s.get() == AppState::FactionSelect);
    let visible: Vec<&str> = sheets
        .list
        .iter()
        .filter(|s| matches(s, &state.filters))
        .map(|s| s.id.as_str())
        .collect();
    let first_groups: Vec<Value> = groups(&sheets.list, &state.filters)
        .into_iter()
        .take(6)
        .map(|(name, ids)| json!({ "kingdom": name, "factions": ids.len() }))
        .collect();
    json!({
        "loaded": state.geometry.is_some(),
        "screen_open": in_screen,
        "size": state.geometry.as_ref().map(|g| [g.width, g.height]),
        "provinces": state.geometry.as_ref().map(|g| g.slots.len()),
        "land_pixels": state.geometry.as_ref().map(|g| g.grid.iter().filter(|s| **s > 0).count()),
        "sheets": sheets.list.len(),
        "kingdoms": kingdoms(&sheets.list).len(),
        "banners": state.geometry.as_ref().map(|g| g.banners.iter().map(|(id, _)| id.clone()).collect::<Vec<_>>()),
        "filters": { "kingdom": state.filters.kingdom, "rank": state.filters.rank },
        "visible_factions": visible.len(),
        "groups": first_groups,
        "hovered": state.hovered,
        "repaints": state.repaints,
        "clicks": state.clicks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sheet(id: &str, kingdom: &str, rank: &str, provinces: &[&str]) -> StartSheet {
        StartSheet {
            id: id.to_owned(),
            name: id.to_uppercase(),
            ruler: String::new(),
            ruler_id: String::new(),
            titles: Vec::new(),
            rank: rank.to_owned(),
            liege_name: String::new(),
            kingdom: kingdom.to_owned(),
            kingdom_name: kingdom.to_uppercase(),
            objectives: Vec::new(),
            provinces: provinces.iter().map(|p| (*p).to_owned()).collect(),
            treasury: 0,
            projected_income: 0,
            armies_count: 0,
        }
    }

    fn square(id: &str, index: i64, x: f32, y: f32, side: f32) -> ProvinceFeature {
        ProvinceFeature {
            index,
            id: id.to_owned(),
            name: id.to_owned(),
            owner: String::new(),
            terrain: String::new(),
            capital_name: String::new(),
            centroid: [x, y],
            capital_px: [x, y],
            has_seat: true,
            neighbors: Vec::new(),
            area_px: 0.0,
            rings: vec![vec![
                [x, y],
                [x + side, y],
                [x + side, y + side],
                [x, y + side],
            ]],
            ..Default::default()
        }
    }

    #[test]
    fn filters_group_and_order_the_list() {
        let sheets = vec![
            sheet("a", "", "", &[]),
            sheet("b", "fr", "kingdom", &["p1"]),
            sheet("c", "fr", "duchy", &["p2"]),
            sheet("d", "en", "kingdom", &["p3"]),
        ];
        let all = groups(&sheets, &Filters::default());
        assert_eq!(all[0].0, "");
        assert_eq!(
            all[1],
            ("FR".to_owned(), vec!["b".to_owned(), "c".to_owned()])
        );
        assert_eq!(all[2].0, "EN");
        let duchies = Filters {
            kingdom: String::new(),
            rank: "duchy".to_owned(),
        };
        assert_eq!(groups(&sheets, &duchies).len(), 1);
        assert_eq!(kingdoms(&sheets).len(), 2);
    }

    #[test]
    fn the_raster_gives_the_owner_of_a_land() {
        let sheets = StartSheets {
            list: vec![
                sheet("a", "x", "kingdom", &["p1"]),
                sheet("b", "x", "county", &["p2"]),
            ],
            asset_root: None,
        };
        let features = vec![
            square("p1", 1, 0.0, 0.0, 100.0),
            square("p2", 2, 100.0, 0.0, 100.0),
            square("p3", 3, 200.0, 0.0, 100.0),
        ];
        let geometry = MapGeometry::build(&features, &sheets.list, &[], 200);
        let a = geometry.center_of("a").unwrap();
        let b = geometry.center_of("b").unwrap();
        assert_eq!(geometry.faction_at(a, &sheets), Some("a"));
        assert_eq!(geometry.faction_at(b, &sheets), Some("b"));
        assert_eq!(geometry.faction_at(Vec2::new(0.001, 0.001), &sheets), None);
        // The third province belongs to no playable faction: it is outside the frame and
        // never answers.
        assert!(geometry.slot_at(Vec2::new(0.999, 0.5)) <= 3);
        assert!(geometry.aspect_ratio() > 1.0);
    }

    #[test]
    fn a_landless_faction_gets_a_banner_inside_the_frame() {
        let sheets = vec![sheet("a", "x", "kingdom", &["p1"]), sheet("c", "", "", &[])];
        let features = vec![square("p1", 1, 0.0, 0.0, 100.0)];
        let geometry =
            MapGeometry::build(&features, &sheets, &[("c".to_owned(), [50.0, 50.0])], 100);
        let (id, at) = &geometry.banners[0];
        assert_eq!(id, "c");
        assert!(at.x > 0.0 && at.x < 1.0 && at.y > 0.0 && at.y < 1.0);
    }

    #[test]
    fn the_hover_card_lists_the_sheet() {
        let mut s = sheet("a", "fr", "kingdom", &["p1"]);
        s.ruler = "Philippe VI".to_owned();
        s.titles = vec!["Roi de France".to_owned()];
        let lines = card_lines(&s, Some("Normal"));
        assert_eq!(lines[0], "A");
        assert!(lines.contains(&"Souverain : Philippe VI".to_owned()));
        assert!(lines.contains(&"Suzerain : aucun (souverain)".to_owned()));
        assert!(lines.contains(&"Difficulté estimée : Normal".to_owned()));
        let landless = card_lines(&sheet("c", "", "", &[]), None);
        assert!(landless.contains(&"Titres : aucun (ost sans terre)".to_owned()));
    }
}
