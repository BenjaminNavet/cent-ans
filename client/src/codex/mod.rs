//! The Codex screen ([`AppState::Codex`]): hub, encyclopedia by category with search, mechanic
//! sheets, Nature entries with their illustration.
//!
//! Ports game/scripts/ui/codex_hub.gd (hub: « Histoire » and « Règles » tabs, common search),
//! game/scripts/codex/{codex_window, codex_store, codex_text}.gd (families, entry page, links,
//! discoveries, history) and game/scripts/ui/{mechanic_sheet, encyclopedia}.gd (rules sheets,
//! subset: see [`rules`]). Everything is built with `ui::kit`.
//!
//! - **Data**: `data/codex_bundle.json` ([`store`]), `data/ui/encyclopedia.json` and the unit,
//!   building, technology and resource definitions ([`rules`]); illustrations under
//!   `game/assets/illustrations` (Nature entries included, ADR 0219).
//! - **Navigation**: « Retour » / Escape return to the state the screen was opened from
//!   ([`CodexReturn`]); ◀ ▶ walk the history of opened sheets; a link `[[cdx_id]]` of a body, a
//!   « Voir aussi » button or a list row opens a sheet (and discovers it).
//! - **Other screens** open a sheet with the message [`OpenCodex`].
//! - **Widgets**: `hub:history|rules`, `fam:all|<i>` (families), `rcat:all|<i>` (rules
//!   categories), rows `entry:<cdx_id>` / `rule:<id>`, `see:<cdx_id>`, `codex:rules`,
//!   `codex:history`, `codex:prev|next|back|clear`.
//! - **Dev hooks** (env, forwarded by `client/run_bg.sh`): `CENT_ANS_CODEX_OPEN=<id>` opens a
//!   sheet, `CENT_ANS_CODEX_QUERY=<text>` fills the search, `CENT_ANS_CODEX_PATH` redirects the
//!   discoveries file (default `codex.json` beside the settings file).
//! - **Dump** section `codex`.
//!
//! - **Overlay**: in the campaign the Codex is an overlay (K, or [`OpenCodex`]) over the live
//!   campaign instead of a state change (which would destroy it): [`CodexOverlay`]. The state
//!   [`AppState::Codex`] stays for the access from the main menu. Escape / « Retour » close it.
//! - **Bubbles** ([`bubbles`]): hover of a `cdx:` link (0,35 s) opens a bubble (title, category,
//!   summary) through the kit's tooltip chain; Alt chains child bubbles; a click opens the sheet.
//! - **Aliases**: [`text::format_auto`] links the first occurrence of each alias in free texts.
//! - **Herbarium** ([`herbarium`]): known technologies discover their plants; the list of
//!   « Médecine et herbier » shows a « — Herbier — » header before the plants.
//! - **Encyclopedia tabs** ([`encyclopedia`]): Traits, Compétences, Suite, Factions, Religion,
//!   Agents. Unread entries are dimmed in the list.
//!
//! Not ported: the decor hover bubble of ADR 0219 (needs the 3D decor picking of the battle and
//! campaign scenes), rich `ib:` entity bubbles, bubble pinning (right click, T).

pub mod bubble_layout;
pub mod bubbles;
pub mod encyclopedia;
pub mod herbarium;
pub mod rules;
pub mod store;
pub mod text;

use std::collections::BTreeMap;

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;
use serde_json::{Value, json};

use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::menu::{CoverFit, KeyEdges};
use crate::paths;
use crate::sim::Sim;
use crate::ui::kit::style::{self, UiType};
use crate::ui::kit::text_field::{TextField, TextFieldChanged, TextInputFocus, text_field_limited};
use crate::ui::kit::{
    ButtonPressed, ListSelected, RichLink, RichStyle, TabButton, TabSelected, UiTheme,
    button_small, caption, heading, list_item, panel_note, panel_page, rich_text, scroll_list,
    separator, tab_bar, typed,
};
use rules::{RulesBook, RulesEntry};
use store::{CodexStore, FAMILIES};

/// Asks for the Codex screen, optionally on a sheet (`cdx_…` or a rules sheet id).
#[derive(Message, Debug, Clone, Default)]
pub struct OpenCodex {
    pub entry: Option<String>,
}

/// True while the Codex is shown as an overlay over the campaign (state stays `Campaign`).
#[derive(Resource, Debug, Default)]
pub struct CodexOverlay {
    pub open: bool,
}

/// Root of the Codex screen (state screen or overlay).
#[derive(Component)]
struct CodexRoot;

/// List row of an unread entry: its label is dimmed.
#[derive(Component)]
struct UnreadRow;

fn dim_unread(rows: Query<&Children, Added<UnreadRow>>, mut colors: Query<&mut TextColor>) {
    for children in &rows {
        for child in children.iter() {
            if let Ok(mut color) = colors.get_mut(child) {
                color.0 = style::INK_FADED;
            }
        }
    }
}

fn codex_active(state: Res<State<AppState>>, overlay: Res<CodexOverlay>) -> bool {
    *state.get() == AppState::Codex || overlay.open
}

fn needs_spawn(overlay: Res<CodexOverlay>, roots: Query<(), With<CodexRoot>>) -> bool {
    overlay.open && roots.is_empty()
}

/// State to return to with « Retour » / Escape: the last state before the Codex.
#[derive(Resource, Debug, Clone, Copy)]
pub struct CodexReturn(pub AppState);

impl Default for CodexReturn {
    fn default() -> Self {
        Self(AppState::MainMenu)
    }
}

/// Sheet to show when the screen opens (set by [`OpenCodex`]).
#[derive(Resource, Debug, Default)]
struct PendingOpen(Option<String>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Hub {
    #[default]
    History,
    Rules,
}

/// The sheet on display: a Codex entry or a rules sheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sheet {
    Entry(String),
    Rule(String),
}

impl Sheet {
    fn id(&self) -> &str {
        match self {
            Self::Entry(id) | Self::Rule(id) => id,
        }
    }
}

/// What the screen shows; the screen rebuilds when this changes.
#[derive(Resource, Debug, Default)]
pub struct CodexView {
    pub hub: Hub,
    /// History filter: index of [`FAMILIES`], `None` = all.
    pub family: Option<usize>,
    /// Rules filter: index of the rules category, `None` = all.
    pub rules_category: Option<usize>,
    pub query: String,
    pub current: Option<Sheet>,
    history: Vec<Sheet>,
    index: usize,
}

impl CodexView {
    fn can_back(&self) -> bool {
        self.index > 0
    }

    fn can_forward(&self) -> bool {
        self.index + 1 < self.history.len()
    }

    fn push(&mut self, sheet: Sheet) {
        if self.current.as_ref() != Some(&sheet) {
            if !self.history.is_empty() {
                self.history.truncate(self.index + 1);
            }
            self.history.push(sheet.clone());
            self.index = self.history.len() - 1;
        }
        self.current = Some(sheet);
    }
}

/// Facts about the page last built (for the dump).
#[derive(Resource, Debug, Default)]
struct PageState {
    herbarium_header: bool,
    listed_unread: usize,
    title: String,
    art: Option<String>,
    links: usize,
    see_also: usize,
    listed: usize,
    rules_listed: usize,
}

#[derive(Resource, Default)]
struct RulesBookRes(RulesBook);

#[derive(Component)]
struct ListBody;
#[derive(Component)]
struct PageBody;
#[derive(Component)]
struct PageScroll;
#[derive(Component)]
struct CounterText;
#[derive(Component)]
struct ResultsText;
/// Row holding the family tab bar (history half).
#[derive(Component)]
struct FamilyBar;
/// Row holding the rules category tab bar, filled once the book exists.
#[derive(Component)]
struct RulesBar;

const LIST_WIDTH: f32 = 330.0;
const SEARCH_FIELD: &str = "codex:search";
const PLACEHOLDER: &str = "Rechercher un nom, un lieu, un mot…";

pub struct CodexPlugin;

impl Plugin for CodexPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<OpenCodex>()
            .init_resource::<CodexStore>()
            .init_resource::<CodexView>()
            .init_resource::<CodexReturn>()
            .init_resource::<CodexOverlay>()
            .init_resource::<PendingOpen>()
            .init_resource::<PageState>()
            .init_resource::<RulesBookRes>()
            .register_dump(dump_codex)
            .add_systems(Startup, load_store)
            .add_systems(
                Update,
                remember_return.run_if(not(in_state(AppState::Codex))),
            )
            .add_systems(OnEnter(AppState::Campaign), close_overlay_state)
            .add_systems(OnExit(AppState::Campaign), close_overlay_state)
            .add_systems(OnEnter(AppState::Codex), (prepare, spawn_screen).chain())
            .add_systems(
                Update,
                (
                    (
                        open_with_key,
                        open_requests,
                        (prepare, spawn_screen).chain().run_if(needs_spawn),
                    )
                        .chain(),
                    (
                        handle_escape.before(crate::menu::pause::pause_escape),
                        handle_buttons,
                        handle_selections,
                        handle_links,
                        sync_search,
                        refresh,
                        dim_unread,
                    )
                        .chain()
                        .run_if(codex_active),
                )
                    .chain(),
            );
        bubbles::plugin(app);
        herbarium::plugin(app);
    }
}

/// The overlay never survives a campaign (re)start or exit; its root is state-scoped.
fn close_overlay_state(mut overlay: ResMut<CodexOverlay>) {
    overlay.open = false;
}

/// K opens the Codex over the campaign.
fn open_with_key(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    overlay: Res<CodexOverlay>,
    paused: Option<Res<crate::menu::pause::GamePaused>>,
    mut requests: MessageWriter<OpenCodex>,
    typing: Res<TextInputFocus>,
) {
    if *state.get() == AppState::Campaign
        && !overlay.open
        && !paused.is_some_and(|paused| paused.0)
        && !typing.is_active()
        && keys.just_pressed(KeyCode::KeyK)
    {
        requests.write(OpenCodex::default());
    }
}

fn load_store(mut store: ResMut<CodexStore>) {
    match paths::find_root()
        .and_then(|root| CodexStore::load(&paths::data_dir(&root), &paths::asset_root(&root)))
    {
        Ok(loaded) => *store = loaded,
        Err(message) => warn!("codex bundle: {message}"),
    }
    store.use_save_file(CodexStore::default_save_path());
}

fn remember_return(state: Res<State<AppState>>, mut back: ResMut<CodexReturn>) {
    if !matches!(state.get(), AppState::Boot | AppState::Codex) {
        back.0 = *state.get();
    }
}

/// [`OpenCodex`]: in the campaign (and over a campaign that already shows it) the overlay, from
/// the other screens the Codex state.
#[allow(clippy::too_many_arguments)]
fn open_requests(
    mut requests: MessageReader<OpenCodex>,
    state: Res<State<AppState>>,
    mut overlay: ResMut<CodexOverlay>,
    mut pending: ResMut<PendingOpen>,
    mut view: ResMut<CodexView>,
    mut store: ResMut<CodexStore>,
    book: Res<RulesBookRes>,
    mut next: ResMut<NextState<AppState>>,
) {
    for request in requests.read() {
        match state.get() {
            AppState::Codex => {
                show_target(&mut view, &mut store, &book.0, request.entry.as_deref())
            }
            AppState::Campaign if overlay.open => {
                show_target(&mut view, &mut store, &book.0, request.entry.as_deref());
            }
            AppState::Campaign => {
                pending.0.clone_from(&request.entry);
                overlay.open = true;
            }
            _ => {
                pending.0.clone_from(&request.entry);
                next.set(AppState::Codex);
            }
        }
    }
}

/// Shows `id` (a Codex entry or a rules sheet) when it exists.
fn show_target(view: &mut CodexView, store: &mut CodexStore, book: &RulesBook, id: Option<&str>) {
    let Some(id) = id else {
        return;
    };
    if store.has(id) {
        navigate(view, store, Sheet::Entry(id.to_owned()));
    } else if book.entry(id).is_some() {
        navigate(view, store, Sheet::Rule(id.to_owned()));
    }
}

/// Builds the rules book once (it needs the loaded game data for the `{rule.*}` values) and
/// applies the pending sheet and the dev hooks.
fn prepare(
    sim: Res<Sim>,
    mut store: ResMut<CodexStore>,
    mut book: ResMut<RulesBookRes>,
    mut view: ResMut<CodexView>,
    mut pending: ResMut<PendingOpen>,
) {
    if book.0.is_empty()
        && let Ok(root) = paths::find_root()
    {
        let constants: BTreeMap<String, f64> = sim
            .data
            .as_ref()
            .map(|data| {
                sim_campaign::rule_constants::rule_constants(data)
                    .into_iter()
                    .map(|(name, value)| (name.to_owned(), value))
                    .collect()
            })
            .unwrap_or_default();
        book.0 = RulesBook::build(
            &paths::data_dir(&root),
            &paths::asset_root(&root),
            &constants,
            &store,
        );
    }
    if let Ok(query) = std::env::var("CENT_ANS_CODEX_QUERY") {
        view.query = query;
    }
    let wanted = pending
        .0
        .take()
        .or_else(|| std::env::var("CENT_ANS_CODEX_OPEN").ok());
    if let Some(id) = wanted {
        if store.has(&id) {
            view.hub = Hub::History;
            view.push(Sheet::Entry(id));
        } else if book.0.entry(&id).is_some() {
            view.hub = Hub::Rules;
            view.push(Sheet::Rule(id));
        }
    }
    // A sheet opened from outside counts as read.
    if let Some(Sheet::Entry(id)) = view.current.clone() {
        store.discover(&id);
    }
}

// ---- screen --------------------------------------------------------------------------------

fn spawn_screen(mut commands: Commands, theme: Res<UiTheme>, state: Res<State<AppState>>) {
    // Overlay (campaign) or full screen (Codex state); both are state-scoped.
    let scope = if *state.get() == AppState::Codex {
        AppState::Codex
    } else {
        AppState::Campaign
    };
    let root = commands
        .spawn((
            Name::new("Codex"),
            CodexRoot,
            DespawnOnExit(scope),
            GlobalZIndex(400),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect {
                    left: Val::Px(24.0),
                    right: Val::Px(24.0),
                    top: Val::Px(18.0),
                    bottom: Val::Px(16.0),
                },
                row_gap: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(style::FE_NIGHT),
        ))
        .id();

    // Header: title, counter, history and back buttons.
    let header = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                ..default()
            },
            ChildOf(root),
        ))
        .id();
    commands.spawn((
        Text::new("✠ Codex"),
        theme.title_font(30.0),
        TextColor(style::FE_GOLD),
        ChildOf(header),
    ));
    commands.spawn((
        Name::new("Counter"),
        CounterText,
        Text::new(""),
        theme.text_font(false, true, UiType::Caption.size()),
        TextColor(style::PRIMARY_INK),
        Node {
            flex_grow: 1.0,
            ..default()
        },
        ChildOf(header),
    ));
    commands.spawn((button_small(&theme, "codex:prev", "◀"), ChildOf(header)));
    commands.spawn((button_small(&theme, "codex:next", "▶"), ChildOf(header)));
    commands.spawn((
        button_small(&theme, "codex:back", "Retour"),
        ChildOf(header),
    ));

    // Search field (typed text) and results line.
    let search_row = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                ..default()
            },
            ChildOf(root),
        ))
        .id();
    commands.spawn((
        text_field_limited(&theme, SEARCH_FIELD, "", PLACEHOLDER, 420.0, 60),
        ChildOf(search_row),
    ));
    commands.spawn((
        button_small(&theme, "codex:clear", "Effacer"),
        ChildOf(search_row),
    ));
    commands.spawn((
        ResultsText,
        Text::new(""),
        theme.text_font(false, true, UiType::Caption.size()),
        TextColor(style::PRIMARY_INK),
        ChildOf(search_row),
    ));

    // Hub tabs.
    commands.spawn((
        tab_bar(
            &theme,
            "hub",
            &[("hub:history", "Histoire"), ("hub:rules", "Règles")],
            0,
        ),
        ChildOf(root),
    ));

    // Body.
    let body = commands.spawn((panel_page(&theme), ChildOf(root))).id();
    let panel_padding = theme.padding("panel");
    tweak(&mut commands, body, move |node| {
        node.flex_grow = 1.0;
        node.min_height = Val::Px(0.0);
        node.padding = panel_padding;
        node.flex_direction = FlexDirection::Column;
        node.row_gap = Val::Px(6.0);
    });
    let mut families: Vec<(String, String)> = vec![("fam:all".to_owned(), "Toutes".to_owned())];
    families.extend(
        FAMILIES
            .iter()
            .enumerate()
            .map(|(index, family)| (format!("fam:{index}"), family.2.to_owned())),
    );
    let family_tabs: Vec<(&str, &str)> = families
        .iter()
        .map(|(name, label)| (name.as_str(), label.as_str()))
        .collect();
    let family_bar = commands
        .spawn((FamilyBar, Node::default(), ChildOf(body)))
        .id();
    commands.spawn((
        tab_bar(&theme, "family", &family_tabs, 0),
        ChildOf(family_bar),
    ));
    commands.spawn((
        RulesBar,
        Node {
            display: Display::None,
            ..default()
        },
        ChildOf(body),
    ));

    let split = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                column_gap: Val::Px(12.0),
                ..default()
            },
            ChildOf(body),
        ))
        .id();
    let list = commands
        .spawn((scroll_list(&theme, "codex:list", 400.0), ChildOf(split)))
        .id();
    let list_padding = scroll_padding(&theme);
    tweak(&mut commands, list, move |node| {
        node.width = Val::Px(LIST_WIDTH);
        node.flex_shrink = 0.0;
        flexible_height(node, list_padding);
    });
    commands.spawn((
        ListBody,
        Node {
            flex_direction: FlexDirection::Column,
            flex_shrink: 0.0,
            row_gap: Val::Px(2.0),
            ..default()
        },
        ChildOf(list),
    ));
    let page = commands
        .spawn((
            PageScroll,
            scroll_list(&theme, "codex:page", 400.0),
            ChildOf(split),
        ))
        .id();
    tweak(&mut commands, page, move |node| {
        node.flex_grow = 1.0;
        node.min_width = Val::Px(0.0);
        flexible_height(node, list_padding);
    });
    commands.spawn((
        PageBody,
        Node {
            flex_direction: FlexDirection::Column,
            flex_shrink: 0.0,
            row_gap: Val::Px(8.0),
            width: Val::Percent(100.0),
            ..default()
        },
        ChildOf(page),
    ));
}

/// Right padding of a kit scroll list, widened for the scrollbar.
fn scroll_padding(theme: &UiTheme) -> UiRect {
    let mut padding = theme.inner_padding("inset", 4.0);
    padding.right = Val::Px(20.0);
    padding
}

/// Gives a kit `scroll_list` a flexible height (the kit's has a fixed one).
fn flexible_height(node: &mut Node, padding: UiRect) {
    node.height = Val::Auto;
    node.min_height = Val::Px(0.0);
    node.padding = padding;
}

/// Edits the `Node` of `entity` once it exists (kit bundles already carry their own `Node`).
fn tweak(
    commands: &mut Commands,
    entity: Entity,
    change: impl FnOnce(&mut Node) + Send + Sync + 'static,
) {
    commands
        .entity(entity)
        .entry::<Node>()
        .and_modify(move |mut node| change(&mut node));
}

// ---- input ---------------------------------------------------------------------------------

/// Opens `sheet` (discovering a Codex entry), switching the hub and filters so it is listed.
fn navigate(view: &mut CodexView, store: &mut CodexStore, sheet: Sheet) {
    match &sheet {
        Sheet::Entry(id) => {
            if !store.has(id) {
                return;
            }
            store.discover(id);
            view.hub = Hub::History;
            if !store.ids_in_family(view.family).contains(&id.as_str()) {
                view.family = None;
            }
            if !view.query.is_empty() {
                let needle = text::fold(view.query.trim());
                if !store.entry(id).is_some_and(|entry| entry.matches(&needle)) {
                    view.query.clear();
                }
            }
        }
        Sheet::Rule(_) => view.hub = Hub::Rules,
    }
    view.push(sheet);
}

/// Leaves the Codex: closes the overlay, or returns to the state it was opened from.
fn leave(
    commands: &mut Commands,
    overlay: &mut CodexOverlay,
    roots: &Query<Entity, With<CodexRoot>>,
    back: &CodexReturn,
    next: &mut NextState<AppState>,
) {
    if overlay.open {
        overlay.open = false;
        for root in roots {
            commands.entity(root).despawn();
        }
    } else {
        next.set(back.0);
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_escape(
    mut commands: Commands,
    mut edges: ResMut<KeyEdges>,
    mut overlay: ResMut<CodexOverlay>,
    roots: Query<Entity, With<CodexRoot>>,
    state: Res<State<AppState>>,
    back: Res<CodexReturn>,
    mut next: ResMut<NextState<AppState>>,
    typing: Res<TextInputFocus>,
) {
    let shown = *state.get() == AppState::Codex || overlay.open;
    if edges.escape && shown && !typing.is_active() {
        edges.escape = false;
        leave(&mut commands, &mut overlay, &roots, &back, &mut next);
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_buttons(
    mut commands: Commands,
    mut overlay: ResMut<CodexOverlay>,
    roots: Query<Entity, With<CodexRoot>>,
    mut pressed: MessageReader<ButtonPressed>,
    mut view: ResMut<CodexView>,
    mut store: ResMut<CodexStore>,
    book: Res<RulesBookRes>,
    back: Res<CodexReturn>,
    mut next: ResMut<NextState<AppState>>,
) {
    for press in pressed.read() {
        let name = press.name.as_str();
        match name {
            "codex:back" => leave(&mut commands, &mut overlay, &roots, &back, &mut next),
            "codex:prev" if view.can_back() => {
                view.index -= 1;
                let sheet = view.history[view.index].clone();
                show_without_recording(&mut view, &store, sheet);
            }
            "codex:next" if view.can_forward() => {
                view.index += 1;
                let sheet = view.history[view.index].clone();
                show_without_recording(&mut view, &store, sheet);
            }
            "codex:clear" if !view.query.is_empty() => view.query.clear(),
            // Rules sheet of the entity of the current entry.
            "codex:rules" => {
                let entity = match &view.current {
                    Some(Sheet::Entry(id)) => store.entry(id).map(|entry| entry.entity.clone()),
                    _ => None,
                };
                if let Some(entity) = entity
                    && book.0.entry(&entity).is_some()
                {
                    navigate(&mut view, &mut store, Sheet::Rule(entity));
                }
            }
            // Historical sheet of the current rules sheet.
            "codex:history" => {
                let target = match &view.current {
                    Some(Sheet::Rule(id)) => store.entry_for_entity(id).map(str::to_owned),
                    _ => None,
                };
                if let Some(id) = target {
                    navigate(&mut view, &mut store, Sheet::Entry(id));
                }
            }
            _ => {
                if let Some(id) = name.strip_prefix("see:") {
                    navigate(&mut view, &mut store, Sheet::Entry(id.to_owned()));
                }
            }
        }
    }
}

fn show_without_recording(view: &mut CodexView, store: &CodexStore, sheet: Sheet) {
    view.hub = match sheet {
        Sheet::Entry(_) => Hub::History,
        Sheet::Rule(_) => Hub::Rules,
    };
    if let Sheet::Entry(id) = &sheet
        && !store.ids_in_family(view.family).contains(&id.as_str())
    {
        view.family = None;
    }
    view.current = Some(sheet);
}

fn handle_selections(
    mut tabs: MessageReader<TabSelected>,
    mut lists: MessageReader<ListSelected>,
    mut view: ResMut<CodexView>,
    mut store: ResMut<CodexStore>,
) {
    for tab in tabs.read() {
        let index = |prefix: &str| {
            tab.name
                .strip_prefix(prefix)
                .and_then(|rest| rest.parse::<usize>().ok())
        };
        match tab.group.as_str() {
            "hub" => {
                view.hub = if tab.name == "hub:rules" {
                    Hub::Rules
                } else {
                    Hub::History
                };
            }
            "family" => view.family = index("fam:"),
            "rcat" => view.rules_category = index("rcat:"),
            _ => {}
        }
    }
    for item in lists.read() {
        if item.group != "codex:list" {
            continue;
        }
        if let Some(id) = item.name.strip_prefix("entry:") {
            navigate(&mut view, &mut store, Sheet::Entry(id.to_owned()));
        } else if let Some(id) = item.name.strip_prefix("rule:") {
            navigate(&mut view, &mut store, Sheet::Rule(id.to_owned()));
        }
    }
}

/// A click on a `[[cdx_id]]` link of a body opens that entry.
fn handle_links(
    mouse: Res<ButtonInput<MouseButton>>,
    links: Query<(&RichLink, &Hovered)>,
    mut view: ResMut<CodexView>,
    mut store: ResMut<CodexStore>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let target = links
        .iter()
        .find(|(_, hovered)| hovered.get())
        .and_then(|(link, _)| text::target_id(&link.0).map(str::to_owned));
    if let Some(id) = target {
        navigate(&mut view, &mut store, Sheet::Entry(id));
    }
}

/// The kit field and [`CodexView::query`] stay equal: typing edits the query, and a query set
/// elsewhere (clear button, navigation, dev hook) is written back to the field.
fn sync_search(
    mut changed: MessageReader<TextFieldChanged>,
    mut view: ResMut<CodexView>,
    mut fields: Query<(&Name, &mut TextField)>,
) {
    for event in changed.read() {
        if event.name == SEARCH_FIELD && view.query != event.text {
            view.query = event.text.clone();
        }
    }
    for (name, mut field) in &mut fields {
        if name.as_str() == SEARCH_FIELD && field.value != view.query {
            field.set(&view.query);
        }
    }
}

// ---- refresh -------------------------------------------------------------------------------

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn refresh(
    mut commands: Commands,
    view: Res<CodexView>,
    store: Res<CodexStore>,
    book: Res<RulesBookRes>,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
    mut page_state: ResMut<PageState>,
    list_body: Query<Entity, With<ListBody>>,
    page_body: Query<Entity, With<PageBody>>,
    page_scroll: Query<Entity, With<PageScroll>>,
    // Only the two tab-bar rows: an unfiltered query would also hide every other `Node`.
    bars: Query<
        (Entity, Option<&Children>, Has<FamilyBar>, Has<RulesBar>),
        Or<(With<FamilyBar>, With<RulesBar>)>,
    >,
    mut tab_buttons: Query<(&Name, &mut TabButton)>,
    mut nodes: Query<&mut Node>,
    mut texts: ParamSet<(
        Query<&mut Text, With<CounterText>>,
        Query<&mut Text, With<ResultsText>>,
    )>,
    mut buttons: Query<(&Name, &mut Visibility), With<bevy::ui_widgets::Button>>,
    fresh: Query<(), Added<ListBody>>,
) {
    let (Ok(list_body), Ok(page_body)) = (list_body.single(), page_body.single()) else {
        return;
    };
    if fresh.is_empty() && !view.is_changed() && !store.is_changed() {
        return;
    }

    // Rules category bar, built once the book exists; bars follow the hub.
    for (entity, children, is_family, is_rules) in &bars {
        if is_rules && children.is_none() && !book.0.is_empty() {
            let mut tabs: Vec<(String, String)> =
                vec![("rcat:all".to_owned(), "Toutes".to_owned())];
            for (index, category) in book.0.categories.iter().enumerate() {
                tabs.push((format!("rcat:{index}"), category.label.to_owned()));
            }
            let refs: Vec<(&str, &str)> = tabs
                .iter()
                .map(|(name, label)| (name.as_str(), label.as_str()))
                .collect();
            commands.spawn((tab_bar(&theme, "rcat", &refs, 0), ChildOf(entity)));
        }
        let shown = (is_family && view.hub == Hub::History) || (is_rules && view.hub == Hub::Rules);
        if let Ok(mut node) = nodes.get_mut(entity) {
            node.display = if shown { Display::Flex } else { Display::None };
        }
    }
    // Tab selection follows the view.
    for (name, mut tab) in &mut tab_buttons {
        let wanted = match tab.group.as_str() {
            "hub" => {
                name.as_str()
                    == if view.hub == Hub::Rules {
                        "hub:rules"
                    } else {
                        "hub:history"
                    }
            }
            "family" => {
                name.as_str()
                    == view
                        .family
                        .map_or("fam:all".to_owned(), |i| format!("fam:{i}"))
            }
            "rcat" => {
                name.as_str()
                    == view
                        .rules_category
                        .map_or("rcat:all".to_owned(), |i| format!("rcat:{i}"))
            }
            _ => continue,
        };
        if tab.selected != wanted {
            tab.selected = wanted;
        }
    }

    let needle = text::fold(view.query.trim());

    // List.
    commands.entity(list_body).despawn_related::<Children>();
    let mut listed = 0;
    let mut rules_listed = 0;
    let mut herbarium_header = false;
    let mut listed_unread = 0;
    match view.hub {
        Hub::History => {
            // In a family mixing sheets and plants the plants form a « Herbier » sub-section
            // at the end of the list.
            let (plants, others): (Vec<&str>, Vec<&str>) = store
                .ids_in_family(view.family)
                .into_iter()
                .filter(|id| store.entry(id).is_some_and(|entry| entry.matches(&needle)))
                .partition(|id| {
                    store
                        .entry(id)
                        .is_some_and(|e| e.category == herbarium::CATEGORY)
                });
            let header = view.family.is_some() && !plants.is_empty() && !others.is_empty();
            let mut rows: Vec<Option<&str>> = others.iter().map(|id| Some(*id)).collect();
            if header {
                rows.push(None);
            }
            rows.extend(plants.iter().map(|id| Some(*id)));
            herbarium_header = header;
            for row in rows {
                let Some(id) = row else {
                    commands.spawn((
                        Name::new("herbarium:header"),
                        Text::new(herbarium::HEADER),
                        theme.text_font(true, false, UiType::Body.size()),
                        TextColor(style::RUBRIC),
                        Node {
                            padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                            ..default()
                        },
                        ChildOf(list_body),
                    ));
                    continue;
                };
                let Some(entry) = store.entry(id) else {
                    continue;
                };
                listed += 1;
                let read = store.is_discovered(id);
                let mark = if read { "" } else { "✧ " };
                let selected = view.current == Some(Sheet::Entry(id.to_owned()));
                let row_entity = commands
                    .spawn((
                        list_item(
                            &theme,
                            "codex:list",
                            &format!("entry:{id}"),
                            &format!("{mark}{}", entry.title),
                            selected,
                        ),
                        ChildOf(list_body),
                    ))
                    .id();
                if !read {
                    listed_unread += 1;
                    commands.entity(row_entity).insert(UnreadRow);
                }
            }
        }
        Hub::Rules => {
            for (_, entry) in book.0.category(view.rules_category) {
                if !entry.search.contains(&needle) {
                    continue;
                }
                rules_listed += 1;
                let selected = view.current == Some(Sheet::Rule(entry.id.clone()));
                commands.spawn((
                    list_item(
                        &theme,
                        "codex:list",
                        &format!("rule:{}", entry.id),
                        &entry.name,
                        selected,
                    ),
                    ChildOf(list_body),
                ));
            }
        }
    }

    // Counter and search results.
    if let Ok(mut counter) = texts.p0().single_mut() {
        counter.0 = match view.hub {
            Hub::History => format!("{} / {} découvertes", store.discovered_count(), store.len()),
            Hub::Rules => format!("{} fiches de règles", book.0.len()),
        };
    }
    if let Ok(mut results) = texts.p1().single_mut() {
        results.0 = if needle.is_empty() {
            String::new()
        } else {
            format!(
                "Histoire : {} · Règles : {}",
                store.match_count(&view.query),
                book.0.match_count(&view.query)
            )
        };
    }
    // History buttons are hidden when there is nowhere to go.
    for (name, mut visibility) in &mut buttons {
        let available = match name.as_str() {
            "codex:prev" => view.can_back(),
            "codex:next" => view.can_forward(),
            _ => continue,
        };
        *visibility = if available {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    // Page.
    commands.entity(page_body).despawn_related::<Children>();
    if let Ok(scroll) = page_scroll.single() {
        commands.entity(scroll).insert(ScrollPosition::default());
    }
    *page_state = PageState {
        listed,
        rules_listed,
        herbarium_header,
        listed_unread,
        ..default()
    };
    match &view.current {
        Some(Sheet::Entry(id)) => {
            if let Some(entry) = store.entry(id) {
                spawn_entry_page(
                    &mut commands,
                    &theme,
                    &assets,
                    &store,
                    &book.0,
                    entry,
                    page_body,
                    &mut page_state,
                );
            }
        }
        Some(Sheet::Rule(id)) => {
            if let Some(entry) = book.0.entry(id) {
                spawn_rules_page(
                    &mut commands,
                    &theme,
                    &assets,
                    &store,
                    entry,
                    page_body,
                    &mut page_state,
                );
            }
        }
        None => {
            commands.spawn((
                typed(
                    &theme,
                    "Choisissez une fiche dans la liste, ou cherchez un nom, un lieu, un mot.",
                    UiType::Body,
                ),
                ChildOf(page_body),
            ));
        }
    }
}

fn spawn_art(commands: &mut Commands, assets: &AssetServer, path: &str, parent: Entity) {
    let image = ImageNode::new(assets.load(path.to_owned()));
    if path.ends_with(".png") {
        commands.spawn((
            Name::new("PageArt"),
            image,
            Node {
                height: Val::Px(220.0),
                aspect_ratio: Some(1.0),
                flex_shrink: 0.0,
                ..default()
            },
            ChildOf(parent),
        ));
    } else {
        commands.spawn((
            Name::new("PageArt"),
            image,
            Node {
                width: Val::Percent(100.0),
                max_width: Val::Px(640.0),
                aspect_ratio: Some(16.0 / 9.0),
                flex_shrink: 0.0,
                ..default()
            },
            CoverFit::new(Vec2::splat(0.5)),
            ChildOf(parent),
        ));
    }
}

fn spawn_title(commands: &mut Commands, theme: &UiTheme, title: &str, meta: &str, parent: Entity) {
    commands.spawn((
        Name::new("PageTitle"),
        Text::new(title),
        theme.title_font(UiType::Title.size()),
        TextColor(style::INK_TITLE),
        ChildOf(parent),
    ));
    if !meta.is_empty() {
        commands.spawn((caption(theme, meta), ChildOf(parent)));
    }
}

fn body_text(
    commands: &mut Commands,
    theme: &UiTheme,
    assets: &AssetServer,
    bbcode: &str,
    parent: Entity,
) {
    commands.spawn((
        rich_text(theme, assets, bbcode, &RichStyle::body(theme)),
        Node {
            width: Val::Percent(100.0),
            ..default()
        },
        ChildOf(parent),
    ));
}

#[allow(clippy::too_many_arguments)]
fn spawn_entry_page(
    commands: &mut Commands,
    theme: &UiTheme,
    assets: &AssetServer,
    store: &CodexStore,
    book: &RulesBook,
    entry: &store::Entry,
    parent: Entity,
    state: &mut PageState,
) {
    let mut meta = vec![entry.category_label()];
    let era = entry.era_label();
    if !era.is_empty() {
        meta.push(era);
    }
    if !entry.latin.is_empty() {
        meta.push(entry.latin.clone());
    }
    spawn_title(commands, theme, &entry.title, &meta.join(" · "), parent);
    state.title.clone_from(&entry.title);
    state.art = store.art_of(&entry.id);
    if let Some(art) = &state.art {
        spawn_art(commands, assets, art, parent);
    }
    if !entry.entity.is_empty() && book.entry(&entry.entity).is_some() {
        let button = commands
            .spawn((
                button_small(theme, "codex:rules", "Voir la fiche de règles"),
                ChildOf(parent),
            ))
            .id();
        tweak(commands, button, |node| {
            node.align_self = AlignSelf::FlexStart
        });
    }
    commands.spawn((separator(theme), ChildOf(parent)));
    let body = if entry.body.is_empty() {
        &entry.summary
    } else {
        &entry.body
    };
    let formatted = text::format(store, body);
    state.links += formatted.matches("[url=").count();
    body_text(commands, theme, assets, &formatted, parent);
    for (title, content) in [
        ("En jeu", &entry.gameplay),
        ("Liberté prise par le jeu", &entry.anachronism),
    ] {
        if content.is_empty() {
            continue;
        }
        let note = commands.spawn((panel_note(theme), ChildOf(parent))).id();
        let formatted = text::format(store, &format!("[b]{title}[/b]\n{content}"));
        commands.spawn((
            rich_text(theme, assets, &formatted, &RichStyle::caption(theme)),
            Node {
                width: Val::Percent(100.0),
                ..default()
            },
            ChildOf(note),
        ));
    }
    let see_also: Vec<&String> = entry.see_also.iter().filter(|id| store.has(id)).collect();
    if !see_also.is_empty() {
        commands.spawn((heading(theme, "Voir aussi"), ChildOf(parent)));
        let row = commands
            .spawn((
                Node {
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(6.0),
                    row_gap: Val::Px(6.0),
                    ..default()
                },
                ChildOf(parent),
            ))
            .id();
        for id in &see_also {
            commands.spawn((
                button_small(theme, &format!("see:{id}"), &store.title(id)),
                ChildOf(row),
            ));
        }
        state.see_also = see_also.len();
    }
    if !entry.sources.is_empty() {
        commands.spawn((
            caption(
                theme,
                &format!("Sources (Wikipédia) : {}", entry.sources.join(" ; ")),
            ),
            ChildOf(parent),
        ));
    }
}

fn spawn_rules_page(
    commands: &mut Commands,
    theme: &UiTheme,
    assets: &AssetServer,
    store: &CodexStore,
    entry: &RulesEntry,
    parent: Entity,
    state: &mut PageState,
) {
    spawn_title(commands, theme, &entry.name, &entry.subtitle, parent);
    state.title.clone_from(&entry.name);
    state.art.clone_from(&entry.art);
    if let Some(art) = &entry.art {
        spawn_art(commands, assets, art, parent);
    }
    if store.entry_for_entity(&entry.id).is_some() {
        let button = commands
            .spawn((
                button_small(theme, "codex:history", "✠ Fiche historique"),
                ChildOf(parent),
            ))
            .id();
        tweak(commands, button, |node| {
            node.align_self = AlignSelf::FlexStart
        });
    }
    commands.spawn((separator(theme), ChildOf(parent)));
    let formatted = text::format(store, &entry.body);
    state.links += formatted.matches("[url=").count();
    body_text(commands, theme, assets, &formatted, parent);
}

// ---- dump ----------------------------------------------------------------------------------

fn dump_codex(world: &World) -> (String, Value) {
    let view = world.resource::<CodexView>();
    let store = world.resource::<CodexStore>();
    let book = &world.resource::<RulesBookRes>().0;
    let page = world.resource::<PageState>();
    let (kind, id) = match &view.current {
        Some(Sheet::Entry(id)) => ("entry", id.as_str()),
        Some(Sheet::Rule(id)) => ("rule", id.as_str()),
        None => ("none", ""),
    };
    let value = json!({
        "hub": format!("{:?}", view.hub),
        "family": view.family,
        "rules_category": view.rules_category,
        "query": view.query,
        "entries_total": store.len(),
        "entries_listed": page.listed,
        "discovered": store.discovered_count(),
        "rules_total": book.len(),
        "rules_categories": book
            .categories
            .iter()
            .map(|c| json!({"id": c.id, "count": c.entries.len()}))
            .collect::<Vec<_>>(),
        "rules_listed": page.rules_listed,
        "current": {
            "kind": kind, "id": id, "title": page.title, "art": page.art,
            "links": page.links, "see_also": page.see_also,
        },
        "history": {
            "len": view.history.len(),
            "index": view.index,
            "ids": view.history.iter().map(Sheet::id).collect::<Vec<_>>(),
        },
        "matches": store.match_count(&view.query),
        "return_to": format!("{:?}", world.resource::<CodexReturn>().0),
        "nature_entries": store.ids_in_family(Some(7)).len(),
        "overlay": world.resource::<CodexOverlay>().open,
        "unread_listed": page.listed_unread,
        "herbarium_header": page.herbarium_header,
        "rules_tabs": book.categories.iter().map(|c| c.id).collect::<Vec<_>>(),
    });
    ("codex".to_owned(), value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> CodexStore {
        CodexStore::parse(
            r#"{"entries":[
              {"id":"cdx_a","title":"Abbaye","category":"batiment","summary":"s","body":"b"},
              {"id":"cdx_c","title":"Chêne","category":"arbre","summary":"s","body":"b"}]}"#
                .as_bytes(),
        )
        .unwrap()
    }

    #[test]
    fn navigating_records_history_and_widens_filters() {
        let mut store = store();
        let mut view = CodexView {
            family: Some(7),
            query: "chene".into(),
            ..default()
        };
        navigate(&mut view, &mut store, Sheet::Entry("cdx_a".into()));
        assert_eq!(view.family, None, "Abbaye is not Nature");
        assert!(view.query.is_empty(), "the search hid the entry");
        navigate(&mut view, &mut store, Sheet::Entry("cdx_c".into()));
        navigate(&mut view, &mut store, Sheet::Entry("cdx_c".into()));
        assert_eq!(view.history.len(), 2);
        assert!(view.can_back() && !view.can_forward());
        navigate(&mut view, &mut store, Sheet::Entry("cdx_missing".into()));
        assert_eq!(view.history.len(), 2);
    }

    #[test]
    fn opening_from_the_middle_of_history_drops_the_forward_branch() {
        let mut store = store();
        let mut view = CodexView::default();
        navigate(&mut view, &mut store, Sheet::Entry("cdx_a".into()));
        navigate(&mut view, &mut store, Sheet::Entry("cdx_c".into()));
        view.index = 0;
        view.current = Some(Sheet::Entry("cdx_a".into()));
        navigate(&mut view, &mut store, Sheet::Rule("unit_x".into()));
        assert_eq!(view.history.len(), 2);
        assert_eq!(view.hub, Hub::Rules);
    }
}
