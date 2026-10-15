//! Campaign minimap (ports game/scripts/map/campaign_minimap.gd + minimap_controller.gd).
//!
//! A small parchment frame with the lands painted by faction colour (the colours of the province
//! fill, `OverlayState::colors`), sea, the armies as dots (the player's ringed), the camera focus
//! and the pings of alerts. Click or drag = [`FocusCamera::Point`]. Two modes: political, and
//! relief (land and sea only: the height-shaded relief of Godot needs the terrain pyramid; the
//! frame is cropped to the extent of the provinces). The fog mask is not ported.

use bevy::asset::RenderAssetUsages;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::ui::RelativeCursorPosition;
use serde_json::{Value, json};

use super::bar::round_button;
use super::{HudSet, HudWidget};
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::armies::{ArmyCommand, ArmyViews, SelectedArmy};
use crate::campaign::camera::FocusCamera;
use crate::campaign::camera::rig::CampaignRig;
use crate::campaign::provinces::data::ProvinceMap;
use crate::campaign::provinces::overlay::OverlayState;
use crate::campaign::provinces::{MapMode, MapModeState};
use crate::campaign::terrain::surface::Surface;
use crate::campaign::weather::fog::FogOfWar;
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::widgets::panel_inset;
use crate::ui::kit::{ButtonPressed, UiTheme};

/// Width of the map in the frame, in logical pixels (`MAP_WIDTH` of Godot).
pub const MAP_WIDTH: f32 = 280.0;
const MAX_MAP_HEIGHT: f32 = 230.0;
/// Resolution of the baked image relative to the display (`TEXTURE_SCALE`).
const TEXTURE_SCALE: f32 = 1.5;
/// Margin around the extent of the provinces (share of its size).
const CROP_MARGIN: f32 = 0.03;
const SEA_F: [f32; 3] = [0.46, 0.55, 0.56];
const SEA_DEEP_F: [f32; 3] = [0.33, 0.42, 0.45];
const LOWLAND_F: [f32; 3] = [0.86, 0.80, 0.62];
const HIGHLAND_F: [f32; 3] = [0.58, 0.47, 0.32];
const PARCHMENT: [f32; 3] = [0.90, 0.84, 0.68];
const INK: [f32; 3] = [0.22, 0.14, 0.07];
const VEIL: [f32; 3] = [0.86, 0.83, 0.76];
/// Metres per map pixel of the composed map (`provinces/projection.rs`).
const METERS_PER_PX: f32 = 719.0;
const PING_SECONDS: f32 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MinimapMode {
    Political,
    Relief,
}

/// Geometry and baked grid of the minimap; pure, unit-tested.
pub struct Grid {
    /// Cropped extent in map pixels: min x, min y, width, height.
    pub crop: [f32; 4],
    pub width: u32,
    pub height: u32,
    /// Province index of each minimap pixel (0 = sea).
    pub cells: Vec<u16>,
}

impl Grid {
    /// Bakes the grid from the province raster.
    pub fn bake(map: &ProvinceMap) -> Self {
        let (mw, mh) = (map.width, map.height);
        let raster = map.raster();
        let (mut min_x, mut min_y, mut max_x, mut max_y) = (mw, mh, 0u32, 0u32);
        for y in 0..mh {
            let row = &raster[(y * mw) as usize..((y + 1) * mw) as usize];
            if let (Some(first), Some(last)) = (
                row.iter().position(|&i| i != 0),
                row.iter().rposition(|&i| i != 0),
            ) {
                min_x = min_x.min(first as u32);
                max_x = max_x.max(last as u32);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }
        if max_x < min_x {
            (min_x, min_y, max_x, max_y) = (0, 0, mw.max(1), mh.max(1));
        }
        let margin = (max_x - min_x).max(max_y - min_y) as f32 * CROP_MARGIN;
        let x0 = (min_x as f32 - margin).max(0.0);
        let y0 = (min_y as f32 - margin).max(0.0);
        let x1 = (max_x as f32 + margin).min(mw as f32);
        let y1 = (max_y as f32 + margin).min(mh as f32);
        let (cw, ch) = ((x1 - x0).max(1.0), (y1 - y0).max(1.0));
        let (view_w, view_h) = Self::display_size(cw, ch);
        let (width, height) = (
            (view_w * TEXTURE_SCALE) as u32,
            (view_h * TEXTURE_SCALE) as u32,
        );
        let mut cells = Vec::with_capacity((width * height) as usize);
        for py in 0..height {
            for px in 0..width {
                let mx = x0 + (px as f32 + 0.5) / width as f32 * cw;
                let my = y0 + (py as f32 + 0.5) / height as f32 * ch;
                cells.push(map.index_at(mx, my));
            }
        }
        Self {
            crop: [x0, y0, cw, ch],
            width,
            height,
            cells,
        }
    }

    /// Display size (logical pixels) of a crop: `MAP_WIDTH` wide, at most `MAX_MAP_HEIGHT` high.
    pub fn display_size(crop_w: f32, crop_h: f32) -> (f32, f32) {
        let height = MAP_WIDTH * crop_h / crop_w;
        if height <= MAX_MAP_HEIGHT {
            (MAP_WIDTH, height)
        } else {
            (MAX_MAP_HEIGHT * crop_w / crop_h, MAX_MAP_HEIGHT)
        }
    }

    /// Share (0-1 on both axes) of a map position in the frame; may fall outside.
    pub fn map_to_share(&self, map_pos: Vec2) -> Vec2 {
        Vec2::new(
            (map_pos.x - self.crop[0]) / self.crop[2],
            (map_pos.y - self.crop[1]) / self.crop[3],
        )
    }

    /// Map position of a share of the frame.
    pub fn share_to_map(&self, share: Vec2) -> Vec2 {
        Vec2::new(
            self.crop[0] + share.x * self.crop[2],
            self.crop[1] + share.y * self.crop[3],
        )
    }

    /// Map position (pixels) of the centre of minimap pixel `(px, py)`.
    pub fn pixel_to_map(&self, px: u32, py: u32) -> Vec2 {
        Vec2::new(
            self.crop[0] + (px as f32 + 0.5) / self.width as f32 * self.crop[2],
            self.crop[1] + (py as f32 + 0.5) / self.height as f32 * self.crop[3],
        )
    }

    /// Height-shaded background (`_relief_color` of `campaign_minimap.gd`): land shaded by
    /// altitude with a north-west light, sea by depth. `height_m(x, y)` is the altitude in
    /// metres at a map pixel, `is_land(x, y)` the land test, `meters_per_px` the map scale.
    pub fn bake_relief(
        &self,
        height_m: impl Fn(f32, f32) -> f32,
        is_land: impl Fn(f32, f32) -> bool,
        meters_per_px: f32,
    ) -> Vec<[f32; 3]> {
        let step = (self.crop[2] / self.width as f32).floor().max(1.0);
        let mut out = Vec::with_capacity((self.width * self.height) as usize);
        for py in 0..self.height {
            for px in 0..self.width {
                let at = self.pixel_to_map(px, py).floor();
                let height = height_m(at.x, at.y);
                let color = if !is_land(at.x, at.y) {
                    lerp3(SEA_F, SEA_DEEP_F, (-height / 2000.0).clamp(0.0, 1.0))
                } else {
                    let t = (height.max(0.0) / 2500.0).powf(0.6).clamp(0.0, 1.0);
                    let base = lerp3(LOWLAND_F, HIGHLAND_F, t);
                    let north_west = height_m(at.x - step, at.y - step);
                    let slope = (north_west - height) / (step * meters_per_px).max(1.0);
                    let dark = (slope * 6.0).clamp(-0.15, 0.3);
                    scale3(base, 1.0 - dark)
                };
                out.push(color);
            }
        }
        out
    }

    /// RGBA8 pixels for `mode` with the province `colors` (indexed by raster index): the
    /// fragment shader of `campaign_minimap.gdshader` on the CPU. `relief` is the baked
    /// background (flat sea / lowland before the terrain is loaded); `fog` hides what the
    /// player does not see (desaturated, veiled, hatched), by cell with a soft edge.
    pub fn paint(
        &self,
        mode: MinimapMode,
        colors: &[Srgba],
        relief: Option<&[[f32; 3]]>,
        fog: Option<&FogOfWar>,
    ) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.cells.len() * 4);
        let (w, h) = (self.width as usize, self.height as usize);
        let faction = |index: u16| -> [f32; 4] {
            if index == 0 {
                return [0.0; 4];
            }
            colors
                .get(usize::from(index))
                .map_or([0.0; 4], |c| [c.red, c.green, c.blue, c.alpha.max(0.0)])
        };
        let fog = fog.filter(|f| f.active && f.mask.is_some());
        for py in 0..h {
            for px in 0..w {
                let i = py * w + px;
                let index = self.cells[i];
                let flat = if index == 0 { SEA_F } else { LOWLAND_F };
                let base = relief.and_then(|r| r.get(i)).copied().unwrap_or(flat);
                let mut col = base;
                if index != 0 {
                    let fc = faction(index);
                    let tint = [fc[0], fc[1], fc[2]];
                    if mode == MinimapMode::Political {
                        let shade = (base[0] + base[1] + base[2]) * 0.333;
                        let paper = scale3(PARCHMENT, 0.82 + 0.25 * shade);
                        col = lerp3(paper, scale3(tint, 0.85 + 0.2 * shade), 0.78 * fc[3]);
                    } else {
                        col = lerp3(base, tint, 0.18 * fc[3]);
                    }
                    let (mut province_edge, mut realm_edge) = (false, false);
                    for (dx, dy) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                        let (nx, ny) = (px as i32 + dx, py as i32 + dy);
                        if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                            continue;
                        }
                        let other = self.cells[ny as usize * w + nx as usize];
                        if other != index {
                            province_edge = true;
                            let fo = faction(other);
                            let dist = ((fo[0] - fc[0]).powi(2)
                                + (fo[1] - fc[1]).powi(2)
                                + (fo[2] - fc[2]).powi(2)
                                + (fo[3] - fc[3]).powi(2))
                            .sqrt();
                            if other == 0 || dist > 0.02 {
                                realm_edge = true;
                            }
                        }
                    }
                    let edge = if mode == MinimapMode::Political {
                        0.22
                    } else {
                        0.12
                    };
                    if province_edge {
                        col = lerp3(col, INK, edge);
                    }
                    if realm_edge {
                        col = lerp3(col, INK, 0.75);
                    }
                    if let Some(fog) = fog {
                        let at = self.pixel_to_map(px as u32, py as u32);
                        let coverage = fog.mask.as_ref().map_or(255, |m| m.coverage_at(at.x, at.y));
                        let hidden = 1.0 - smoothstep(0.3, 0.7, f32::from(coverage) / 255.0);
                        if hidden > 0.0 {
                            let lum = col[0] * 0.299 + col[1] * 0.587 + col[2] * 0.114;
                            let grey = lerp3(col, [lum; 3], 0.8);
                            let mut fogged = lerp3(grey, VEIL, 0.55);
                            if (px + py) % 5 >= 3 {
                                fogged = lerp3(fogged, INK, 0.08);
                            }
                            col = lerp3(col, fogged, hidden);
                        }
                    }
                }
                out.extend_from_slice(&[
                    (col[0].clamp(0.0, 1.0) * 255.0).round() as u8,
                    (col[1].clamp(0.0, 1.0) * 255.0).round() as u8,
                    (col[2].clamp(0.0, 1.0) * 255.0).round() as u8,
                    255,
                ]);
            }
        }
        out
    }
}

fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn scale3(a: [f32; 3], k: f32) -> [f32; 3] {
    [a[0] * k, a[1] * k, a[2] * k]
}

fn smoothstep(lo: f32, hi: f32, x: f32) -> f32 {
    let t = ((x - lo) / (hi - lo)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[derive(Resource)]
pub struct Minimap {
    grid: Grid,
    image: Handle<Image>,
    pub mode: MinimapMode,
    painted_colors: Vec<Srgba>,
    painted_mode: Option<MinimapMode>,
    /// Height-shaded background, baked once the terrain is loaded.
    relief: Option<Vec<[f32; 3]>>,
    painted_relief: bool,
    painted_fog: (bool, u64),
    pub fogged: bool,
    pub repaints: u32,
    pub orders: u32,
    pub last_order: Option<Vec2>,
    pub clicks: u32,
    pub last_target: Option<Vec2>,
    pub dots: usize,
    pub pings: Vec<(Vec2, f32)>,
}

#[derive(Component)]
struct MinimapView;

#[derive(Component)]
struct MinimapOverlay;

#[derive(Component)]
struct Dot;

#[derive(Component)]
struct FocusMark;

/// Asks the minimap to ping a map position (alerts: a red ring for two seconds).
#[derive(Message, Debug, Clone, Copy)]
pub struct MinimapPing(pub Vec2);

pub fn plugin(app: &mut App) {
    app.add_message::<MinimapPing>()
        .register_dump(dump_minimap)
        .add_systems(OnExit(AppState::Campaign), forget_minimap)
        .add_systems(
            Update,
            (
                spawn_minimap,
                paint_minimap,
                minimap_input,
                rebuild_dots,
                move_focus_mark,
                mode_buttons,
                highlight_mode,
            )
                .chain()
                .in_set(HudSet::Ui),
        );
}

fn forget_minimap(mut commands: Commands) {
    commands.remove_resource::<Minimap>();
}

/// Builds the minimap once the province map exists (it is loaded by the provinces plugin when the
/// campaign opens), and again after a save is loaded.
fn spawn_minimap(
    mut commands: Commands,
    map: Option<Res<ProvinceMap>>,
    existing: Option<Res<Minimap>>,
    column: Query<Entity, With<super::MinimapSlot>>,
    mut images: ResMut<Assets<Image>>,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
) {
    let Some(map) = map else { return };
    let Ok(column) = column.single() else { return };
    if existing.is_some() {
        return;
    }
    let grid = Grid::bake(&map);
    let pixels = grid.paint(MinimapMode::Political, &[], None, None);
    let image = images.add(Image::new(
        Extent3d {
            width: grid.width,
            height: grid.height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ));
    let (view_w, view_h) = Grid::display_size(grid.crop[2], grid.crop[3]);
    commands
        .spawn((
            Name::new("hud:minimap"),
            super::LabelObstacle,
            ChildOf(column),
            HudWidget,
            Hovered::default(),
            panel_inset(&theme),
        ))
        .insert(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            padding: UiRect::all(Val::Px(6.0)),
            ..default()
        })
        .with_children(|frame| {
            frame
                .spawn((
                    Name::new("hud:minimap_view"),
                    MinimapView,
                    ImageNode::new(image.clone()),
                    RelativeCursorPosition::default(),
                    Tooltip::new("Cliquez ou glissez pour déplacer la caméra."),
                    Node {
                        width: Val::Px(view_w),
                        height: Val::Px(view_h),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                ))
                .with_children(|view| {
                    view.spawn((
                        Name::new("hud:minimap_overlay"),
                        MinimapOverlay,
                        Pickable::IGNORE,
                        Node {
                            position_type: PositionType::Absolute,
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                    ));
                    view.spawn((
                        Name::new("hud:minimap_focus"),
                        FocusMark,
                        Pickable::IGNORE,
                        Node {
                            position_type: PositionType::Absolute,
                            width: Val::Px(14.0),
                            height: Val::Px(14.0),
                            border: UiRect::all(Val::Px(1.5)),
                            ..default()
                        },
                        BorderColor::all(Color::srgb(1.0, 0.96, 0.82)),
                    ));
                    // Relief switch of the minimap itself, on the corner of the view.
                    view.spawn((
                        round_button(
                            &theme,
                            &assets,
                            "hud:minimap_relief",
                            "lens_relief",
                            "",
                            22.0,
                        ),
                        Tooltip::new(
                            "[b]Minicarte[/b]\nBascule entre couleurs des royaumes et relief.",
                        ),
                    ))
                    .insert(Node {
                        position_type: PositionType::Absolute,
                        top: Val::Px(3.0),
                        right: Val::Px(3.0),
                        width: Val::Px(22.0),
                        height: Val::Px(22.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(1.5)),
                        border_radius: BorderRadius::MAX,
                        ..default()
                    });
                });
            // Map modes (lenses) and legend under the frame (Godot: the mode buttons of the
            // minimap); the minimap's own relief switch sits on its corner.
            frame
                .spawn((
                    Name::new("hud:map_modes"),
                    Node {
                        column_gap: Val::Px(5.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                ))
                .with_children(|row| {
                    for mode in MapMode::ALL {
                        row.spawn((
                            round_button(
                                &theme,
                                &assets,
                                &format!("hud:mode:{}", mode.key()),
                                mode_icon(mode),
                                "",
                                MODE_BUTTON,
                            ),
                            ModeButton(mode),
                            Tooltip::new(&format!(
                                "[b]Carte : {}[/b]\n{}",
                                mode.label_fr(),
                                mode.tooltip_fr()
                            )),
                        ));
                    }
                    row.spawn((
                        Node {
                            width: Val::Px(1.0),
                            height: Val::Px(MODE_BUTTON - 6.0),
                            ..default()
                        },
                        BackgroundColor(crate::ui::kit::style::INK_FADED.with_alpha(0.5)),
                    ));
                    row.spawn((
                        round_button(&theme, &assets, "hud:legend", "map_legend", "", MODE_BUTTON),
                        Tooltip::new("[b]Légende[/b]\nCe que montrent les symboles de la carte."),
                    ));
                });
        });
    commands.insert_resource(Minimap {
        grid,
        image,
        mode: MinimapMode::Political,
        painted_colors: Vec::new(),
        painted_mode: None,
        relief: None,
        painted_relief: false,
        painted_fog: (false, 0),
        fogged: false,
        repaints: 0,
        orders: 0,
        last_order: None,
        clicks: 0,
        last_target: None,
        dots: 0,
        pings: Vec::new(),
    });
}

/// Repaints the image when the province colours, the mode, the fog or the relief changed.
#[allow(clippy::too_many_arguments)]
fn paint_minimap(
    minimap: Option<ResMut<Minimap>>,
    overlay: Option<Res<OverlayState>>,
    surface: Option<Res<Surface>>,
    land: Option<Res<crate::campaign::armies::landmask::LandMask>>,
    fog: Option<Res<FogOfWar>>,
    mut images: ResMut<Assets<Image>>,
) {
    let (Some(mut minimap), Some(overlay)) = (minimap, overlay) else {
        return;
    };
    if minimap.relief.is_none()
        && let Some(surface) = surface.as_ref()
    {
        let cells = &minimap.grid;
        let relief = cells.bake_relief(
            |x, y| surface.heights.height_m_at(x, y),
            |x, y| match land.as_deref() {
                Some(mask) if mask.is_loaded() => mask.is_land(x, y),
                _ => cells_land(cells, x, y),
            },
            METERS_PER_PX,
        );
        minimap.relief = Some(relief);
    }
    let fog_key = fog
        .as_ref()
        .map_or((false, 0), |f| (f.active && f.mask.is_some(), f.revision));
    let relief_ready = minimap.relief.is_some();
    if minimap.painted_mode == Some(minimap.mode)
        && minimap.painted_colors == overlay.colors
        && minimap.painted_relief == relief_ready
        && minimap.painted_fog == fog_key
    {
        return;
    }
    let pixels = minimap.grid.paint(
        minimap.mode,
        &overlay.colors,
        minimap.relief.as_deref(),
        fog.as_deref(),
    );
    if let Some(mut image) = images.get_mut(&minimap.image) {
        image.data = Some(pixels);
    }
    minimap.painted_colors = overlay.colors.clone();
    minimap.painted_mode = Some(minimap.mode);
    minimap.painted_relief = relief_ready;
    minimap.painted_fog = fog_key;
    minimap.fogged = fog_key.0;
    minimap.repaints += 1;
}

/// Land test from the province cells (when the land mask is not loaded).
fn cells_land(grid: &Grid, x: f32, y: f32) -> bool {
    let share = grid.map_to_share(Vec2::new(x, y));
    if !(0.0..1.0).contains(&share.x) || !(0.0..1.0).contains(&share.y) {
        return false;
    }
    let (px, py) = (
        (share.x * grid.width as f32) as usize,
        (share.y * grid.height as f32) as usize,
    );
    grid.cells[py * grid.width as usize + px] != 0
}

fn minimap_input(
    minimap: Option<ResMut<Minimap>>,
    view: Query<&RelativeCursorPosition, With<MinimapView>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut focus: MessageWriter<FocusCamera>,
    selected: Option<Res<SelectedArmy>>,
    mut commands: MessageWriter<ArmyCommand>,
) {
    let (Some(mut minimap), Ok(cursor)) = (minimap, view.single()) else {
        return;
    };
    let Some(normalized) = cursor.normalized else {
        return;
    };
    // Right click = march order of the selected army to this map point (`order_march_to`).
    if cursor.cursor_over && mouse.just_pressed(MouseButton::Right) {
        let target = minimap.grid.share_to_map(normalized + Vec2::splat(0.5));
        if let Some(army) = selected.and_then(|s| s.0.clone()) {
            commands.write(ArmyCommand::Move {
                army,
                target: [target.x, target.y],
            });
            minimap.orders += 1;
            minimap.last_order = Some(target);
        }
        return;
    }
    if !cursor.cursor_over || !mouse.pressed(MouseButton::Left) {
        return;
    }
    let share = normalized + Vec2::splat(0.5);
    let target = minimap.grid.share_to_map(share);
    if mouse.just_pressed(MouseButton::Left) {
        minimap.clicks += 1;
    }
    minimap.last_target = Some(target);
    focus.write(FocusCamera::Point(target));
}

/// Army dots and pings, rebuilt when the army views change or a ping lives.
#[allow(clippy::too_many_arguments)]
fn rebuild_dots(
    mut commands: Commands,
    minimap: Option<ResMut<Minimap>>,
    views: Option<Res<ArmyViews>>,
    overlay: Query<Entity, With<MinimapOverlay>>,
    mut pings: MessageReader<MinimapPing>,
    time: Res<Time>,
    mut built_for: Local<Option<usize>>,
) {
    let (Some(mut minimap), Ok(overlay)) = (minimap, overlay.single()) else {
        return;
    };
    let mut changed = false;
    for ping in pings.read() {
        minimap.pings.push((ping.0, PING_SECONDS));
        changed = true;
    }
    if !minimap.pings.is_empty() {
        let dt = time.delta_secs();
        minimap.pings.iter_mut().for_each(|p| p.1 -= dt);
        let before = minimap.pings.len();
        minimap.pings.retain(|p| p.1 > 0.0);
        changed |= before != minimap.pings.len() || before > 0;
    }
    let views_changed = views.as_ref().is_some_and(|v| v.is_changed());
    if !changed && !views_changed && built_for.is_some() {
        return;
    }
    *built_for = Some(0);
    commands.entity(overlay).despawn_children();
    let (view_w, view_h) = Grid::display_size(minimap.grid.crop[2], minimap.grid.crop[3]);
    let mut count = 0;
    commands.entity(overlay).with_children(|layer| {
        if let Some(views) = &views {
            for army in &views.0 {
                let share = minimap
                    .grid
                    .map_to_share(Vec2::new(army.point[0], army.point[1]));
                if !(0.0..=1.0).contains(&share.x) || !(0.0..=1.0).contains(&share.y) {
                    continue;
                }
                let size = if army.player { 6.0 } else { 4.0 };
                layer.spawn((
                    Dot,
                    Pickable::IGNORE,
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(share.x * view_w - size / 2.0),
                        top: Val::Px(share.y * view_h - size / 2.0),
                        width: Val::Px(size),
                        height: Val::Px(size),
                        border: if army.player {
                            UiRect::all(Val::Px(1.5))
                        } else {
                            UiRect::ZERO
                        },
                        ..default()
                    },
                    BackgroundColor(Color::srgb(army.color[0], army.color[1], army.color[2])),
                    BorderColor::all(Color::srgb(0.95, 0.80, 0.30)),
                ));
                count += 1;
            }
        }
        for (point, left) in &minimap.pings {
            let share = minimap.grid.map_to_share(*point);
            let size = 10.0 + 18.0 * (1.0 - left / PING_SECONDS);
            layer.spawn((
                Dot,
                Pickable::IGNORE,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(share.x * view_w - size / 2.0),
                    top: Val::Px(share.y * view_h - size / 2.0),
                    width: Val::Px(size),
                    height: Val::Px(size),
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::MAX,
                    ..default()
                },
                BorderColor::all(Color::srgb(0.80, 0.16, 0.12)),
            ));
        }
    });
    minimap.dots = count;
}

fn move_focus_mark(
    minimap: Option<Res<Minimap>>,
    rig: Option<Res<CampaignRig>>,
    mut mark: Query<&mut Node, With<FocusMark>>,
) {
    let (Some(minimap), Some(rig), Ok(mut node)) = (minimap, rig, mark.single_mut()) else {
        return;
    };
    let (view_w, view_h) = Grid::display_size(minimap.grid.crop[2], minimap.grid.crop[3]);
    let share = minimap
        .grid
        .map_to_share(Vec2::new(rig.focus.x, rig.focus.z))
        .clamp(Vec2::ZERO, Vec2::ONE);
    node.left = Val::Px(share.x * view_w - 7.0);
    node.top = Val::Px(share.y * view_h - 7.0);
}

/// Diameter of a map-mode button.
const MODE_BUTTON: f32 = 26.0;

/// A map-mode (lens) button.
#[derive(Component)]
struct ModeButton(MapMode);

/// Ink icon of a map mode (`lens_*` of the ink family).
fn mode_icon(mode: MapMode) -> &'static str {
    match mode {
        MapMode::Political => "lens_political",
        MapMode::Terrain => "lens_relief",
        MapMode::Diplomacy => "lens_diplomacy",
        MapMode::Unrest => "lens_unrest",
        MapMode::Wealth => "lens_wealth",
        MapMode::Population => "lens_population",
    }
}

/// The current map mode's button is inked in rubric on light parchment.
fn highlight_mode(
    mode: Option<Res<MapModeState>>,
    mut buttons: Query<(&ModeButton, &mut BorderColor, &mut BackgroundColor)>,
) {
    let current = mode.map_or(MapMode::Political, |m| m.mode);
    for (button, mut border, mut background) in &mut buttons {
        let on = button.0 == current;
        let (edge, face) = if on {
            (
                crate::ui::kit::style::RUBRIC,
                crate::ui::kit::style::PARCHMENT_LIGHT,
            )
        } else {
            (
                crate::ui::kit::style::GOLD,
                crate::ui::kit::style::PARCHMENT,
            )
        };
        if border.top != edge {
            *border = BorderColor::all(edge);
        }
        if background.0 != face {
            background.0 = face;
        }
    }
}

fn mode_buttons(mut pressed: MessageReader<ButtonPressed>, minimap: Option<ResMut<Minimap>>) {
    let Some(mut minimap) = minimap else { return };
    for press in pressed.read() {
        match press.name.as_str() {
            "hud:minimap_mode:political" => minimap.mode = MinimapMode::Political,
            "hud:minimap_mode:relief" => minimap.mode = MinimapMode::Relief,
            "hud:minimap_relief" => {
                minimap.mode = match minimap.mode {
                    MinimapMode::Political => MinimapMode::Relief,
                    _ => MinimapMode::Political,
                };
            }
            _ => {}
        }
    }
}

fn dump_minimap(world: &World) -> (String, Value) {
    let Some(minimap) = world.get_resource::<Minimap>() else {
        return ("minimap".to_owned(), json!({ "present": false }));
    };
    (
        "minimap".to_owned(),
        json!({
            "present": true,
            "size": [minimap.grid.width, minimap.grid.height],
            "crop": minimap.grid.crop,
            "mode": format!("{:?}", minimap.mode),
            "repaints": minimap.repaints,
            "army_dots": minimap.dots,
            "pings": minimap.pings.len(),
            "clicks": minimap.clicks,
            "relief_baked": minimap.relief.is_some(),
            "fogged": minimap.fogged,
            "orders": minimap.orders,
            "last_order": minimap.last_order.map(|t| [t.x, t.y]),
            "last_target": minimap.last_target.map(|t| [t.x, t.y]),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid() -> Grid {
        Grid {
            crop: [100.0, 50.0, 400.0, 200.0],
            width: 4,
            height: 2,
            cells: vec![0, 1, 1, 2, 0, 1, 2, 2],
        }
    }

    #[test]
    fn share_and_map_round_trip() {
        let g = grid();
        let map = Vec2::new(300.0, 150.0);
        let share = g.map_to_share(map);
        assert_eq!(share, Vec2::new(0.5, 0.5));
        assert_eq!(g.share_to_map(share), map);
    }

    #[test]
    fn display_size_fits_the_frame() {
        let (w, h) = Grid::display_size(400.0, 200.0);
        assert_eq!((w, h), (MAP_WIDTH, 140.0));
        let (w, h) = Grid::display_size(100.0, 400.0);
        assert_eq!(h, MAX_MAP_HEIGHT);
        assert!(w < MAP_WIDTH);
    }

    #[test]
    fn paints_sea_land_and_province_colours() {
        let g = grid();
        let colors = vec![
            Srgba::NONE,
            Srgba::new(1.0, 0.0, 0.0, 1.0),
            Srgba::new(0.0, 0.0, 1.0, 1.0),
        ];
        let political = g.paint(MinimapMode::Political, &colors, None, None);
        assert_eq!(political.len(), 4 * 2 * 4);
        // Sea pixel: the flat sea colour.
        assert_eq!(political[0], (SEA_F[0] * 255.0).round() as u8);
        // Pixel 1 (province 1) leans red.
        assert!(political[4] > political[5]);
        let relief = g.paint(MinimapMode::Relief, &colors, None, None);
        assert_eq!(relief[0], political[0]);
    }

    #[test]
    fn fog_veils_unseen_land_only() {
        let g = Grid {
            crop: [0.0, 0.0, 4.0, 2.0],
            width: 4,
            height: 2,
            cells: vec![1; 8],
        };
        let colors = vec![Srgba::NONE, Srgba::new(1.0, 0.0, 0.0, 1.0)];
        let clear = g.paint(MinimapMode::Political, &colors, None, None);
        let fog = FogOfWar {
            active: true,
            mask: Some(crate::campaign::weather::fog::FogMask {
                width: 4,
                height: 2,
                texel_px: 1.0,
                coverage: vec![255, 255, 0, 0, 255, 255, 0, 0],
            }),
            ..default()
        };
        let fogged = g.paint(MinimapMode::Political, &colors, None, Some(&fog));
        // Seen pixels are untouched, unseen ones lose their red tint.
        assert_eq!(&fogged[0..4], &clear[0..4]);
        assert_ne!(&fogged[8..12], &clear[8..12]);
        assert!(
            i32::from(fogged[8]) - i32::from(fogged[9]) < i32::from(clear[8]) - i32::from(clear[9])
        );
    }

    #[test]
    fn relief_shades_slopes_and_depths() {
        let g = Grid {
            crop: [0.0, 0.0, 8.0, 8.0],
            width: 8,
            height: 8,
            cells: vec![1; 64],
        };
        let flat = g.bake_relief(|_, _| 100.0, |_, _| true, 700.0);
        let high = g.bake_relief(|_, _| 2000.0, |_, _| true, 700.0);
        assert!(high[10][0] < flat[10][0], "higher land is darker");
        let sea = g.bake_relief(|_, _| -1000.0, |_, _| false, 700.0);
        assert!(sea[0][0] < SEA_F[0]);
    }
}
