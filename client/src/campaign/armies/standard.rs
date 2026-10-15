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
//! Not ported: the heraldic texture of the flag (`banners/*`, `heraldry/*`, modes 1 and 2 of
//! `map_banner.gdshader`, plain faction colour instead), the stance badge on the pole, the
//! stern pole of a fleet (fleets keep their own glTF sails), the fall of the pole to the marker
//! foot in the far view.

use std::collections::HashMap;
use std::f32::consts::TAU;

use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;
use serde_json::{Value, json};
use sim_campaign::ArmyId;

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
const FLAG_SIZE: Vec2 = Vec2::new(3.4, 2.3);
/// Height of a man (`ArmyFigures.FIGURE_SCALE`), used by the bob.
const FIGURE_SCALE: f32 = super::figures::SKINNED_SCALE;

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
}

#[derive(Default, Debug, Clone)]
pub struct StandardReport {
    pub standards: usize,
    pub swaying: usize,
    pub max_amp: f32,
    pub max_bob: f32,
    pub max_tilt_deg: f32,
}

/// The standards of the armies and the settings.
#[derive(Resource, Default)]
pub struct Standards {
    pub settings: SwaySettings,
    parts: HashMap<ArmyId, Parts>,
    pub report: StandardReport,
}

/// Loads the sway settings when the campaign opens.
pub fn load_settings(mut standards: ResMut<Standards>) {
    standards.settings = crate::paths::find_root()
        .map(|root| SwaySettings::load(&crate::paths::data_dir(&root)))
        .unwrap_or_default();
}

pub fn reset(mut standards: ResMut<Standards>) {
    standards.parts.clear();
    standards.report = StandardReport::default();
}

fn spawn_parts(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    id: &ArmyId,
    color: [f32; 3],
) -> [Entity; 3] {
    let mut part = |part: StandardPart, mesh: Mesh, material: StandardMaterial| {
        commands
            .spawn((
                Name::new(format!("Standard_{:?}_{}", part, id.as_str())),
                part,
                StandardOf(id.clone()),
                Mesh3d(meshes.add(mesh)),
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
        Cylinder::new(0.1, POLE_HEIGHT).mesh().resolution(6).into(),
        StandardMaterial {
            base_color: Color::srgb(0.22, 0.15, 0.09),
            perceptual_roughness: 0.75,
            ..default()
        },
    );
    let finial = part(
        StandardPart::Finial,
        Sphere::new(0.22).mesh().uv(8, 4),
        StandardMaterial {
            base_color: Color::srgb(0.78, 0.58, 0.2),
            metallic: 0.9,
            perceptual_roughness: 0.35,
            ..default()
        },
    );
    // Origin on the pole, cloth hanging to the right and down (`QuadMesh_flag.center_offset`).
    let flag = part(
        StandardPart::Flag,
        Mesh::from(Rectangle::new(FLAG_SIZE.x, FLAG_SIZE.y)).translated_by(Vec3::new(
            FLAG_SIZE.x * 0.5,
            -FLAG_SIZE.y * 0.5,
            0.0,
        )),
        StandardMaterial {
            base_color: Color::srgb(color[0], color[1], color[2]),
            unlit: true,
            cull_mode: None,
            double_sided: true,
            ..default()
        },
    );
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
    time: Res<Time>,
    views: Res<ArmyViews>,
    glides: Res<Glides>,
    skinned: Res<SkinnedArmies>,
    zoom: Option<Res<ZoomView>>,
    camera: Query<&GlobalTransform, With<Camera3d>>,
    markers: Query<(&ArmyMarker, &Transform)>,
    existing: Query<(), With<StandardPart>>,
    mut parts_query: Query<(&StandardPart, &mut Transform, &mut Visibility), Without<ArmyMarker>>,
) {
    let standards = &mut *standards;
    let dt = time.delta_secs();
    let cfg = standards.settings;
    let lord_scale = skinned.lord_scale.max(1.0);
    let hidden_by_zoom = zoom.is_some_and(|z| z.strategic > HIDE_STRATEGIC);
    let camera_position = camera.iter().next().map(|c| c.translation());
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
            let entities = spawn_parts(
                &mut commands,
                &mut meshes,
                &mut materials,
                &marker.id,
                view.color,
            );
            standards.parts.insert(
                marker.id.clone(),
                Parts {
                    entities,
                    sway: Sway::default(),
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
        let walking = marker.walking && cfg.enabled && skinned.enabled;
        parts
            .sway
            .advance(walking, dt, skinned.group_factor(&marker.id, bearer), &cfg);
        let (bob, local) = if cfg.enabled {
            parts.sway.pose(&cfg)
        } else {
            (0.0, Quat::IDENTITY)
        };
        let tilt = tilt_in_marker(local, marker.heading);
        let anchor = Quat::from_rotation_y(marker.heading) * hand(lord_scale) + Vec3::Y * bob;
        let scale = transform.scale;
        let shown = skinned.enabled && !glides.active.contains_key(&marker.id) && !hidden_by_zoom;
        report.standards += 1;
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
                StandardPart::Pole => POLE_REST_Y,
                StandardPart::Finial => FINIAL_REST_Y,
                StandardPart::Flag => FLAG_REST_Y,
            };
            let local_position = anchor + tilt * Vec3::new(0.0, rest_y, 0.0);
            part_transform.translation = transform.translation + scale * local_position;
            part_transform.scale = scale;
            part_transform.rotation = match part {
                StandardPart::Flag => {
                    // Turned towards the camera round the vertical (`BILLBOARD_FIXED_Y`), then
                    // swaying with the pole; the cloth flutters (the walk adds a gust).
                    let to_camera = camera_position.map_or(Vec3::Z, |c| {
                        let d = c - part_transform.translation;
                        Vec3::new(d.x, 0.0, d.z).normalize_or(Vec3::Z)
                    });
                    let yaw = to_camera.x.atan2(to_camera.z);
                    let flutter =
                        (time.elapsed_secs() * 3.2 + marker.point.x * 0.1 + marker.point.y * 0.07)
                            .sin()
                            * 0.16
                            * (1.0 + parts.sway.amp * cfg.cloth_gust);
                    tilt * Quat::from_rotation_y(yaw + flutter)
                }
                _ => tilt,
            };
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
    (
        "campaign_standards".into(),
        json!({
            "enabled": standards.settings.enabled,
            "standards": r.standards,
            "flags_shown": shown,
            "swaying": r.swaying,
            "max_amp": r.max_amp,
            "max_bob": r.max_bob,
            "max_tilt_deg": r.max_tilt_deg,
        }),
    )
}

pub struct StandardPlugin;

impl Plugin for StandardPlugin {
    fn build(&self, app: &mut App) {
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
