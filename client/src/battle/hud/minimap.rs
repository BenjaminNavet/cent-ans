//! Battle minimap (bottom right, left of the speed controls). Port of battle_minimap.gd: the
//! field shaded by relief, woods, mud, hedges and ditches, coast, roads, streams, river with its
//! fords, bridges; the regiments as dots in the side colours (diamonds for the enemy, for
//! colour-blind readability); the camera's footprint on the ground; a pulsing ring where an alert
//! was clicked. Left click or drag moves the camera there, right click orders the selection to
//! move to that point ([`MoveSelectionTo`], as the ground does).
//!
//! The picture is rasterised on the CPU into one [`Image`] (twice the node's size): the static
//! layer once when the battle opens, the dots and the frame again when they change. The map is
//! turned half a turn when the player attacks (his camp stays at the bottom), as `flipped`.

use bevy::input::ButtonInput;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::ui::RelativeCursorPosition;
use serde_json::{Value, json};
use sim_battle::{Battlefield, SideId};

use super::{BattleHudRoot, HudData};
use crate::app::AppState;
use crate::battle::orders::input::MoveSelectionTo;
use crate::battle::orders::picking::ground_point;
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::{BattleCamera, BattleCameraState};
use crate::ui::kit::tooltip::Tooltip;

/// Size of the node (`BattleMinimap.SIZE`).
pub const MAP_SIZE: Vec2 = Vec2::new(168.0, 104.0);
/// Pixels of the picture per logical pixel.
const SCALE: u32 = 2;
/// Duration of the ping ring (s).
pub const PING_SECONDS: f32 = 2.5;
const INK: [u8; 4] = [56, 36, 18, 255];
/// Minimum delay between two redraws caused by the camera alone (s).
const CAMERA_REDRAW_S: f32 = 0.05;

/// Ask the minimap to pulse a ring at a world point (x, z): sent when an alert is clicked.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct MinimapPing(pub Vec2);

#[derive(Component)]
pub struct MinimapNode;

/// What the minimap did, for the dump.
#[derive(Resource, Default, Debug)]
pub struct MinimapState {
    pub built: bool,
    pub flipped: bool,
    pub field: Vec2,
    pub dots: usize,
    pub frame: Option<[Vec2; 4]>,
    pub camera_clicks: u32,
    pub orders: u32,
    pub pings: u32,
    pub redraws: u32,
    pub last_world: Option<Vec2>,
    dragging: bool,
    ping: Option<(Vec2, f32)>,
    base: Vec<u8>,
    image: Handle<Image>,
    drawn_revision: u64,
    drawn_frame: Option<[Vec2; 4]>,
    since_camera: f32,
}

// --- Geometry (pure) ---------------------------------------------------------------------

/// Map position (logical pixels, 0..size) of a world point.
pub fn to_map(world: Vec2, field: Vec2, flipped: bool) -> Vec2 {
    let mut t = world / field;
    if flipped {
        t = Vec2::ONE - t;
    }
    t * MAP_SIZE
}

/// World point under a map position (clamped to the node).
pub fn to_world(local: Vec2, field: Vec2, flipped: bool) -> Vec2 {
    let mut t = (local / MAP_SIZE).clamp(Vec2::ZERO, Vec2::ONE);
    if flipped {
        t = Vec2::ONE - t;
    }
    t * field
}

// --- Raster --------------------------------------------------------------------------------

/// RGBA8 canvas with the few primitives the minimap needs.
#[derive(Clone)]
pub struct Raster {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl Raster {
    pub fn new(width: u32, height: u32, fill: [u8; 4]) -> Self {
        let mut data = Vec::with_capacity((width * height * 4) as usize);
        for _ in 0..width * height {
            data.extend_from_slice(&fill);
        }
        Self {
            width,
            height,
            data,
        }
    }

    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let at = ((y * self.width + x) * 4) as usize;
        [
            self.data[at],
            self.data[at + 1],
            self.data[at + 2],
            self.data[at + 3],
        ]
    }

    /// Alpha-blends `color` (straight alpha) over the pixel.
    pub fn blend(&mut self, x: i32, y: i32, color: [u8; 4]) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let at = ((y as u32 * self.width + x as u32) * 4) as usize;
        let a = f32::from(color[3]) / 255.0;
        for (slot, value) in self.data[at..at + 3].iter_mut().zip(color) {
            let old = f32::from(*slot);
            *slot = (old + (f32::from(value) - old) * a).round() as u8;
        }
        self.data[at + 3] = 255;
    }

    pub fn disc(&mut self, centre: Vec2, radius: f32, color: [u8; 4]) {
        let r = radius.max(0.5);
        for y in (centre.y - r).floor() as i32..=(centre.y + r).ceil() as i32 {
            for x in (centre.x - r).floor() as i32..=(centre.x + r).ceil() as i32 {
                let d = Vec2::new(x as f32 + 0.5, y as f32 + 0.5).distance(centre);
                if d <= r {
                    self.blend(x, y, color);
                }
            }
        }
    }

    /// Hollow ring of the given thickness.
    pub fn ring(&mut self, centre: Vec2, radius: f32, thickness: f32, color: [u8; 4]) {
        let outer = radius + thickness * 0.5;
        for y in (centre.y - outer).floor() as i32..=(centre.y + outer).ceil() as i32 {
            for x in (centre.x - outer).floor() as i32..=(centre.x + outer).ceil() as i32 {
                let d = Vec2::new(x as f32 + 0.5, y as f32 + 0.5).distance(centre);
                if (d - radius).abs() <= thickness * 0.5 {
                    self.blend(x, y, color);
                }
            }
        }
    }

    pub fn rect(&mut self, min: Vec2, max: Vec2, color: [u8; 4]) {
        for y in min.y.floor() as i32..max.y.ceil() as i32 {
            for x in min.x.floor() as i32..max.x.ceil() as i32 {
                self.blend(x, y, color);
            }
        }
    }

    /// Diamond of half-diagonal `r`.
    pub fn diamond(&mut self, centre: Vec2, r: f32, color: [u8; 4]) {
        for y in (centre.y - r).floor() as i32..=(centre.y + r).ceil() as i32 {
            for x in (centre.x - r).floor() as i32..=(centre.x + r).ceil() as i32 {
                let d = Vec2::new(x as f32 + 0.5, y as f32 + 0.5) - centre;
                if d.x.abs() + d.y.abs() <= r {
                    self.blend(x, y, color);
                }
            }
        }
    }

    /// Line of the given thickness (stamped discs along the segment).
    pub fn line(&mut self, a: Vec2, b: Vec2, thickness: f32, color: [u8; 4]) {
        let length = a.distance(b);
        let steps = (length * 2.0).ceil().max(1.0) as u32;
        let half = thickness * 0.5;
        for i in 0..=steps {
            let p = a.lerp(b, i as f32 / steps as f32);
            if half <= 0.75 {
                self.blend(p.x.floor() as i32, p.y.floor() as i32, color);
            } else {
                // Opaque stamps only: translucent ones would pile up along the line.
                self.disc(p, half, color);
            }
        }
    }

    pub fn polyline(&mut self, points: &[Vec2], thickness: f32, color: [u8; 4]) {
        for pair in points.windows(2) {
            self.line(pair[0], pair[1], thickness, color);
        }
    }

    pub fn frame(&mut self, color: [u8; 4], thickness: f32) {
        let (w, h) = (self.width as f32, self.height as f32);
        self.rect(Vec2::ZERO, Vec2::new(w, thickness), color);
        self.rect(Vec2::new(0.0, h - thickness), Vec2::new(w, h), color);
        self.rect(Vec2::ZERO, Vec2::new(thickness, h), color);
        self.rect(Vec2::new(w - thickness, 0.0), Vec2::new(w, h), color);
    }
}

fn lerp_color(a: [f32; 3], b: [f32; 3], t: f32) -> [u8; 4] {
    let mix = |i: usize| ((a[i] + (b[i] - a[i]) * t) * 255.0).round() as u8;
    [mix(0), mix(1), mix(2), 255]
}

fn rgba(r: f32, g: f32, b: f32, a: f32) -> [u8; 4] {
    [
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8,
        (a * 255.0).round() as u8,
    ]
}

/// The static layer: relief, woods, mud, obstacles, coast, roads, streams, river, bridges.
pub fn render_base(field: &Battlefield, flipped: bool) -> Raster {
    let (w, h) = ((MAP_SIZE.x as u32) * SCALE, (MAP_SIZE.y as u32) * SCALE);
    let size = Vec2::new(field.width as f32, field.depth as f32);
    let k = SCALE as f32;
    let mut raster = Raster::new(w, h, [199, 189, 140, 255]);
    // Relief: shade by height between the lowest and highest point of the grid.
    let (mut low, mut high) = (f64::INFINITY, f64::NEG_INFINITY);
    for value in &field.heights {
        low = low.min(*value);
        high = high.max(*value);
    }
    let span = (high - low).max(0.01);
    if field.nx >= 2 && field.nz >= 2 && field.heights.len() == field.nx * field.nz {
        for py in 0..h {
            for px in 0..w {
                let map = Vec2::new(px as f32 + 0.5, py as f32 + 0.5) / k;
                let world = to_world(map, size, flipped);
                let t = ((field.height(f64::from(world.x), f64::from(world.y)) - low) / span)
                    .clamp(0.0, 1.0) as f32;
                let color = lerp_color([0.78, 0.74, 0.55], [0.55, 0.47, 0.33], t);
                let at = ((py * w + px) * 4) as usize;
                raster.data[at..at + 4].copy_from_slice(&color);
            }
        }
    }
    let map = |x: f64, z: f64| to_map(Vec2::new(x as f32, z as f32), size, flipped) * k;
    let scale_x = MAP_SIZE.x / size.x * k;
    for zone in &field.forests {
        raster.disc(
            map(zone.x, zone.z),
            zone.radius as f32 * scale_x,
            rgba(0.3, 0.42, 0.2, 0.75),
        );
    }
    for zone in &field.mud {
        raster.disc(
            map(zone.x, zone.z),
            zone.radius as f32 * scale_x,
            rgba(0.45, 0.33, 0.2, 0.55),
        );
    }
    for obstacle in &field.obstacles {
        let (color, width) = match obstacle.kind.key() {
            "ditch" => (rgba(0.35, 0.3, 0.22, 0.9), 1.5),
            "fence" => (rgba(0.5, 0.4, 0.28, 0.9), 1.0),
            "palisade" => (rgba(0.42, 0.27, 0.12, 1.0), 2.5),
            _ => (rgba(0.28, 0.4, 0.18, 0.9), 2.0),
        };
        raster.line(
            map(obstacle.a.0, obstacle.a.1),
            map(obstacle.b.0, obstacle.b.1),
            width * k * 0.5,
            color,
        );
    }
    if let Some(coast) = &field.coast {
        raster.line(
            map(coast.shore_x, 0.0),
            map(coast.shore_x, size.y.into()),
            2.0 * k * 0.5,
            rgba(0.3, 0.45, 0.65, 1.0),
        );
    }
    for road in &field.roads {
        let points: Vec<Vec2> = road.points.iter().map(|p| map(p.0, p.1)).collect();
        let width = if road.kind.key() == "main" { 1.5 } else { 1.0 };
        raster.polyline(&points, width * k * 0.5, rgba(0.62, 0.5, 0.34, 0.85));
    }
    for stream in &field.streams {
        let points: Vec<Vec2> = stream.points.iter().map(|p| map(p.0, p.1)).collect();
        raster.polyline(&points, 1.0 * k * 0.5, rgba(0.36, 0.52, 0.7, 1.0));
    }
    if let Some(river) = &field.river {
        let points: Vec<Vec2> = river
            .polyline(10.0, field.width)
            .iter()
            .map(|p| map(p.0, p.1))
            .collect();
        let thickness = (river.width as f32 * scale_x).max(2.0 * k * 0.5);
        raster.polyline(&points, thickness, rgba(0.3, 0.45, 0.65, 1.0));
        for ford in &river.fords {
            let z = river.center_z(ford.x);
            raster.line(
                map(ford.x - ford.half_width, z),
                map(ford.x + ford.half_width, z),
                (river.width as f32 * scale_x * 0.6).max(2.0),
                rgba(0.72, 0.8, 0.86, 1.0),
            );
        }
    }
    for bridge in &field.bridges {
        let along =
            Vec2::new(bridge.dir.0 as f32, bridge.dir.1 as f32) * bridge.length as f32 * 0.5;
        let centre = Vec2::new(bridge.x as f32, bridge.z as f32);
        let a = to_map(centre - along, size, flipped) * k;
        let b = to_map(centre + along, size, flipped) * k;
        raster.line(a, b, 3.0 * k * 0.5, rgba(0.35, 0.26, 0.18, 1.0));
    }
    raster
}

/// Draws the dynamic layer over a copy of `base`.
#[allow(clippy::too_many_arguments)]
pub fn render_dynamic(
    base: &Raster,
    size: Vec2,
    flipped: bool,
    own: &[Vec2],
    enemy: &[Vec2],
    colors: [Color; 2],
    frame: Option<[Vec2; 4]>,
    ping: Option<(Vec2, f32)>,
) -> Raster {
    let mut raster = base.clone();
    let k = SCALE as f32;
    let tint = |c: Color| {
        let s = c.to_srgba();
        rgba(s.red, s.green, s.blue, 1.0)
    };
    for p in own {
        let at = to_map(*p, size, flipped) * k;
        raster.rect(
            at - Vec2::splat(2.0 * k * 0.5),
            at + Vec2::splat(2.0 * k * 0.5),
            tint(colors[0]),
        );
    }
    for p in enemy {
        let at = to_map(*p, size, flipped) * k;
        raster.diamond(at, 3.5 * k * 0.5, tint(colors[1]));
    }
    if let Some(corners) = frame {
        let mut points: Vec<Vec2> = corners
            .iter()
            .map(|p| to_map(*p, size, flipped) * k)
            .collect();
        points.push(points[0]);
        raster.polyline(&points, 1.0, rgba(1.0, 0.97, 0.85, 1.0));
    }
    if let Some((world, age)) = ping {
        let t = (age / PING_SECONDS).clamp(0.0, 1.0);
        let radius = (2.0 + 12.0 * t) * k * 0.5;
        raster.ring(
            to_map(world, size, flipped) * k,
            radius,
            2.5 * k * 0.5,
            rgba(1.0, 0.85, 0.3, 1.0 - t),
        );
    }
    raster.frame(INK, 1.5 * k * 0.5);
    raster
}

fn upload(images: &mut Assets<Image>, handle: &Handle<Image>, raster: &Raster) {
    if let Some(mut image) = images.get_mut(handle) {
        image.data = Some(raster.data.clone());
    }
}

// --- Systems -------------------------------------------------------------------------------

pub struct MinimapPlugin;

impl Plugin for MinimapPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<MinimapPing>()
            .init_resource::<MinimapState>()
            .add_systems(OnEnter(AppState::Battle), reset)
            .add_systems(
                Update,
                (spawn, interact, redraw)
                    .chain()
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>),
            );
    }
}

fn reset(mut state: ResMut<MinimapState>) {
    *state = MinimapState::default();
}

/// Builds the node once the HUD root exists.
fn spawn(
    mut commands: Commands,
    roots: Query<Entity, With<BattleHudRoot>>,
    existing: Query<(), With<MinimapNode>>,
    mut session: ResMut<BattleSession>,
    mut state: ResMut<MinimapState>,
    mut images: ResMut<Assets<Image>>,
) {
    let Ok(root) = roots.single() else { return };
    if !existing.is_empty() || state.built {
        return;
    }
    let player = session.player.unwrap_or(SideId::Attacker);
    let (base, size) = {
        let field = session.sim_mut().field();
        let flipped = player == SideId::Attacker;
        (
            render_base(field, flipped),
            Vec2::new(field.width as f32, field.depth as f32),
        )
    };
    let image = Image::new(
        Extent3d {
            width: base.width,
            height: base.height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        base.data.clone(),
        TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::default(),
    );
    let handle = images.add(image);
    state.built = true;
    state.flipped = player == SideId::Attacker;
    state.field = size;
    state.base = base.data;
    state.image = handle.clone();
    commands.spawn((
        Name::new("Minimap"),
        MinimapNode,
        RelativeCursorPosition::default(),
        Hovered::default(),
        Tooltip::new(
            "[b]Minicarte[/b]\nClic ou glisser : déplacer la caméra. Clic droit : envoyer la sélection à cet endroit (Maj : ordre en file).",
        ),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(super::speed::SPEED_PANEL_WIDTH + 14.0),
            bottom: Val::Px(6.0),
            width: Val::Px(MAP_SIZE.x),
            height: Val::Px(MAP_SIZE.y),
            ..default()
        },
        ImageNode::new(handle),
        ChildOf(root),
    ));
}

/// Camera and orders from the pointer.
#[allow(clippy::too_many_arguments)]
fn interact(
    nodes: Query<&RelativeCursorPosition, With<MinimapNode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<MinimapState>,
    mut session: ResMut<BattleSession>,
    camera: Option<ResMut<BattleCameraState>>,
    mut moves: MessageWriter<MoveSelectionTo>,
) {
    let Ok(relative) = nodes.single() else {
        return;
    };
    let Some(normalized) = relative.normalized else {
        return;
    };
    let local = (normalized + Vec2::splat(0.5)) * MAP_SIZE;
    let world = to_world(local, state.field, state.flipped);
    if buttons.just_pressed(MouseButton::Left) && relative.cursor_over {
        state.dragging = true;
        state.camera_clicks += 1;
    }
    if !buttons.pressed(MouseButton::Left) {
        state.dragging = false;
    }
    if state.dragging
        && let Some(mut camera) = camera
    {
        camera.target.x = world.x;
        camera.target.z = world.y;
        state.last_world = Some(world);
    }
    if relative.cursor_over && buttons.just_released(MouseButton::Right) {
        let height = session
            .sim_mut()
            .field()
            .height(f64::from(world.x), f64::from(world.y)) as f32;
        moves.write(MoveSelectionTo {
            point: Vec3::new(world.x, height, world.y),
            queued: keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]),
            run: false,
        });
        state.orders += 1;
        state.last_world = Some(world);
    }
}

/// Footprint of the camera on the ground: the four corners of the view, rays kept below the
/// horizon so that a high tilt still gives a finite quad.
fn camera_frame(
    camera: &Camera,
    transform: &GlobalTransform,
    field: &Battlefield,
) -> Option<[Vec2; 4]> {
    let size = camera.logical_viewport_size()?;
    let corners = [
        Vec2::new(0.0, 0.0),
        Vec2::new(size.x, 0.0),
        Vec2::new(size.x, size.y),
        Vec2::new(0.0, size.y),
    ];
    let mut out = [Vec2::ZERO; 4];
    for (slot, corner) in out.iter_mut().zip(corners) {
        let ray = camera.viewport_to_world(transform, corner).ok()?;
        let mut dir = Vec3::from(ray.direction);
        if dir.y > -0.03 {
            dir.y = -0.03;
            dir = dir.normalize();
        }
        let point = ground_point(ray.origin, dir, field);
        *slot = Vec2::new(point.x, point.z);
    }
    Some(out)
}

#[allow(clippy::too_many_arguments)]
fn redraw(
    time: Res<Time<Real>>,
    data: Res<HudData>,
    mut state: ResMut<MinimapState>,
    mut session: ResMut<BattleSession>,
    mut pings: MessageReader<MinimapPing>,
    cameras: Query<(&Camera, &GlobalTransform), With<BattleCamera>>,
    mut images: ResMut<Assets<Image>>,
) {
    if !state.built {
        return;
    }
    let delta = time.delta_secs();
    state.since_camera += delta;
    for ping in pings.read() {
        state.ping = Some((ping.0, 0.0));
        state.pings += 1;
    }
    let mut ping_active = false;
    if let Some((_, age)) = state.ping.as_mut() {
        *age += delta;
        ping_active = *age < PING_SECONDS;
    }
    if !ping_active {
        state.ping = None;
    }
    let frame = cameras
        .single()
        .ok()
        .and_then(|(camera, transform)| camera_frame(camera, transform, session.sim_mut().field()));
    let moved = match (frame, state.drawn_frame) {
        (Some(a), Some(b)) => a.iter().zip(b).any(|(p, q)| p.distance(q) > 1.0),
        (a, b) => a.is_some() != b.is_some(),
    };
    let dots_changed = data.revision != state.drawn_revision;
    if !(dots_changed || ping_active || (moved && state.since_camera >= CAMERA_REDRAW_S)) {
        return;
    }
    let own: Vec<Vec2> = data
        .own
        .iter()
        .filter(|u| u.present)
        .map(|u| Vec2::new(u.x as f32, u.z as f32))
        .collect();
    let enemy: Vec<Vec2> = data
        .enemy
        .iter()
        .filter(|u| u.present)
        .map(|u| Vec2::new(u.x as f32, u.z as f32))
        .collect();
    let base = Raster {
        width: MAP_SIZE.x as u32 * SCALE,
        height: MAP_SIZE.y as u32 * SCALE,
        data: std::mem::take(&mut state.base),
    };
    let raster = render_dynamic(
        &base,
        state.field,
        state.flipped,
        &own,
        &enemy,
        data.colors,
        frame,
        state.ping,
    );
    state.base = base.data;
    upload(&mut images, &state.image, &raster);
    state.dots = own.len() + enemy.len();
    state.frame = frame;
    state.drawn_frame = frame;
    state.drawn_revision = data.revision;
    state.since_camera = 0.0;
    state.redraws += 1;
}

pub fn dump(world: &World) -> Value {
    let state = world.resource::<MinimapState>();
    json!({
        "built": state.built,
        "flipped": state.flipped,
        "field": [state.field.x, state.field.y],
        "dots": state.dots,
        "frame": state.frame.map(|f| f.iter().map(|p| [p.x.round(), p.y.round()]).collect::<Vec<_>>()),
        "camera_clicks": state.camera_clicks,
        "orders": state.orders,
        "pings": state.pings,
        "redraws": state.redraws,
        "last_world": state.last_world.map(|p| [p.x.round(), p.y.round()]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_round_trip() {
        let field = Vec2::new(1200.0, 800.0);
        for flipped in [false, true] {
            let world = Vec2::new(300.0, 600.0);
            let back = to_world(to_map(world, field, flipped), field, flipped);
            assert!(back.distance(world) < 1e-3);
        }
    }

    #[test]
    fn flipped_map_puts_the_far_corner_at_the_origin() {
        let field = Vec2::new(1200.0, 800.0);
        assert_eq!(to_map(field, field, true), Vec2::ZERO);
        assert_eq!(to_map(Vec2::ZERO, field, false), Vec2::ZERO);
        assert_eq!(
            to_world(Vec2::new(-5.0, 999.0), field, false),
            Vec2::new(0.0, 800.0)
        );
    }

    #[test]
    fn raster_primitives_stay_inside() {
        let mut raster = Raster::new(20, 10, [0, 0, 0, 255]);
        raster.disc(Vec2::new(-3.0, 4.0), 5.0, [255, 0, 0, 255]);
        raster.line(
            Vec2::new(-10.0, -10.0),
            Vec2::new(40.0, 20.0),
            3.0,
            [0, 255, 0, 255],
        );
        raster.diamond(Vec2::new(10.0, 5.0), 3.0, [0, 0, 255, 255]);
        assert_eq!(raster.pixel(10, 5), [0, 0, 255, 255]);
        assert_eq!(raster.data.len(), 20 * 10 * 4);
    }

    #[test]
    fn dynamic_layer_draws_dots_and_keeps_the_base() {
        let base = Raster::new(336, 208, [200, 190, 140, 255]);
        let colors = [Color::srgb(0.0, 0.0, 1.0), Color::srgb(1.0, 0.0, 0.0)];
        let raster = render_dynamic(
            &base,
            Vec2::new(1200.0, 800.0),
            false,
            &[Vec2::new(600.0, 400.0)],
            &[Vec2::new(300.0, 200.0)],
            colors,
            None,
            None,
        );
        assert_eq!(raster.pixel(168, 104), [0, 0, 255, 255]);
        assert_eq!(raster.pixel(84, 52), [255, 0, 0, 255]);
        assert_eq!(base.pixel(168, 104), [200, 190, 140, 255]);
        assert_eq!(raster.pixel(100, 100), [200, 190, 140, 255]);
    }
}
