//! Selection, hover, movement orders, path preview, reachable area and order ripples.
//! Port of `army_movement_controller.gd`, `army_movement_path.gd`, `army_movement_bubble.gd`,
//! `reachable_markers.gd`, `order_ripple.gd`. Orders are submitted to `CampaignState` exactly like
//! `godot-bridge` `campaign_sim_movement.rs::run_order_json`.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use data_model::{FactionId, SettlementId};
use serde_json::{Value, json};
use sim_campaign::{ArmyId, MoveOrderTarget, Order, OrderOutcome};

use super::bubble::{self, BubbleHandles, BubbleMaterial, BubbleMesh, build_mesh};
use super::feedback::{CursorMode, HoverHint};
use super::landmask::{LandMask, is_water};
use super::markers::{ArmyMarker, ArmyPlate, camera_distance, screen_rect};
use super::path::{PathView, preview_from_plan, view_from_waypoints};
use super::pick::{Pickable, pick_scored};
use super::pointer::PointerOverUi;
use super::projection::{px_to_world, ray_ground};
use super::sea::{
    Glide, Glides, SeaClick, SeaLanes, auto_resolve_pending, prefers_sea, route_points, sea_click,
    settlement_at, voyage_hint, voyage_stops,
};
use super::views::ArmyViews;
use super::{HoveredArmy, SelectedArmy};
use crate::campaign::markers::places::Places;
use crate::campaign::panels::diplomacy::AskWarDeclaration;
use crate::campaign::panels::diplomacy::war::{march_follow_up, needs_declaration};
use crate::campaign::terrain::relief_state::ReliefState;
use crate::campaign::terrain::surface::{GroundHeight, Surface};
use crate::sim::Sim;

const PATH_COLOR: Color = Color::srgba(0.88, 0.57, 0.26, 0.9);
/// Order ripple (`order_ripple.gd`): first ring, delayed paler echo, radii in screen pixels.
const RIPPLE_DURATION: f32 = 0.5;
const RIPPLE_ECHO_DELAY: f32 = 0.12;
const RIPPLE_ECHO_ALPHA: f32 = 0.5;
const RIPPLE_START_RADIUS: f32 = 4.0;
const RIPPLE_END_RADIUS: f32 = 34.0;
/// The displayed relief follows the camera zoom: re-place overlays at most this often.
const RELIEF_TICK_SECONDS: f32 = 0.2;

/// Intent of the player (mouse, keys, script, tests).
#[derive(Message, Debug, Clone, PartialEq)]
pub enum ArmyCommand {
    Select(Option<ArmyId>),
    /// March to a map pixel.
    Move {
        army: ArmyId,
        target: [f32; 2],
    },
    /// Close in on an enemy army and fight.
    Attack {
        army: ArmyId,
        target: ArmyId,
    },
    /// Attack a place of an enemy: march to it, and storm it at once if the march opens the
    /// siege (`army_movement_controller.order_attack_settlement`).
    AttackPlace {
        army: ArmyId,
        settlement: SettlementId,
    },
    /// Sail from the port the army stands in to `to_port` (ADR 0167).
    Embark {
        army: ArmyId,
        to_port: SettlementId,
    },
}

/// What the hovered ground says about a voyage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SeaHover {
    #[default]
    None,
    /// Water with a reachable port: a right click sails.
    Port,
    /// Water, but no voyage (army in the field, no route).
    Refused,
}

/// Bumped when the displayed relief moved enough to re-place the ground overlays.
#[derive(Resource, Default)]
pub struct ReliefTick {
    pub version: u32,
    elapsed: f32,
}

/// Result of the last order, as `run_order_json` reports it.
#[derive(Resource, Default, Debug, Clone, PartialEq)]
pub struct OrderLog {
    pub issued: u32,
    pub accepted: u32,
    pub last: Option<OrderResult>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrderResult {
    pub kind: &'static str,
    pub army: String,
    pub ok: bool,
    pub error: String,
    pub stop: String,
    pub cost: u32,
    pub walked: usize,
    pub planned_cells: usize,
    pub position: [f32; 2],
    pub movement_left: u32,
}

#[derive(Resource, Default)]
pub struct PathPreviewState {
    pub view: Option<PathView>,
    target: Option<[i32; 2]>,
    tick: u32,
    pub sea: SeaHover,
    /// The pointer is over something a right click would attack (an enemy or peace army, a place
    /// of a faction we are at war / at peace with): the attack cursor.
    pub attack: bool,
    /// Stance towards the place under the pointer (only when it is attackable).
    place_attack: Option<Stance>,
    /// Hover text of the previewed order.
    pub hint: Option<String>,
}

#[derive(Resource, Default)]
pub struct Reachable {
    pub army: Option<ArmyId>,
    /// Mask of the cells reached this turn / by the end of the next one (`game_api::reach_mask`).
    pub mask: Option<game_api::ReachMask>,
    pub count: usize,
    pub next_count: usize,
    pub budget: u32,
    dirty: bool,
    tick: u32,
}

/// Ripples: map pixel and age in seconds.
#[derive(Resource, Default)]
pub struct Ripples(Vec<(Vec2, f32)>);

/// Throttled change counter of the displayed relief: bumped when the vertical scale, gain or
/// squash of the relief moved (camera zoom), at most every [`RELIEF_TICK_SECONDS`].
pub fn track_relief(
    time: Res<Time>,
    relief: Option<Res<ReliefState>>,
    mut tick: ResMut<ReliefTick>,
    mut seen: Local<[f32; 3]>,
) {
    tick.elapsed += time.delta_secs();
    let Some(relief) = relief else {
        return;
    };
    let now = [relief.vertical_scale, relief.gain, relief.squash];
    let moved = now
        .iter()
        .zip(seen.iter())
        .any(|(a, b)| (a - b).abs() > 1e-4);
    if moved && tick.elapsed >= RELIEF_TICK_SECONDS {
        tick.elapsed = 0.0;
        tick.version += 1;
        *seen = now;
    }
}

pub fn reset(
    mut selected: ResMut<SelectedArmy>,
    mut hovered: ResMut<HoveredArmy>,
    mut preview: ResMut<PathPreviewState>,
    mut reachable: ResMut<Reachable>,
    mut glides: ResMut<Glides>,
    mut hint: ResMut<HoverHint>,
    mut ripples: ResMut<Ripples>,
) {
    selected.0 = None;
    hovered.0 = None;
    *preview = PathPreviewState::default();
    *reachable = Reachable::default();
    *glides = Glides::default();
    *hint = HoverHint::default();
    ripples.0.clear();
}

fn cursor_position(window: &Query<&Window, With<PrimaryWindow>>) -> Option<Vec2> {
    window.iter().next().and_then(|w| w.cursor_position())
}

/// Hover + mouse commands: left click selects, right click orders a march or an attack.
#[allow(clippy::too_many_arguments)]
pub fn hover_and_input(
    window: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    markers: Query<(&ArmyMarker, &Transform)>,
    plates: Query<(&ArmyPlate, &ComputedNode, &UiGlobalTransform)>,
    views: Res<ArmyViews>,
    sim: Res<Sim>,
    places: Res<Places>,
    ground: GroundHeight,
    mouse: Res<ButtonInput<MouseButton>>,
    selected: Res<SelectedArmy>,
    mut hovered: ResMut<HoveredArmy>,
    pointer: Res<PointerOverUi>,
    mut commands: MessageWriter<ArmyCommand>,
    mut war_asks: MessageWriter<AskWarDeclaration>,
    ui_scale: Res<bevy::ui::UiScale>,
) {
    let Some(cursor) = cursor_position(&window) else {
        return;
    };
    // Over a panel or a dialog the map neither hovers nor takes clicks.
    if pointer.over {
        if hovered.0.is_some() {
            hovered.0 = None;
        }
        return;
    }
    let Some((camera, camera_transform)) = camera.iter().next() else {
        return;
    };
    let items: Vec<Pickable> = markers
        .iter()
        .map(|(marker, transform)| Pickable {
            id: marker.id.as_str().to_owned(),
            rect: screen_rect(camera, camera_transform, transform, marker.height),
            plate: plates
                .iter()
                .find(|(p, _, _)| p.id == marker.id)
                .map(|(_, node, transform)| {
                    // UI layout pixels (x `UiScale`) to the window pixels of the cursor.
                    let scale = node.inverse_scale_factor() * ui_scale.0;
                    let center = transform.translation * scale;
                    Rect::from_center_size(center, node.size() * scale)
                }),
        })
        .collect();
    let under = pick_scored(&items, cursor, None).filter(|r| r.direct);
    let id = under.as_ref().and_then(|r| ArmyId::parse(&r.id));
    if hovered.0 != id {
        hovered.0 = id.clone();
    }
    if mouse.just_pressed(MouseButton::Left)
        && let Some(id) = id.clone()
    {
        commands.write(ArmyCommand::Select(Some(id)));
    }
    if mouse.just_pressed(MouseButton::Right) {
        let Some(army) = selected.0.clone() else {
            return;
        };
        let picked = pick_scored(&items, cursor, Some(army.as_str()))
            .filter(|r| r.direct)
            .and_then(|r| ArmyId::parse(&r.id));
        // An army of a faction at peace with us: confirm the declaration of war first, then
        // attack (`army_movement_controller.ask_war`).
        if let Some(target) = picked.clone()
            && let Some(view) = views.get(&target)
            && !view.hostile
            && !view.player
            && let (Some(data), Some(state)) = (&sim.data, &sim.state)
            && needs_declaration(data, state, &sim.player, &view.faction)
        {
            let name = data.faction_name(&view.faction);
            war_asks.write(AskWarDeclaration {
                target: view.faction.as_str().to_owned(),
                target_label: format!("l'armée de {name}"),
                then: Some(Order::Attack {
                    army,
                    target_army: target,
                }),
                assault_after: None,
            });
            return;
        }
        let target_army = picked.filter(|t| views.get(t).is_some_and(|v| v.hostile));
        if let Some(target) = target_army {
            commands.write(ArmyCommand::Attack { army, target });
        } else if let Ok(ray) = camera.viewport_to_world(camera_transform, cursor)
            && let Some(px) = ray_ground(ray, &ground)
        {
            // A colony reached by sea (`prefers_sea`): embark instead of marching round.
            let distance = camera_distance(camera_transform, Vec3::new(px.x, 0.0, px.y));
            if let (Some(data), Some(state)) = (&sim.data, &sim.state)
                && let Some((port, point)) = settlement_at(&places, px, distance)
            {
                let stance = place_stance(data, state, &sim.player, &places, &port);
                if stance == Stance::Peace {
                    // A place of a faction at peace: confirm the war first, then march and storm.
                    if let Some(place) = places.get(port.as_str()) {
                        let faction = FactionId::new(place.controller.clone()).ok();
                        if let Some(faction) = faction {
                            war_asks.write(AskWarDeclaration {
                                target: faction.as_str().to_owned(),
                                target_label: place.name.clone(),
                                then: Some(Order::MoveArmy {
                                    army: army.clone(),
                                    target: MoveOrderTarget::Place(
                                        sim_campaign::Place::Settlement(port.clone()),
                                    ),
                                }),
                                assault_after: Some(army),
                            });
                            return;
                        }
                    }
                }
                if prefers_sea(state, data, &army, &port, point) {
                    commands.write(ArmyCommand::Embark {
                        army,
                        to_port: port,
                    });
                    return;
                }
                if stance == Stance::War {
                    commands.write(ArmyCommand::AttackPlace {
                        army,
                        settlement: port,
                    });
                    return;
                }
            }
            commands.write(ArmyCommand::Move {
                army,
                target: [px.x, px.y],
            });
        }
    }
}

/// Script/test hook (no rule): `Tab` selects the next player army, `KeyM` orders the selected
/// army to march by the offset in `CENT_ANS_ARMY_MOVE` (`dx,dy` map pixels; default tries a few).
/// `CENT_ANS_ARMY_AUTOTEST=1` does both once, a few frames after the armies appear;
/// `CENT_ANS_ARMY_AUTOTEST=sea` instead selects a player army standing in a port and "clicks"
/// the middle of a sea lane leaving it (the water-click path of ADR 0167).
#[allow(clippy::too_many_arguments)]
pub fn test_hook(
    sim: Res<Sim>,
    lanes: Res<SeaLanes>,
    keys: Res<ButtonInput<KeyCode>>,
    keymap: Res<crate::settings::KeyMap>,
    views: Res<ArmyViews>,
    selected: Res<SelectedArmy>,
    mut commands: MessageWriter<ArmyCommand>,
    mut frames: Local<u32>,
    mut auto_done: Local<bool>,
) {
    // Dev keys, only with the test environment (M is the economy window otherwise).
    let dev_keys = std::env::var_os("CENT_ANS_ARMY_AUTOTEST").is_some()
        || std::env::var_os("CENT_ANS_ARMY_KEYS").is_some();
    let mut cycle = dev_keys && hook_pressed(&keymap, "army_test_cycle", KeyCode::Tab, &keys);
    let mut march = dev_keys && hook_pressed(&keymap, "army_test_march", KeyCode::KeyM, &keys);
    let sea_mode = std::env::var("CENT_ANS_ARMY_AUTOTEST").is_ok_and(|v| v == "sea");
    if sea_mode && !*auto_done && !views.0.is_empty() {
        *frames += 1;
        if *frames == 10 {
            *auto_done = true;
            if let Some(click) = sea_autotest_click(&sim, &lanes, &views) {
                commands.write(ArmyCommand::Select(Some(click.0.clone())));
                commands.write(ArmyCommand::Move {
                    army: click.0,
                    target: click.1,
                });
            }
        }
        return;
    }
    if !*auto_done && std::env::var_os("CENT_ANS_ARMY_AUTOTEST").is_some() && !views.0.is_empty() {
        *frames += 1;
        if *frames == 5 {
            cycle = true;
        }
        if *frames == 10 {
            march = true;
            *auto_done = true;
        }
    }
    if cycle {
        let players: Vec<&ArmyId> = views.0.iter().filter(|v| v.player).map(|v| &v.id).collect();
        let next = match &selected.0 {
            Some(cur) => players
                .iter()
                .position(|id| *id == cur)
                .map_or(0, |i| (i + 1) % players.len().max(1)),
            None => 0,
        };
        if let Some(id) = players.get(next) {
            commands.write(ArmyCommand::Select(Some((*id).clone())));
        }
    }
    if march && let Some(view) = selected.0.as_ref().and_then(|id| views.get(id)) {
        for offset in test_offsets() {
            commands.write(ArmyCommand::Move {
                army: view.id.clone(),
                target: [view.point[0] + offset[0], view.point[1] + offset[1]],
            });
        }
    }
}

/// A key of the dev hook: the `KeyMap` action when it has a binding, else its default key
/// (`army_test_*` actions are not in the settings sheet).
fn hook_pressed(
    keymap: &crate::settings::KeyMap,
    action: &str,
    fallback: KeyCode,
    keys: &ButtonInput<KeyCode>,
) -> bool {
    // Same fallback rule as every campaign shortcut (see `hud::keys`).
    super::super::hud::keys::action_down(Some(keymap), action, fallback, keys, true)
}

/// A player army in a port with a voyage, and the middle of a lane towards the destination.
fn sea_autotest_click(
    sim: &Sim,
    lanes: &SeaLanes,
    views: &ArmyViews,
) -> Option<(ArmyId, [f32; 2])> {
    let (data, state) = (sim.data.as_ref()?, sim.state.as_ref()?);
    let mut best: Option<(ArmyId, [f32; 2])> = None;
    for view in views.0.iter().filter(|v| v.player && v.in_settlement) {
        let army = state.army(&view.id)?;
        let from = army.settlement()?;
        let mut ports: Vec<_> = sim_campaign::voyage::sea_voyages(state, data, &army.faction, from)
            .into_keys()
            .collect();
        ports.sort();
        for port in ports {
            let Some(points) = lanes.lane_points(from.as_str(), port.as_str()) else {
                continue;
            };
            let mid = points[points.len() / 2];
            best = Some((view.id.clone(), [mid.x, mid.y]));
            break;
        }
        if best.is_some() {
            break;
        }
    }
    best
}

fn test_offsets() -> Vec<[f32; 2]> {
    if let Ok(text) = std::env::var("CENT_ANS_ARMY_MOVE")
        && let Some((dx, dy)) = text.split_once(',')
        && let (Ok(dx), Ok(dy)) = (dx.trim().parse(), dy.trim().parse())
    {
        return vec![[dx, dy]];
    }
    vec![[0.0, -12.0]]
}

#[allow(clippy::too_many_arguments)]
pub fn execute_commands(
    mut messages: MessageReader<ArmyCommand>,
    mut sim: ResMut<Sim>,
    mut selected: ResMut<SelectedArmy>,
    mut log: ResMut<OrderLog>,
    mut ripples: ResMut<Ripples>,
    mut preview: ResMut<PathPreviewState>,
    mut glides: ResMut<Glides>,
    lanes: Res<SeaLanes>,
    surface: Option<Res<Surface>>,
    mask: Res<LandMask>,
) {
    for command in messages.read() {
        match command {
            ArmyCommand::Select(id) => {
                if selected.0 != *id {
                    selected.0 = id.clone();
                    preview.view = None;
                    preview.target = None;
                }
            }
            ArmyCommand::Move { army, target } => {
                let water = is_water(&mask, surface.as_deref(), target[0], target[1]);
                let ok = if water {
                    let click = match (&sim.data, &sim.state) {
                        (Some(data), Some(state)) => Some(sea_click(state, data, army, *target)),
                        _ => None,
                    };
                    match click {
                        Some(SeaClick::Embark(port)) => {
                            embark(&mut sim, &mut log, &mut glides, &lanes, army, &port)
                        }
                        Some(SeaClick::Refused(text)) => {
                            refuse(&mut log, "embark", army, text);
                            false
                        }
                        None => false,
                    }
                } else {
                    let order = Order::MoveArmy {
                        army: army.clone(),
                        target: MoveOrderTarget::Point {
                            x: target[0],
                            y: target[1],
                        },
                    };
                    run_order(&mut sim, &mut log, "move", army, order);
                    log.last.as_ref().is_some_and(|r| r.ok)
                };
                if ok {
                    ripples.0.push((Vec2::new(target[0], target[1]), 0.0));
                    preview.view = None;
                    preview.target = None;
                }
            }
            ArmyCommand::Attack { army, target } => {
                let order = Order::Attack {
                    army: army.clone(),
                    target_army: target.clone(),
                };
                run_order(&mut sim, &mut log, "attack", army, order);
            }
            ArmyCommand::AttackPlace { army, settlement } => {
                attack_place(&mut sim, &mut log, army, settlement);
                preview.view = None;
                preview.target = None;
            }
            ArmyCommand::Embark { army, to_port } => {
                if embark(&mut sim, &mut log, &mut glides, &lanes, army, to_port) {
                    preview.view = None;
                    preview.target = None;
                }
            }
        }
    }
}

/// How the player stands towards the holder of something it could attack
/// (`army_movement_controller.relation_to`): war = direct attack, peace = attack after a
/// declaration of war, friend = no attack (self, allies, unknown).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stance {
    War,
    Peace,
    Friend,
}

/// Stance towards a faction.
pub fn faction_stance(
    data: &data_model::GameData,
    state: &sim_campaign::CampaignState,
    player: &FactionId,
    faction: &FactionId,
) -> Stance {
    if faction == player {
        Stance::Friend
    } else if state.is_at_war(faction, player) {
        Stance::War
    } else if needs_declaration(data, state, player, faction) {
        Stance::Peace
    } else {
        Stance::Friend
    }
}

/// Stance towards the current holder of a place.
pub fn place_stance(
    data: &data_model::GameData,
    state: &sim_campaign::CampaignState,
    player: &FactionId,
    places: &Places,
    place: &SettlementId,
) -> Stance {
    places
        .get(place.as_str())
        .and_then(|p| FactionId::new(p.controller.clone()).ok())
        .map_or(Stance::Friend, |f| faction_stance(data, state, player, &f))
}

/// `AttackPlace`: the army that already holds the siege storms it; otherwise it marches, and a
/// march that opens the siege of that very place is followed by the assault (a refused assault
/// leaves the march result, as in Godot).
fn attack_place(sim: &mut Sim, log: &mut OrderLog, army: &ArmyId, place: &SettlementId) {
    let here = sim
        .state
        .as_ref()
        .and_then(|s| s.army(army))
        .and_then(|a| a.settlement().cloned());
    if here.as_ref() == Some(place) {
        run_order(
            sim,
            log,
            "assault",
            army,
            Order::Assault { army: army.clone() },
        );
        return;
    }
    let march = Order::MoveArmy {
        army: army.clone(),
        target: MoveOrderTarget::Place(sim_campaign::Place::Settlement(place.clone())),
    };
    let outcome = run_order_outcome(sim, log, "attack_place", army, march);
    let follow = match (&outcome, &sim.state) {
        (Some(outcome), Some(state)) => march_follow_up(state, outcome, army, place),
        _ => None,
    };
    if let Some(order) = follow {
        let kind = if matches!(order, Order::Assault { .. }) {
            "assault"
        } else {
            "attack"
        };
        let marched = log.last.clone();
        run_order(sim, log, kind, army, order);
        if !log.last.as_ref().is_some_and(|r| r.ok) {
            log.last = marched;
        }
    }
}

/// A refused order (nothing reaches the core), reported like a core refusal.
fn refuse(log: &mut OrderLog, kind: &'static str, army: &ArmyId, error: &str) {
    log.issued += 1;
    log.last = Some(OrderResult {
        kind,
        army: army.as_str().to_owned(),
        ok: false,
        error: error.to_owned(),
        stop: String::new(),
        cost: 0,
        walked: 0,
        planned_cells: 0,
        position: [0.0; 2],
        movement_left: 0,
    });
}

/// `Embark` (ADR 0167): submits the order, glides the army along the route, and auto-resolves a
/// naval battle the crossing may have raised (never played in 3D).
fn embark(
    sim: &mut Sim,
    log: &mut OrderLog,
    glides: &mut Glides,
    lanes: &SeaLanes,
    army: &ArmyId,
    port: &SettlementId,
) -> bool {
    let plan = match (&sim.data, &sim.state) {
        (Some(data), Some(state)) => state.army(army).and_then(|entry| {
            let from = entry.settlement()?.clone();
            Some((from, voyage_stops(state, data, army, port)))
        }),
        _ => None,
    };
    run_order(
        sim,
        log,
        "embark",
        army,
        Order::Embark {
            army: army.clone(),
            to_port: port.clone(),
        },
    );
    let ok = log.last.as_ref().is_some_and(|r| r.ok);
    if ok && let (Some((from, stops)), Some(data), Some(state)) = (plan, &sim.data, &sim.state) {
        // Stops after the one the army reached (an interception keeps it at a port of call).
        let reached = state.army(army).and_then(|a| a.settlement()).cloned();
        let cut = reached
            .and_then(|r| stops.iter().position(|s| *s == r))
            .map_or(stops.len(), |i| i + 1);
        let names: Vec<String> = stops[..cut].iter().map(|s| s.as_str().to_owned()).collect();
        let point_of = |id: &str| {
            SettlementId::new(id)
                .ok()
                .and_then(|id| data.settlement_point(&id))
                .map(|p| Vec2::new(p[0], p[1]))
        };
        let route = route_points(lanes, &point_of, from.as_str(), &names);
        if let Some(glide) = Glide::new(route) {
            glides.active.insert(army.clone(), glide);
            glides.started += 1;
        }
    }
    if ok {
        auto_resolve_pending(sim);
    }
    ok
}

/// Slides the voyaging armies along their route, then rebuilds their figures on land.
pub fn advance_glides(time: Res<Time>, mut glides: ResMut<Glides>, mut views: ResMut<ArmyViews>) {
    if glides.active.is_empty() {
        return;
    }
    let dt = time.delta_secs();
    let mut done = Vec::new();
    for (id, glide) in glides.active.iter_mut() {
        glide.elapsed += dt;
        if glide.done() {
            done.push(id.clone());
        }
    }
    if !done.is_empty() {
        for id in &done {
            glides.active.remove(id);
        }
        glides.finished += done.len() as u32;
        // The army lands: rebuild its figures from the current list.
        views.set_changed();
    }
}

/// Naval battles of the player are never played: resolve whatever is pending, whenever it appears
/// (an AI interception during the turn change).
pub fn auto_resolve_naval(mut sim: ResMut<Sim>) {
    let pending = sim
        .state
        .as_ref()
        .is_some_and(|s| !s.naval.pending.is_empty());
    if pending {
        auto_resolve_pending(&mut sim);
    }
}

/// Mirrors `run_order_json` (godot-bridge): validate, apply, report the march.
fn run_order(sim: &mut Sim, log: &mut OrderLog, kind: &'static str, army: &ArmyId, order: Order) {
    run_order_outcome(sim, log, kind, army, order);
}

/// `run_order`, also returning what the core reports for an accepted order.
fn run_order_outcome(
    sim: &mut Sim,
    log: &mut OrderLog,
    kind: &'static str,
    army: &ArmyId,
    order: Order,
) -> Option<OrderOutcome> {
    log.issued += 1;
    let fail = |error: String| OrderResult {
        kind,
        army: army.as_str().to_owned(),
        ok: false,
        error,
        stop: String::new(),
        cost: 0,
        walked: 0,
        planned_cells: 0,
        position: [0.0; 2],
        movement_left: 0,
    };
    let (Some(data), Some(state)) = (sim.data.clone(), sim.state.as_mut()) else {
        log.last = Some(fail("aucune campagne en cours".into()));
        return None;
    };
    let mut accepted = None;
    let result = match state.submit_order_outcome(&data, order) {
        Err(error) => fail(error.to_string()),
        Ok(outcome) => {
            accepted = Some(outcome.clone());
            let entry = state.army(army);
            let position = entry.map_or([0.0; 2], |a| state.army_point(&data, a));
            let movement_left = entry.map_or(0, |a| a.movement_left);
            let mut result = OrderResult {
                ok: true,
                position,
                movement_left,
                ..fail(String::new())
            };
            if let OrderOutcome::Moved(report) = outcome {
                result.cost = report.cost;
                result.walked = report.walked.len();
                result.planned_cells = report.planned_path.len();
                result.stop = serde_json::to_value(&report.stop)
                    .ok()
                    .and_then(|v| v.get("kind").and_then(Value::as_str).map(str::to_owned))
                    .unwrap_or_default();
            }
            result
        }
    };
    if result.ok {
        log.accepted += 1;
    }
    log.last = Some(result);
    accepted
}

/// Reachable area of the selected army, recomputed when the selection or the sim changes.
pub fn refresh_overlays(
    sim: Res<Sim>,
    selected: Res<SelectedArmy>,
    mut reachable: ResMut<Reachable>,
) {
    if !(selected.is_changed() || sim.is_changed()) {
        return;
    }
    let (Some(data), Some(state), Some(army)) = (&sim.data, &sim.state, &selected.0) else {
        if reachable.army.is_some() {
            *reachable = Reachable {
                dirty: true,
                ..Reachable::default()
            };
        }
        return;
    };
    if state.army(army).is_none() {
        *reachable = Reachable {
            dirty: true,
            ..Reachable::default()
        };
        return;
    }
    let Some(mask) = game_api::reach_mask(data, state, army) else {
        return;
    };
    reachable.army = Some(army.clone());
    reachable.count = mask.cells;
    reachable.next_count = mask.next_cells;
    reachable.budget = mask.budget;
    reachable.mask = Some(mask);
    reachable.dirty = true;
}

/// Builds / refreshes the bubble: the mask texture when the area changed, the relief grid when
/// the area or the displayed relief changed (one entity, mesh and mask replaced in place).
#[allow(clippy::too_many_arguments)]
pub fn update_bubble(
    mut commands: Commands,
    ground: GroundHeight,
    tick: Res<ReliefTick>,
    mut reachable: ResMut<Reachable>,
    mut handles: ResMut<BubbleHandles>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<BubbleMaterial>>,
    mut bubble: Query<&mut Visibility, With<BubbleMesh>>,
) {
    if !(reachable.dirty || reachable.tick != tick.version) {
        return;
    }
    let new_area = reachable.dirty;
    reachable.dirty = false;
    reachable.tick = tick.version;
    let Some(mask) = reachable
        .mask
        .as_ref()
        .filter(|_| reachable.army.is_some() && reachable.count + reachable.next_count > 1)
    else {
        for mut visibility in &mut bubble {
            *visibility = Visibility::Hidden;
        }
        handles.quads = 0;
        return;
    };
    let (mesh, quads) = build_mesh(Vec2::from(mask.origin), Vec2::from(mask.size), &ground);
    handles.quads = quads;
    handles.builds += 1;
    let (image, levels) = bubble::mask_image(mask);
    handles.mask_size = [mask.width, mask.height];
    handles.mask_mips = levels;
    let (Some(mesh_handle), Some(material_handle)) =
        (handles.mesh.clone(), handles.material.clone())
    else {
        let mesh_handle = meshes.add(mesh);
        let material_handle = materials.add(BubbleMaterial {
            params: bubble::params_for(mask),
            mask: Some(images.add(image)),
        });
        handles.mesh = Some(mesh_handle.clone());
        handles.material = Some(material_handle.clone());
        commands.spawn((
            Name::new("ArmyMovementBubble"),
            BubbleMesh,
            Mesh3d(mesh_handle),
            MeshMaterial3d(material_handle),
            Transform::default(),
            Visibility::Inherited,
            bevy::light::NotShadowCaster,
            bevy::state::state_scoped::DespawnOnExit(crate::app::AppState::Campaign),
        ));
        return;
    };
    let _ = meshes.insert(mesh_handle.id(), mesh);
    if new_area && let Some(mut material) = materials.get_mut(&material_handle) {
        material.params = bubble::params_for(mask);
        material.mask = Some(images.add(image));
    }
    for mut visibility in &mut bubble {
        *visibility = Visibility::Inherited;
    }
}

/// Back to a fresh state when the screen closes: the bubble entity is despawned with it.
pub fn forget_bubble(mut handles: ResMut<BubbleHandles>) {
    *handles = BubbleHandles::default();
}

/// Path preview of the selected army towards the cursor ground point (throttled by cell), sea
/// voyage preview over water, and the hover text.
#[allow(clippy::too_many_arguments)]
pub fn preview_under_cursor(
    sim: Res<Sim>,
    selected: Res<SelectedArmy>,
    hovered: Res<HoveredArmy>,
    views: Res<ArmyViews>,
    window: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    ground: GroundHeight,
    surface: Option<Res<Surface>>,
    mask: Res<LandMask>,
    places: Res<Places>,
    lanes: Res<SeaLanes>,
    tick: Res<ReliefTick>,
    mut preview: ResMut<PathPreviewState>,
    mut hint: ResMut<HoverHint>,
    pointer: Res<PointerOverUi>,
) {
    if pointer.over {
        if preview.view.is_some() || preview.sea != SeaHover::None || preview.hint.is_some() {
            preview.view = None;
            preview.sea = SeaHover::None;
            preview.hint = None;
        }
        preview.attack = false;
        preview.place_attack = None;
        set_hint(&mut hint, None);
        return;
    }
    let (Some(data), Some(state), Some(army)) = (&sim.data, &sim.state, &selected.0) else {
        if preview.view.is_some() || preview.sea != SeaHover::None {
            preview.view = None;
            preview.sea = SeaHover::None;
            preview.hint = None;
        }
        preview.attack = false;
        preview.place_attack = None;
        set_hint(&mut hint, None);
        return;
    };
    let army_stance = hovered
        .0
        .as_ref()
        .filter(|h| *h != army)
        .and_then(|h| views.get(h))
        .filter(|v| !v.player)
        .map_or(Stance::Friend, |v| {
            faction_stance(data, state, &sim.player, &v.faction)
        });
    let attack_text = match army_stance {
        Stance::War => Some("Armée ennemie — clic droit pour attaquer".to_owned()),
        Stance::Peace => {
            Some("Armée en paix — clic droit pour attaquer (déclare la guerre)".to_owned())
        }
        Stance::Friend => None,
    };
    let fresh = (|| {
        let cursor = cursor_position(&window)?;
        let (camera, camera_transform) = camera.iter().next()?;
        let ray = camera.viewport_to_world(camera_transform, cursor).ok()?;
        let px = ray_ground(ray, &ground)?;
        Some((
            px,
            camera_distance(camera_transform, Vec3::new(px.x, 0.0, px.y)),
        ))
    })();
    let Some((px, distance)) = fresh else {
        preview.sea = SeaHover::None;
        preview.place_attack = None;
        preview.attack = army_stance != Stance::Friend;
        set_hint(&mut hint, attack_text);
        return;
    };
    let cell = [(px.x / 4.0) as i32, (px.y / 4.0) as i32];
    if preview.target != Some(cell) || preview.tick != tick.version {
        preview.target = Some(cell);
        preview.tick = tick.version;
        let water = is_water(&mask, surface.as_deref(), px.x, px.y);
        let sea_target: Option<SeaClick> = if water {
            Some(sea_click(state, data, army, [px.x, px.y]))
        } else {
            settlement_at(&places, px, distance)
                .filter(|(id, point)| prefers_sea(state, data, army, id, *point))
                .map(|(id, _)| SeaClick::Embark(id))
        };
        if let Some(click) = sea_target {
            match click {
                SeaClick::Embark(port) => {
                    let stops = voyage_stops(state, data, army, &port);
                    let from = state
                        .army(army)
                        .and_then(|a| a.settlement())
                        .map(|s| s.as_str().to_owned())
                        .unwrap_or_default();
                    let names: Vec<String> = stops.iter().map(|s| s.as_str().to_owned()).collect();
                    let point_of = |id: &str| {
                        SettlementId::new(id)
                            .ok()
                            .and_then(|id| data.settlement_point(&id))
                            .map(|p| Vec2::new(p[0], p[1]))
                    };
                    let route = route_points(&lanes, &point_of, &from, &names);
                    preview.view = (route.len() >= 2)
                        .then(|| view_from_waypoints(&route, Vec::new(), &ground, distance));
                    preview.sea = SeaHover::Port;
                    preview.place_attack = None;
                    preview.hint = Some(voyage_hint(data, &port, &stops));
                }
                SeaClick::Refused(text) => {
                    preview.place_attack = None;
                    preview.view = None;
                    preview.sea = SeaHover::Refused;
                    preview.hint = Some(text.to_owned());
                }
            }
        } else {
            preview.sea = SeaHover::None;
            preview.place_attack = settlement_at(&places, px, distance)
                .map(|(id, _)| place_stance(data, state, &sim.player, &places, &id))
                .filter(|s| *s != Stance::Friend);
            let plan = state.plan_path(data, army, [px.x, px.y]);
            preview.hint = match &plan {
                Some(plan) => {
                    let when = if plan.reachable_this_turn() {
                        "ce tour".to_owned()
                    } else {
                        format!("{} tours", plan.turns())
                    };
                    Some(format!(
                        "→ {when}, coût {} — clic droit pour marcher",
                        plan.cost
                    ))
                }
                None => Some("Aucun chemin".to_owned()),
            };
            preview.view = plan.and_then(|plan| preview_from_plan(&plan, &ground, distance));
        }
    }
    preview.attack = army_stance != Stance::Friend || preview.place_attack.is_some();
    let place_text = preview.place_attack.map(|s| {
        if s == Stance::Peace {
            "clic droit pour donner l’assaut (déclare la guerre)".to_owned()
        } else {
            "clic droit pour donner l’assaut".to_owned()
        }
    });
    let shown = attack_text.or(place_text).or_else(|| preview.hint.clone());
    set_hint(&mut hint, shown);
}

fn set_hint(hint: &mut HoverHint, text: Option<String>) {
    if hint.0 != text {
        hint.0 = text;
    }
}

/// Screen-space radius in pixels to a world radius at `distance` for a perspective camera.
fn pixels_to_world(pixels: f32, distance: f32, fov: f32, viewport_height: f32) -> f32 {
    pixels * 2.0 * (fov * 0.5).tan() * distance / viewport_height.max(1.0)
}

#[allow(clippy::too_many_arguments)]
pub fn draw_overlays(
    mut gizmos: Gizmos,
    time: Res<Time>,
    preview: Res<PathPreviewState>,
    mut ripples: ResMut<Ripples>,
    views: Res<ArmyViews>,
    selected: Res<SelectedArmy>,
    ground: GroundHeight,
    camera: Query<(&Camera, &GlobalTransform, &Projection), With<Camera3d>>,
) {
    let color = if preview.sea == SeaHover::Port {
        Color::srgba(0.25, 0.5, 0.85, 0.9)
    } else {
        PATH_COLOR
    };
    if let Some(view) = &preview.view {
        gizmos.linestrip(view.points.iter().copied(), color);
        for &end in view.turn_ends.iter().filter_map(|i| view.corners.get(*i)) {
            gizmos.sphere(Isometry3d::from_translation(end), 0.8, color);
        }
    }
    if let Some(view) = selected.0.as_ref().and_then(|id| views.get(id)) {
        let at = px_to_world(Vec2::new(view.point[0], view.point[1]), &ground) + Vec3::Y * 0.2;
        gizmos.circle(
            Isometry3d::new(at, Quat::from_rotation_arc(Vec3::Z, Vec3::Y)),
            3.0,
            Color::srgb(1.0, 0.9, 0.4),
        );
    }
    let dt = time.delta_secs();
    let Some((cam, cam_transform, projection)) = camera.iter().next() else {
        return;
    };
    let fov = match projection {
        Projection::Perspective(p) => p.fov,
        _ => 0.8,
    };
    let height = cam.logical_viewport_size().map_or(720.0, |s| s.y);
    ripples.0.retain_mut(|(at_px, age)| {
        *age += dt;
        if *age >= RIPPLE_DURATION + RIPPLE_ECHO_DELAY {
            return false;
        }
        let at = px_to_world(*at_px, &ground) + Vec3::Y * 0.2;
        let distance = cam_transform.translation().distance(at);
        for (t, alpha_scale) in [
            (*age / RIPPLE_DURATION, 1.0),
            (
                (*age - RIPPLE_ECHO_DELAY) / RIPPLE_DURATION,
                RIPPLE_ECHO_ALPHA,
            ),
        ] {
            if !(0.0..1.0).contains(&t) {
                continue;
            }
            let radius_px = RIPPLE_START_RADIUS + (RIPPLE_END_RADIUS - RIPPLE_START_RADIUS) * t;
            gizmos.circle(
                Isometry3d::new(at, cam_transform.rotation()),
                pixels_to_world(radius_px, distance, fov, height),
                Color::srgba(0.25, 0.14, 0.06, (1.0 - t) * alpha_scale),
            );
        }
        true
    });
}

/// `--dump-state` section `campaign_armies`.
pub fn dump_armies(world: &World) -> (String, Value) {
    let views = world.resource::<ArmyViews>();
    let selected = world.resource::<SelectedArmy>();
    let log = world.resource::<OrderLog>();
    let preview = world.resource::<PathPreviewState>();
    let reachable = world.resource::<Reachable>();
    let glides = world.resource::<Glides>();
    let bubble = world.resource::<BubbleHandles>();
    let cursor = world.resource::<CursorMode>();
    let hint = world.resource::<HoverHint>();
    let lanes = world.resource::<SeaLanes>();
    let figures = world
        .try_query::<&super::figures::ArmyFigure>()
        .map_or(0, |mut q| q.iter(world).count());
    let scenes_ready = world
        .try_query::<&super::figures::Tinted>()
        .map_or(0, |mut q| q.iter(world).count());
    let tinted = world.resource::<super::figures::LiveryCache>().tinted;
    let ships = world
        .try_query::<&super::figures::ArmyFigure>()
        .map_or(0, |mut q| {
            q.iter(world).filter(|f| f.path.contains("cog")).count()
        });
    let bivouacs = world
        .try_query::<&super::figures::ArmyFigure>()
        .map_or(0, |mut q| {
            q.iter(world).filter(|f| f.path.contains("bivouac")).count()
        });
    let bivouacs_shown = world
        .try_query::<(&super::figures::ArmyFigure, &Visibility)>()
        .map_or(0, |mut q| {
            q.iter(world)
                .filter(|(f, v)| f.path.contains("bivouac") && **v != Visibility::Hidden)
                .count()
        });
    let markers = world
        .try_query::<&ArmyMarker>()
        .map_or(0, |mut q| q.iter(world).count());
    let standoff_markers = world.try_query::<&ArmyMarker>().map_or(0, |mut q| {
        q.iter(world).filter(|m| m.standoff.is_some()).count()
    });
    let plates = world.resource::<super::markers::PlateStats>();
    let mask = world.resource::<LandMask>();
    let fog = world.resource::<crate::campaign::weather::FogOfWar>();
    let skinned = super::skinned::dump_fields(world.resource::<super::skinned::SkinnedArmies>());
    let pointer = world.resource::<PointerOverUi>();
    let last = log.last.as_ref().map(|r| {
        json!({
            "kind": r.kind, "army": r.army, "ok": r.ok, "error": r.error, "stop": r.stop,
            "cost": r.cost, "walked_points": r.walked, "planned_cells": r.planned_cells,
            "position": r.position, "movement_left": r.movement_left,
        })
    });
    let mut section = json!({
            "armies_shown": views.0.len(),
            "player_armies": views.0.iter().filter(|v| v.player).count(),
            "hostile_armies": views.0.iter().filter(|v| v.hostile).count(),
            "marker_entities": markers,
            "garrison_standoff_markers": standoff_markers,
            "plates_visible": plates.visible,
            "plates_displaced": plates.displaced,
            "plates_hidden": plates.hidden,
            "plate_overlaps": plates.overlaps(),
            "land_mask_loaded": mask.is_loaded(),
            "land_mask_size": [mask.size().0, mask.size().1],
            "fog_active": fog.active,
            "armies_hidden_by_fog": world.resource::<super::views::FogHidden>().0,
            "selected": selected.0.as_ref().map(ArmyId::as_str),
            "orders_issued": log.issued,
            "orders_accepted": log.accepted,
            "last_order": last,
            "path_length": preview.view.as_ref().map_or(0, |v| v.points.len()),
            "path_turns": preview.view.as_ref().map_or(0, |v| v.turns),
            "reachable_cells": reachable.count,
            "reachable_next_cells": reachable.next_count,
            "movement_budget": reachable.budget,
            "bubble_quads": bubble.quads,
            "bubble_builds": bubble.builds,
            "figure_scenes": figures,
            "ship_scenes": ships,
            "mesh_primitives_seen": scenes_ready,
            "livery_materials_tinted": tinted,
            "sea_lanes": lanes.len(),
            "voyages_started": glides.started,
            "voyages_finished": glides.finished,
            "voyages_gliding": glides.active.len(),
            "pending_naval_battles": world.resource::<Sim>().state.as_ref().map_or(0, |s| s.naval.pending.len()),
            "cursor": cursor.name(),
            "attack_hover": preview.attack,
            "hover_hint": hint.0,
            "ripples_alive": world.resource::<Ripples>().0.len(),
            "plate_recomputes": plates.recomputes,
            "plate_reuses": plates.reused,
            "pointer_over_ui": pointer.over,
            "pointer_blocked_frames": pointer.blocked_frames,
    });
    if let (Some(section), Some(extra)) = (section.as_object_mut(), skinned.as_object()) {
        section.extend(extra.clone());
    }
    if let Some(section) = section.as_object_mut() {
        section.insert("bivouac_scenes".into(), json!(bivouacs));
        section.insert("bivouacs_shown".into(), json!(bivouacs_shown));
        section.insert("bubble_mask".into(), json!(bubble.mask_size));
        section.insert("bubble_mask_mips".into(), json!(bubble.mask_mips));
        section.extend(super::bubbles::dump_fields(world));
    }
    ("campaign_armies".into(), section)
}
