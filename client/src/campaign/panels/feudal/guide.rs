//! Three-step feudal guide (`FeudalController.TUTORIAL_STEPS` and its `tutorial_*` methods).
//!
//! Starts after the end of a turn once the general tutorial is not running, never twice
//! (`Settings::feudal_tutorial_done`, Godot `feudal_tutorial/done`). Same card as the general
//! guide (« Continuer » / « Passer l’étape » / « Plus tard » / « Passer le tutoriel »). Steps:
//! open the feudal tree (objective met when the panel is open), the map (manual step), the
//! war escalation box (manual step).
//!
//! Differences from Godot: there is no « Féodalité » map filter in the Bevy client yet
//! (`feudal_map_lens.gd` not ported), so the second step is manual and its text no longer
//! sends the player to a filter; the pulsing target frame of the general guide is not drawn.
//! Dev hook: `CENT_ANS_FEUDAL_GUIDE=1` starts the guide at the first frame (`off` disables it).
//! Widget names: `feudal_guide:next`, `:skip`, `:later`, `:quit`.

use bevy::prelude::*;
use serde_json::json;

use super::FeudalUi;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::hud::flow::TurnAdvanced;
use crate::campaign::progress::tutorial::Tutorial;
use crate::menu::pause::GamePaused;
use crate::settings::{Settings, SettingsFile, SettingsScreen};
use crate::ui::kit::richtext::{RichStyle, rich_text};
use crate::ui::kit::style::{self, UiType};
use crate::ui::kit::widgets::{button_primary, button_small, caption, panel_sized, separator};
use crate::ui::kit::{ButtonPressed, UiTheme, typed};

const CARD_WIDTH: f32 = 380.0;
const CHECK_INTERVAL: f32 = 0.25;
const GUIDE_Z: GlobalZIndex = GlobalZIndex(5);
const ENV_GUIDE: &str = "CENT_ANS_FEUDAL_GUIDE";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Objective {
    /// Met when the feudal tree is open.
    TreeOpen,
    /// Read-only step: « Continuer ».
    Manual,
}

pub struct Step {
    pub id: &'static str,
    pub title: &'static str,
    pub text: &'static str,
    pub objective: &'static str,
    pub kind: Objective,
}

pub const STEPS: [Step; 3] = [
    Step {
        id: "feudal_tree",
        title: "Votre place dans la féodalité",
        text: "Au-dessus des factions, des titres : royaumes, duchés, comtés. Cliquez sur votre écu, en haut à gauche : la section « Féodalité » du panneau de faction donne vos obligations et vos objectifs, et son bouton [b]« Arbre féodal… »[/b] (ou Menu → Arbre féodal) montre votre suzerain, vos vassaux et leur loyauté. Un vassal mécontent peut refuser l’ost, se révolter ou prêter hommage ailleurs.",
        objective: "Ouvrir l’arbre féodal.",
        kind: Objective::TreeOpen,
    },
    Step {
        id: "feudal_lands",
        title: "Les fiefs et leurs titres",
        text: "Dans la fiche d’une province, le fil d’Ariane donne ses titres, du royaume au comté ; l’arbre féodal liste les terres tenues sous chaque titre, et l’écu parti marque les doubles allégeances, comme la Guyenne anglaise tenue du roi de France.",
        objective: "",
        kind: Objective::Manual,
    },
    Step {
        id: "feudal_war",
        title: "Avant de déclarer la guerre",
        text: "Attaquer un vassal appelle son suzerain, puis le suzerain de celui-ci. Avant toute déclaration, le cadre [b]« Qui peut entrer en guerre »[/b] estime chaque maillon (probable, incertain, improbable). Un vassal félon peut être frappé de [b]commise[/b] : vous reprenez ses fiefs si vous gagnez. Voir le Codex, « Vassalité ».",
        objective: "",
        kind: Objective::Manual,
    },
];

#[derive(Resource, Default)]
pub struct FeudalGuide {
    /// Current step; `None` while the guide is not running.
    pub index: Option<usize>,
    pub done: bool,
    pub postponed: bool,
    synced: bool,
    check_timer: f32,
    root: Option<Entity>,
    built_for: Option<(usize, bool)>,
    /// Steps completed by their objective (dump).
    pub completed: Vec<String>,
}

impl FeudalGuide {
    pub fn active(&self) -> bool {
        self.index.is_some()
    }

    fn start(&mut self) {
        self.index = Some(0);
        self.postponed = false;
        self.check_timer = 0.0;
    }

    fn advance(&mut self) {
        match self.index {
            Some(i) if i + 1 < STEPS.len() => self.index = Some(i + 1),
            Some(_) => self.finish(),
            None => {}
        }
    }

    fn finish(&mut self) {
        self.index = None;
        self.done = true;
    }

    /// Whether the guide may start by itself (`maybe_start_tutorial`).
    pub fn may_start(&self, general_active: bool) -> bool {
        !self.active() && !self.done && !self.postponed && !general_active
    }
}

/// Whether the objective of step `index` is met.
pub fn objective_met(index: usize, tree_open: bool) -> bool {
    matches!(STEPS[index].kind, Objective::TreeOpen) && tree_open
}

#[derive(Component)]
struct GuideCard;

pub fn plugin(app: &mut App) {
    app.init_resource::<FeudalGuide>()
        .register_dump(dump)
        .add_systems(OnEnter(AppState::Campaign), reset_on_enter)
        .add_systems(
            Update,
            (autostart, handle_buttons, check_objective, rebuild, persist)
                .chain()
                .run_if(in_state(AppState::Campaign)),
        )
        .add_systems(OnExit(AppState::Campaign), close_on_exit);
}

fn reset_on_enter(mut guide: ResMut<FeudalGuide>, mut commands: Commands) {
    if let Some(root) = guide.root.take() {
        commands.entity(root).despawn();
    }
    guide.built_for = None;
}

fn close_on_exit(mut guide: ResMut<FeudalGuide>, mut commands: Commands) {
    if let Some(root) = guide.root.take() {
        commands.entity(root).despawn();
    }
    guide.built_for = None;
}

/// Reads `feudal_tutorial/done` once, then starts the guide after a turn (or by the dev hook).
fn autostart(
    mut guide: ResMut<FeudalGuide>,
    mut advanced: MessageReader<TurnAdvanced>,
    settings: Option<Res<Settings>>,
    tutorial: Res<Tutorial>,
    mut hook_checked: Local<bool>,
) {
    if !guide.synced {
        guide.synced = true;
        guide.done = settings.as_ref().is_some_and(|s| s.feudal_tutorial_done);
    }
    let hook = std::env::var(ENV_GUIDE).ok();
    if hook.as_deref() == Some("off") {
        advanced.clear();
        return;
    }
    if !*hook_checked {
        *hook_checked = true;
        if hook.as_deref().is_some_and(|v| !v.is_empty() && v != "0") {
            guide.start();
            return;
        }
    }
    let turned = advanced.read().count() > 0;
    if turned && guide.may_start(tutorial.active()) {
        guide.start();
    }
}

fn handle_buttons(mut pressed: MessageReader<ButtonPressed>, mut guide: ResMut<FeudalGuide>) {
    for press in pressed.read() {
        let Some(name) = press.name.strip_prefix("feudal_guide:") else {
            continue;
        };
        match name {
            "next" | "skip" => guide.advance(),
            "later" => {
                guide.index = None;
                guide.postponed = true;
            }
            "quit" => guide.finish(),
            _ => {}
        }
    }
}

/// Moves on when the objective of the current step is met (checked 4 times a second).
fn check_objective(mut guide: ResMut<FeudalGuide>, ui: Res<FeudalUi>, time: Res<Time<Real>>) {
    let Some(index) = guide.index else { return };
    guide.check_timer -= time.delta_secs();
    if guide.check_timer > 0.0 {
        return;
    }
    guide.check_timer = CHECK_INTERVAL;
    if objective_met(index, ui.open) {
        guide.completed.push(STEPS[index].id.to_owned());
        guide.advance();
    }
}

fn rebuild(
    mut commands: Commands,
    mut guide: ResMut<FeudalGuide>,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
    paused: Option<Res<GamePaused>>,
    settings_screen: Option<Res<SettingsScreen>>,
) {
    let covered = paused.is_some_and(|p| p.0) || settings_screen.is_some_and(|s| s.root.is_some());
    let want = guide.index.filter(|_| !covered);
    let key = want.map(|i| (i, false));
    if key == guide.built_for && guide.root.is_some() == key.is_some() {
        return;
    }
    if let Some(root) = guide.root.take() {
        commands.entity(root).despawn();
    }
    guide.built_for = key;
    let Some(index) = want else { return };
    guide.root = Some(spawn(&mut commands, &theme, &assets, index));
}

fn spawn(commands: &mut Commands, theme: &UiTheme, assets: &AssetServer, index: usize) -> Entity {
    let step = &STEPS[index];
    let theme = theme.clone();
    let body = RichStyle::body(&theme);
    commands
        .spawn((
            Name::new("FeudalGuideCard"),
            GuideCard,
            DespawnOnExit(AppState::Campaign),
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
            crate::ui::kit::layout::FitZone::Left,
            GUIDE_Z,
        ))
        .with_children(|slot| {
            slot.spawn(panel_sized(&theme, CARD_WIDTH))
                .with_children(|page| {
                    page.spawn(caption(
                        &theme,
                        &format!(
                            "Guide de la féodalité — étape {} sur {}",
                            index + 1,
                            STEPS.len()
                        ),
                    ));
                    page.spawn((
                        Name::new("FeudalGuideTitle"),
                        typed(&theme, step.title, UiType::Heading),
                    ));
                    page.spawn(rich_text(&theme, assets, step.text, &body));
                    if !step.objective.is_empty() {
                        page.spawn(separator(&theme));
                        page.spawn((
                            Name::new("FeudalGuideObjective"),
                            Text::new(format!("Objectif : {}", step.objective)),
                            theme.text_font(true, false, UiType::Body.size()),
                            TextColor(style::RUBRIC),
                        ));
                    }
                    page.spawn(Node {
                        column_gap: Val::Px(6.0),
                        flex_wrap: FlexWrap::Wrap,
                        row_gap: Val::Px(4.0),
                        ..default()
                    })
                    .with_children(|row| {
                        if step.kind == Objective::Manual {
                            let label = if index + 1 >= STEPS.len() {
                                "Terminer"
                            } else {
                                "Continuer"
                            };
                            row.spawn(button_primary(&theme, "feudal_guide:next", label));
                        } else {
                            row.spawn(button_small(&theme, "feudal_guide:skip", "Passer l’étape"));
                        }
                        row.spawn(button_small(&theme, "feudal_guide:later", "Plus tard"));
                        row.spawn(button_small(
                            &theme,
                            "feudal_guide:quit",
                            "Passer le tutoriel",
                        ));
                    });
                });
        })
        .id()
}

/// Writes `feudal_tutorial/done` to the settings once the guide is finished.
fn persist(
    guide: Res<FeudalGuide>,
    settings: Option<ResMut<Settings>>,
    file: Option<ResMut<SettingsFile>>,
) {
    if !guide.done || std::env::var(ENV_GUIDE).is_ok() {
        return;
    }
    let (Some(mut settings), Some(mut file)) = (settings, file) else {
        return;
    };
    if settings.feudal_tutorial_done {
        return;
    }
    let inner = settings.bypass_change_detection();
    inner.feudal_tutorial_done = true;
    file.save(inner);
}

fn dump(world: &World) -> (String, serde_json::Value) {
    let guide = world.resource::<FeudalGuide>();
    (
        "feudal_guide".into(),
        json!({
            "active": guide.active(),
            "step": guide.index,
            "step_id": guide.index.map(|i| STEPS[i].id),
            "done": guide.done,
            "postponed": guide.postponed,
            "card": guide.root.is_some(),
            "completed": guide.completed,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guide_runs_three_steps_then_finishes() {
        let mut guide = FeudalGuide::default();
        assert!(guide.may_start(false));
        assert!(!guide.may_start(true));
        guide.start();
        assert_eq!(guide.index, Some(0));
        guide.advance();
        guide.advance();
        assert_eq!(guide.index, Some(2));
        guide.advance();
        assert!(guide.done && !guide.active());
        assert!(!guide.may_start(false));
    }

    #[test]
    fn first_step_objective_is_the_open_tree() {
        assert!(!objective_met(0, false));
        assert!(objective_met(0, true));
        assert!(!objective_met(1, true));
    }
}
