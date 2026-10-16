//! Deployment phase of a battle of the player.
//!
//! Ports game/scripts/battle/deployment_controller.gd: the simulation is frozen
//! (`BattleSim::begin_deployment`), the player moves their regiments inside their zone (right
//! click: same facing; right drag: oriented line; the places come from the core,
//! `sim_battle::deployment::plan_deployment`), then « Commencer la bataille » (or Enter) calls
//! `start_battle`. Every rule (zone, refusals) is in Rust; this module opens the phase, frames the
//! camera on the zone, draws the zone and the ghosts of a drag, and shows the banner.
//!
//! Dev hook: `CENT_ANS_NO_DEPLOY=1` skips the phase (the simulation runs at once, as before this
//! lot); it is also skipped while `CENT_ANS_ORDERS_DEMO` plays its script.

use bevy::prelude::*;
use sim_battle::deployment::{DeploySpot, DeployUnit, plan_deployment};
use sim_battle::{BattleSim, Battlefield, SideId};

use super::BattleSelection;
use super::drag::DRAG_PX;
use super::formations::GroupFormations;
use super::input::Gesture;
use super::overlay::{OrdersGizmos, outline_rect};
use super::picking;
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::{BattleCamera, BattleCameraState};
use crate::ui::kit::modal::ToastRequest;
use crate::ui::kit::style::UiTier;
use crate::ui::kit::widgets::{button_primary, caption, panel_page};
use crate::ui::kit::{ButtonPressed, UiTheme};

/// State of the phase.
#[derive(Resource, Debug, Default)]
pub struct Deployment {
    /// The phase was tried for this battle: do not try again.
    pub tried: bool,
    pub opened: bool,
    /// The simulation is deploying right now (refreshed every frame).
    pub active: bool,
    /// Zones of the player `[x0, z0, x1, z1]` (several for an ambusher's flanks).
    pub zones: Vec<[f32; 4]>,
    /// Zone meshes (shader of `zone.rs`) alive right now.
    pub zone_meshes: usize,
}

#[derive(Component)]
pub struct DeploymentBanner;

/// Places `plan_deployment` gives `ids` for a right click (`p0 == p1`) or a right drag `p0 -> p1`
/// (ground points), `eye` the camera position. Pure on the simulation: nothing moves.
pub fn plan_spots(sim: &BattleSim, ids: &[u32], p0: Vec3, p1: Vec3, eye: Vec3) -> Vec<DeploySpot> {
    plan_deployment(
        ids,
        (f64::from(p0.x), f64::from(p0.z)),
        (f64::from(p1.x), f64::from(p1.z)),
        (f64::from(eye.x), f64::from(eye.z)),
        |id| {
            sim.unit(id)
                .map(|u| {
                    let (width, depth) = u.extent();
                    DeployUnit {
                        x: u.x,
                        z: u.z,
                        facing: u.facing,
                        soldiers: u.soldiers(),
                        width,
                        depth,
                    }
                })
                .unwrap_or_default()
        },
        |id, share| {
            sim.unit(id).map_or((0.0, 0.0), |u| {
                u.extent_for_width((share > 0.0).then_some(share))
            })
        },
    )
}

/// Places the regiments at `spots` (`deploy_unit_width`); returns how many were accepted and the
/// French refusals.
pub fn place_spots(sim: &mut BattleSim, spots: &[DeploySpot]) -> (u32, Vec<String>) {
    let mut placed = 0;
    let mut errors = Vec::new();
    for spot in spots {
        let result = crate::battle::scene::replay::deploy_unit(
            sim,
            spot.id,
            spot.x,
            spot.z,
            spot.facing.is_finite().then_some(spot.facing),
            (spot.share > 0.0).then_some(spot.share),
        );
        match result {
            Ok(()) => placed += 1,
            Err(error) => errors.push(sim.error_text(&error)),
        }
    }
    (placed, errors)
}

/// Ground point under screen position `pos`.
pub fn ground_under(
    camera: &Camera,
    transform: &GlobalTransform,
    pos: Vec2,
    field: &Battlefield,
) -> Option<Vec3> {
    let ray = camera.viewport_to_world(transform, pos).ok()?;
    Some(picking::ground_point(ray.origin, *ray.direction, field))
}

fn dev_skipped() -> bool {
    let set = |name: &str| std::env::var(name).is_ok_and(|v| v != "0");
    set("CENT_ANS_NO_DEPLOY") || set("CENT_ANS_ORDERS_DEMO")
}

/// Opens the phase once the session and the camera exist (player battles only), and keeps
/// `active` in line with the simulation.
pub fn refresh_deployment(
    mut deployment: ResMut<Deployment>,
    mut session: ResMut<BattleSession>,
    camera_state: Option<ResMut<BattleCameraState>>,
) {
    if !deployment.tried
        && let Some(mut camera_state) = camera_state
    {
        deployment.tried = true;
        if let Some(player) = session.player
            && !dev_skipped()
        {
            let sim = session.sim_mut();
            if sim.ticks() == 0 && crate::battle::scene::replay::begin_deployment(sim) {
                deployment.zones = sim
                    .deployment_zones(player)
                    .iter()
                    .map(|z| [z.x0 as f32, z.z0 as f32, z.x1 as f32, z.z1 as f32])
                    .collect();
                deployment.opened = true;
                if let Some(zone) = deployment.zones.first().copied() {
                    frame_zone(&mut camera_state, zone, player);
                }
            }
        }
    }
    deployment.active = deployment.opened && session.sim_mut().is_deploying();
}

/// Camera on the player's zone: the whole zone on screen, looking towards the enemy.
fn frame_zone(state: &mut BattleCameraState, zone: [f32; 4], player: SideId) {
    let width = zone[2] - zone[0];
    let depth = zone[3] - zone[1];
    let ahead = if player == SideId::Attacker {
        1.0
    } else {
        -1.0
    };
    state.target = Vec3::new(
        (zone[0] + zone[2]) * 0.5,
        0.0,
        (zone[1] + zone[3]) * 0.5 + ahead * depth * 0.1,
    );
    state.target_distance = (width * 0.4).max(depth * 0.8).clamp(110.0, 210.0);
    state.yaw = if ahead > 0.0 {
        std::f32::consts::PI
    } else {
        0.0
    };
}

/// Banner while the phase is open: prompt and the start button.
pub fn sync_banner(
    mut commands: Commands,
    deployment: Res<Deployment>,
    theme: Res<UiTheme>,
    banners: Query<Entity, With<DeploymentBanner>>,
) {
    let present = !banners.is_empty();
    if deployment.active && !present {
        commands.spawn((
            Name::new("DeploymentBanner"),
            DeploymentBanner,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(104.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            GlobalZIndex(UiTier::Banner.z()),
            Pickable::IGNORE,
            DespawnOnExit(crate::app::AppState::Battle),
            children![(
                panel_page(&theme),
                children![(
                    Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(16.0),
                        ..default()
                    },
                    children![
                        caption(&theme, "Déploiement : placez vos troupes"),
                        button_primary(&theme, "deploy:start", "Commencer la bataille (Entrée)"),
                    ]
                )]
            )],
        ));
    } else if !deployment.active && present {
        for banner in &banners {
            commands.entity(banner).despawn();
        }
    }
}

pub fn banner_buttons(
    mut pressed: MessageReader<ButtonPressed>,
    mut session: ResMut<BattleSession>,
    deployment: Res<Deployment>,
    mut toasts: MessageWriter<ToastRequest>,
) {
    for press in pressed.read() {
        if press.name != "deploy:start" || !deployment.active {
            continue;
        }
        let sim = session.sim_mut();
        match crate::battle::scene::replay::start_battle(sim) {
            Ok(()) => {
                toasts.write(ToastRequest::new("La bataille commence."));
            }
            Err(error) => {
                let text = sim.error_text(&error);
                toasts.write(ToastRequest::new(&text));
            }
        }
    }
}

/// Ghosts of the right button held (the zone itself is the mesh of `zone.rs`).
#[allow(clippy::too_many_arguments)]
pub fn draw_deployment(
    mut gizmos: Gizmos<OrdersGizmos>,
    mut session: ResMut<BattleSession>,
    deployment: Res<Deployment>,
    selection: Res<BattleSelection>,
    gesture: Res<Gesture>,
    formations: Res<GroupFormations>,
    cameras: Query<(&Camera, &GlobalTransform), With<BattleCamera>>,
) {
    if !deployment.active {
        return;
    }
    let player = session.player.unwrap_or(SideId::Attacker);
    let rgb = session.colors[usize::from(player == SideId::Defender)];
    let tint = Color::srgb(rgb[0], rgb[1], rgb[2]).mix(&Color::WHITE, 0.3);
    let sim = session.sim_mut();
    let field = sim.field();
    // Ghosts of the drag in progress (a preset proposal draws its own, in `formations`).
    if formations.active.is_some() || selection.units.is_empty() {
        return;
    }
    let (Some(press), Ok((camera, transform))) = (gesture.right_press, cameras.single()) else {
        return;
    };
    let Some(end) = ground_under(camera, transform, gesture.cursor, field) else {
        return;
    };
    let (p0, p1) = if (press.pos - gesture.cursor).length() > DRAG_PX {
        (press.ground, end)
    } else {
        (end, end)
    };
    for spot in plan_spots(sim, &selection.units, p0, p1, transform.translation()) {
        let facing = if spot.facing.is_finite() {
            spot.facing as f32
        } else {
            sim.unit(spot.id).map_or(0.0, |u| u.facing as f32)
        };
        outline_rect(
            &mut gizmos,
            sim.field(),
            Vec2::new(spot.x as f32, spot.z as f32),
            facing,
            spot.width as f32,
            spot.depth as f32,
            tint.with_alpha(0.55),
        );
    }
}
