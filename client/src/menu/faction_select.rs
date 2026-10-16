//! Faction selection (game/scripts/ui/faction_select.gd): [`AppState::FactionSelect`].
//!
//! Tabs « Départs recommandés » (cards), « Défis singuliers » (cards) and « Toutes les factions »
//! (list of every playable faction, by name); on the right the pitch of the chosen faction
//! (introduction, forces, faiblesses) and the four difficulty levels (`game_api::difficulty_levels`);
//! « Retour » / « Commencer ». « Commencer » sets `Sim::player`, builds the 1337 campaign
//! (`Sim::new_campaign`), applies the difficulty to the new state and enters the campaign.
//! Not ported: the clickable map and kingdom / rank filters (`FactionMapPicker`), ruler portrait
//! and key figures on the cards, the seed field, the loading screen.
//!
//! Tutorial invite: « Commencer » first asks « Jouer le didacticiel ? » (when the guide is enabled
//! in the settings and not finished). « Oui » starts the campaign and sends
//! [`StartTutorial`] on entering it; « Non » (or Escape) puts the guide aside (`postponed`), so it
//! does not start by itself and waits for the Aide menu / advisor.

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;
use bevy::ui_widgets::Button as WidgetButton;
use data_model::FactionId;
use serde_json::{Value, json};
use sim_campaign::difficulty::Difficulty;

use super::faction_map::{self, FactionMapState, StartSheets};
use super::saves::SaveProgress;
use super::{CoverFit, MenuData, spawn_backdrop};
use crate::app::AppState;
use crate::campaign::progress::StartTutorial;
use crate::campaign::progress::tutorial::Tutorial;
use crate::settings::Settings;
use crate::sim::Sim;
use crate::ui::fader::FadeToState;
use crate::ui::kit::fa::FaPlate;
use crate::ui::kit::modal::ToastRequest;
use crate::ui::kit::modal::{ModalQueue, ModalResult, ModalSpec, PRIORITY_DECISION};
use crate::ui::kit::style::{self, UiType};
use crate::ui::kit::{
    ButtonPressed, ListSelected, TabSelected, Toggled, UiTheme, button_primary, button_small,
    heading, panel_page, scroll_list, tab_bar, tab_pane, toggle_button, typed,
};

/// The faction and difficulty currently chosen on the screen.
#[derive(Resource, Debug, Clone)]
pub struct FactionChoice {
    pub faction: String,
    pub difficulty: String,
}

impl Default for FactionChoice {
    fn default() -> Self {
        Self {
            faction: crate::sim::DEFAULT_FACTION.to_owned(),
            difficulty: "normal".to_owned(),
        }
    }
}

/// Key of the tutorial invitation in the modal queue.
pub const INVITE_KEY: &str = "menu:tutorial_invite";

/// State of the tutorial invitation of a new campaign.
#[derive(Resource, Default, Debug)]
pub struct TutorialInvite {
    /// The question is on screen; the campaign starts with the answer.
    pub asking: bool,
    /// « Oui » was answered: the campaign being entered sends [`StartTutorial`].
    pub start_on_enter: bool,
    /// Answers given so far (dump).
    pub answers: Vec<bool>,
}

/// Whether « Commencer » must ask about the guide first.
pub fn invite_wanted(settings: Option<&Settings>, tutorial: &Tutorial) -> bool {
    settings.is_none_or(|s| s.tutorial_enabled) && !tutorial.finished && !tutorial.active()
}

#[derive(Component)]
struct FactionCard(String);

#[derive(Component)]
struct DetailBox;

#[derive(Component)]
struct DifficultyButton(String);

#[derive(Component)]
struct DifficultyText;

#[derive(Component)]
struct StartCaption;

/// The main menu cartouche must never show behind the faction screen.
fn clear_main_menu(
    mut commands: Commands,
    roots: Query<Entity, With<super::main_menu::MainMenuRoot>>,
) {
    for root in &roots {
        commands.entity(root).despawn();
    }
}

const CARD_WIDTH: f32 = 220.0;
const ART_HEIGHT: f32 = 112.0;

pub struct FactionSelectPlugin;

impl Plugin for FactionSelectPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FactionChoice>()
            .init_resource::<TutorialInvite>()
            .add_systems(
                OnEnter(AppState::FactionSelect),
                (clear_main_menu, faction_map::prepare, spawn_screen).chain(),
            )
            .add_systems(OnEnter(AppState::Campaign), send_start_tutorial)
            .add_systems(
                Update,
                (
                    handle_buttons,
                    answer_invite,
                    handle_list,
                    refresh_choice,
                    card_visuals,
                )
                    .chain()
                    .run_if(in_state(AppState::FactionSelect)),
            )
            .add_systems(
                Update,
                enter_starts.run_if(in_state(AppState::FactionSelect)),
            )
            .add_systems(
                Update,
                (
                    faction_map::map_pointer,
                    faction_map::map_buttons,
                    faction_map::refresh_list,
                    faction_map::repaint,
                    faction_map::hover_card,
                )
                    .chain()
                    .run_if(in_state(AppState::FactionSelect)),
            );
    }
}

fn faction_label(sim: &Sim, id: &str) -> String {
    match (&sim.data, FactionId::new(id)) {
        (Some(data), Ok(id)) => data.faction_name(&id),
        _ => id.to_owned(),
    }
}

/// Playable factions sorted by display name.
fn playable(sim: &Sim) -> Vec<(String, String)> {
    let Some(data) = &sim.data else {
        return Vec::new();
    };
    let mut list: Vec<(String, String)> = data
        .factions
        .values()
        .filter(|faction| faction.playable)
        .map(|faction| {
            (
                faction.id.as_str().to_owned(),
                data.faction_name(&faction.id),
            )
        })
        .collect();
    list.sort_by_key(|entry| entry.1.to_lowercase());
    list
}

fn primary_color(sim: &Sim, id: &str) -> Color {
    sim.data
        .as_ref()
        .zip(FactionId::new(id).ok())
        .and_then(|(data, id)| data.factions.get(&id))
        .and_then(|f| Srgba::hex(f.heraldry.primary_color.as_str()).ok())
        .map_or(Color::srgb(0.4, 0.4, 0.5), Color::from)
}

#[allow(clippy::too_many_arguments)]
fn spawn_screen(
    mut commands: Commands,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
    menu: Res<MenuData>,
    sim: Res<Sim>,
    mut choice: ResMut<FactionChoice>,
    map_state: Res<FactionMapState>,
    start_sheets: Res<StartSheets>,
) {
    let front = &menu.front;
    let all = playable(&sim);
    // Opening the screen selects the player's faction when it is playable.
    if choice.faction.is_empty() || !all.iter().any(|(id, _)| *id == choice.faction) {
        choice.faction = front
            .recommended()
            .first()
            .cloned()
            .unwrap_or_else(|| crate::sim::DEFAULT_FACTION.to_owned());
    }
    if let Some(data) = &sim.data
        && !game_api::difficulty_levels(data)
            .iter()
            .any(|level| level.id == choice.difficulty)
        && let Some(default) = game_api::difficulty_levels(data)
            .into_iter()
            .find(|level| level.is_default)
    {
        choice.difficulty = default.id;
    }

    let root = commands
        .spawn((
            Name::new("FactionSelect"),
            DespawnOnExit(AppState::FactionSelect),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
        ))
        .id();
    spawn_backdrop(&mut commands, &assets, root);
    let column = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect {
                    left: Val::Px(24.0),
                    right: Val::Px(24.0),
                    top: Val::Px(20.0),
                    bottom: Val::Px(18.0),
                },
                row_gap: Val::Px(12.0),
                ..default()
            },
            ChildOf(root),
        ))
        .id();
    // Header: title and start date.
    let header = commands
        .spawn((
            Node {
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            },
            ChildOf(column),
        ))
        .id();
    commands.spawn((
        Text::new("Choisissez votre couronne"),
        theme.title_font(30.0),
        TextColor(Color::srgb(0.97, 0.92, 0.80)),
        ChildOf(header),
    ));
    commands.spawn((
        Name::new("StartDate"),
        Text::new(front.start_date_label()),
        theme.text_font(false, true, UiType::Heading.size()),
        TextColor(style::FE_GOLD),
        ChildOf(header),
    ));
    // Body.
    let body = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                column_gap: Val::Px(18.0),
                ..default()
            },
            ChildOf(column),
        ))
        .id();
    let left = framed(&mut commands, &theme, body, None, 1.0);
    let special = front.special();
    let mut tabs: Vec<(&str, &str)> = vec![("tab:recommended", "Départs recommandés")];
    if !special.is_empty() {
        tabs.push(("tab:special", "Défis singuliers"));
    }
    tabs.push(("tab:all", "Toutes les factions"));
    commands.spawn((tab_bar(&theme, "start", &tabs, 0), ChildOf(left)));

    let pane = commands
        .spawn((tab_pane("start", "tab:recommended", true), ChildOf(left)))
        .id();
    spawn_cards(
        &mut commands,
        &theme,
        &assets,
        &sim,
        front,
        &start_sheets,
        &front.recommended(),
        pane,
    );
    if !special.is_empty() {
        let pane = commands
            .spawn((tab_pane("start", "tab:special", false), ChildOf(left)))
            .id();
        if !front.special_starts.text.is_empty() {
            commands.spawn((
                typed(&theme, &front.special_starts.text, UiType::Body),
                ChildOf(pane),
            ));
        }
        spawn_cards(
            &mut commands,
            &theme,
            &assets,
            &sim,
            front,
            &start_sheets,
            &special,
            pane,
        );
    }
    let pane = commands
        .spawn((tab_pane("start", "tab:all", false), ChildOf(left)))
        .id();
    faction_map::spawn_tab(&mut commands, &theme, &map_state, &start_sheets, pane, root);

    // Right column: pitch and difficulty.
    let side = commands
        .spawn((
            Node {
                width: Val::Px(420.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                ..default()
            },
            ChildOf(body),
        ))
        .id();
    let detail = framed(&mut commands, &theme, side, Some(420.0), 1.0);
    commands
        .entity(detail)
        .insert((Name::new("Detail"), DetailBox));
    commands
        .entity(detail)
        .entry::<Node>()
        .and_modify(|mut n| n.overflow = Overflow::clip());
    let difficulty = framed(&mut commands, &theme, side, Some(420.0), 0.0);
    commands.spawn((
        heading(&theme, "Difficulté de la campagne"),
        ChildOf(difficulty),
    ));
    let row = commands
        .spawn((
            Node {
                column_gap: Val::Px(6.0),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            ChildOf(difficulty),
        ))
        .id();
    if let Some(data) = &sim.data {
        for level in game_api::difficulty_levels(data) {
            commands.spawn((
                toggle_button(
                    &theme,
                    &format!("difficulty:{}", level.id),
                    &level.label,
                    level.id == choice.difficulty,
                ),
                DifficultyButton(level.id.clone()),
                ChildOf(row),
            ));
        }
    }
    commands.spawn((
        Name::new("DifficultyText"),
        DifficultyText,
        Text::new(""),
        theme.text_font(false, true, UiType::Body.size()),
        TextColor(style::INK),
        ChildOf(difficulty),
    ));

    // Actions.
    let actions = commands
        .spawn((
            Node {
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            ChildOf(column),
        ))
        .id();
    commands.spawn((
        button_small(&theme, "Retour", "Retour"),
        FaPlate("plate_leather_brass"),
        ChildOf(actions),
    ));
    // The kit button already carries its caption text; `StartCaption` marks the button so the
    // caption system rewrites that single text (« Commencer — France »), never adds a second one.
    commands.spawn((
        StartCaption,
        button_primary(&theme, "Commencer", "Commencer"),
        FaPlate("plate_crimson_brass"),
        ChildOf(actions),
    ));
}

/// Parchment panel child of `parent`: `width` px (or growing in the row), `grow` flex factor.
fn framed(
    commands: &mut Commands,
    theme: &UiTheme,
    parent: Entity,
    width: Option<f32>,
    grow: f32,
) -> Entity {
    let panel = commands.spawn((panel_page(theme), ChildOf(parent))).id();
    commands.entity(panel).insert(Node {
        width: width.map_or(Val::Auto, Val::Px),
        flex_grow: grow,
        min_width: Val::Px(0.0),
        min_height: Val::Px(0.0),
        padding: theme.padding("panel"),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(6.0),
        ..default()
    });
    panel
}

#[allow(clippy::too_many_arguments)]
fn spawn_cards(
    commands: &mut Commands,
    theme: &UiTheme,
    assets: &AssetServer,
    sim: &Sim,
    front: &super::front_end::FrontEnd,
    sheets: &StartSheets,
    ids: &[String],
    pane: Entity,
) {
    let scroll = commands
        .spawn((scroll_list(theme, "cards:scroll", 500.0), ChildOf(pane)))
        .id();
    let wrap = commands
        .spawn((
            Node {
                flex_wrap: FlexWrap::Wrap,
                column_gap: Val::Px(14.0),
                row_gap: Val::Px(14.0),
                flex_shrink: 0.0,
                ..default()
            },
            ChildOf(scroll),
        ))
        .id();
    for id in ids {
        let pitch = front.pitch(id);
        let card = commands
            .spawn((
                Name::new(format!("card:{id}")),
                FactionCard(id.clone()),
                WidgetButton,
                Hovered::default(),
                Node {
                    width: Val::Px(CARD_WIDTH),
                    flex_direction: FlexDirection::Column,
                    border: UiRect::all(Val::Px(3.0)),
                    ..default()
                },
                BackgroundColor(style::FE_VELLUM),
                BorderColor::all(style::FE_GOLD_DARK),
                ChildOf(wrap),
            ))
            .id();
        let art = commands
            .spawn((
                Node {
                    height: Val::Px(ART_HEIGHT),
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(style::FE_NIGHT),
                ChildOf(card),
            ))
            .id();
        match pitch.and_then(|p| p.illustration.clone()) {
            Some(path) => {
                commands.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    ImageNode::new(assets.load(path)),
                    CoverFit::new(Vec2::splat(0.5)),
                    ChildOf(art),
                ));
            }
            None => {
                commands.spawn((
                    Node {
                        height: Val::Px(ART_HEIGHT - 28.0),
                        ..default()
                    },
                    ImageNode::new(assets.load(format!("heraldry/{id}.png"))),
                    ChildOf(art),
                ));
            }
        }
        commands.spawn((
            Node {
                height: Val::Px(6.0),
                ..default()
            },
            BackgroundColor(primary_color(sim, id)),
            ChildOf(card),
        ));
        let body = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(10.0)),
                    row_gap: Val::Px(4.0),
                    ..default()
                },
                ChildOf(card),
            ))
            .id();
        let title_row = commands
            .spawn((
                Node {
                    column_gap: Val::Px(8.0),
                    align_items: AlignItems::Center,
                    ..default()
                },
                ChildOf(body),
            ))
            .id();
        commands.spawn((
            Node {
                width: Val::Px(38.0),
                height: Val::Px(44.0),
                flex_shrink: 0.0,
                ..default()
            },
            ImageNode::new(assets.load(format!("heraldry/{id}.png"))),
            ChildOf(title_row),
        ));
        commands.spawn((
            Text::new(faction_label(sim, id)),
            theme.title_font(UiType::Heading.size()),
            TextColor(style::INK),
            Node {
                flex_shrink: 1.0,
                ..default()
            },
            ChildOf(title_row),
        ));
        if let Some(pitch) = pitch {
            commands.spawn((
                Text::new(pitch.tagline.clone()),
                theme.text_font(false, true, UiType::Body.size()),
                TextColor(style::FE_GULES),
                ChildOf(body),
            ));
            commands.spawn((
                Text::new(format!(
                    "Difficulté : {}",
                    front.difficulty_label(pitch.difficulty)
                )),
                theme.text_font(true, false, UiType::Caption.size()),
                TextColor(style::INK),
                ChildOf(body),
            ));
        }
        if let Some(sheet) = sheets.get(id) {
            ruler_row(commands, theme, assets, sheets, sheet, 44.0, body);
            key_figures(commands, theme, sheet, body);
        }
    }
}

/// Ruler portrait (when the file exists) with his name and main title.
fn ruler_row(
    commands: &mut Commands,
    theme: &UiTheme,
    assets: &AssetServer,
    sheets: &StartSheets,
    sheet: &game_api::start_sheets::StartSheet,
    size: f32,
    parent: Entity,
) {
    if sheet.ruler.is_empty() {
        return;
    }
    let row = commands
        .spawn((
            Name::new("RulerRow"),
            Node {
                column_gap: Val::Px(8.0),
                align_items: AlignItems::Center,
                ..default()
            },
            ChildOf(parent),
        ))
        .id();
    if let Some(path) = sheets.portrait(&sheet.ruler_id) {
        commands.spawn((
            Name::new("RulerPortrait"),
            Node {
                width: Val::Px(size),
                height: Val::Px(size),
                flex_shrink: 0.0,
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BorderColor::all(style::FE_GOLD_DARK),
            ImageNode::new(assets.load(path)),
            ChildOf(row),
        ));
    }
    let texts = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_shrink: 1.0,
                ..default()
            },
            ChildOf(row),
        ))
        .id();
    commands.spawn((
        Text::new("Souverain"),
        theme.text_font(false, true, UiType::Caption.size()),
        TextColor(style::INK_FADED),
        ChildOf(texts),
    ));
    commands.spawn((
        Text::new(sheet.ruler.clone()),
        theme.title_font(UiType::Body.size()),
        TextColor(style::INK),
        ChildOf(texts),
    ));
    if let Some(title) = sheet.titles.first() {
        commands.spawn((
            Text::new(title.clone()),
            theme.text_font(false, false, UiType::Caption.size()),
            TextColor(style::INK_FADED),
            ChildOf(texts),
        ));
    }
}

/// Comparable key figures of a start (provinces, treasury, income, armies).
fn key_figures(
    commands: &mut Commands,
    theme: &UiTheme,
    sheet: &game_api::start_sheets::StartSheet,
    parent: Entity,
) {
    let grid = commands
        .spawn((
            Name::new("KeyFigures"),
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(1.0),
                ..default()
            },
            ChildOf(parent),
        ))
        .id();
    let rows = [
        ("Provinces", sheet.provinces.len().to_string()),
        (
            "Trésor",
            format!("{} ₶", crate::ui::kit::fr::digits(sheet.treasury)),
        ),
        (
            "Revenu",
            format!(
                "{} ₶ / saison",
                crate::ui::kit::fr::digits(sheet.projected_income)
            ),
        ),
        ("Armées", sheet.armies_count.to_string()),
    ];
    for (key, value) in rows {
        let line = commands
            .spawn((
                Node {
                    justify_content: JustifyContent::SpaceBetween,
                    column_gap: Val::Px(8.0),
                    ..default()
                },
                ChildOf(grid),
            ))
            .id();
        commands.spawn((
            Text::new(key),
            theme.text_font(false, true, UiType::Caption.size()),
            TextColor(style::INK_FADED),
            ChildOf(line),
        ));
        commands.spawn((
            Text::new(value),
            theme.text_font(false, false, UiType::Caption.size()),
            TextColor(style::INK),
            ChildOf(line),
        ));
    }
}

/// Starts the campaign with the current choice.
fn start_campaign(
    sim: &mut Sim,
    choice: &FactionChoice,
    progress: &mut SaveProgress,
) -> Result<(), String> {
    sim.player = FactionId::new(choice.faction.clone()).map_err(|e| e.to_string())?;
    sim.new_campaign()?;
    if let (Some(state), Some(level)) =
        (sim.state.as_mut(), Difficulty::from_key(&choice.difficulty))
    {
        state.set_difficulty(level);
    }
    progress.saved_turn = None;
    Ok(())
}

/// « Commencer » (button or Enter): asks about the guide first when it applies, else starts.
fn request_start(
    settings: Option<&Settings>,
    tutorial: &Tutorial,
    invite: &mut TutorialInvite,
    modals: &mut ModalQueue,
) -> bool {
    if invite.asking {
        return false;
    }
    if !invite_wanted(settings, tutorial) {
        return true;
    }
    invite.asking = true;
    modals.request(
        INVITE_KEY,
        PRIORITY_DECISION,
        ModalSpec {
            title: "Jouer le didacticiel ?".to_owned(),
            text: "Un guide en quelques étapes accompagne les premières saisons : provinces, \
                   recrutement, armées, diplomatie. On le retrouve ensuite dans l’aide (F1)."
                .to_owned(),
            yes_label: "Oui".to_owned(),
            no_label: "Non".to_owned(),
            width: 460.0,
        },
    );
    false
}

/// Starts the campaign and enters it, or reports why not.
fn begin_campaign(
    sim: &mut Sim,
    choice: &FactionChoice,
    progress: &mut SaveProgress,
    next: &mut MessageWriter<FadeToState>,
    toasts: &mut MessageWriter<ToastRequest>,
) {
    match start_campaign(sim, choice, progress) {
        Ok(()) => {
            next.write(FadeToState(AppState::Campaign));
        }
        Err(message) => {
            toasts.write(ToastRequest::new(&message));
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_buttons(
    mut pressed: MessageReader<ButtonPressed>,
    mut choice: ResMut<FactionChoice>,
    mut sim: ResMut<Sim>,
    mut next: MessageWriter<FadeToState>,
    mut toasts: MessageWriter<ToastRequest>,
    mut progress: ResMut<SaveProgress>,
    mut difficulty_buttons: Query<(&DifficultyButton, &mut Toggled)>,
    settings: Option<Res<Settings>>,
    tutorial: Res<Tutorial>,
    mut invite: ResMut<TutorialInvite>,
    mut modals: ResMut<ModalQueue>,
) {
    for press in pressed.read() {
        let name = press.name.as_str();
        if let Some(id) = name.strip_prefix("card:") {
            choice.faction = id.to_owned();
        } else if let Some(level) = name.strip_prefix("difficulty:") {
            choice.difficulty = level.to_owned();
            for (button, mut toggled) in &mut difficulty_buttons {
                toggled.0 = button.0 == level;
            }
        } else if name == "Retour" {
            next.write(FadeToState(AppState::MainMenu));
        } else if name == "Commencer"
            && request_start(settings.as_deref(), &tutorial, &mut invite, &mut modals)
        {
            begin_campaign(&mut sim, &choice, &mut progress, &mut next, &mut toasts);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn enter_starts(
    edges: Res<super::KeyEdges>,
    choice: Res<FactionChoice>,
    mut sim: ResMut<Sim>,
    mut next: MessageWriter<FadeToState>,
    mut progress: ResMut<SaveProgress>,
    mut toasts: MessageWriter<ToastRequest>,
    settings: Option<Res<Settings>>,
    tutorial: Res<Tutorial>,
    mut invite: ResMut<TutorialInvite>,
    mut modals: ResMut<ModalQueue>,
) {
    if edges.enter && request_start(settings.as_deref(), &tutorial, &mut invite, &mut modals) {
        begin_campaign(&mut sim, &choice, &mut progress, &mut next, &mut toasts);
    }
}

/// The answer to the invitation: starts the campaign, with the guide asked for or put aside.
#[allow(clippy::too_many_arguments)]
fn answer_invite(
    mut results: MessageReader<ModalResult>,
    choice: Res<FactionChoice>,
    mut sim: ResMut<Sim>,
    mut next: MessageWriter<FadeToState>,
    mut progress: ResMut<SaveProgress>,
    mut toasts: MessageWriter<ToastRequest>,
    mut tutorial: ResMut<Tutorial>,
    mut invite: ResMut<TutorialInvite>,
) {
    for result in results.read() {
        if result.key != INVITE_KEY {
            continue;
        }
        invite.asking = false;
        invite.answers.push(result.confirmed);
        invite.start_on_enter = result.confirmed;
        if !result.confirmed && !tutorial.finished {
            tutorial.postponed = Some(0);
        }
        begin_campaign(&mut sim, &choice, &mut progress, &mut next, &mut toasts);
    }
}

/// A new campaign was entered after « Oui »: the guide starts.
fn send_start_tutorial(
    mut invite: ResMut<TutorialInvite>,
    mut start: MessageWriter<StartTutorial>,
) {
    if std::mem::take(&mut invite.start_on_enter) {
        start.write(StartTutorial);
    }
}

fn handle_list(
    mut selected: MessageReader<ListSelected>,
    mut choice: ResMut<FactionChoice>,
    mut tabs: MessageReader<TabSelected>,
) {
    for event in selected.read() {
        if let Some(id) = event.name.strip_prefix("faction:") {
            choice.faction = id.to_owned();
        }
    }
    tabs.clear();
}

/// Rebuilds the pitch, the start caption and the difficulty text after a change of choice.
#[allow(clippy::too_many_arguments)]
fn refresh_choice(
    mut commands: Commands,
    choice: Res<FactionChoice>,
    mut last: Local<Option<(String, String)>>,
    detail: Query<Entity, With<DetailBox>>,
    start_buttons: Query<&Children, With<StartCaption>>,
    mut texts: Query<&mut Text, Without<DifficultyText>>,
    mut difficulty_text: Query<&mut Text, With<DifficultyText>>,
    theme: Res<UiTheme>,
    sim: Res<Sim>,
    menu: Res<MenuData>,
    start_sheets: Res<StartSheets>,
    assets: Res<AssetServer>,
) {
    let key = (choice.faction.clone(), choice.difficulty.clone());
    if last.as_ref() == Some(&key) {
        return;
    }
    let faction_changed = last.as_ref().is_none_or(|(faction, _)| *faction != key.0);
    *last = Some(key);
    let name = faction_label(&sim, &choice.faction);
    for children in &start_buttons {
        for child in children.iter() {
            if let Ok(mut text) = texts.get_mut(child) {
                text.0 = format!("Commencer — {name}");
            }
        }
    }
    if let Some(data) = &sim.data {
        for mut text in &mut difficulty_text {
            text.0 = game_api::difficulty_levels(data)
                .into_iter()
                .find(|level| level.id == choice.difficulty)
                .map(|level| level.description)
                .unwrap_or_default();
        }
    }
    if !faction_changed {
        return;
    }
    let Ok(detail) = detail.single() else { return };
    commands.entity(detail).despawn_related::<Children>();
    commands.spawn((
        Name::new("DetailName"),
        Text::new(name),
        theme.title_font(UiType::Title.size()),
        TextColor(style::INK_TITLE),
        ChildOf(detail),
    ));
    let Some(pitch) = menu.front.pitch(&choice.faction) else {
        commands.spawn((
            typed(
                &theme,
                "Aucune présentation pour cette faction.",
                UiType::Body,
            ),
            ChildOf(detail),
        ));
        return;
    };
    commands.spawn((
        Text::new(pitch.tagline.clone()),
        theme.text_font(false, true, UiType::Body.size()),
        TextColor(style::FE_GULES),
        ChildOf(detail),
    ));
    if let Some(sheet) = start_sheets.get(&choice.faction) {
        ruler_row(
            &mut commands,
            &theme,
            &assets,
            &start_sheets,
            sheet,
            64.0,
            detail,
        );
        key_figures(&mut commands, &theme, sheet, detail);
    }
    commands.spawn((typed(&theme, &pitch.intro, UiType::Body), ChildOf(detail)));
    for (title, lines) in [
        ("Forces", &pitch.strengths),
        ("Faiblesses", &pitch.weaknesses),
    ] {
        if lines.is_empty() {
            continue;
        }
        commands.spawn((heading(&theme, title), ChildOf(detail)));
        for line in lines {
            commands.spawn((
                typed(&theme, &format!("• {line}"), UiType::Caption),
                ChildOf(detail),
            ));
        }
    }
}

fn card_visuals(
    choice: Res<FactionChoice>,
    mut cards: Query<(&FactionCard, &Hovered, &mut BorderColor, &mut Node)>,
) {
    for (card, hovered, mut border, mut node) in &mut cards {
        let chosen = card.0 == choice.faction;
        let color = if chosen {
            style::FE_GOLD
        } else if hovered.get() {
            style::GOLD_PALE
        } else {
            style::FE_GOLD_DARK
        };
        let width = if chosen { Val::Px(4.0) } else { Val::Px(3.0) };
        if node.border.left != width {
            node.border = UiRect::all(width);
        }
        if border.top != color {
            *border = BorderColor::all(color);
        }
    }
}

/// Section `menu.faction_choice`.
pub fn dump(world: &World) -> Value {
    let choice = world.resource::<FactionChoice>();
    let invite = world.resource::<TutorialInvite>();
    let mut cards = world.try_query::<&FactionCard>().unwrap();
    let cards: Vec<String> = cards.iter(world).map(|c| c.0.clone()).collect();
    json!({
        "faction": choice.faction,
        "difficulty": choice.difficulty,
        "cards": cards,
        "tutorial_invite": {
            "asking": invite.asking,
            "start_on_enter": invite.start_on_enter,
            "answers": invite.answers,
            "postponed": world.get_resource::<Tutorial>().and_then(|t| t.postponed),
            "tutorial_active": world.get_resource::<Tutorial>().is_some_and(|t| t.active()),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_invitation_asks_only_while_the_guide_is_enabled_and_unfinished() {
        let mut tutorial = Tutorial::default();
        let mut settings = Settings::default();
        assert!(invite_wanted(Some(&settings), &tutorial));
        assert!(invite_wanted(None, &tutorial));
        settings.tutorial_enabled = false;
        assert!(!invite_wanted(Some(&settings), &tutorial));
        settings.tutorial_enabled = true;
        tutorial.finished = true;
        assert!(!invite_wanted(Some(&settings), &tutorial));
    }

    #[test]
    fn a_second_start_while_asking_is_ignored() {
        let tutorial = Tutorial::default();
        let mut invite = TutorialInvite::default();
        let mut modals = ModalQueue::default();
        assert!(!request_start(None, &tutorial, &mut invite, &mut modals));
        assert!(invite.asking);
        assert_eq!(modals.active_key(), INVITE_KEY);
        assert!(!request_start(None, &tutorial, &mut invite, &mut modals));
        let off = Settings {
            tutorial_enabled: false,
            ..Settings::default()
        };
        invite.asking = false;
        assert!(request_start(
            Some(&off),
            &tutorial,
            &mut invite,
            &mut modals
        ));
    }
}
