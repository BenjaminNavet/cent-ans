//! Group formations (presets Attaque / Défense / Marche).
//!
//! Ports game/scripts/battle/battle_formation_picker.gd. No rule here: the places come from the
//! core (`BattleSim::formation_slots`, data `data/rules/group_formations.json`); this module
//! shows them as ghosts and turns them into ordinary orders:
//! - in battle, an active preset plus a right click (or right drag for the facing) gives one
//!   `move` per regiment (frontage of the slot, `match_speed`, `group_tag`) and locks the group
//!   in that shape;
//! - during the deployment, « Placer en formation » proposes the places (ghosts) of the selection
//!   (or of the whole army), a right click moves the proposal, a second press validates it
//!   (`deploy_unit_width` with the frontage).
//!
//! The picker panel sits at the bottom right above the card band; Alt+Shift+1..6 toggles a preset.

use bevy::prelude::*;
use sim_battle::group_formation::{self, FormationSlot};
use sim_battle::{BattleSim, Command, SideId};

use super::BattleSelection;
use super::deploy::{self, Deployment};
use super::drag::{self, DRAG_PX};
use super::input::Gesture;
use super::overlay::{OrdersGizmos, ground_line, outline_rect};
use super::picking::UnitView;
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::BattleCamera;
use crate::ui::kit::modal::ToastRequest;
use crate::ui::kit::style::UiTier;
use crate::ui::kit::widgets::{Toggled, button_small, caption, panel_page, toggle_button};
use crate::ui::kit::{ButtonPressed, UiTheme};

/// Setback (m) of the proposed front from the front edge of the deployment zone.
const ZONE_SETBACK: f32 = 30.0;
/// Space under the picker for the card band (px).
const BAND_SPACE: f32 = 150.0;
const STANCES: [(&str, &str); 3] = [
    ("attack", "Attaque"),
    ("defense", "Défense"),
    ("march", "Marche"),
];
const GHOST: Color = Color::srgba(0.95, 0.85, 0.4, 0.8);

/// A place of a regiment in a formation, ready to be ordered or drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slot {
    pub id: u32,
    pub x: f32,
    pub z: f32,
    pub facing: f32,
    /// Frontage to order (0: the regiment keeps its formation).
    pub order_width: f32,
    pub width: f32,
    pub depth: f32,
}

#[derive(Debug, Clone)]
struct Anchor {
    ids: Vec<u32>,
    point: Vec2,
    facing: f32,
}

/// Picker state.
#[derive(Resource, Debug, Default)]
pub struct GroupFormations {
    /// Active preset id (`None`: right clicks keep their ordinary meaning).
    pub active: Option<String>,
    /// Deployment: proposal waiting for validation.
    pub pending: Vec<Slot>,
    anchor: Option<Anchor>,
    /// Preset changed: recompute the proposal.
    dirty: bool,
    pub collapsed: bool,
    auto_collapse_done: bool,
    /// Orders given through a preset (dump).
    pub orders: u32,
}

impl GroupFormations {
    /// Activates preset `id`, or deactivates it if it was already active; the text for a toast.
    pub fn toggle(&mut self, id: &str) -> String {
        let name = name_of(id);
        if self.active.as_deref() == Some(id) {
            self.active = None;
            self.pending.clear();
            self.anchor = None;
            return "Formation de groupe désactivée.".to_owned();
        }
        self.active = Some(id.to_owned());
        self.dirty = true;
        format!("« {name} » : clic droit pour placer la sélection (glisser : orientation).")
    }

    /// Alt+Shift+`index` (1 = first preset of the data); the toast text, `None` if out of range.
    pub fn select_index(&mut self, index: usize) -> Option<String> {
        let presets = group_formation::presets();
        let id = presets.get(index.checked_sub(1)?)?.id.clone();
        Some(self.toggle(&id))
    }
}

fn name_of(id: &str) -> String {
    group_formation::presets()
        .iter()
        .find(|p| p.id == id)
        .map(|p| p.name_fr.clone())
        .unwrap_or_default()
}

/// The places preset `preset` gives `ids` for a front centred on `point`, facing `facing`.
pub fn slots_for(
    sim: &BattleSim,
    preset: &str,
    ids: &[u32],
    point: Vec2,
    facing: f32,
) -> Vec<Slot> {
    let raw: Vec<FormationSlot> = sim.formation_slots(
        preset,
        ids,
        f64::from(point.x),
        f64::from(point.y),
        f64::from(facing),
    );
    raw.into_iter()
        .map(|slot| {
            let order_width = slot.width.unwrap_or(0.0);
            let extent = sim.unit(slot.id).map(|u| {
                if order_width > 0.0 {
                    u.extent_for_width(Some(order_width))
                } else {
                    u.extent()
                }
            });
            let (width, depth) = extent.unwrap_or((0.0, 0.0));
            Slot {
                id: slot.id,
                x: slot.x as f32,
                z: slot.z as f32,
                facing: slot.facing as f32,
                order_width: order_width as f32,
                width: width as f32,
                depth: depth as f32,
            }
        })
        .collect()
}

/// Facing of a plain click: from the centre of the regiments `ids` towards `point`; too close,
/// their mean facing.
fn facing_for(views: &[UnitView], ids: &[u32], point: Vec2) -> f32 {
    let chosen: Vec<&UnitView> = views.iter().filter(|u| ids.contains(&u.id)).collect();
    if chosen.is_empty() {
        return 0.0;
    }
    let centre = chosen
        .iter()
        .fold(Vec2::ZERO, |sum, u| sum + Vec2::new(u.pos.x, u.pos.z))
        / chosen.len() as f32;
    let dir = point - centre;
    if dir.length() < 5.0 {
        let facings: Vec<f32> = chosen.iter().map(|u| u.facing).collect();
        return drag::mean_facing(&facings);
    }
    dir.x.atan2(dir.y)
}

/// Point and facing of a right click (`dragged` false) or of a right drag `p0 -> p1` (front
/// opposite to the camera `eye`) for `ids`.
pub fn aim(
    views: &[UnitView],
    ids: &[u32],
    p0: Vec3,
    p1: Vec3,
    eye: Vec3,
    dragged: bool,
) -> (Vec2, f32) {
    if dragged && let Some(line) = drag::drag_line(p0, p1, eye) {
        return (Vec2::new(line.mid.x, line.mid.z), line.facing);
    }
    let point = Vec2::new(p1.x, p1.z);
    (point, facing_for(views, ids, point))
}

/// One `move` per place: width to order, the pace of the slowest and a common group tag
/// (`tag` 0: untagged).
pub fn slot_commands(places: &[Slot], tag: u32, run: bool, queued: bool) -> Vec<Command> {
    places
        .iter()
        .map(|place| Command::Move {
            units: vec![place.id],
            x: f64::from(place.x),
            z: f64::from(place.z),
            run,
            facing: Some(f64::from(place.facing)),
            queue: queued,
            width: (place.order_width > 0.0).then_some(f64::from(place.order_width)),
            match_speed: true,
            group_tag: (tag != 0).then_some(tag),
        })
        .collect()
}

// ----- deployment ------------------------------------------------------------------------------

/// Regiments aimed at during the deployment: the selection, otherwise the whole present army.
pub fn deploy_ids(views: &[UnitView], player: SideId, selection: &[u32]) -> Vec<u32> {
    views
        .iter()
        .filter(|u| {
            u.side == player && u.present && (selection.is_empty() || selection.contains(&u.id))
        })
        .map(|u| u.id)
        .collect()
}

/// Default front of the proposal: middle of the zone, a little behind its front edge, facing the
/// enemy.
fn default_anchor(zone: [f32; 4], player: SideId) -> (Vec2, f32) {
    let cx = (zone[0] + zone[2]) * 0.5;
    if player == SideId::Attacker {
        (Vec2::new(cx, zone[3] - ZONE_SETBACK), 0.0)
    } else {
        (Vec2::new(cx, zone[1] + ZONE_SETBACK), std::f32::consts::PI)
    }
}

impl GroupFormations {
    fn propose(&mut self, sim: &BattleSim, ids: Vec<u32>, point: Vec2, facing: f32) {
        self.pending = match &self.active {
            Some(preset) => slots_for(sim, preset, &ids, point, facing),
            None => Vec::new(),
        };
        self.anchor = Some(Anchor { ids, point, facing });
    }

    /// Right click (or drag) during the deployment with an active preset: the proposal moves.
    #[allow(clippy::too_many_arguments)]
    pub fn deploy_click(
        &mut self,
        sim: &BattleSim,
        views: &[UnitView],
        player: SideId,
        selection: &[u32],
        zone: Option<[f32; 4]>,
        p0: Vec3,
        p1: Vec3,
        eye: Vec3,
        dragged: bool,
    ) {
        let ids = deploy_ids(views, player, selection);
        let (point, facing) = aim(views, &ids, p0, p1, eye, dragged);
        let facing = match zone {
            Some(zone) if !dragged => default_anchor(zone, player).1,
            _ => facing,
        };
        self.propose(sim, ids, point, facing);
    }

    /// Validates the proposal (`deploy_unit_width` with the frontage); placed count and refusals.
    pub fn apply(&mut self, sim: &mut BattleSim) -> (u32, Vec<String>) {
        let mut placed = 0;
        let mut errors = Vec::new();
        for slot in std::mem::take(&mut self.pending) {
            let result = sim.deploy_unit_width(
                slot.id,
                f64::from(slot.x),
                f64::from(slot.z),
                Some(f64::from(slot.facing)),
                (slot.order_width > 0.0).then_some(f64::from(slot.order_width)),
            );
            match result {
                Ok(()) => placed += 1,
                Err(error) => errors.push(sim.error_text(&error)),
            }
        }
        self.anchor = None;
        (placed, errors)
    }
}

// ----- panel -----------------------------------------------------------------------------------

#[derive(Component)]
pub struct PickerBody;
#[derive(Component)]
pub struct PickerCollapse;
#[derive(Component)]
pub struct PickerPlace;
#[derive(Component)]
pub struct PickerCancel;
#[derive(Component)]
pub struct PresetButton(pub String);

pub fn spawn_picker(mut commands: Commands, theme: Res<UiTheme>) {
    let presets = group_formation::presets();
    if presets.is_empty() {
        return;
    }
    commands
        .spawn((
            Name::new("FormationPicker"),
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(12.0),
                bottom: Val::Px(BAND_SPACE),
                ..default()
            },
            GlobalZIndex(UiTier::Hud.z() + 1),
            DespawnOnExit(crate::app::AppState::Battle),
        ))
        .with_children(|root| {
            root.spawn(panel_page(&theme)).with_children(|panel| {
                panel
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(8.0),
                        ..default()
                    })
                    .with_children(|header| {
                        header.spawn(caption(&theme, "Formations de groupe"));
                        header.spawn((
                            PickerCollapse,
                            button_small(&theme, "formation:collapse", "▾"),
                        ));
                    });
                panel
                    .spawn((
                        PickerBody,
                        Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(4.0),
                            ..default()
                        },
                    ))
                    .with_children(|body| {
                        for (stance, label) in STANCES {
                            let row: Vec<_> = presets
                                .iter()
                                .filter(|p| p.stance.key() == stance)
                                .collect();
                            if row.is_empty() {
                                continue;
                            }
                            body.spawn(Node {
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                column_gap: Val::Px(4.0),
                                ..default()
                            })
                            .with_children(|line| {
                                line.spawn((
                                    caption(&theme, label),
                                    Node {
                                        min_width: Val::Px(62.0),
                                        ..default()
                                    },
                                ));
                                for preset in row {
                                    line.spawn((
                                        PresetButton(preset.id.clone()),
                                        toggle_button(
                                            &theme,
                                            &format!("formation:{}", preset.id),
                                            &preset.name_fr,
                                            false,
                                        ),
                                    ));
                                }
                            });
                        }
                        body.spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(4.0),
                            ..default()
                        })
                        .with_children(|actions| {
                            actions.spawn((
                                PickerPlace,
                                button_small(&theme, "formation:place", "Placer en formation"),
                            ));
                            actions.spawn((
                                PickerCancel,
                                button_small(&theme, "formation:cancel", "Annuler"),
                            ));
                        });
                    });
            });
        });
}

fn set_caption(
    entity: Entity,
    text: &str,
    children: &Query<&Children>,
    texts: &mut Query<&mut Text>,
) {
    let Ok(kids) = children.get(entity) else {
        return;
    };
    for kid in kids.iter() {
        if let Ok(mut label) = texts.get_mut(kid)
            && label.0 != text
        {
            label.0 = text.to_owned();
        }
    }
}

/// Buttons of the panel: preset toggles, fold, place / cancel of the deployment.
#[allow(clippy::too_many_arguments)]
pub fn picker_buttons(
    mut pressed: MessageReader<ButtonPressed>,
    mut state: ResMut<GroupFormations>,
    mut session: ResMut<BattleSession>,
    deployment: Res<Deployment>,
    selection: Res<BattleSelection>,
    views: Res<super::input::BattleViews>,
    mut toasts: MessageWriter<ToastRequest>,
) {
    for press in pressed.read() {
        let Some(key) = press.name.strip_prefix("formation:") else {
            continue;
        };
        match key {
            "collapse" => {
                state.collapsed = !state.collapsed;
                state.auto_collapse_done = true;
            }
            "cancel" => {
                state.pending.clear();
                state.anchor = None;
            }
            "place" => {
                if !deployment.active || state.active.is_none() {
                    continue;
                }
                if state.pending.is_empty() {
                    let Some(player) = session.player else {
                        continue;
                    };
                    let Some(zone) = deployment.zones.first().copied() else {
                        continue;
                    };
                    let (point, facing) = default_anchor(zone, player);
                    let ids = deploy_ids(&views.0, player, &selection.units);
                    let sim = session.sim_mut();
                    state.propose(sim, ids, point, facing);
                } else {
                    let (_, errors) = state.apply(session.sim_mut());
                    for error in errors {
                        toasts.write(ToastRequest::new(&error));
                    }
                }
            }
            id => {
                let text = state.toggle(id);
                toasts.write(ToastRequest::new(&text));
            }
        }
    }
}

/// Keeps the widgets in line with the state: pressed preset, body folded, place / cancel only
/// during the deployment, caption of the place button; folds on its own once the fight starts.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn sync_picker(
    mut state: ResMut<GroupFormations>,
    deployment: Res<Deployment>,
    session: Res<BattleSession>,
    mut preset_buttons: Query<(&PresetButton, &mut Toggled)>,
    mut body: Query<
        &mut Node,
        (
            With<PickerBody>,
            Without<PickerPlace>,
            Without<PickerCancel>,
        ),
    >,
    mut place: Query<(Entity, &mut Node), (With<PickerPlace>, Without<PickerCancel>)>,
    mut cancel: Query<&mut Node, (With<PickerCancel>, Without<PickerPlace>)>,
    collapse: Query<Entity, With<PickerCollapse>>,
    children: Query<&Children>,
    mut texts: Query<&mut Text>,
) {
    if !state.auto_collapse_done && deployment.opened && !deployment.active && !session.paused {
        state.auto_collapse_done = true;
        state.collapsed = true;
    }
    for (button, mut toggled) in &mut preset_buttons {
        let on = state.active.as_deref() == Some(button.0.as_str());
        if toggled.0 != on {
            toggled.0 = on;
        }
    }
    let show = |on: bool| if on { Display::Flex } else { Display::None };
    for mut node in &mut body {
        node.display = show(!state.collapsed);
    }
    for (entity, mut node) in &mut place {
        node.display = show(deployment.active);
        let text = if state.pending.is_empty() {
            "Placer en formation"
        } else {
            "Valider la formation"
        };
        set_caption(entity, text, &children, &mut texts);
    }
    for mut node in &mut cancel {
        node.display = show(deployment.active && !state.pending.is_empty());
    }
    for entity in &collapse {
        set_caption(
            entity,
            if state.collapsed { "▸" } else { "▾" },
            &children,
            &mut texts,
        );
    }
}

// ----- ghosts ----------------------------------------------------------------------------------

/// Proposal during the deployment (live while the right button is held, recomputed when the
/// preset changes) and ghosts of the places a right click would give in battle.
#[allow(clippy::too_many_arguments)]
pub fn draw_ghosts(
    mut gizmos: Gizmos<OrdersGizmos>,
    mut state: ResMut<GroupFormations>,
    mut session: ResMut<BattleSession>,
    deployment: Res<Deployment>,
    selection: Res<BattleSelection>,
    views: Res<super::input::BattleViews>,
    gesture: Res<Gesture>,
    cameras: Query<(&Camera, &GlobalTransform), With<BattleCamera>>,
) {
    let Some(player) = session.player else { return };
    let Ok((camera, transform)) = cameras.single() else {
        return;
    };
    let eye = transform.translation();
    let zone = deployment.zones.first().copied();
    let sim = session.sim_mut();
    let end = deploy::ground_under(camera, transform, gesture.cursor, sim.field());
    let held = gesture.right_press;
    if state.active.is_some() && deployment.active {
        if let (Some(press), Some(end)) = (held, end) {
            let dragged = (press.pos - gesture.cursor).length() > DRAG_PX;
            let (p0, p1) = if dragged {
                (press.ground, end)
            } else {
                (end, end)
            };
            state.deploy_click(
                sim,
                &views.0,
                player,
                &selection.units,
                zone,
                p0,
                p1,
                eye,
                dragged,
            );
        } else if state.dirty
            && let Some(anchor) = state.anchor.clone()
        {
            state.propose(sim, anchor.ids, anchor.point, anchor.facing);
        }
        state.dirty = false;
        let color = GHOST;
        for slot in &state.pending {
            outline_rect(
                &mut gizmos,
                sim.field(),
                Vec2::new(slot.x, slot.z),
                slot.facing,
                slot.width,
                slot.depth,
                color,
            );
        }
        return;
    }
    state.dirty = false;
    // In battle: ghosts while the right button is held on the ground with a preset active.
    let (Some(preset), Some(press), Some(end)) = (state.active.clone(), held, end) else {
        return;
    };
    if selection.units.is_empty() || gesture.hover_enemy.is_some() {
        return;
    }
    let ids: Vec<u32> = selection
        .units
        .iter()
        .copied()
        .filter(|id| views.0.iter().any(|u| u.id == *id && u.available()))
        .collect();
    let dragged = (press.pos - gesture.cursor).length() > DRAG_PX;
    let (point, facing) = aim(&views.0, &ids, press.ground, end, eye, dragged);
    for slot in slots_for(sim, &preset, &ids, point, facing) {
        outline_rect(
            &mut gizmos,
            sim.field(),
            Vec2::new(slot.x, slot.z),
            slot.facing,
            slot.width,
            slot.depth,
            GHOST,
        );
        if let Some(unit) = views.0.iter().find(|u| u.id == slot.id) {
            gizmos.linestrip(
                ground_line(
                    sim.field(),
                    Vec2::new(unit.pos.x, unit.pos.z),
                    Vec2::new(slot.x, slot.z),
                    0.5,
                ),
                GHOST.with_alpha(0.35),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_activates_then_deactivates() {
        let mut state = GroupFormations::default();
        let first = group_formation::presets()[0].id.clone();
        assert!(state.toggle(&first).contains('«'));
        assert_eq!(state.active.as_deref(), Some(first.as_str()));
        assert!(state.toggle(&first).contains("désactivée"));
        assert_eq!(state.active, None);
    }

    #[test]
    fn shortcut_index_is_one_based_and_bounded() {
        let mut state = GroupFormations::default();
        assert!(state.select_index(0).is_none());
        assert!(state.select_index(1).is_some());
        assert!(state.select_index(99).is_none());
    }

    #[test]
    fn slot_commands_carry_width_tag_and_pace() {
        let place = Slot {
            id: 3,
            x: 10.0,
            z: 20.0,
            facing: 1.0,
            order_width: 40.0,
            width: 40.0,
            depth: 8.0,
        };
        let plain = Slot {
            order_width: 0.0,
            id: 4,
            ..place
        };
        let commands = slot_commands(&[place, plain], 7, true, true);
        let Command::Move {
            width,
            group_tag,
            match_speed,
            queue,
            run,
            ..
        } = &commands[0]
        else {
            panic!("a move")
        };
        assert_eq!(
            (*width, *group_tag, *match_speed, *queue, *run),
            (Some(40.0), Some(7), true, true, true)
        );
        let Command::Move { width, .. } = &commands[1] else {
            panic!("a move")
        };
        assert_eq!(*width, None);
        let Command::Move { group_tag, .. } = &slot_commands(&[plain], 0, false, false)[0] else {
            panic!("a move")
        };
        assert_eq!(*group_tag, None);
    }

    #[test]
    fn default_anchor_sits_behind_the_front_edge_facing_the_enemy() {
        let zone = [0.0, 0.0, 400.0, 100.0];
        let (point, facing) = default_anchor(zone, SideId::Attacker);
        assert_eq!((point.x, point.y, facing), (200.0, 70.0, 0.0));
        let (point, facing) = default_anchor(zone, SideId::Defender);
        assert_eq!((point.y, facing), (30.0, std::f32::consts::PI));
    }
}
