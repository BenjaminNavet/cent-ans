//! Chronicle panel (lot chronicle, ADR 0357): pending decisions, journal by turn, sealed news
//! letters filtered by interest. Ports `game/scripts/ui/chronicle_window.gd` (decision window),
//! `journal_view.gd` (genre filter, « Monde » tab, line styles), `news_letters.gd` (letters),
//! `news_interest.gd` (the interest filter, now the core's `NewsRelevance`) and
//! `map/chronicle_controller.gd` (button counter, expired notices).
//!
//! Data: `game_api::chronicle` (views, tested in `core/`); the client keeps the history of
//! the turns it has seen in [`log::ChronicleLog`] (fed by `TurnAdvanced`), because the core only
//! keeps the last turn's events. The news filter is the setting `interface/news_filter`.
//!
//! Opens on `OpenPanel::Chronicle` or key F5 (toggle); closes on Escape, `chronicle:close`,
//! `OpenPanel::CloseAll`. Widget names (agent `click`): `chronicle:tab_decisions|journal|letters`,
//! `chronicle:genre:<id>`, `chronicle:world`, `chronicle:option:<decision>:<index>`,
//! `chronicle:letter:<index>` (unfold), `chronicle:dismiss:<index>`, `chronicle:close`.
//! Dev hook: `CENT_ANS_OPEN_CHRONICLE=decisions|journal|letters` opens the panel on entering the
//! campaign.
//!
//! Not ported: wax seals with heraldry (a coloured disc stands for them), Codex links in the
//! text, event miniatures (`assets/events/*.jpg`), the click-to-focus-camera of a letter, the
//! window queue through `ModalQueue` (the decisions are listed in one tab instead).

pub mod log;

use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use game_api::chronicle::{ChronicleView, NewsMode, genre_choices};
use serde_json::{Value, json};
use sim_campaign::Order;

use super::OpenPanel;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::hud::flow::TurnAdvanced;
use crate::menu::KeyEdges;
use crate::settings::Settings;
use crate::sim::Sim;
use crate::ui::kit::fa::FaUi;
use crate::ui::kit::modal::ToastRequest;
use crate::ui::kit::panel_stack::displaces;
use crate::ui::kit::richtext::{RichStyle, rich_text};
use crate::ui::kit::style::{self, UiTier, UiType};
use crate::ui::kit::text_field::{TextInputFocus, keyboard_free};
use crate::ui::kit::widgets::{
    button_small, caption, panel_sized, scroll_list, separator, toggle_button,
};
use crate::ui::kit::{ButtonPressed, UiTheme, typed};
use log::ChronicleLog;

const PANEL_WIDTH: f32 = 720.0;
const LIST_HEIGHT: f32 = 360.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Decisions,
    Journal,
    Letters,
}

impl Tab {
    fn name(self) -> &'static str {
        match self {
            Tab::Decisions => "decisions",
            Tab::Journal => "journal",
            Tab::Letters => "letters",
        }
    }
}

pub struct PanelChroniclePlugin;

impl Plugin for PanelChroniclePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChroniclePanel>()
            .init_resource::<ChronicleLog>()
            .add_systems(OnEnter(AppState::Campaign), reset_on_enter)
            .add_systems(
                Update,
                (
                    open_close.before(crate::menu::pause::pause_escape),
                    record_turns,
                    watch_state,
                    handle_buttons,
                    rebuild,
                )
                    .chain()
                    .run_if(in_state(AppState::Campaign)),
            )
            .add_systems(OnExit(AppState::Campaign), close_on_exit)
            .register_dump(dump_chronicle);
    }
}

/// State of the screen, kept across rebuilds.
#[derive(Resource, Default)]
pub struct ChroniclePanel {
    pub open: bool,
    root: Option<Entity>,
    tab: Tab,
    genre: String,
    world_open: bool,
    /// Index of the unfolded letter.
    letter_open: Option<usize>,
    dirty: bool,
    /// Turn and pending-decision count the screen was built for.
    built_for: (u32, usize),
    last_order: String,
}

#[derive(Component)]
struct ChronicleRoot;

fn reset_on_enter(mut log: ResMut<ChronicleLog>, mut panel: ResMut<ChroniclePanel>) {
    *log = ChronicleLog::default();
    panel.genre = "all".into();
    panel.world_open = false;
    panel.letter_open = None;
    panel.open = false;
    if let Ok(tab) = std::env::var("CENT_ANS_OPEN_CHRONICLE") {
        panel.tab = match tab.as_str() {
            "journal" => Tab::Journal,
            "letters" => Tab::Letters,
            _ => Tab::Decisions,
        };
        panel.open = true;
        panel.dirty = true;
    }
}

fn close_on_exit(mut commands: Commands, mut panel: ResMut<ChroniclePanel>) {
    if let Some(root) = panel.root.take() {
        commands.entity(root).despawn();
    }
    panel.open = false;
}

fn open_close(
    mut panel: ResMut<ChroniclePanel>,
    mut messages: MessageReader<OpenPanel>,
    keys: Res<ButtonInput<KeyCode>>,
    keymap: Option<Res<crate::settings::KeyMap>>,
    focus: Option<Res<TextInputFocus>>,
    mut edges: ResMut<KeyEdges>,
    mut held: Local<bool>,
) {
    for message in messages.read() {
        match message {
            OpenPanel::Chronicle => {
                panel.open = true;
                panel.dirty = true;
            }
            other if displaces(other, "chronicle") => panel.open = false,
            _ => {}
        }
    }
    // Edge on `pressed` (scripted keys, see tech); a focused text field swallows the key.
    let f5 = keyboard_free(focus)
        && crate::campaign::hud::keys::action_down(
            keymap.as_deref(),
            "map_toggle_chronicle",
            KeyCode::F5,
            &keys,
            false,
        );
    let f5_edge = f5 && !*held;
    *held = f5;
    if f5_edge {
        panel.open = !panel.open;
        panel.dirty = panel.open;
    }
    if panel.open && edges.escape {
        panel.open = false;
        edges.escape = false;
    }
}

/// Records every resolved turn into the history and raises the notices of expired decisions.
fn record_turns(
    mut advanced: MessageReader<TurnAdvanced>,
    sim: Res<Sim>,
    settings: Option<Res<Settings>>,
    mut log: ResMut<ChronicleLog>,
    mut panel: ResMut<ChroniclePanel>,
    mut toasts: MessageWriter<ToastRequest>,
) {
    let mode = settings
        .as_ref()
        .map_or(NewsMode::Interest, |s| NewsMode::parse(&s.news_filter));
    for turn in advanced.read() {
        let (Some(data), Some(state)) = (&sim.data, &sim.state) else {
            continue;
        };
        let journal = game_api::turn_journal(data, state, &sim.player, &turn.events, mode);
        log.record(&journal);
        for notice in &journal.expired {
            toasts.write(ToastRequest::new(notice));
        }
        panel.letter_open = None;
        panel.dirty = true;
    }
}

/// Rebuilds when the turn or the number of pending decisions changed under an open panel.
fn watch_state(mut panel: ResMut<ChroniclePanel>, sim: Res<Sim>) {
    if !panel.open {
        return;
    }
    if let (Some(data), Some(state)) = (&sim.data, &sim.state) {
        let count = state
            .chronicle
            .pending_decisions
            .iter()
            .filter(|d| d.faction == sim.player)
            .count();
        let _ = data;
        if (state.turn, count) != panel.built_for {
            panel.dirty = true;
        }
    }
}

fn handle_buttons(
    mut pressed: MessageReader<ButtonPressed>,
    mut panel: ResMut<ChroniclePanel>,
    mut log: ResMut<ChronicleLog>,
    mut sim: ResMut<Sim>,
    mut toasts: MessageWriter<ToastRequest>,
) {
    for press in pressed.read() {
        let Some(name) = press.name.strip_prefix("chronicle:") else {
            continue;
        };
        if !panel.open {
            continue;
        }
        match name {
            "close" => panel.open = false,
            "tab_decisions" => panel.tab = Tab::Decisions,
            "tab_journal" => panel.tab = Tab::Journal,
            "tab_letters" => panel.tab = Tab::Letters,
            "world" => panel.world_open = !panel.world_open,
            _ => {
                if let Some(genre) = name.strip_prefix("genre:") {
                    panel.genre = genre.to_owned();
                } else if let Some(index) = name.strip_prefix("letter:") {
                    let index = index.parse::<usize>().ok();
                    panel.letter_open = if panel.letter_open == index {
                        None
                    } else {
                        index
                    };
                } else if let Some(index) = name.strip_prefix("dismiss:") {
                    if let Ok(index) = index.parse::<usize>() {
                        log.dismiss(index);
                        panel.letter_open = None;
                    }
                } else if let Some(rest) = name.strip_prefix("option:") {
                    let mut parts = rest.split(':');
                    let (Some(decision), Some(option)) = (
                        parts.next().and_then(|p| p.parse::<u32>().ok()),
                        parts.next().and_then(|p| p.parse::<usize>().ok()),
                    ) else {
                        continue;
                    };
                    let Some(data) = sim.data.clone() else {
                        continue;
                    };
                    let Some(state) = sim.state.as_mut() else {
                        continue;
                    };
                    match state
                        .submit_order_outcome(&data, Order::ChooseEventOption { decision, option })
                    {
                        Ok(_) => {
                            panel.last_order = format!("ok:{decision}:{option}");
                            toasts.write(ToastRequest::new("Décision prise."));
                        }
                        Err(error) => {
                            panel.last_order = format!("error:{error}");
                            toasts.write(ToastRequest::new(&error.to_string()));
                        }
                    }
                }
            }
        }
        panel.dirty = true;
    }
}

/// Spawns or despawns the screen to match `open` / `dirty`.
fn rebuild(
    mut commands: Commands,
    mut panel: ResMut<ChroniclePanel>,
    sim: Res<Sim>,
    log: Res<ChronicleLog>,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
    settings: Option<Res<Settings>>,
) {
    if !panel.open {
        if let Some(root) = panel.root.take() {
            commands.entity(root).despawn();
        }
        return;
    }
    if !panel.dirty && panel.root.is_some() {
        return;
    }
    panel.dirty = false;
    let (Some(data), Some(state)) = (&sim.data, &sim.state) else {
        return;
    };
    if let Some(root) = panel.root.take() {
        commands.entity(root).despawn();
    }
    let view = game_api::chronicle_view(data, state, &sim.player);
    panel.built_for = (state.turn, view.count);
    let mode = settings
        .as_ref()
        .map_or(NewsMode::Interest, |s| NewsMode::parse(&s.news_filter));
    let root = spawn_screen(&mut commands, &theme, &assets, &view, &log, &panel, mode);
    panel.root = Some(root);
}

fn spawn_screen(
    commands: &mut Commands,
    theme: &UiTheme,
    assets: &AssetServer,
    view: &ChronicleView,
    log: &ChronicleLog,
    panel: &ChroniclePanel,
    mode: NewsMode,
) -> Entity {
    let theme = theme.clone();
    let body = RichStyle::body(&theme);
    commands
        .spawn((
            Name::new("ChroniclePanel"),
            ChronicleRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)),
            UiTier::Panel.index(),
        ))
        .with_children(|root| {
            root.spawn(panel_sized(&theme, PANEL_WIDTH))
                .with_children(|page| {
                    page.spawn(Node {
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|bar| {
                        bar.spawn((
                            Name::new("ChronicleTitle"),
                            Text::new("✠ Chronique"),
                            theme.title_font(UiType::Title.size()),
                            TextColor(style::INK_TITLE),
                        ));
                        bar.spawn(button_small(&theme, "chronicle:close", "Fermer"));
                    });
                    page.spawn(Node {
                        column_gap: Val::Px(6.0),
                        ..default()
                    })
                    .with_children(|tabs| {
                        let decisions = format!("Décisions ({})", view.count);
                        let letters = format!("Lettres ({})", log.letters.len());
                        for (tab, name, label) in [
                            (
                                Tab::Decisions,
                                "chronicle:tab_decisions",
                                decisions.as_str(),
                            ),
                            (Tab::Journal, "chronicle:tab_journal", "Journal"),
                            (Tab::Letters, "chronicle:tab_letters", letters.as_str()),
                        ] {
                            tabs.spawn(toggle_button(&theme, name, label, panel.tab == tab));
                        }
                    });
                    page.spawn(separator(&theme));
                    match panel.tab {
                        Tab::Decisions => decisions_tab(page, &theme, assets, &body, view),
                        Tab::Journal => journal_tab(page, &theme, assets, &body, log, panel),
                        Tab::Letters => letters_tab(page, &theme, log, panel, mode),
                    }
                });
        })
        .id()
}

/// Height of the chronicle seal (`ChronicleWindow.SEAL_HEIGHT`).
const SEAL_HEIGHT: f32 = 34.0;

fn decisions_tab(
    page: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    assets: &AssetServer,
    body: &RichStyle,
    view: &ChronicleView,
) {
    page.spawn(caption(theme, &view.tooltip));
    page.spawn(scroll_list(theme, "chronicle:decisions", LIST_HEIGHT))
        .with_children(|list| {
            for decision in &view.decisions {
                list.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    margin: UiRect::bottom(Val::Px(10.0)),
                    align_items: AlignItems::FlexStart,
                    ..default()
                })
                .with_children(|card| {
                    let kind = if decision.historical {
                        "✠ Chronique du temps"
                    } else {
                        "✠ Nouvelles du royaume"
                    };
                    // Real wax seal before the rubric of the historical decisions
                    // (`chronicle_window.gd`: `FaUi.seal_rect("chronicle", SEAL_HEIGHT)`).
                    card.spawn(Node {
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(6.0),
                        ..default()
                    })
                    .with_children(|head| {
                        if decision.historical
                            && let Some(seal) =
                                FaUi::global().seal(assets, "chronicle", SEAL_HEIGHT)
                        {
                            head.spawn(seal);
                        }
                        head.spawn((
                            Text::new(kind),
                            theme.text_font(false, false, UiType::Caption.size()),
                            TextColor(style::RUBRIC),
                        ));
                    });
                    card.spawn(typed(theme, &decision.title, UiType::Heading));
                    let mut meta = Vec::new();
                    if !decision.province_name.is_empty() {
                        meta.push(decision.province_name.clone());
                    }
                    meta.push(if decision.expires_in <= 1 {
                        "à décider ce tour-ci".to_owned()
                    } else {
                        format!("à décider sous {} tours", decision.expires_in)
                    });
                    card.spawn(caption(theme, &meta.join(" — ")));
                    card.spawn(rich_text(
                        theme,
                        assets,
                        &format!("[i]{}[/i]", decision.text),
                        body,
                    ));
                    for option in &decision.options {
                        let name = format!("chronicle:option:{}:{}", decision.id, option.index);
                        let mut button = card.spawn(button_small(theme, &name, &option.text));
                        if !option.allowed {
                            button.insert(InteractionDisabled);
                        }
                        let effects = if option.allowed {
                            option.effects_text.replace('\n', " · ")
                        } else if option.reason.is_empty() {
                            "Impossible.".to_owned()
                        } else {
                            format!("Impossible : {}", option.reason)
                        };
                        if !effects.is_empty() {
                            card.spawn(caption(theme, &format!("   {effects}")));
                        }
                    }
                });
            }
            if view.decisions.is_empty() {
                list.spawn(caption(theme, "Rien à décider pour l’instant."));
            }
        });
}

fn journal_tab(
    page: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    assets: &AssetServer,
    body: &RichStyle,
    log: &ChronicleLog,
    panel: &ChroniclePanel,
) {
    page.spawn(Node {
        flex_wrap: FlexWrap::Wrap,
        column_gap: Val::Px(4.0),
        row_gap: Val::Px(4.0),
        ..default()
    })
    .with_children(|bar| {
        for choice in genre_choices() {
            let name = format!("chronicle:genre:{}", choice.id);
            bar.spawn(toggle_button(
                theme,
                &name,
                &choice.label,
                panel.genre == choice.id,
            ));
        }
    });
    page.spawn(scroll_list(theme, "chronicle:journal", LIST_HEIGHT))
        .with_children(|list| {
            let shown = log.shown_lines(&panel.genre);
            if log.lines.is_empty() {
                list.spawn(caption(theme, "Aucun événement pour l’instant."));
            } else if shown.is_empty() {
                list.spawn(caption(theme, "Rien de ce genre."));
            }
            for (_, text) in shown {
                list.spawn(rich_text(theme, assets, &text, body));
            }
            if log.world_count() > 0 {
                let label = format!(
                    "{} Monde ({})",
                    if panel.world_open { "▾" } else { "▸" },
                    log.world_count()
                );
                list.spawn(toggle_button(
                    theme,
                    "chronicle:world",
                    &label,
                    panel.world_open,
                ));
                if panel.world_open {
                    for (_, text) in log.shown_world(&panel.genre) {
                        list.spawn(rich_text(theme, assets, &text, &RichStyle::caption(theme)));
                    }
                }
            }
        });
}

fn letters_tab(
    page: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    log: &ChronicleLog,
    panel: &ChroniclePanel,
    mode: NewsMode,
) {
    page.spawn(caption(
        theme,
        &format!(
            "Nouvelles reçues : {} (réglage « Nouvelles reçues »).",
            mode.label()
        ),
    ));
    page.spawn(scroll_list(theme, "chronicle:letters", LIST_HEIGHT))
        .with_children(|list| {
            if log.letters.is_empty() {
                list.spawn(caption(theme, "Aucune lettre."));
            }
            for (index, letter) in log.letters.iter().enumerate() {
                let open = panel.letter_open == Some(index);
                list.spawn(Node {
                    column_gap: Val::Px(8.0),
                    align_items: AlignItems::Center,
                    margin: UiRect::bottom(Val::Px(4.0)),
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        Name::new(format!("Seal {index}")),
                        Node {
                            width: Val::Px(30.0),
                            height: Val::Px(30.0),
                            flex_shrink: 0.0,
                            border_radius: BorderRadius::MAX,
                            ..default()
                        },
                        BackgroundColor(wax_color(&letter.wax)),
                    ));
                    row.spawn(Node {
                        flex_direction: FlexDirection::Column,
                        flex_grow: 1.0,
                        flex_basis: Val::Px(0.0),
                        align_items: AlignItems::FlexStart,
                        ..default()
                    })
                    .with_children(|column| {
                        column.spawn((
                            Text::new(letter.rubric.to_uppercase()),
                            theme.text_font(false, false, UiType::Caption.size()),
                            TextColor(style::RUBRIC),
                        ));
                        let name = format!("chronicle:letter:{index}");
                        column.spawn(button_small(theme, &name, &letter.title));
                        if open {
                            column.spawn(caption(theme, &letter.text));
                            column.spawn(caption(theme, &format!("— {}", letter.interest)));
                        }
                    });
                    let name = format!("chronicle:dismiss:{index}");
                    row.spawn(button_small(theme, &name, "Écarter"));
                });
            }
        });
}

/// Wax colours of `HudStyle` (`WAX`, `WAX_GREEN`, `WAX_LIGHT`) and the dark brown of a death.
fn wax_color(family: &str) -> Color {
    match family {
        "green" => Color::srgb(0.24, 0.34, 0.18),
        "dark" => Color::srgb(0.25, 0.14, 0.09),
        "light" => Color::srgb(0.70, 0.22, 0.14),
        _ => Color::srgb(0.55, 0.11, 0.08),
    }
}

fn dump_chronicle(world: &World) -> (String, Value) {
    let panel = world.resource::<ChroniclePanel>();
    let log = world.resource::<ChronicleLog>();
    let sim = world.resource::<Sim>();
    let view = match (&sim.data, &sim.state) {
        (Some(data), Some(state)) => Some(game_api::chronicle_view(data, state, &sim.player)),
        _ => None,
    };
    let mode = world
        .get_resource::<Settings>()
        .map_or("interest", |s| NewsMode::parse(&s.news_filter).as_str());
    (
        "panel_chronicle".to_owned(),
        json!({
            "open": panel.open,
            "built": panel.root.is_some(),
            "tab": panel.tab.name(),
            "genre": panel.genre,
            "news_mode": mode,
            "pending_decisions": view.as_ref().map_or(0, |v| v.count),
            "decision_titles": view.as_ref().map(|v| v.decisions.iter().map(|d| d.title.clone()).collect::<Vec<_>>()),
            "turns_recorded": log.turns,
            "journal_lines": log.line_count(),
            "world_lines": log.world_count(),
            "letters": log.letters.len(),
            "letter_rubrics": log.letters.iter().map(|l| l.rubric.clone()).collect::<Vec<_>>(),
            "last_expired": log.last_expired,
            "last_order": panel.last_order,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::kit::text_field::TextInputFocus;

    /// F5 toggles the chronicle, but not while a text field has the keyboard.
    #[test]
    fn f5_ignored_while_typing() {
        let mut app = App::new();
        app.init_resource::<ChroniclePanel>()
            .init_resource::<KeyEdges>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<TextInputFocus>()
            .add_message::<OpenPanel>()
            .add_systems(Update, open_close);
        let field = app.world_mut().spawn_empty().id();
        app.world_mut().resource_mut::<TextInputFocus>().0 = Some(field);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F5);
        app.update();
        assert!(!app.world().resource::<ChroniclePanel>().open);
        // Focus dropped and the key released then pressed again: it opens.
        app.world_mut().resource_mut::<TextInputFocus>().0 = None;
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::F5);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F5);
        app.update();
        assert!(app.world().resource::<ChroniclePanel>().open);
    }

    #[test]
    fn wax_families_differ() {
        assert_ne!(wax_color("red"), wax_color("green"));
        assert_ne!(wax_color("dark"), wax_color("light"));
    }

    #[test]
    fn tab_names_are_stable() {
        assert_eq!(Tab::default().name(), "decisions");
        assert_eq!(Tab::Letters.name(), "letters");
    }
}
