//! Battle tutorial (« Didacticiel de bataille », port of game/scripts/battle/battle_prologue.gd):
//! a small skirmish of summer 1337 built as a custom battle, guided in steps read from
//! `data/tutorial/battle_prologue.json`.
//!
//! - [`data`]: the file, the pass conditions and their pure evaluation (tested without a scene).
//! - Here: the run ([`PrologueRun`]), the parchment card (same look as the campaign guide:
//!   advisor text, objective, « Continuer » / « Passer l’étape » / « Passer le tutoriel »,
//!   table of contents), the gold frame on the interface element or regiment of the step, the
//!   passive enemy (`set_ai` off + `set_hold`, until the `enemy_ai` step) and the defeat card
//!   (« Recommencer » / « Fermer »).
//!
//! **Start**: the menu message [`StartBattlePrologue`] (« Oui » of the invitation, or the entry
//! « Didacticiel de bataille » of the Batailles menu) builds the launch (label `tutorial:...`,
//! not held by the invitation) and the run. **End**: finishing the guide without a defeat calls
//! [`PrologueInvite::mark_done`]. The rules stay in the core: this module only reads the battle
//! (`BattleSession`, `BattleSelection`, `BattleCameraState`) and flips the enemy AI / hold.
//!
//! Dev hook: `CENT_ANS_PROLOGUE_DEMO=1` plays every step through the real order messages (see
//! `demo`) so that a dump shows the whole traversal; use it with `CENT_ANS_NO_DEPLOY=1`.

pub mod data;
mod demo;

use bevy::prelude::*;
use serde_json::{Value, json};
use sim_battle::SideId;

use self::data::{Baseline, CameraObs, Condition, Observed, Prologue, UnitObs, evaluate};
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::battle::orders::BattleSelection;
use crate::battle::orders::input::BattleViews;
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::{BattleCamera, BattleCameraState};
use crate::campaign::progress::target::{Shape, pulse};
use crate::flow::LeaveBattle;
use crate::menu::battle_prologue_invite::{PrologueInvite, StartBattlePrologue};
use crate::menu::battles::{LaunchBattle, data_dir};
use crate::menu::saves::SaveDir;
use crate::ui::kit::modal::ReduceMotion;
use crate::ui::kit::richtext::{RichStyle, rich_text};
use crate::ui::kit::style::{self, UiTier, UiType};
use crate::ui::kit::widgets::{button_primary, button_small, caption, panel_sized, separator};
use crate::ui::kit::{ButtonPressed, UiTheme, typed};

/// Label of the launch (the invitation does not hold it).
pub const LAUNCH_LABEL: &str = "tutorial:battle_prologue";
const DATA_FILE: &str = "tutorial/battle_prologue.json";
const CARD_WIDTH: f32 = 380.0;
/// Seconds between two checks of the objective.
const CHECK_INTERVAL: f32 = 0.25;
/// Seconds a met objective stays on screen before the next step.
const DONE_PAUSE: f32 = 0.8;
const GOLD: Color = Color::srgb(0.95, 0.78, 0.25);
const SHADOW: Color = Color::srgba(0.1, 0.05, 0.0, 0.7);
const POINT_RADIUS: f32 = 34.0;
/// Z order of the card: above the battle HUD.
const CARD_Z: GlobalZIndex = GlobalZIndex(5);

/// The tutorial being played (exists from the launch until the battle is left).
#[derive(Resource)]
pub struct PrologueRun {
    pub data: Prologue,
    pub step: usize,
    baseline: Baseline,
    /// Guide finished or skipped (the card is gone, the enemy fights).
    pub done: bool,
    pub defeated: bool,
    pub enemy_ai: bool,
    /// Passive enemy installed (needs the session).
    passive_set: bool,
    check_timer: f32,
    done_timer: Option<f32>,
    pub toc_open: bool,
    /// Steps passed by their objective, in order.
    pub completed: Vec<String>,
    pub skipped: u32,
    /// `PrologueInvite::mark_done` was called.
    pub marked_done: bool,
    root: Option<Entity>,
    built_for: Option<(usize, bool, bool, bool)>,
    /// Shape of the highlighted element (dump).
    pub shape: Option<Shape>,
    pub target_source: String,
}

impl PrologueRun {
    fn new(data: Prologue) -> Self {
        Self {
            data,
            step: 0,
            baseline: Baseline::default(),
            done: false,
            defeated: false,
            enemy_ai: false,
            passive_set: false,
            check_timer: 0.0,
            done_timer: None,
            toc_open: false,
            completed: Vec::new(),
            skipped: 0,
            marked_done: false,
            root: None,
            built_for: None,
            shape: None,
            target_source: String::new(),
        }
    }

    fn current(&self) -> &data::Step {
        &self.data.steps[self.step.min(self.data.steps.len() - 1)]
    }
}

/// Set when « Recommencer » leaves the battle: the main menu starts the tutorial again.
#[derive(Resource)]
struct RestartPrologue;

#[derive(Component)]
struct PrologueHalo;
#[derive(Component)]
struct PrologueArrow;

pub struct BattleTutorialPlugin;

impl Plugin for BattleTutorialPlugin {
    fn build(&self, app: &mut App) {
        app.register_dump(dump)
            .init_resource::<demo::PrologueDemo>()
            .add_systems(
                Update,
                (entry_button, start_prologue, restart_if_asked)
                    .chain()
                    .run_if(in_state(AppState::MainMenu)),
            )
            .add_systems(OnEnter(AppState::Battle), spawn_halo)
            .add_systems(OnExit(AppState::Battle), close_run)
            .add_systems(
                Update,
                (
                    install_passive_enemy,
                    handle_buttons,
                    demo::run_demo,
                    check_step,
                    rebuild_card,
                    update_halo,
                )
                    .chain()
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<PrologueRun>)
                    .run_if(resource_exists::<BattleSession>),
            );
    }
}

/// `data/tutorial/battle_prologue.json`, read and checked.
pub fn load() -> Result<Prologue, String> {
    let dir = data_dir().ok_or("data/ not found")?;
    let text = std::fs::read_to_string(dir.join(DATA_FILE)).map_err(|e| e.to_string())?;
    Prologue::parse(&text)
}

/// Is the tutorial data present (the menu entry shows)?
pub fn available() -> bool {
    data_dir().is_some_and(|dir| dir.join(DATA_FILE).is_file())
}

/// Name of the entry of the Batailles menu.
pub const MENU_ENTRY: &str = "Didacticiel de bataille";

/// The Batailles menu entry asks for the tutorial (same message as « Oui » of the invitation).
fn entry_button(
    mut pressed: MessageReader<ButtonPressed>,
    mut starts: MessageWriter<StartBattlePrologue>,
) {
    if pressed.read().any(|p| p.name == MENU_ENTRY) {
        starts.write(StartBattlePrologue);
    }
}

/// Menu: the invitation said yes, or the entry of the Batailles menu was chosen.
fn start_prologue(
    mut commands: Commands,
    mut starts: MessageReader<StartBattlePrologue>,
    mut launches: MessageWriter<LaunchBattle>,
    mut invite: ResMut<PrologueInvite>,
    dir: Option<Res<SaveDir>>,
    mut toasts: MessageWriter<crate::ui::kit::modal::ToastRequest>,
) {
    if starts.read().last().is_none() {
        return;
    }
    let prologue = match load() {
        Ok(p) => p,
        Err(error) => {
            warn!("battle tutorial: {error}");
            toasts.write(crate::ui::kit::modal::ToastRequest::new(
                "Le didacticiel de bataille est introuvable.",
            ));
            return;
        }
    };
    if let Some(dir) = dir {
        invite.ensure_path(&dir.0);
    }
    let Ok(config) = prologue.custom_battle() else {
        return;
    };
    // The passive enemy is installed by the run itself (AI off + hold, until `enemy_ai`).
    let launch = LaunchBattle::new(config, prologue.seed(), LAUNCH_LABEL);
    info!("battle tutorial: launch");
    commands.insert_resource(PrologueRun::new(prologue));
    launches.write(launch);
}

/// Back in the menu after « Recommencer ».
fn restart_if_asked(
    mut commands: Commands,
    marker: Option<Res<RestartPrologue>>,
    mut starts: MessageWriter<StartBattlePrologue>,
) {
    if marker.is_some() {
        commands.remove_resource::<RestartPrologue>();
        starts.write(StartBattlePrologue);
    }
}

fn close_run(mut commands: Commands, run: Option<Res<PrologueRun>>) {
    if run.is_some() {
        commands.remove_resource::<PrologueRun>();
    }
}

/// Enemy AI off and camp held until the step marked `enemy_ai` (NT11 `set_hold`).
fn set_enemy_ai(session: &mut BattleSession, enabled: bool) {
    let Some(side) = session.player.map(SideId::other) else {
        return;
    };
    let sim = session.sim_mut();
    sim.set_ai(side, enabled);
    sim.set_hold(side, !enabled);
}

fn install_passive_enemy(mut run: ResMut<PrologueRun>, mut session: ResMut<BattleSession>) {
    if run.passive_set || session.player.is_none() {
        return;
    }
    run.passive_set = true;
    if run.data.enemy_passive {
        set_enemy_ai(&mut session, false);
    }
    let observed = observe(&mut session, None, &[]);
    run.baseline = Baseline::of(&observed);
}

/// The battle as the conditions read it.
fn observe(
    session: &mut BattleSession,
    camera: Option<&BattleCameraState>,
    selected: &[u32],
) -> Observed {
    let player = session.player;
    let paused = session.paused;
    let sim = session.sim_mut();
    let units = sim
        .units()
        .iter()
        .filter(|u| !u.synthetic)
        .map(|u| {
            let (width, _) = u.extent();
            UnitObs {
                id: u.id,
                own: Some(u.side) == player,
                present: u.present(),
                category: u.category.key().to_owned(),
                state: u.state.key().to_owned(),
                x: u.x as f32,
                z: u.z as f32,
                width: width as f32,
                formation: u.formation.key().to_owned(),
            }
        })
        .collect();
    Observed {
        units,
        selected: selected.to_vec(),
        paused,
        camera: camera.map(|c| CameraObs {
            target: c.target,
            distance: c.distance,
            yaw: c.yaw,
        }),
        finished: sim.is_finished(),
        player_won: sim.winner().is_some() && sim.winner() == player,
    }
}

impl PrologueRun {
    fn go_to(&mut self, index: usize, observed: &Observed, session: &mut BattleSession) {
        if self.defeated || index >= self.data.steps.len() {
            return;
        }
        self.step = index;
        self.done_timer = None;
        self.check_timer = 0.0;
        self.toc_open = false;
        self.baseline = Baseline::of(observed);
        if self.data.steps[index].enemy_ai {
            self.enemy_ai = true;
            set_enemy_ai(session, true);
        }
    }

    /// Next step, or the end of the guide after the last one.
    fn advance(
        &mut self,
        observed: &Observed,
        session: &mut BattleSession,
        invite: &mut PrologueInvite,
    ) {
        if self.step + 1 >= self.data.steps.len() {
            self.finish(session, invite);
        } else {
            let next = self.step + 1;
            self.go_to(next, observed, session);
        }
    }

    /// End of the guide (last step or « Passer le tutoriel »): the enemy plays again; outside a
    /// defeat the tutorial counts as done.
    fn finish(&mut self, session: &mut BattleSession, invite: &mut PrologueInvite) {
        if self.done {
            return;
        }
        self.done = true;
        self.enemy_ai = true;
        set_enemy_ai(session, true);
        if !self.defeated {
            invite.mark_done();
            self.marked_done = true;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_buttons(
    mut pressed: MessageReader<ButtonPressed>,
    mut run: ResMut<PrologueRun>,
    mut session: ResMut<BattleSession>,
    mut invite: ResMut<PrologueInvite>,
    camera: Option<Res<BattleCameraState>>,
    selection: Res<BattleSelection>,
    mut leave: MessageWriter<LeaveBattle>,
    mut commands: Commands,
) {
    for press in pressed.read() {
        let Some(name) = press.name.strip_prefix("prologue:") else {
            continue;
        };
        if run.done {
            continue;
        }
        if run.defeated {
            match name {
                "restart" => {
                    run.done = true;
                    commands.insert_resource(RestartPrologue);
                    leave.write(LeaveBattle { abandoned: true });
                }
                "next" => run.done = true,
                _ => {}
            }
            continue;
        }
        let observed = observe(&mut session, camera.as_deref(), &selection.units);
        if name == "toc" {
            run.toc_open = !run.toc_open;
        } else if let Some(index) = name.strip_prefix("step:").and_then(|i| i.parse().ok()) {
            run.go_to(index, &observed, &mut session);
        } else if name == "next" || name == "skip" {
            if name == "skip" {
                run.skipped += 1;
            }
            run.advance(&observed, &mut session, &mut invite);
        } else if name == "quit" {
            run.finish(&mut session, &mut invite);
        }
    }
}

fn check_step(
    mut run: ResMut<PrologueRun>,
    mut session: ResMut<BattleSession>,
    mut invite: ResMut<PrologueInvite>,
    camera: Option<Res<BattleCameraState>>,
    selection: Res<BattleSelection>,
    time: Res<Time<Real>>,
) {
    if run.done || run.defeated || !run.passive_set {
        return;
    }
    if let Some(left) = run.done_timer {
        let left = left - time.delta_secs();
        if left <= 0.0 {
            run.done_timer = None;
            let observed = observe(&mut session, camera.as_deref(), &selection.units);
            run.advance(&observed, &mut session, &mut invite);
        } else {
            run.done_timer = Some(left);
        }
        return;
    }
    run.check_timer -= time.delta_secs();
    if run.check_timer > 0.0 {
        return;
    }
    run.check_timer = CHECK_INTERVAL;
    let observed = observe(&mut session, camera.as_deref(), &selection.units);
    // The battle ended before the victory step (the enemy broke earlier): go there, or show the
    // farewell of the defeat.
    if observed.finished {
        if !observed.player_won {
            run.defeated = true;
            return;
        }
        if let Some(victory) = run.data.index_of("victory")
            && victory > run.step
        {
            run.go_to(victory, &observed, &mut session);
        }
    }
    let condition = run.current().condition.clone();
    if condition != Condition::Manual && evaluate(&condition, &observed, &run.baseline) {
        let id = run.current().id.clone();
        run.completed.push(id);
        run.done_timer = Some(DONE_PAUSE);
    }
}

// ---- card ------------------------------------------------------------------------------------

fn rebuild_card(
    mut commands: Commands,
    mut run: ResMut<PrologueRun>,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
) {
    let key = (!run.done).then_some((
        run.step,
        run.done_timer.is_some(),
        run.toc_open,
        run.defeated,
    ));
    if key == run.built_for && run.root.is_some() == key.is_some() {
        return;
    }
    if let Some(root) = run.root.take() {
        commands.entity(root).despawn();
    }
    run.built_for = key;
    if key.is_none() {
        return;
    }
    let root = spawn_card(&mut commands, &theme, &assets, &run);
    run.root = Some(root);
}

fn spawn_card(
    commands: &mut Commands,
    theme: &UiTheme,
    assets: &AssetServer,
    run: &PrologueRun,
) -> Entity {
    let theme = theme.clone();
    let body = RichStyle::body(&theme);
    let step = run.current();
    let total = run.data.steps.len();
    let done = run.done_timer.is_some();
    let advisor = &run.data.advisor;
    let (title, text, objective) = if run.defeated {
        (
            run.data.defeat.title.clone(),
            run.data.defeat.text.clone(),
            String::new(),
        )
    } else {
        let text = if advisor.is_empty() {
            step.text.clone()
        } else {
            format!("[i]{advisor} :[/i] {}", step.text)
        };
        (step.title.clone(), text, step.objective.clone())
    };
    let manual = step.manual();
    let last = run.step + 1 >= total;
    let titles: Vec<String> = if run.toc_open && !run.defeated {
        run.data.steps.iter().map(|s| s.title.clone()).collect()
    } else {
        Vec::new()
    };
    let (index, defeated) = (run.step, run.defeated);
    commands
        .spawn((
            Name::new("PrologueCard"),
            DespawnOnExit(AppState::Battle),
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
            crate::ui::kit::layout::FitZone::Left,
            CARD_Z,
        ))
        .with_children(|slot| {
            slot.spawn(panel_sized(&theme, CARD_WIDTH))
                .with_children(|page| {
                    if !defeated {
                        page.spawn(Node {
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|header| {
                            header.spawn(caption(
                                &theme,
                                &format!("Didacticiel — étape {} sur {}", index + 1, total),
                            ));
                            header.spawn(button_small(&theme, "prologue:toc", "☰ Étapes"));
                        });
                    }
                    if !titles.is_empty() {
                        page.spawn(Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::FlexStart,
                            ..default()
                        })
                        .with_children(|list| {
                            for (i, title) in titles.iter().enumerate() {
                                let mark = match i.cmp(&index) {
                                    std::cmp::Ordering::Less => "✔",
                                    std::cmp::Ordering::Equal => "▸",
                                    std::cmp::Ordering::Greater => " ",
                                };
                                list.spawn(button_small(
                                    &theme,
                                    &format!("prologue:step:{i}"),
                                    &format!("{mark} {}. {title}", i + 1),
                                ));
                            }
                        });
                        page.spawn(separator(&theme));
                    }
                    page.spawn((
                        Name::new("PrologueTitle"),
                        typed(&theme, &title, UiType::Heading),
                    ));
                    page.spawn(rich_text(&theme, assets, &text, &body));
                    if !objective.is_empty() {
                        page.spawn(separator(&theme));
                        page.spawn((
                            Name::new("PrologueObjective"),
                            Text::new(if done {
                                format!("✔ {objective}")
                            } else {
                                format!("Objectif : {objective}")
                            }),
                            theme.text_font(true, false, UiType::Body.size()),
                            TextColor(if done { style::GOOD } else { style::RUBRIC }),
                        ));
                    }
                    page.spawn(Node {
                        column_gap: Val::Px(6.0),
                        flex_wrap: FlexWrap::Wrap,
                        row_gap: Val::Px(4.0),
                        ..default()
                    })
                    .with_children(|row| {
                        if defeated {
                            row.spawn(button_primary(&theme, "prologue:restart", "Recommencer"));
                            row.spawn(button_small(&theme, "prologue:next", "Fermer"));
                        } else if manual {
                            let label = if last { "Terminer" } else { "Continuer" };
                            row.spawn(button_primary(&theme, "prologue:next", label));
                            row.spawn(button_small(&theme, "prologue:quit", "Passer le tutoriel"));
                        } else {
                            let mut skip =
                                row.spawn(button_small(&theme, "prologue:skip", "Passer l’étape"));
                            if done {
                                skip.insert(bevy::ui::InteractionDisabled);
                            }
                            row.spawn(button_small(&theme, "prologue:quit", "Passer le tutoriel"));
                        }
                    });
                });
        })
        .id()
}

// ---- gold frame --------------------------------------------------------------------------------

fn spawn_halo(mut commands: Commands, run: Option<Res<PrologueRun>>) {
    if run.is_none() {
        return;
    }
    commands
        .spawn((
            Name::new("PrologueHalo"),
            PrologueHalo,
            DespawnOnExit(AppState::Battle),
            Node {
                position_type: PositionType::Absolute,
                border: UiRect::all(Val::Px(3.0)),
                ..default()
            },
            BorderColor::all(GOLD),
            BackgroundColor(Color::NONE),
            Visibility::Hidden,
            Pickable::IGNORE,
            UiTier::Tutorial.index(),
        ))
        .with_children(|halo| {
            halo.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(-6.0),
                    top: Val::Px(-6.0),
                    right: Val::Px(-6.0),
                    bottom: Val::Px(-6.0),
                    border: UiRect::all(Val::Px(3.0)),
                    ..default()
                },
                BorderColor::all(SHADOW),
                Pickable::IGNORE,
            ));
        });
    commands.spawn((
        Name::new("PrologueArrow"),
        PrologueArrow,
        DespawnOnExit(AppState::Battle),
        Node {
            position_type: PositionType::Absolute,
            ..default()
        },
        Text::new("▼"),
        TextFont {
            font_size: bevy::text::FontSize::Px(30.0),
            ..default()
        },
        TextColor(GOLD),
        Visibility::Hidden,
        Pickable::IGNORE,
        UiTier::Tutorial.index(),
    ));
}

type UiRects<'w, 's> = Query<
    'w,
    's,
    (
        &'static Name,
        &'static bevy::ui::ComputedNode,
        &'static bevy::ui::UiGlobalTransform,
        &'static InheritedVisibility,
    ),
>;

/// Layout-pixel rectangle of the visible widget called `wanted`.
fn widget_rect(ui: &UiRects, wanted: &str) -> Option<Shape> {
    let mut union: Option<(Vec2, Vec2)> = None;
    for (name, node, transform, visibility) in ui {
        if name.as_str() != wanted || !visibility.get() {
            continue;
        }
        let center = transform.translation * node.inverse_scale_factor;
        let half = node.size() * node.inverse_scale_factor * 0.5;
        if half.x < 1.0 || half.y < 1.0 {
            continue;
        }
        let (min, max) = (center - half, center + half);
        union = Some(union.map_or((min, max), |(a, b)| (a.min(min), b.max(max))));
    }
    union.map(|(min, max)| Shape::Rect { min, max })
}

/// Named widgets that carry each interface target.
fn widget_for(target: &str) -> Option<&'static str> {
    match target {
        "cards" => Some("Cards"),
        "pause" => Some("speed:pause"),
        "minimap" => Some("MapSlot"),
        _ => None,
    }
}

/// Screen point (layout pixels) above the regiment of `target` (`enemy`, `unit:<category>`).
fn unit_point(
    target: &str,
    player: Option<SideId>,
    views: &BattleViews,
    camera: (&Camera, &GlobalTransform),
    scale: f32,
) -> Option<Shape> {
    let wanted = |view: &crate::battle::orders::picking::UnitView| -> bool {
        if !view.present {
            return false;
        }
        match target {
            "enemy" => Some(view.side) != player,
            "unit:cavalry" => Some(view.side) == player && view.cavalry,
            "unit:ranged" => Some(view.side) == player && view.can_shoot,
            _ => false,
        }
    };
    let view = views.0.iter().find(|v| wanted(v))?;
    let point = camera
        .0
        .world_to_viewport(camera.1, view.pos + Vec3::Y * 2.0)
        .ok()?;
    let size = camera.0.logical_viewport_size()?;
    (point.x >= 0.0 && point.y >= 0.0 && point.x <= size.x && point.y <= size.y)
        .then(|| Shape::Point(point / scale.max(0.1)))
}

type HaloQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Node,
        &'static mut Visibility,
        &'static mut BorderColor,
    ),
    (With<PrologueHalo>, Without<PrologueArrow>),
>;
type ArrowQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Node,
        &'static mut Visibility,
        &'static mut TextColor,
        &'static mut Text,
    ),
    (With<PrologueArrow>, Without<PrologueHalo>),
>;

#[allow(clippy::too_many_arguments)]
fn update_halo(
    mut run: ResMut<PrologueRun>,
    session: Res<BattleSession>,
    views: Res<BattleViews>,
    cameras: Query<(&Camera, &GlobalTransform), With<BattleCamera>>,
    ui: UiRects,
    scale: Res<UiScale>,
    time: Res<Time<Real>>,
    reduce: Res<ReduceMotion>,
    mut halo: HaloQuery,
    mut arrow: ArrowQuery,
) {
    let target = run.current().target.clone();
    let shown = !run.done && !run.defeated && run.done_timer.is_none() && !target.is_empty();
    let mut resolved: Option<(Shape, String)> = None;
    if shown {
        if let Some(name) = widget_for(&target) {
            resolved = widget_rect(&ui, name).map(|s| (s, format!("ui:{name}")));
        } else if let Some(camera) = cameras.iter().next() {
            resolved = unit_point(&target, session.player, &views, camera, scale.0)
                .map(|s| (s, "world".to_owned()));
        }
    }
    match resolved {
        Some((shape, source)) => {
            run.shape = Some(shape);
            run.target_source = source;
        }
        None => {
            run.shape = None;
            run.target_source.clear();
        }
    }
    let strength = pulse(time.elapsed_secs(), reduce.0);
    let Some(shape) = run.shape else {
        for (_, mut visibility, _) in &mut halo {
            visibility.set_if_neq(Visibility::Hidden);
        }
        for (_, mut visibility, _, _) in &mut arrow {
            visibility.set_if_neq(Visibility::Hidden);
        }
        return;
    };
    let grow = 4.0 + 2.0 * strength;
    let (min, max) = match shape {
        Shape::Rect { min, max } => (min - Vec2::splat(grow), max + Vec2::splat(grow)),
        Shape::Point(p) => (
            p - Vec2::splat(POINT_RADIUS + grow),
            p + Vec2::splat(POINT_RADIUS + grow),
        ),
    };
    for (mut node, mut visibility, mut border) in &mut halo {
        node.left = Val::Px(min.x);
        node.top = Val::Px(min.y);
        node.width = Val::Px(max.x - min.x);
        node.height = Val::Px(max.y - min.y);
        node.border_radius = match shape {
            Shape::Point(_) => BorderRadius::MAX,
            Shape::Rect { .. } => BorderRadius::ZERO,
        };
        *border = BorderColor::all(GOLD.with_alpha(0.65 + 0.35 * strength));
        visibility.set_if_neq(Visibility::Inherited);
    }
    let bob = if reduce.0 { 0.0 } else { 6.0 * strength };
    let arrow_size = 30.0;
    let center = (min + max) * 0.5;
    for (mut node, mut visibility, mut color, mut glyph) in &mut arrow {
        let above = min.y - arrow_size - 4.0 + bob;
        let (y, up) = if above < 2.0 {
            (max.y + 4.0 - bob, true)
        } else {
            (above, false)
        };
        node.left = Val::Px(center.x - arrow_size * 0.5);
        node.top = Val::Px(y);
        let wanted = if up { "▲" } else { "▼" };
        if glyph.0 != wanted {
            glyph.0 = wanted.to_owned();
        }
        color.0 = GOLD.with_alpha(0.8 + 0.2 * strength);
        visibility.set_if_neq(Visibility::Inherited);
    }
}

fn dump(world: &World) -> (String, Value) {
    let Some(run) = world.get_resource::<PrologueRun>() else {
        return ("battle_prologue".to_owned(), json!({ "active": false }));
    };
    let invite = world.get_resource::<PrologueInvite>();
    let step = run.current();
    (
        "battle_prologue".to_owned(),
        json!({
            "active": true,
            "steps": run.data.steps.len(),
            "step": run.step,
            "step_id": step.id,
            "title": step.title,
            "target": step.target,
            "highlight": run.shape.map(|s| match s {
                Shape::Rect { min, max } => json!({"rect": [min.x.round(), min.y.round(), max.x.round(), max.y.round()]}),
                Shape::Point(p) => json!({"point": [p.x.round(), p.y.round()]}),
            }),
            "highlight_source": run.target_source,
            "card_visible": run.root.is_some(),
            "objective_met": run.done_timer.is_some(),
            "completed": run.completed,
            "skipped": run.skipped,
            "done": run.done,
            "defeated": run.defeated,
            "enemy_ai": run.enemy_ai,
            "passive_installed": run.passive_set,
            "marked_done": run.marked_done,
            "invite_done": invite.map(|i| i.flags.done),
            "demo": demo::report(world),
        }),
    )
}
