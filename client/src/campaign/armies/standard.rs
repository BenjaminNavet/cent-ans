//! Standard of the armies: pole, finial and flag in the faction colour, carried by the lord (or the
//! bearer on foot), swaying with his steps. Port of the pole of `army_marker.gd` and of the
//! sway of `army_figures.gd` (`_update_bearer_sway`, `bearer_anchor`, `bearer_tilt`; lot AS2,
//! settings `standard_sway` of `data/fx/campaign_army_walk.json`).
//!
//! On foot or on horse the foot of the pole follows the bearer's hand and bobs at every step; the
//! pole rolls from one foot to the other and leans forwards, all fading in and out with the
//! walk. The flag is a quad turned towards the camera around the pole (the Godot shader also
//! waves the cloth: here the whole flag only flutters a little, stronger when the troop walks).
//!
//! The flag shows the army's banner (`cloth.rs`: royal, house, faction banner, shield, plain
//! colour; vertex-wave shader `shaders/flag.wesl`, port of `map_banner.gdshader`, lit with the
//! ground light (`VegLight`) by the same wrapped Lambert as Godot). At sea the pole stands in the
//! stern castle of the first cog (`ArmyFigures.bearer_anchor`, kind `fleet`). The stance badge on
//! the finial is `panels/diplomacy/stance_badge.rs`. Not ported: the pitch and roll of the ship
//! (the cogs do not pitch here), the fall of the pole to the marker foot in the far view.

use std::collections::HashMap;
use std::f32::consts::TAU;

use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;
use serde_json::{Value, json};
use sim_campaign::ArmyId;

use super::cloth::{
    ClothChoice, ClothMode, ClothQuery, ClothSource, FlagMaterial, HouseBook, POLE_TALL,
};
use super::figures::{SKINNED_BEARER_SLOT, lord_slot};
use super::markers::ArmyMarker;
use super::sea::Glides;
use super::skinned::{HIDE_STRATEGIC, SkinnedArmies};
use super::views::ArmyViews;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::camera::zoom::ZoomView;

/// Hand of the lord, per unit of enlargement (`ArmyFigures.LORD_HAND`).
const LORD_HAND: Vec3 = Vec3::new(0.4, 3.8, 0.7);
/// Pole in the hand of the bearer on foot (`POLE_IN_HAND`).
const POLE_IN_HAND: Vec2 = Vec2::new(0.35, 0.55);
/// Rest heights above the hand (`army_marker.tscn`): pole centre, finial, flag top.
const POLE_HEIGHT: f32 = 8.4;
const POLE_REST_Y: f32 = 4.2;
const FINIAL_REST_Y: f32 = 8.55;
const FLAG_REST_Y: f32 = 8.2;
/// Height of a man (`ArmyFigures.FIGURE_SCALE`), used by the bob.
const FIGURE_SCALE: f32 = super::figures::SKINNED_SCALE;
/// Foot of the stern pole in the ship frame (`STERN_STAFF` x `SHIP_SCALE`), sunk by 3 in the marker
/// frame (`bearer_anchor`, fleet).
const STERN_STAFF: Vec3 = Vec3::new(-0.9, 0.95, 0.0);
const STERN_SINK: f32 = 3.0;

/// Foot of the pole of a fleet in the marker frame, before the marker's own rotation.
pub fn stern_anchor() -> Vec3 {
    STERN_STAFF * super::figures::SHIP_SCALE - Vec3::Y * STERN_SINK
}

/// `standard_sway` and `enabled` of `data/fx/campaign_army_walk.json`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SwaySettings {
    pub enabled: bool,
    pub step_hz: f32,
    pub bob: f32,
    pub roll_deg: f32,
    pub lean_deg: f32,
    pub fade_per_s: f32,
    pub cloth_gust: f32,
}

impl Default for SwaySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            step_hz: 1.9,
            bob: 0.05,
            roll_deg: 3.0,
            lean_deg: 2.0,
            fade_per_s: 6.0,
            cloth_gust: 0.6,
        }
    }
}

impl SwaySettings {
    pub fn load(data_dir: &std::path::Path) -> Self {
        let mut settings = Self::default();
        let json = std::fs::read_to_string(data_dir.join("fx/campaign_army_walk.json"))
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok());
        if let Some(json) = json {
            settings.enabled = json["enabled"].as_bool().unwrap_or(true);
            let sway = &json["standard_sway"];
            let f = |key: &str, old: f32| sway[key].as_f64().map_or(old, |v| v as f32);
            settings.step_hz = f("step_hz", settings.step_hz);
            settings.bob = f("bob", settings.bob);
            settings.roll_deg = f("roll_deg", settings.roll_deg);
            settings.lean_deg = f("lean_deg", settings.lean_deg);
            settings.fade_per_s = f("fade_per_s", settings.fade_per_s);
            settings.cloth_gust = f("cloth_gust", settings.cloth_gust);
        }
        settings
    }
}

/// Amplitude (0 at rest, 1 walking) and step phase of one standard.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Sway {
    pub amp: f32,
    pub phase: f32,
}

impl Sway {
    /// Advances by `dt`: the amplitude moves towards 1 walking and 0 at rest, the phase at the
    /// pace of the steps (`cadence` = clock factor of the bearer's figure).
    pub fn advance(&mut self, walking: bool, dt: f32, cadence: f32, cfg: &SwaySettings) {
        let target = if walking { 1.0 } else { 0.0 };
        let step = cfg.fade_per_s * dt;
        self.amp += (target - self.amp).clamp(-step, step);
        if self.amp > 0.0 {
            self.phase = (self.phase + dt * cfg.step_hz * cadence * TAU).rem_euclid(TAU * 2.0);
        }
    }

    /// Vertical bob (marker units) and the rotation of the pole in the troop frame (rolls round
    /// the forward axis, leans forwards). A step is half a turn of `phase * 0.5`.
    pub fn pose(&self, cfg: &SwaySettings) -> (f32, Quat) {
        if self.amp <= 0.0 {
            return (0.0, Quat::IDENTITY);
        }
        let swing = (self.phase * 0.5).sin();
        let bob = swing.abs() * cfg.bob * FIGURE_SCALE * self.amp;
        let roll = swing * cfg.roll_deg.to_radians() * self.amp;
        let lean = cfg.lean_deg.to_radians() * self.amp;
        (
            bob,
            Quat::from_rotation_x(roll) * Quat::from_rotation_z(-lean),
        )
    }
}

/// Tilt of the pole in the marker frame: the troop-frame rotation conjugated by the heading
/// (the troop looks at +X, `heading` turns it).
pub fn tilt_in_marker(local: Quat, heading: f32) -> Quat {
    let yaw = Quat::from_rotation_y(heading);
    yaw * local * yaw.inverse()
}

/// Foot of the pole in the troop frame: the lord's hand or the bearer's hand.
pub fn hand(lord_scale: f32) -> Vec3 {
    if lord_scale > 1.001 {
        let slot = lord_slot(lord_scale);
        Vec3::new(
            slot.x + LORD_HAND.x * lord_scale,
            LORD_HAND.y * lord_scale,
            slot.y + LORD_HAND.z * lord_scale,
        )
    } else {
        Vec3::new(
            SKINNED_BEARER_SLOT.x + POLE_IN_HAND.x,
            0.0,
            SKINNED_BEARER_SLOT.y + POLE_IN_HAND.y,
        )
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandardPart {
    Pole,
    Finial,
    Flag,
}

#[derive(Component)]
pub struct StandardOf(pub ArmyId);

struct Parts {
    entities: [Entity; 3],
    sway: Sway,
    material: Handle<FlagMaterial>,
    /// Rest heights of pole, finial and flag, and the stretch of the pole (hung banners ride on
    /// a taller pole).
    rest_y: [f32; 3],
    pole_stretch: f32,
    choice: ClothChoice,
}

#[derive(Default, Debug, Clone)]
pub struct StandardReport {
    pub standards: usize,
    pub swaying: usize,
    pub max_amp: f32,
    pub max_bob: f32,
    pub max_tilt_deg: f32,
    pub at_sea: usize,
    /// Flag materials that received the ground light.
    pub lit: usize,
}

/// The standards of the armies and the settings.
#[derive(Resource, Default)]
pub struct Standards {
    pub settings: SwaySettings,
    parts: HashMap<ArmyId, Parts>,
    houses: Option<HouseBook>,
    asset_root: std::path::PathBuf,
    meshes: HashMap<(u8, u32), Handle<Mesh>>,
    pub report: StandardReport,
}

/// Loads the sway settings when the campaign opens.
pub fn load_settings(mut standards: ResMut<Standards>) {
    let root = crate::paths::find_root().ok();
    standards.settings = root
        .as_ref()
        .map(|root| SwaySettings::load(&crate::paths::data_dir(root)))
        .unwrap_or_default();
    standards.houses = root
        .as_ref()
        .map(|root| HouseBook::load(&crate::paths::data_dir(root)));
    standards.asset_root = root
        .as_ref()
        .map(|root| crate::paths::asset_root(root))
        .unwrap_or_default();
}

pub fn reset(mut standards: ResMut<Standards>) {
    standards.parts.clear();
    standards.meshes.clear();
    standards.report = StandardReport::default();
}

fn spawn_parts(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    id: &ArmyId,
    flag_mesh: Handle<Mesh>,
    flag_material: Handle<FlagMaterial>,
) -> [Entity; 3] {
    let mut part = |part: StandardPart, mesh: Handle<Mesh>, material: StandardMaterial| {
        commands
            .spawn((
                Name::new(format!("Standard_{:?}_{}", part, id.as_str())),
                part,
                StandardOf(id.clone()),
                Mesh3d(mesh),
                MeshMaterial3d(materials.add(material)),
                Transform::default(),
                Visibility::Inherited,
                NotShadowCaster,
                DespawnOnExit(AppState::Campaign),
            ))
            .id()
    };
    let pole = part(
        StandardPart::Pole,
        meshes.add(Mesh::from(
            Cylinder::new(0.1, POLE_HEIGHT).mesh().resolution(6),
        )),
        StandardMaterial {
            base_color: Color::srgb(0.22, 0.15, 0.09),
            perceptual_roughness: 0.75,
            ..default()
        },
    );
    let finial = part(
        StandardPart::Finial,
        meshes.add(Sphere::new(0.22).mesh().uv(8, 4)),
        StandardMaterial {
            base_color: Color::srgb(0.78, 0.58, 0.2),
            metallic: 0.9,
            perceptual_roughness: 0.35,
            ..default()
        },
    );
    // Origin on the pole; the cloth shape and the wave live in `cloth.rs` / `flag.wesl`.
    let flag = commands
        .spawn((
            Name::new(format!("Standard_Flag_{}", id.as_str())),
            StandardPart::Flag,
            StandardOf(id.clone()),
            Mesh3d(flag_mesh),
            MeshMaterial3d(flag_material),
            Transform::default(),
            Visibility::Inherited,
            NotShadowCaster,
            DespawnOnExit(AppState::Campaign),
        ))
        .id();
    [pole, finial, flag]
}

/// One system: spawns the standard of every marker that lacks one, drops the ones of armies that
/// left, advances the sway and places the three parts.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn update_standards(
    mut commands: Commands,
    mut standards: ResMut<Standards>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut flag_materials: ResMut<Assets<FlagMaterial>>,
    asset_server: Option<Res<AssetServer>>,
    time: Res<Time>,
    views: Res<ArmyViews>,
    glides: Res<Glides>,
    skinned: Res<SkinnedArmies>,
    zoom: Option<Res<ZoomView>>,
    light: Option<Res<crate::campaign::vegetation::light::VegLight>>,
    markers: Query<(&ArmyMarker, &Transform)>,
    existing: Query<(), With<StandardPart>>,
    mut parts_query: Query<(&StandardPart, &mut Transform, &mut Visibility), Without<ArmyMarker>>,
) {
    let standards = &mut *standards;
    let dt = time.delta_secs();
    let cfg = standards.settings;
    let lord_scale = skinned.lord_scale.max(1.0);
    let hidden_by_zoom = zoom.is_some_and(|z| z.strategic > HIDE_STRATEGIC);
    let mut report = StandardReport::default();
    let mut live: Vec<&ArmyId> = Vec::new();
    for (marker, transform) in &markers {
        let Some(view) = views.get(&marker.id) else {
            continue;
        };
        live.push(&marker.id);
        // A standard whose entities are gone (state change) is spawned again.
        if standards
            .parts
            .get(&marker.id)
            .is_some_and(|p| p.entities.iter().any(|e| existing.get(*e).is_err()))
        {
            standards.parts.remove(&marker.id);
        }
        if !standards.parts.contains_key(&marker.id) {
            let choice = {
                let house = view.general.as_deref().and_then(|g| {
                    standards
                        .houses
                        .as_mut()?
                        .house_of(g, view.faction.as_str())
                });
                let query = ClothQuery {
                    faction: view.faction.as_str(),
                    royal: view.royal,
                    house: house.as_deref(),
                };
                let assets = &standards.asset_root;
                super::cloth::choose(&query, |f| super::cloth::png_size(&assets.join(f)))
            };
            let shape = super::cloth::shape(choice.mode, choice.aspect);
            let mesh_key = (choice.mode as u8, shape.size.y.to_bits());
            let flag_mesh = standards
                .meshes
                .entry(mesh_key)
                .or_insert_with(|| meshes.add(super::cloth::cloth_mesh(&shape)))
                .clone();
            let texture = choice
                .file
                .as_ref()
                .and_then(|f| asset_server.as_ref().map(|s| s.load::<Image>(f.clone())));
            let material = flag_materials.add(FlagMaterial::new(view.color, &choice, texture));
            let entities = spawn_parts(
                &mut commands,
                &mut meshes,
                &mut materials,
                &marker.id,
                flag_mesh,
                material.clone(),
            );
            let tall = choice.mode == ClothMode::Banner;
            standards.parts.insert(
                marker.id.clone(),
                Parts {
                    entities,
                    sway: Sway::default(),
                    material,
                    rest_y: if tall {
                        [POLE_TALL * 0.5, POLE_TALL + 0.15, POLE_TALL - 0.35]
                    } else {
                        [POLE_REST_Y, FINIAL_REST_Y, FLAG_REST_Y]
                    },
                    pole_stretch: if tall { POLE_TALL / POLE_HEIGHT } else { 1.0 },
                    choice,
                },
            );
            continue;
        }
        let Some(parts) = standards.parts.get_mut(&marker.id) else {
            continue;
        };
        let bearer = if lord_scale > 1.001 {
            ("cavalry", 0)
        } else {
            ("infantry", 0)
        };
        let at_sea = glides.active.contains_key(&marker.id);
        let walking = marker.walking && cfg.enabled && skinned.enabled && !at_sea;
        parts
            .sway
            .advance(walking, dt, skinned.group_factor(&marker.id, bearer), &cfg);
        let (bob, local) = if cfg.enabled {
            parts.sway.pose(&cfg)
        } else {
            (0.0, Quat::IDENTITY)
        };
        let tilt = tilt_in_marker(local, marker.heading);
        // At sea the pole stands in the stern of the flagship, which heads along the route.
        let anchor = if at_sea {
            transform.rotation * stern_anchor()
        } else {
            Quat::from_rotation_y(marker.heading) * hand(lord_scale) + Vec3::Y * bob
        };
        let scale = transform.scale;
        let shown = skinned.enabled && !hidden_by_zoom;
        report.standards += 1;
        if at_sea {
            report.at_sea += 1;
        }
        if parts.sway.amp > 0.0 {
            report.swaying += 1;
        }
        report.max_amp = report.max_amp.max(parts.sway.amp);
        report.max_bob = report.max_bob.max(bob);
        report.max_tilt_deg = report
            .max_tilt_deg
            .max(tilt.angle_between(Quat::IDENTITY).to_degrees());
        for entity in parts.entities {
            let Ok((part, mut part_transform, mut visibility)) = parts_query.get_mut(entity) else {
                continue;
            };
            let wanted = if shown {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            if *visibility != wanted {
                *visibility = wanted;
            }
            let rest_y = match part {
                StandardPart::Pole => parts.rest_y[0],
                StandardPart::Finial => parts.rest_y[1],
                StandardPart::Flag => parts.rest_y[2],
            };
            let local_position = anchor + tilt * Vec3::new(0.0, rest_y, 0.0);
            part_transform.translation = transform.translation + scale * local_position;
            part_transform.scale = match part {
                StandardPart::Pole => scale * Vec3::new(1.0, parts.pole_stretch, 1.0),
                _ => scale,
            };
            // The flag is turned towards the camera and waved by its shader: only the pole's
            // lean is handed over (`pole_dir`, `gust`).
            part_transform.rotation = match part {
                StandardPart::Flag => Quat::IDENTITY,
                _ => tilt,
            };
        }
        if let Some(mut material) = flag_materials.get_mut(&parts.material) {
            let pole_dir = (tilt * Vec3::Y).normalize_or(Vec3::Y);
            let gust = parts.sway.amp * cfg.cloth_gust;
            let wanted = pole_dir.extend(gust);
            if material.params.pole_gust != wanted {
                material.params.pole_gust = wanted;
            }
            if let Some(light) = light.as_deref()
                && light.sun_dir.w > 0.5
                && (material.params.sun_dir != light.sun_dir
                    || material.params.sun_color != light.sun_color
                    || material.params.ambient != light.ambient)
            {
                material.params.sun_dir = light.sun_dir;
                material.params.sun_color = light.sun_color;
                material.params.ambient = light.ambient;
            }
            if material.params.sun_dir.w > 0.5 {
                report.lit += 1;
            }
        }
    }
    standards.parts.retain(|id, parts| {
        let keep = live.contains(&id);
        if !keep {
            for entity in parts.entities {
                commands.entity(entity).despawn();
            }
        }
        keep
    });
    standards.report = report;
}

pub fn dump_standards(world: &World) -> (String, Value) {
    let standards = world.resource::<Standards>();
    let r = &standards.report;
    let shown = world
        .try_query::<(&StandardPart, &Visibility)>()
        .map_or(0, |mut q| {
            q.iter(world)
                .filter(|(p, v)| **p == StandardPart::Flag && **v != Visibility::Hidden)
                .count()
        });
    let mut by_source = std::collections::BTreeMap::<&str, usize>::new();
    let mut by_mode = std::collections::BTreeMap::<&str, usize>::new();
    let mut armies: Vec<Value> = Vec::new();
    let views = world.get_resource::<ArmyViews>();
    for (id, parts) in &standards.parts {
        let source = match parts.choice.source {
            ClothSource::Royal => "royal",
            ClothSource::House => "house",
            ClothSource::FactionBanner => "faction_banner",
            ClothSource::Shield => "shield",
            ClothSource::PlainColour => "plain_colour",
        };
        *by_source.entry(source).or_default() += 1;
        let mode = match parts.choice.mode {
            ClothMode::Flag => "flag",
            ClothMode::Pennant => "pennant",
            ClothMode::Banner => "banner",
        };
        *by_mode.entry(mode).or_default() += 1;
        armies.push(json!({
            "army": id.as_str(),
            "faction": views.and_then(|v| v.get(id)).map(|v| v.faction.as_str().to_owned()),
            "banner": parts.choice.id(),
            "source": source,
            "mode": mode,
        }));
    }
    armies.sort_by_key(|a| a["army"].as_str().map(str::to_owned));
    let textured = standards
        .parts
        .values()
        .filter(|p| p.choice.file.is_some())
        .count();
    (
        "campaign_standards".into(),
        json!({
            "enabled": standards.settings.enabled,
            "standards": r.standards,
            "flags_shown": shown,
            "swaying": r.swaying,
            "at_sea": r.at_sea,
            "lit": r.lit,
            "max_amp": r.max_amp,
            "max_bob": r.max_bob,
            "max_tilt_deg": r.max_tilt_deg,
            "textured": textured,
            "fallback_colour": standards.parts.len() - textured,
            "by_source": by_source,
            "by_mode": by_mode,
            "armies": armies,
        }),
    )
}

pub struct StandardPlugin;

impl Plugin for StandardPlugin {
    fn build(&self, app: &mut App) {
        super::cloth::register_shader(app);
        if app.world().contains_resource::<Assets<Mesh>>() {
            app.add_plugins(MaterialPlugin::<FlagMaterial>::default());
        }
        app.init_resource::<Standards>()
            .register_dump(dump_standards)
            .add_systems(OnEnter(AppState::Campaign), load_settings)
            .add_systems(OnExit(AppState::Campaign), reset);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amplitude_fades_in_and_out() {
        let cfg = SwaySettings::default();
        let mut sway = Sway::default();
        for _ in 0..30 {
            sway.advance(true, 1.0 / 60.0, 1.0, &cfg);
        }
        assert!((sway.amp - 1.0).abs() < 1e-5, "{}", sway.amp);
        for _ in 0..30 {
            sway.advance(false, 1.0 / 60.0, 1.0, &cfg);
        }
        assert!(sway.amp < 1e-5);
    }

    #[test]
    fn rest_has_no_bob_nor_tilt() {
        let cfg = SwaySettings::default();
        let (bob, tilt) = Sway::default().pose(&cfg);
        assert_eq!(bob, 0.0);
        assert_eq!(tilt, Quat::IDENTITY);
    }

    #[test]
    fn walking_bobs_and_leans_within_the_settings() {
        let cfg = SwaySettings::default();
        let mut sway = Sway {
            amp: 1.0,
            phase: 1.0,
        };
        let mut max_bob = 0.0f32;
        let mut max_angle = 0.0f32;
        for _ in 0..200 {
            sway.advance(true, 1.0 / 60.0, 1.0, &cfg);
            let (bob, tilt) = sway.pose(&cfg);
            max_bob = max_bob.max(bob);
            max_angle = max_angle.max(tilt.angle_between(Quat::IDENTITY));
        }
        assert!(max_bob > 0.0 && max_bob <= cfg.bob * FIGURE_SCALE + 1e-6);
        // Roll and lean combined stay near the sum of the two angles.
        assert!(max_angle > 0.0 && max_angle <= (cfg.roll_deg + cfg.lean_deg).to_radians() * 1.01);
    }

    #[test]
    fn tilt_follows_the_heading() {
        let local = Quat::from_rotation_z(-0.1);
        // Heading 0: the troop looks at +X, the lean tips the pole towards +X.
        let up = tilt_in_marker(local, 0.0) * Vec3::Y;
        assert!(up.x > 0.0 && up.z.abs() < 1e-6);
        // A quarter turn about Y sends +X to -Z (right-handed): the lean follows.
        let turned = tilt_in_marker(local, std::f32::consts::FRAC_PI_2) * Vec3::Y;
        assert!(turned.z < -0.05 && turned.x.abs() < 1e-5);
    }

    #[test]
    fn stern_pole_stands_in_the_cog_castle() {
        let anchor = stern_anchor();
        assert!(anchor.x < -4.0 && anchor.y > 2.0 && anchor.y < 3.0);
        // The fleet heading rotates it with the marker.
        let turned = Quat::from_rotation_y(std::f32::consts::PI) * anchor;
        assert!(turned.x > 4.0);
    }

    #[test]
    fn hand_is_the_lords_or_the_bearers() {
        let lord = hand(1.8);
        assert!((lord.y - 3.8 * 1.8).abs() < 1e-5);
        let bearer = hand(1.0);
        assert_eq!(bearer.y, 0.0);
        assert!((bearer.x - (SKINNED_BEARER_SLOT.x + 0.35)).abs() < 1e-6);
    }

    #[test]
    fn settings_read_the_bundled_data() {
        let root = crate::paths::find_root().expect("root");
        let cfg = SwaySettings::load(&crate::paths::data_dir(&root));
        assert!(cfg.enabled);
        assert!((cfg.step_hz - 1.9).abs() < 1e-6);
        assert!((cfg.cloth_gust - 0.6).abs() < 1e-6);
    }
}
