//! Pointer and keyboard to selection and orders.
//!
//! Ports game/scripts/battle/battle_input.gd: left click / box / Shift add / double click for the
//! quick selection, right click to move or attack (Shift queues, Alt walks the attack), right
//! drag for a line with a width (and pivot on the spot), locked groups, Ctrl+1..9 groups and the
//! hotkey table. Orders go to `sim_battle::BattleSim::issue_command` exactly as
//! `godot-bridge` does (`battle_sim_orders.rs`), without depending on the bridge.
//!
//! Raw mouse/keyboard are first turned into [`PointerEvent`] messages so that the dev hook (and
//! tests) drive the very same code as a real mouse.

use std::collections::VecDeque;

use bevy::input::mouse::MouseButton;
use bevy::picking::hover::HoverMap;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use sim_battle::{BattleSim, Command, CommandError, Formation, PreviewError, SideId, UnitMode};

use super::BattleSelection;
use super::drag::{self, DRAG_PX, Modifiers};
use super::groups::BattleGroups;
use super::hotkeys::{self, Action};
use super::picking::{self, UnitView};
use super::select;
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::{BattleCamera, BattleCameraState};
use crate::ui::kit::modal::ToastRequest;

/// Double click window (s).
const DOUBLE_CLICK_S: f32 = 0.35;
/// A left drag longer than this (px) is a box.
const BOX_PX: f32 = 8.0;
/// Second press of a group key within this time (s) centres the camera on the group.
const GROUP_AGAIN_S: f32 = 0.6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerKind {
    LeftDown,
    LeftUp,
    RightDown,
    RightUp,
    Move,
}

/// A mouse event at window position `pos` (logical px). Written by [`read_pointer`]; the dev hook
/// writes the same.
#[derive(Message, Debug, Clone, Copy)]
pub struct PointerEvent {
    pub kind: PointerKind,
    pub pos: Vec2,
    pub mods: Modifiers,
}

/// An order given by another part of the interface (unit cards, ability bar, formation menu):
/// sent to the simulation like any order of this module.
#[derive(Message, Debug, Clone)]
pub struct IssueBattleCommand(pub Command);

/// Move the selection to a ground point (right click on the minimap).
#[derive(Message, Debug, Clone, Copy)]
pub struct MoveSelectionTo {
    pub point: Vec3,
    pub queued: bool,
    pub run: bool,
}

/// What the cursor is over, for the cursor icon.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Context {
    #[default]
    Ground,
    Own,
    Attack,
}

#[derive(Debug, Clone, Copy)]
pub struct RightPress {
    pub pos: Vec2,
    pub ground: Vec3,
}

/// Mouse gesture in progress and hover state.
#[derive(Resource, Debug, Clone)]
pub struct Gesture {
    pub left_press: Option<Vec2>,
    pub right_press: Option<RightPress>,
    pub cursor: Vec2,
    pub hover_own: Option<u32>,
    pub hover_enemy: Option<u32>,
    pub context: Context,
    last_left: Option<(u32, f32)>,
    last_right: f32,
    last_group: (u8, f32),
    /// Current box in window px, for the selection rectangle.
    pub drag_rect: Option<(Vec2, Vec2)>,
    pub shift: bool,
}

impl Default for Gesture {
    fn default() -> Self {
        Self {
            left_press: None,
            right_press: None,
            cursor: Vec2::ZERO,
            hover_own: None,
            hover_enemy: None,
            context: Context::Ground,
            last_left: None,
            last_right: -10.0,
            last_group: (0, -10.0),
            drag_rect: None,
            shift: false,
        }
    }
}

/// One order sent to the simulation (dump, tests).
#[derive(Debug, Clone)]
pub struct IssuedOrder {
    pub tick: u64,
    pub command: serde_json::Value,
    /// `"ok"` or the refusal of the core (French).
    pub result: String,
}

#[derive(Resource, Debug, Default)]
pub struct OrderLog {
    pub accepted: u32,
    pub refused: u32,
    pub recent: VecDeque<IssuedOrder>,
}

impl OrderLog {
    const KEEP: usize = 12;

    fn record(&mut self, order: IssuedOrder) {
        if order.result == "ok" {
            self.accepted += 1;
        } else {
            self.refused += 1;
        }
        self.recent.push_back(order);
        while self.recent.len() > Self::KEEP {
            self.recent.pop_front();
        }
    }
}

/// Regiments of the battle as of the latest frame.
#[derive(Resource, Debug, Default)]
pub struct BattleViews(pub Vec<UnitView>);

/// Camera of the battle: projection and rays for picking.
pub struct CameraView<'a> {
    camera: &'a Camera,
    transform: &'a GlobalTransform,
}

impl CameraView<'_> {
    pub fn project(&self, world: Vec3) -> Option<Vec2> {
        self.camera.world_to_viewport(self.transform, world).ok()
    }

    pub fn ray(&self, screen: Vec2) -> Option<Ray3d> {
        self.camera.viewport_to_world(self.transform, screen).ok()
    }

    pub fn eye(&self) -> Vec3 {
        self.transform.translation()
    }
}

/// Everything the logic below touches, gathered by the systems.
pub struct Ctx<'a> {
    pub sim: &'a mut BattleSim,
    pub views: &'a [UnitView],
    pub player: Option<SideId>,
    pub selection: &'a mut BattleSelection,
    pub groups: &'a mut BattleGroups,
    pub gesture: &'a mut Gesture,
    pub log: &'a mut OrderLog,
    pub camera: Option<CameraView<'a>>,
    pub now: f32,
    pub toasts: Vec<String>,
    /// Camera focus asked (group key pressed twice, idle unit).
    pub focus: Option<Vec3>,
}

impl Ctx<'_> {
    fn enemy(&self) -> Option<SideId> {
        self.player.map(SideId::other)
    }

    fn ground(&self, screen: Vec2) -> Option<Vec3> {
        let ray = self.camera.as_ref()?.ray(screen)?;
        Some(picking::ground_point(
            ray.origin,
            *ray.direction,
            self.sim.field(),
        ))
    }

    fn pick(&self, screen: Vec2, side: Option<SideId>) -> Option<u32> {
        let side = side?;
        let camera = self.camera.as_ref()?;
        picking::pick_unit(screen, self.views, side, &|p| camera.project(p))
    }

    /// Selected regiments able to take orders (not routing, withdrawing or gone).
    pub fn available_selection(&self) -> Vec<u32> {
        self.selection
            .units
            .iter()
            .copied()
            .filter(|id| self.views.iter().any(|u| u.id == *id && u.available()))
            .collect()
    }

    /// Sends `command` to the simulation as the player; logs it; toasts a refusal.
    pub fn issue(&mut self, command: Command) -> bool {
        let json = serde_json::to_value(&command).unwrap_or_default();
        let tick = self.sim.ticks();
        let result = match self.sim.issue_command(command) {
            Ok(()) => "ok".to_owned(),
            Err(error) => {
                let text = refusal_fr(self.sim, &error);
                self.toasts.push(text.clone());
                text
            }
        };
        let ok = result == "ok";
        self.log.record(IssuedOrder {
            tick,
            command: json,
            result,
        });
        ok
    }

    pub fn select(&mut self, ids: Vec<u32>) {
        self.selection.units = ids;
    }
}

fn refusal_fr(sim: &BattleSim, error: &CommandError) -> String {
    match error {
        CommandError::QueueFull { .. } | CommandError::Deploying | CommandError::OutsideZone(_) => {
            sim.error_text(error)
        }
        other => other.to_string(),
    }
}

/// French reason of a refused preview (tooltip of the forbidden cursor).
pub fn reason_fr(error: PreviewError) -> &'static str {
    match error {
        PreviewError::UnknownUnit | PreviewError::Unavailable => "Ce régiment ne répond plus",
        PreviewError::OutsideField => "Hors du champ de bataille",
        PreviewError::OutsideZone => "Hors de la zone de déploiement",
        PreviewError::Unreachable => "Aucun chemin jusque-là",
    }
}

// ----- pointer gestures ---------------------------------------------------------------------

/// Applies one pointer event (port of `_unhandled_input` mouse branches).
pub fn pointer(ctx: &mut Ctx, event: PointerEvent) {
    ctx.gesture.cursor = event.pos;
    ctx.gesture.shift = event.mods.shift;
    match event.kind {
        PointerKind::LeftDown => ctx.gesture.left_press = Some(event.pos),
        PointerKind::LeftUp => finish_left(ctx, event.pos, event.mods.shift),
        PointerKind::RightDown => {
            let ground = ctx.ground(event.pos).unwrap_or_default();
            ctx.gesture.right_press = Some(RightPress {
                pos: event.pos,
                ground,
            });
        }
        PointerKind::RightUp => finish_right(ctx, event.pos, event.mods),
        PointerKind::Move => {
            hover(ctx, event.pos);
            if let Some(press) = ctx.gesture.left_press {
                let (min, max) = (press.min(event.pos), press.max(event.pos));
                ctx.gesture.drag_rect = ((max - min).length() > BOX_PX).then_some((min, max));
            }
        }
    }
}

fn hover(ctx: &mut Ctx, pos: Vec2) {
    let own = ctx.pick(pos, ctx.player);
    let enemy = if own.is_none() {
        ctx.pick(pos, ctx.enemy())
    } else {
        None
    };
    ctx.gesture.hover_own = own;
    ctx.gesture.hover_enemy = enemy;
    ctx.gesture.context = if enemy.is_some() && !ctx.selection.units.is_empty() {
        Context::Attack
    } else if own.is_some() {
        Context::Own
    } else {
        Context::Ground
    };
}

fn finish_left(ctx: &mut Ctx, pos: Vec2, additive: bool) {
    let Some(press) = ctx.gesture.left_press.take() else {
        return;
    };
    ctx.gesture.drag_rect = None;
    let Some(player) = ctx.player else {
        return;
    };
    if !additive {
        ctx.selection.units.clear();
    }
    if (pos - press).length() > BOX_PX {
        let (min, max) = (press.min(pos), press.max(pos));
        let ids = match &ctx.camera {
            Some(camera) => {
                picking::box_select(min, max, ctx.views, player, &|p| camera.project(p))
            }
            None => Vec::new(),
        };
        for id in ids {
            if !ctx.selection.units.contains(&id) {
                ctx.selection.units.push(id);
            }
        }
        return;
    }
    let Some(picked) = ctx.pick(pos, Some(player)) else {
        return;
    };
    let double = ctx
        .gesture
        .last_left
        .is_some_and(|(id, at)| id == picked && ctx.now - at < DOUBLE_CLICK_S);
    ctx.gesture.last_left = Some((picked, ctx.now));
    if double {
        let ids = select::same_type_of(ctx.views, player, picked);
        ctx.select(ids);
    } else if additive && ctx.selection.units.contains(&picked) {
        ctx.selection.units.retain(|id| *id != picked);
    } else if !ctx.selection.units.contains(&picked) {
        ctx.selection.units.push(picked);
    }
}

fn finish_right(ctx: &mut Ctx, pos: Vec2, mods: Modifiers) {
    let Some(press) = ctx.gesture.right_press.take() else {
        return;
    };
    if ctx.selection.units.is_empty() || ctx.player.is_none() {
        return;
    }
    let queued = mods.shift;
    if ctx.sim.is_deploying() {
        deploy_selection(ctx, press, pos);
        return;
    }
    let double = ctx.now - ctx.gesture.last_right < DOUBLE_CLICK_S;
    ctx.gesture.last_right = ctx.now;
    let ids = ctx.available_selection();
    if ids.is_empty() {
        return;
    }
    let lock = ctx.groups.locked_group_for(&ctx.selection.units);
    if (press.pos - pos).length() > DRAG_PX {
        let Some(end) = ctx.ground(pos) else { return };
        let eye = ctx.camera.as_ref().map(CameraView::eye).unwrap_or_default();
        let Some(line) = drag::drag_line(press.ground, end, eye) else {
            return;
        };
        if let Some(tag) = lock {
            issue_locked(
                ctx,
                tag,
                Vec2::new(line.mid.x, line.mid.z),
                Some(line.facing),
                double,
                queued,
            );
            return;
        }
        let pivot = drag::pivot_orders(ctx.views, &ids, line.mid, line.facing, queued);
        if !pivot.is_empty() {
            for command in pivot {
                ctx.issue(command);
            }
            return;
        }
        let point = Vec2::new(line.mid.x, line.mid.z);
        if !path_allowed(ctx, &ids, point, Some(line.facing), line.width, queued) {
            return;
        }
        ctx.issue(drag::move_command(
            &ids,
            point,
            double,
            Some(line.facing),
            Some(line.width),
            queued,
        ));
        return;
    }
    if let Some(enemy) = ctx.pick(pos, ctx.enemy()) {
        ctx.issue(drag::attack_command(&ids, enemy, !mods.alt, queued));
        return;
    }
    if let Some(point) = ctx.ground(pos) {
        order_move_to(ctx, point, queued, double);
    }
}

/// Move order of the selection to `point` (right click on the ground or on the minimap): a
/// locked group as a block, otherwise one `move` when at least one regiment has a path.
pub fn order_move_to(ctx: &mut Ctx, point: Vec3, queued: bool, run: bool) {
    if ctx.selection.units.is_empty() || ctx.player.is_none() {
        return;
    }
    if ctx.sim.is_deploying() {
        place_units(
            ctx,
            &ctx.selection.units.clone(),
            Vec2::new(point.x, point.z),
            None,
            0.0,
        );
        return;
    }
    let ids = ctx.available_selection();
    if ids.is_empty() {
        return;
    }
    let target = Vec2::new(point.x, point.z);
    if let Some(tag) = ctx.groups.locked_group_for(&ctx.selection.units) {
        issue_locked(ctx, tag, target, None, run, queued);
        return;
    }
    if !path_allowed(ctx, &ids, target, None, 0.0, queued) {
        return;
    }
    ctx.issue(drag::move_command(&ids, target, run, None, None, queued));
}

/// The order is only sent when at least one regiment has a path there (verdict of the core's
/// `preview_group_width`); otherwise a toast and nothing sent.
fn path_allowed(
    ctx: &mut Ctx,
    ids: &[u32],
    point: Vec2,
    facing: Option<f32>,
    width: f32,
    queued: bool,
) -> bool {
    let legs = ctx.sim.preview_group_width(
        ids,
        f64::from(point.x),
        f64::from(point.y),
        facing.map(f64::from),
        (width > 0.0).then_some(f64::from(width)),
        queued,
    );
    if legs.is_empty() || legs.iter().any(|leg| leg.path.is_ok()) {
        return true;
    }
    let reason = legs
        .iter()
        .find_map(|leg| leg.path.as_ref().err().copied())
        .map_or("Aucun chemin jusque-là", reason_fr);
    ctx.toasts
        .push(format!("Ordre impossible : {}.", reason.to_lowercase()));
    false
}

fn issue_locked(
    ctx: &mut Ctx,
    tag: u32,
    point: Vec2,
    facing: Option<f32>,
    run: bool,
    queued: bool,
) {
    let Some(group) = ctx.groups.locks.get(&tag) else {
        return;
    };
    let places = drag::rigid_places(&group.shape, &group.ids, point, facing);
    if places.is_empty() {
        return;
    }
    let reachable = places.iter().any(|&(id, x, z, f)| {
        ctx.sim
            .preview_group_width(
                &[id],
                f64::from(x),
                f64::from(z),
                Some(f64::from(f)),
                None,
                queued,
            )
            .iter()
            .any(|leg| leg.path.is_ok())
    });
    if !reachable {
        ctx.toasts
            .push("Ordre impossible : aucun chemin jusque-là.".to_owned());
        return;
    }
    for command in drag::locked_orders(&places, tag, run, queued) {
        ctx.issue(command);
    }
}

// ----- deployment (basic placement of the selection) ------------------------------------------

fn deploy_selection(ctx: &mut Ctx, press: RightPress, release: Vec2) {
    let Some(end) = ctx.ground(release) else {
        return;
    };
    let ids = ctx.selection.units.clone();
    if (press.pos - release).length() > DRAG_PX {
        let eye = ctx.camera.as_ref().map(CameraView::eye).unwrap_or_default();
        if let Some(line) = drag::drag_line(press.ground, end, eye) {
            place_units(
                ctx,
                &ids,
                Vec2::new(line.mid.x, line.mid.z),
                Some(line.facing),
                line.width,
            );
        }
        return;
    }
    place_units(ctx, &ids, Vec2::new(end.x, end.z), None, 0.0);
}

/// Places `ids` side by side around `centre` (facing kept unless given); during deployment only.
fn place_units(ctx: &mut Ctx, ids: &[u32], centre: Vec2, facing: Option<f32>, width: f32) {
    let spacing = |id: &u32| {
        ctx.views
            .iter()
            .find(|u| u.id == *id)
            .map_or(20.0, |u| u.width + 4.0)
    };
    let total: f32 = ids.iter().map(spacing).sum();
    let f = facing.unwrap_or(0.0);
    let right = Vec2::new(f.cos(), -f.sin());
    let mut offset = -total * 0.5;
    let placements: Vec<(u32, Vec2)> = ids
        .iter()
        .map(|id| {
            let step = spacing(id);
            let at = centre + right * (offset + step * 0.5);
            offset += step;
            (*id, at)
        })
        .collect();
    let share = if width > 0.0 && !ids.is_empty() {
        Some(f64::from(width) / ids.len() as f64)
    } else {
        None
    };
    for (id, at) in placements {
        let result = ctx.sim.deploy_unit_width(
            id,
            f64::from(at.x),
            f64::from(at.y),
            facing.map(f64::from),
            share,
        );
        if let Err(error) = result {
            let text = ctx.sim.error_text(&error);
            ctx.toasts.push(text);
        }
    }
}

// ----- keyboard actions -----------------------------------------------------------------------

/// Performs a hotkey action on the selection.
pub fn perform(ctx: &mut Ctx, action: Action) {
    let Some(player) = ctx.player else { return };
    match action {
        Action::SelectAll => {
            let ids = select::all_of(ctx.views, player, |_| true);
            ctx.select(ids);
        }
        Action::SelectShooters => {
            let ids = select::all_of(ctx.views, player, |u| u.can_shoot);
            ctx.select(ids);
        }
        Action::SelectCavalry => {
            let ids = select::all_of(ctx.views, player, |u| u.cavalry);
            ctx.select(ids);
        }
        Action::NextIdle | Action::PrevIdle => {
            let dir = if action == Action::NextIdle { 1 } else { -1 };
            match select::next_idle(ctx.views, player, &ctx.selection.units, dir) {
                Some(id) => {
                    ctx.focus = ctx.views.iter().find(|u| u.id == id).map(|u| u.pos);
                    ctx.select(vec![id]);
                }
                None => ctx.toasts.push("Aucune unité au repos.".to_owned()),
            }
        }
        Action::Group { number, save } => group_key(ctx, number, save),
        Action::LockGroup => {
            if ctx.selection.units.is_empty() {
                return;
            }
            let ids = ctx.selection.units.clone();
            let text = match ctx.groups.toggle_lock(&ids, ctx.views) {
                Some(_) => {
                    "Groupe verrouillé : il se déplace d’un bloc, à l’allure du plus lent (Ctrl+G : déverrouiller)."
                }
                None => "Groupe déverrouillé.",
            };
            ctx.toasts.push(text.to_owned());
        }
        order => unit_order(ctx, order),
    }
}

fn group_key(ctx: &mut Ctx, number: u8, save: bool) {
    if save {
        let ids = ctx.selection.units.clone();
        ctx.groups.save(number, &ids);
        return;
    }
    let ids = ctx.groups.recall(number, ctx.views);
    if ids.is_empty() {
        return;
    }
    let again = ctx.selection.units == ids
        && ctx.gesture.last_group.0 == number
        && ctx.now - ctx.gesture.last_group.1 < GROUP_AGAIN_S;
    ctx.gesture.last_group = (number, ctx.now);
    if again {
        let centre = ids
            .iter()
            .filter_map(|id| ctx.views.iter().find(|u| u.id == *id))
            .fold((Vec3::ZERO, 0.0), |(sum, n), u| (sum + u.pos, n + 1.0));
        if centre.1 > 0.0 {
            ctx.focus = Some(centre.0 / centre.1);
        }
    }
    ctx.select(ids);
}

fn mode_flag(unit: &sim_battle::Unit, mode: UnitMode) -> bool {
    match mode {
        UnitMode::Run => unit.mode_run,
        UnitMode::Guard => unit.guard,
        UnitMode::Skirmish => unit.skirmish,
        UnitMode::Melee => unit.melee_mode,
        UnitMode::Breach => unit.breach,
    }
}

/// Orders on the available selection (halt, fire at will, formation, pursuit, modes).
fn unit_order(ctx: &mut Ctx, action: Action) {
    let ids = ctx.available_selection();
    if ids.is_empty() {
        return;
    }
    let units: Vec<&sim_battle::Unit> = ctx
        .sim
        .units()
        .iter()
        .filter(|u| ids.contains(&u.id))
        .collect();
    let mut commands: Vec<Command> = Vec::new();
    let mut notice: Option<String> = None;
    match action {
        Action::Halt => commands.push(Command::Halt { units: ids.clone() }),
        Action::Withdraw => commands.push(Command::Withdraw { units: ids.clone() }),
        Action::FireAtWill => {
            let shooters: Vec<u32> = units
                .iter()
                .filter(|u| u.can_shoot())
                .map(|u| u.id)
                .collect();
            let enable = units.iter().any(|u| u.can_shoot() && !u.fire_at_will);
            if !shooters.is_empty() {
                commands.push(Command::FireAtWill {
                    units: shooters,
                    enabled: enable,
                });
            }
        }
        Action::CycleFormation => {
            for unit in &units {
                let cycle: Vec<Formation> =
                    Formation::all().filter(|f| f.def().allows(unit)).collect();
                if cycle.len() < 2 {
                    continue;
                }
                let at = cycle.iter().position(|f| *f == unit.formation).unwrap_or(0);
                commands.push(Command::Formation {
                    units: vec![unit.id],
                    kind: cycle[(at + 1) % cycle.len()],
                });
            }
        }
        Action::Pursue => {
            if let Some(player) = ctx.player {
                let groups = select::pursuit_groups(ctx.views, player, &ids);
                if groups.is_empty() {
                    notice = Some("Aucun fuyard ennemi à poursuivre.".to_owned());
                }
                for (target, chasers) in groups {
                    commands.push(Command::Pursue {
                        units: chasers,
                        target,
                        queue: false,
                    });
                }
            }
        }
        Action::Mode(mode) => {
            let able: Vec<&&sim_battle::Unit> = units
                .iter()
                .filter(|u| ctx.sim.available_modes(u).contains(&mode))
                .collect();
            if able.is_empty() {
                notice = Some(format!(
                    "Aucune unité sélectionnée ne peut passer en mode {}.",
                    hotkeys::label_fr(Action::Mode(mode))
                ));
            } else {
                let enable = able.iter().any(|u| !mode_flag(u, mode));
                commands.push(Command::SetMode {
                    units: able.iter().map(|u| u.id).collect(),
                    mode,
                    enabled: enable,
                });
            }
        }
        _ => {}
    }
    if let Some(text) = notice {
        ctx.toasts.push(text);
    }
    for command in commands {
        ctx.issue(command);
    }
}

/// Mods of the keyboard now (Cmd counts as Ctrl).
pub fn modifiers(keys: &ButtonInput<KeyCode>) -> Modifiers {
    Modifiers {
        shift: keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]),
        ctrl: keys.any_pressed([
            KeyCode::ControlLeft,
            KeyCode::ControlRight,
            KeyCode::SuperLeft,
            KeyCode::SuperRight,
        ]),
        alt: keys.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]),
    }
}

// ----- systems --------------------------------------------------------------------------------

/// Copies the regiments of the simulation each frame and drops from the selection and the locks
/// those that are gone.
pub fn refresh_views(
    mut session: ResMut<BattleSession>,
    mut views: ResMut<BattleViews>,
    mut selection: ResMut<BattleSelection>,
    mut groups: ResMut<BattleGroups>,
) {
    views.0 = picking::views(session.sim_mut());
    let present = |id: &u32| views.0.iter().any(|u| u.id == *id && u.present);
    if selection.units.iter().any(|id| !present(id)) {
        selection.units.retain(|id| present(id));
    }
    groups.prune_locks(&views.0);
}

/// True when the cursor is over a visible piece of interface (not a transparent container).
fn over_ui(
    hover: Option<&HoverMap>,
    nodes: &Query<(Option<&BackgroundColor>, Option<&ImageNode>), With<Node>>,
) -> bool {
    let Some(hover) = hover else { return false };
    hover.values().flat_map(|hits| hits.keys()).any(|entity| {
        nodes.get(*entity).is_ok_and(|(background, image)| {
            image.is_some() || background.is_some_and(|b| b.0.alpha() > 0.05)
        })
    })
}

/// Mouse buttons and motion to [`PointerEvent`]s.
#[allow(clippy::too_many_arguments)]
pub fn read_pointer(
    windows: Query<&Window, With<PrimaryWindow>>,
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    hover: Option<Res<HoverMap>>,
    nodes: Query<(Option<&BackgroundColor>, Option<&ImageNode>), With<Node>>,
    mut last: Local<Option<Vec2>>,
    mut events: MessageWriter<PointerEvent>,
) {
    let Ok(window) = windows.single() else { return };
    let Some(pos) = window.cursor_position() else {
        return;
    };
    let mods = modifiers(&keys);
    let blocked = over_ui(hover.as_deref(), &nodes);
    if *last != Some(pos) {
        *last = Some(pos);
        events.write(PointerEvent {
            kind: PointerKind::Move,
            pos,
            mods,
        });
    }
    let mut send = |kind: PointerKind| {
        events.write(PointerEvent { kind, pos, mods });
    };
    if buttons.just_pressed(MouseButton::Left) && !blocked {
        send(PointerKind::LeftDown);
    }
    if buttons.just_released(MouseButton::Left) {
        send(PointerKind::LeftUp);
    }
    if buttons.just_pressed(MouseButton::Right) && !blocked {
        send(PointerKind::RightDown);
    }
    if buttons.just_released(MouseButton::Right) {
        send(PointerKind::RightUp);
    }
}

/// Every message and key of the frame, applied in order.
#[allow(clippy::too_many_arguments)]
pub fn apply_inputs(
    time: Res<Time<Real>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut pointer_events: MessageReader<PointerEvent>,
    mut selects: MessageReader<super::SelectBattleUnits>,
    mut commands_in: MessageReader<IssueBattleCommand>,
    mut moves: MessageReader<MoveSelectionTo>,
    mut session: ResMut<BattleSession>,
    views: Res<BattleViews>,
    mut selection: ResMut<BattleSelection>,
    mut groups: ResMut<BattleGroups>,
    mut gesture: ResMut<Gesture>,
    mut log: ResMut<OrderLog>,
    cameras: Query<(&Camera, &GlobalTransform), With<BattleCamera>>,
    mut camera_state: Option<ResMut<BattleCameraState>>,
    mut toasts: MessageWriter<ToastRequest>,
) {
    let player = session.player;
    let camera = cameras
        .single()
        .ok()
        .map(|(camera, transform)| CameraView { camera, transform });
    let mut ctx = Ctx {
        sim: session.sim_mut(),
        views: &views.0,
        player,
        selection: &mut selection,
        groups: &mut groups,
        gesture: &mut gesture,
        log: &mut log,
        camera,
        now: time.elapsed_secs(),
        toasts: Vec::new(),
        focus: None,
    };
    for request in selects.read() {
        let ids: Vec<u32> = request
            .units
            .iter()
            .copied()
            .filter(|id| {
                ctx.views
                    .iter()
                    .any(|u| u.id == *id && u.present && Some(u.side) == player)
            })
            .collect();
        if request.additive {
            for id in ids {
                if !ctx.selection.units.contains(&id) {
                    ctx.selection.units.push(id);
                }
            }
        } else {
            ctx.select(ids);
        }
    }
    for event in pointer_events.read() {
        pointer(&mut ctx, *event);
    }
    for request in moves.read() {
        order_move_to(&mut ctx, request.point, request.queued, request.run);
    }
    for IssueBattleCommand(command) in commands_in.read() {
        ctx.issue(command.clone());
    }
    let mods = modifiers(&keys);
    for key in keys.get_just_pressed() {
        match key {
            KeyCode::Escape => {
                if !ctx.selection.units.is_empty() {
                    ctx.selection.units.clear();
                }
            }
            KeyCode::Enter | KeyCode::NumpadEnter => {
                if ctx.sim.is_deploying()
                    && let Err(error) = ctx.sim.start_battle()
                {
                    let text = ctx.sim.error_text(&error);
                    ctx.toasts.push(text);
                }
            }
            key => {
                if let Some(action) = hotkeys::action_for(*key, mods) {
                    perform(&mut ctx, action);
                }
            }
        }
    }
    let (texts, focus) = (std::mem::take(&mut ctx.toasts), ctx.focus);
    drop(ctx);
    for text in texts {
        toasts.write(ToastRequest::new(&text));
    }
    if let (Some(point), Some(state)) = (focus, camera_state.as_mut()) {
        state.target = Vec3::new(point.x, point.y, point.z);
    }
}

/// What the simulation holds for the regiments that matter (selected, or with an order): the
/// proof that an order really reached `sim-battle` (dump section `battle_orders`).
#[derive(Resource, Debug, Default)]
pub struct SimOrders(pub serde_json::Value);

/// Refreshes [`SimOrders`] from the simulation: destination, target, formation, modes.
pub fn snapshot_sim_orders(
    mut session: ResMut<BattleSession>,
    selection: Res<BattleSelection>,
    mut out: ResMut<SimOrders>,
) {
    let player = session.player;
    let sim = session.sim_mut();
    let rows: Vec<serde_json::Value> = sim
        .units()
        .iter()
        .filter(|u| {
            !u.synthetic
                && Some(u.side) == player
                && (selection.units.contains(&u.id)
                    || u.destination.is_some()
                    || u.target.is_some())
        })
        .take(24)
        .map(|u| {
            serde_json::json!({
                "id": u.id,
                "name": u.name.as_ref(),
                "x": u.x, "z": u.z,
                "destination": u.destination,
                "destination_facing": u.destination_facing,
                "target": u.target,
                "running": u.running,
                "formation": u.formation.key(),
                "fire_at_will": u.fire_at_will,
                "state": u.state.key(),
                "withdrawing": u.withdrawing,
            })
        })
        .collect();
    out.0 = serde_json::json!({ "tick": sim.ticks(), "units": rows });
}
