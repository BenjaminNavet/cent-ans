//! Victory / defeat / end-of-campaign screen (port of the end dialog of `victory_controller.gd`):
//! an illumination fills the screen with a vellum card at the bottom giving the outcome text,
//! the figures of the campaign and the score (tooltip-less: the breakdown is printed), then
//! « Continuer à jouer » (the campaign goes on) or « Menu principal ».
//!
//! The screen shows once per outcome: when `CampaignState::outcome` changes from what the
//! campaign had on entry (`Sim` changed after a turn, a battle result or a loaded save).
//! Widget names: `ending:continue`, `ending:menu`. Dev hook: `CENT_ANS_FORCE_OUTCOME=victory|defeat|ended`
//! sets the outcome after the first frame (to look at the screen without playing to 1453).

use bevy::prelude::*;
use game_api::progress::{EndView, end_view};
use serde_json::{Value, json};
use sim_campaign::victory::{Outcome, OutcomeKind};

use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::sim::Sim;
use crate::ui::kit::style::{self, UiTier, UiType};
use crate::ui::kit::widgets::{button_primary, caption, panel_illuminated};
use crate::ui::kit::{ButtonPressed, UiTheme, typed};

#[derive(Resource, Default)]
pub struct EndScreen {
    /// Outcome already announced (`ongoing`, `victory`, `defeat`, `ended`).
    announced: String,
    root: Option<Entity>,
    pub shown: u32,
    pub continued: u32,
    pub view: Option<EndView>,
}

pub fn plugin(app: &mut App) {
    app.init_resource::<EndScreen>()
        .register_dump(dump)
        .add_systems(OnEnter(AppState::Campaign), reset_on_enter)
        .add_systems(
            Update,
            (force_outcome, watch, handle_buttons)
                .chain()
                .run_if(in_state(AppState::Campaign)),
        )
        .add_systems(OnExit(AppState::Campaign), close_on_exit);
}

fn kind_of(sim: &Sim) -> String {
    match sim.state.as_ref().and_then(|s| s.outcome.as_ref()) {
        None => "ongoing",
        Some(o) => match o.kind {
            OutcomeKind::Victory => "victory",
            OutcomeKind::Defeat => "defeat",
            OutcomeKind::Ended => "ended",
        },
    }
    .to_owned()
}

fn reset_on_enter(mut screen: ResMut<EndScreen>, sim: Res<Sim>) {
    *screen = EndScreen {
        announced: kind_of(&sim),
        ..default()
    };
}

fn close_on_exit(mut commands: Commands, mut screen: ResMut<EndScreen>) {
    if let Some(root) = screen.root.take() {
        commands.entity(root).despawn();
    }
}

/// Dev hook: forces an outcome once (see the module doc).
fn force_outcome(mut sim: ResMut<Sim>, mut done: Local<bool>) {
    if *done {
        return;
    }
    let Ok(word) = std::env::var("CENT_ANS_FORCE_OUTCOME") else {
        *done = true;
        return;
    };
    let Some(state) = sim.state.as_mut() else {
        return;
    };
    *done = true;
    let kind = match word.as_str() {
        "defeat" => OutcomeKind::Defeat,
        "ended" => OutcomeKind::Ended,
        _ => OutcomeKind::Victory,
    };
    state.outcome = Some(Outcome {
        kind,
        turn: state.turn(),
        text_fr: "Résultat forcé pour examen (CENT_ANS_FORCE_OUTCOME).".to_owned(),
        score: 0,
    });
}

fn watch(
    mut commands: Commands,
    mut screen: ResMut<EndScreen>,
    sim: Res<Sim>,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
) {
    if !sim.is_changed() {
        return;
    }
    let kind = kind_of(&sim);
    if kind == screen.announced {
        return;
    }
    screen.announced = kind.clone();
    if kind == "ongoing" {
        return;
    }
    let (Some(data), Some(state)) = (&sim.data, &sim.state) else {
        return;
    };
    let Some(view) = end_view(data, state) else {
        return;
    };
    if let Some(root) = screen.root.take() {
        commands.entity(root).despawn();
    }
    screen.root = Some(spawn(&mut commands, &theme, &assets, &view));
    screen.shown += 1;
    screen.view = Some(view);
}

fn spawn(commands: &mut Commands, theme: &UiTheme, assets: &AssetServer, view: &EndView) -> Entity {
    let theme = theme.clone();
    commands
        .spawn((
            Name::new("CampaignEnding"),
            DespawnOnExit(AppState::Campaign),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(style::FE_NIGHT),
            UiTier::Modal.index(),
        ))
        .with_children(|root| {
            root.spawn((
                Name::new("EndingArt"),
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    ..default()
                },
                ImageNode::new(assets.load(view.art.clone()))
                    .with_color(Color::srgb(0.9, 0.87, 0.82)),
            ));
            root.spawn(Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(48.0),
                width: Val::Px(740.0),
                ..default()
            })
            .with_children(|slot| {
                slot.spawn(panel_illuminated(&theme)).with_children(|card| {
                    card.spawn(Node {
                        width: Val::Percent(100.0),
                        justify_content: JustifyContent::Center,
                        ..default()
                    })
                    .with_child((
                        Name::new("EndingTitle"),
                        Text::new(&view.title),
                        theme.title_font(40.0),
                        TextColor(style::RUBRIC),
                    ));
                    card.spawn(typed(
                        &theme,
                        &format!("{}\nScore final : {}.", view.text, view.score),
                        UiType::Body,
                    ));
                    card.spawn(caption(&theme, &view.lines.join("\n")));
                    card.spawn(Node {
                        column_gap: Val::Px(16.0),
                        justify_content: JustifyContent::Center,
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn(button_primary(
                            &theme,
                            "ending:continue",
                            "Continuer à jouer",
                        ));
                        row.spawn(button_primary(&theme, "ending:menu", "Menu principal"));
                    });
                });
            });
        })
        .id()
}

fn handle_buttons(
    mut pressed: MessageReader<ButtonPressed>,
    mut commands: Commands,
    mut screen: ResMut<EndScreen>,
    mut sim: ResMut<Sim>,
    mut next: MessageWriter<crate::ui::fader::FadeToState>,
) {
    for press in pressed.read() {
        match press.name.as_str() {
            "ending:continue" => {
                if let Some(root) = screen.root.take() {
                    commands.entity(root).despawn();
                }
                screen.continued += 1;
            }
            "ending:menu" => {
                if let Some(root) = screen.root.take() {
                    commands.entity(root).despawn();
                }
                sim.state = None;
                next.write(crate::ui::fader::FadeToState(AppState::MainMenu));
            }
            _ => {}
        }
    }
}

fn dump(world: &World) -> (String, Value) {
    let screen = world.resource::<EndScreen>();
    (
        "progress_end".to_owned(),
        json!({
            "announced": screen.announced,
            "visible": screen.root.is_some(),
            "shown": screen.shown,
            "continued": screen.continued,
            "view": screen.view.as_ref().map(|v| json!({
                "kind": v.kind, "title": v.title, "score": v.score, "turn": v.turn,
                "art": v.art, "lines": v.lines, "breakdown": v.breakdown,
            })),
        }),
    )
}
