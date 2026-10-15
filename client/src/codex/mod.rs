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
//! Not ported: auto-linking of aliases in simulation texts, hover bubbles of keywords
//! (`codex_bubbles.gd`, `bubble_layout.gd`, the decor hover bubble of ADR 0219), the herbarium
//! sub-section header (`herbarium.gd`), the discovered / unread dimming of list rows.

pub mod rules;
pub mod store;
pub mod text;

use std::collections::BTreeMap;

use bevy::input::keyboard::{Key, KeyboardInput};
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
use crate::ui::kit::{
    ButtonPressed, ListSelected, RichLink, RichStyle, TabButton, TabSelected, UiTheme,
    button_small, caption, heading, list_item, panel_inset, panel_note, panel_page, rich_text,
    scroll_list, separator, tab_bar, typed,
};
use rules::{RulesBook, RulesEntry};
use store::{CodexStore, FAMILIES};

/// Asks for the Codex screen, optionally on a sheet (`cdx_…` or a rules sheet id).
#[derive(Message, Debug, Clone, Default)]
pub struct OpenCodex {
    pub entry: Option<String>,
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
struct SearchText;
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
const PLACEHOLDER: &str = "Rechercher un nom, un lieu, un mot…";

pub struct CodexPlugin;

impl Plugin for CodexPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<OpenCodex>()
            .init_resource::<CodexStore>()
            .init_resource::<CodexView>()
            .init_resource::<CodexReturn>()
            .init_resource::<PendingOpen>()
            .init_resource::<PageState>()
            .init_resource::<RulesBookRes>()
            .register_dump(dump_codex)
            .add_systems(Startup, load_store)
            .add_systems(
                Update,
                (remember_return, open_requests).run_if(not(in_state(AppState::Codex))),
            )
            .add_systems(OnEnter(AppState::Codex), (prepare, spawn_screen).chain())
            .add_systems(
                Update,
                (
                    handle_escape.before(crate::menu::pause::pause_escape),
                    handle_buttons,
                    handle_selections,
                    handle_links,
                    type_query,
                    refresh,
                )
                    .chain()
                    .run_if(in_state(AppState::Codex)),
            );
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

fn open_requests(
    mut requests: MessageReader<OpenCodex>,
    mut pending: ResMut<PendingOpen>,
    mut next: ResMut<NextState<AppState>>,
) {
    for request in requests.read() {
        pending.0.clone_from(&request.entry);
        next.set(AppState::Codex);
    }
}

/// Builds the rules book once (it needs the loaded game data for the `{rule.*}` values) and
/// applies the pending sheet and the dev hooks.
fn prepare(
    sim: Res<Sim>,
    store: Res<CodexStore>,
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
}

// ---- screen --------------------------------------------------------------------------------

fn spawn_screen(mut commands: Commands, theme: Res<UiTheme>) {
    let root = commands
        .spawn((
            Name::new("Codex"),
            DespawnOnExit(AppState::Codex),
            Node {
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
    let field = commands
        .spawn((
            Name::new("Search"),
            panel_inset(&theme),
            ChildOf(search_row),
        ))
        .id();
    tweak(&mut commands, field, |node| {
        node.width = Val::Px(420.0);
        node.padding = UiRect::axes(Val::Px(10.0), Val::Px(6.0));
    });
    commands.spawn((
        SearchText,
        Text::new(PLACEHOLDER),
        theme.text_font(false, false, UiType::Body.size()),
        TextColor(style::INK_FADED),
        ChildOf(field),
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

fn handle_escape(
    mut edges: ResMut<KeyEdges>,
    back: Res<CodexReturn>,
    mut next: ResMut<NextState<AppState>>,
) {
    if edges.escape {
        edges.escape = false;
        next.set(back.0);
    }
}

fn handle_buttons(
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
            "codex:back" => next.set(back.0),
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

/// Typing edits the common search field.
fn type_query(mut keys: MessageReader<KeyboardInput>, mut view: ResMut<CodexView>) {
    for key in keys.read() {
        if !key.state.is_pressed() {
            continue;
        }
        match &key.logical_key {
            Key::Character(chars) if chars.chars().all(|c| !c.is_control()) => {
                if view.query.chars().count() < 60 {
                    view.query.push_str(chars);
                }
            }
            Key::Space if view.query.chars().count() < 60 && !view.query.is_empty() => {
                view.query.push(' ');
            }
            Key::Backspace => {
                view.query.pop();
            }
            _ => {}
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
    bars: Query<(Entity, Option<&Children>, Has<FamilyBar>, Has<RulesBar>)>,
    mut tab_buttons: Query<(&Name, &mut TabButton)>,
    mut nodes: Query<&mut Node>,
    mut texts: ParamSet<(
        Query<(&mut Text, &mut TextColor), With<SearchText>>,
        Query<&mut Text, With<CounterText>>,
        Query<&mut Text, With<ResultsText>>,
    )>,
    mut buttons: Query<(&Name, &mut Visibility), With<bevy::ui_widgets::Button>>,
    mut started: Local<bool>,
) {
    let (Ok(list_body), Ok(page_body)) = (list_body.single(), page_body.single()) else {
        return;
    };
    if *started && !view.is_changed() && !store.is_changed() {
        return;
    }
    *started = true;

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

    // Search field text.
    if let Ok((mut text, mut color)) = texts.p0().single_mut() {
        if view.query.is_empty() {
            text.0 = PLACEHOLDER.to_owned();
            color.0 = style::INK_FADED;
        } else {
            text.0 = format!("{}|", view.query);
            color.0 = style::INK;
        }
    }
    let needle = text::fold(view.query.trim());

    // List.
    commands.entity(list_body).despawn_related::<Children>();
    let mut listed = 0;
    let mut rules_listed = 0;
    match view.hub {
        Hub::History => {
            for id in store.ids_in_family(view.family) {
                let Some(entry) = store.entry(id) else {
                    continue;
                };
                if !entry.matches(&needle) {
                    continue;
                }
                listed += 1;
                let mark = if store.is_discovered(id) { "" } else { "✧ " };
                let selected = view.current == Some(Sheet::Entry(id.to_owned()));
                commands.spawn((
                    list_item(
                        &theme,
                        "codex:list",
                        &format!("entry:{id}"),
                        &format!("{mark}{}", entry.title),
                        selected,
                    ),
                    ChildOf(list_body),
                ));
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
    if let Ok(mut counter) = texts.p1().single_mut() {
        counter.0 = match view.hub {
            Hub::History => format!("{} / {} découvertes", store.discovered_count(), store.len()),
            Hub::Rules => format!("{} fiches de règles", book.0.len()),
        };
    }
    if let Ok(mut results) = texts.p2().single_mut() {
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
