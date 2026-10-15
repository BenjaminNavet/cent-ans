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

use super::{HudSet, HudWidget};
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::armies::ArmyViews;
use crate::campaign::camera::FocusCamera;
use crate::campaign::camera::rig::CampaignRig;
use crate::campaign::provinces::data::ProvinceMap;
use crate::campaign::provinces::overlay::OverlayState;
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::widgets::{button_small, panel_inset};
use crate::ui::kit::{ButtonPressed, UiTheme};

/// Width of the map in the frame, in logical pixels (`MAP_WIDTH` of Godot).
pub const MAP_WIDTH: f32 = 280.0;
const MAX_MAP_HEIGHT: f32 = 230.0;
/// Resolution of the baked image relative to the display (`TEXTURE_SCALE`).
const TEXTURE_SCALE: f32 = 1.5;
/// Margin around the extent of the provinces (share of its size).
const CROP_MARGIN: f32 = 0.03;
const SEA: [u8; 4] = [117, 140, 143, 255];
const LOWLAND: [u8; 4] = [219, 204, 158, 255];
const BORDER_DARKEN: f32 = 0.82;
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

    /// RGBA8 pixels for `mode` with the province `colors` (indexed by raster index).
    pub fn paint(&self, mode: MinimapMode, colors: &[Srgba]) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.cells.len() * 4);
        let w = self.width as usize;
        for (i, &index) in self.cells.iter().enumerate() {
            let pixel = if index == 0 {
                SEA
            } else if mode == MinimapMode::Relief {
                LOWLAND
            } else {
                let color = colors
                    .get(usize::from(index))
                    .copied()
                    .unwrap_or(Srgba::NONE);
                if color.alpha <= 0.0 {
                    LOWLAND
                } else {
                    let base = color.to_u8_array();
                    // A thin border where the neighbour belongs to another province.
                    let right = self.cells.get(i + 1).copied().unwrap_or(index);
                    let below = self.cells.get(i + w).copied().unwrap_or(index);
                    if (right != index && (i + 1) % w != 0) || below != index {
                        [
                            (f32::from(base[0]) * BORDER_DARKEN) as u8,
                            (f32::from(base[1]) * BORDER_DARKEN) as u8,
                            (f32::from(base[2]) * BORDER_DARKEN) as u8,
                            255,
                        ]
                    } else {
                        [base[0], base[1], base[2], 255]
                    }
                }
            };
            out.extend_from_slice(&pixel);
        }
        out
    }
}

#[derive(Resource)]
pub struct Minimap {
    grid: Grid,
    image: Handle<Image>,
    pub mode: MinimapMode,
    painted_colors: Vec<Srgba>,
    painted_mode: Option<MinimapMode>,
    pub repaints: u32,
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
    column: Query<Entity, With<super::RightColumn>>,
    mut images: ResMut<Assets<Image>>,
    theme: Res<UiTheme>,
) {
    let Some(map) = map else { return };
    let Ok(column) = column.single() else { return };
    if existing.is_some() {
        return;
    }
    let grid = Grid::bake(&map);
    let pixels = grid.paint(MinimapMode::Political, &[]);
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
            ChildOf(column),
            HudWidget,
            Hovered::default(),
            panel_inset(&theme),
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                padding: UiRect::all(Val::Px(6.0)),
                ..default()
            },
        ))
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
                });
            frame
                .spawn(Node {
                    column_gap: Val::Px(4.0),
                    justify_content: JustifyContent::FlexEnd,
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        button_small(&theme, "hud:minimap_mode:political", "Politique"),
                        Tooltip::new("Couleurs des royaumes"),
                    ));
                    row.spawn((
                        button_small(&theme, "hud:minimap_mode:relief", "Relief"),
                        Tooltip::new("Terres et mers"),
                    ));
                    row.spawn((
                        button_small(&theme, "hud:legend", "Légende"),
                        Tooltip::new("Ce que montrent les symboles de la carte"),
                    ));
                });
        });
    commands.insert_resource(Minimap {
        grid,
        image,
        mode: MinimapMode::Political,
        painted_colors: Vec::new(),
        painted_mode: None,
        repaints: 0,
        clicks: 0,
        last_target: None,
        dots: 0,
        pings: Vec::new(),
    });
}

/// Repaints the image when the province colours or the mode changed.
fn paint_minimap(
    minimap: Option<ResMut<Minimap>>,
    overlay: Option<Res<OverlayState>>,
    mut images: ResMut<Assets<Image>>,
) {
    let (Some(mut minimap), Some(overlay)) = (minimap, overlay) else {
        return;
    };
    if minimap.painted_mode == Some(minimap.mode) && minimap.painted_colors == overlay.colors {
        return;
    }
    let pixels = minimap.grid.paint(minimap.mode, &overlay.colors);
    if let Some(mut image) = images.get_mut(&minimap.image) {
        image.data = Some(pixels);
    }
    minimap.painted_colors = overlay.colors.clone();
    minimap.painted_mode = Some(minimap.mode);
    minimap.repaints += 1;
}

fn minimap_input(
    minimap: Option<ResMut<Minimap>>,
    view: Query<&RelativeCursorPosition, With<MinimapView>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut focus: MessageWriter<FocusCamera>,
) {
    let (Some(mut minimap), Ok(cursor)) = (minimap, view.single()) else {
        return;
    };
    let Some(normalized) = cursor.normalized else {
        return;
    };
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

fn mode_buttons(mut pressed: MessageReader<ButtonPressed>, minimap: Option<ResMut<Minimap>>) {
    let Some(mut minimap) = minimap else { return };
    for press in pressed.read() {
        match press.name.as_str() {
            "hud:minimap_mode:political" => minimap.mode = MinimapMode::Political,
            "hud:minimap_mode:relief" => minimap.mode = MinimapMode::Relief,
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
        let political = g.paint(MinimapMode::Political, &colors);
        assert_eq!(political.len(), 4 * 2 * 4);
        assert_eq!(&political[0..4], &SEA);
        // Pixel 1 (province 1, interior of the first row) is red.
        assert_eq!(political[4], 255);
        let relief = g.paint(MinimapMode::Relief, &colors);
        assert_eq!(&relief[4..8], &LOWLAND);
        assert_eq!(&relief[0..4], &SEA);
    }
}
