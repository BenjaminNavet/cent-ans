//! Event feed, top right under the bar (lot TS-T6): the news of the last turns grouped in
//! foldable rubrics (War, Diplomacy, Armies and recruitment, Economy, ...), a header with a
//! counter per rubric (`+` / `−`), short entries with the rubric's icon; a click on an entry does
//! what a letter of the Chronicle does (camera on its province, else the sheet of the faction).
//!
//! Choice (docs/wip/ts-t6.md): a separate compact panel rather than a rework of the alert
//! column, which stays the end-of-turn cluster (alerts to act on) while this is the log of what
//! happened. Folded to its header by default; hidden while a window is open (the windows dock on
//! the same side). The rubric of each event kind is data (`data/ui/event_feed.json`); the lines
//! are the ones of the core's `turn_journal` (same source and news filter as the Chronicle). The
//! unfolded state and the folded rubrics are kept in the interface settings.

use std::collections::BTreeSet;

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use game_api::chronicle::{JournalEntry, NewsMode, TurnJournal};
use serde_json::{Value, json};

use super::flow::TurnAdvanced;
use super::{HudSet, HudWidget};
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::camera::FocusCamera;
use crate::campaign::panels::OpenPanel;
use crate::campaign::panels::diplomacy::DiplomacyUi;
use crate::paths;
use crate::settings::{Settings, SettingsFile};
use crate::sim::Sim;
use crate::ui::kit::icons::icon;
use crate::ui::kit::layout::FitZone;
use crate::ui::kit::panel_stack::PanelStack;
use crate::ui::kit::style::UiTier;
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::widgets::{button_small, caption, panel_note, scroll_list};
use crate::ui::kit::{ButtonPressed, UiTheme};

const TOGGLE: &str = "feed:toggle";
const FOLD_PREFIX: &str = "feed:fold:";
const ENTRY_PREFIX: &str = "feed:entry:";
const INDENT: f32 = 20.0;
const ICON_SIZE: f32 = 16.0;

// ---- data ----------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct Category {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub kinds: Vec<String>,
}

/// `data/ui/event_feed.json`.
#[derive(Debug, Clone, PartialEq)]
pub struct FeedConfig {
    pub width: f32,
    pub max_per_category: usize,
    pub keep_turns: u32,
    pub max_chars: usize,
    pub list_max_height: f32,
    pub row_height: f32,
    pub other: String,
    pub world_category: String,
    pub default_folded: Vec<String>,
    pub categories: Vec<Category>,
}

impl Default for FeedConfig {
    fn default() -> Self {
        Self::from_json(&Value::Null)
    }
}

impl FeedConfig {
    pub fn from_json(doc: &Value) -> Self {
        let text = |v: &Value| v.as_str().unwrap_or_default().to_owned();
        let categories: Vec<Category> = doc["categories"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|c| Category {
                id: text(&c["id"]),
                label: text(&c["label"]),
                icon: text(&c["icon"]),
                kinds: c["kinds"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(text)
                    .collect(),
            })
            .filter(|c| !c.id.is_empty())
            .collect();
        let other = doc["other"].as_str().unwrap_or("world").to_owned();
        Self {
            width: doc["width_px"].as_f64().unwrap_or(276.0) as f32,
            max_per_category: doc["max_per_category"].as_u64().unwrap_or(6) as usize,
            keep_turns: doc["keep_turns"].as_u64().unwrap_or(3) as u32,
            max_chars: doc["max_chars"].as_u64().unwrap_or(46) as usize,
            list_max_height: doc["list_max_height_px"].as_f64().unwrap_or(280.0) as f32,
            row_height: doc["row_height_px"].as_f64().unwrap_or(24.0) as f32,
            world_category: doc["world_category"].as_str().unwrap_or(&other).to_owned(),
            default_folded: doc["default_folded"]
                .as_array()
                .into_iter()
                .flatten()
                .map(text)
                .collect(),
            categories,
            other,
        }
    }

    pub fn load() -> Self {
        let doc = paths::find_root()
            .ok()
            .and_then(|root| std::fs::read(root.join("data/ui/event_feed.json")).ok())
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
            .unwrap_or(Value::Null);
        Self::from_json(&doc)
    }

    /// Rubric of an event kind (`other` when unlisted).
    pub fn category_of(&self, kind: &str) -> &str {
        self.categories
            .iter()
            .find(|c| c.kinds.iter().any(|k| k == kind))
            .map_or(self.other.as_str(), |c| c.id.as_str())
    }

    pub fn category(&self, id: &str) -> Option<&Category> {
        self.categories.iter().find(|c| c.id == id)
    }
}

// ---- pure feed -----------------------------------------------------------------------------

/// What a click on an entry does (same rule as the letters of the Chronicle).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryAction {
    Province(String),
    Faction(String),
    None,
}

pub fn entry_action(province: Option<&str>, faction: Option<&str>, player: &str) -> EntryAction {
    match (province, faction) {
        (Some(p), _) if !p.is_empty() => EntryAction::Province(p.to_owned()),
        (_, Some(f)) if !f.is_empty() && f != player => EntryAction::Faction(f.to_owned()),
        _ => EntryAction::None,
    }
}

/// `text` cut to `max_chars` characters with an ellipsis.
pub fn shorten(text: &str, max_chars: usize) -> String {
    let flat: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= max_chars {
        return flat;
    }
    let cut: String = flat.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{}…", cut.trim_end())
}

fn escape(text: &str) -> String {
    text.replace('[', "[lb]").replace(']', "[rb]")
}

#[derive(Debug, Clone, PartialEq)]
pub struct FeedEntry {
    pub id: u32,
    pub category: String,
    pub kind: String,
    pub text: String,
    pub date: String,
    pub turn: u32,
    pub province: Option<String>,
    pub faction: Option<String>,
    pub loss: bool,
}

/// The kept entries, newest first.
#[derive(Resource, Debug, Default, Clone, PartialEq)]
pub struct EventFeed {
    pub entries: Vec<FeedEntry>,
    next_id: u32,
    /// Bumped by every change (rebuild trigger).
    pub revision: u64,
}

impl EventFeed {
    fn push(&mut self, entry: &JournalEntry, world: bool, journal: &TurnJournal, cfg: &FeedConfig) {
        let category = if world {
            cfg.world_category.clone()
        } else {
            cfg.category_of(&entry.kind).to_owned()
        };
        self.next_id += 1;
        self.entries.push(FeedEntry {
            id: self.next_id,
            category,
            kind: entry.kind.clone(),
            text: entry.text.clone(),
            date: journal.date_label.clone(),
            turn: journal.turn,
            province: entry.province.clone(),
            faction: entry.faction.clone(),
            loss: entry.loss,
        });
    }

    /// Adds a resolved turn on top: older turns than `keep_turns` leave, each rubric keeps its
    /// `max_per_category` newest entries.
    pub fn record(&mut self, journal: &TurnJournal, cfg: &FeedConfig) {
        let mut fresh = EventFeed {
            next_id: self.next_id,
            ..default()
        };
        for entry in &journal.entries {
            fresh.push(entry, false, journal, cfg);
        }
        for entry in &journal.world {
            fresh.push(entry, true, journal, cfg);
        }
        self.next_id = fresh.next_id;
        // Newest first; inside a turn the core's order (most important first) is kept.
        fresh.entries.append(&mut self.entries);
        let oldest = journal
            .turn
            .saturating_sub(cfg.keep_turns.saturating_sub(1));
        let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        fresh.entries.retain(|e| {
            let n = seen.entry(e.category.clone()).or_default();
            *n += 1;
            e.turn >= oldest && *n <= cfg.max_per_category
        });
        self.entries = fresh.entries;
        self.revision += 1;
    }

    pub fn count(&self, category: &str) -> usize {
        self.entries
            .iter()
            .filter(|e| e.category == category)
            .count()
    }

    pub fn in_category<'a>(&'a self, category: &'a str) -> impl Iterator<Item = &'a FeedEntry> {
        self.entries.iter().filter(move |e| e.category == category)
    }
}

/// UI state: unfolded panel and folded rubrics (mirrors the settings).
#[derive(Resource, Debug, Default, Clone, PartialEq)]
pub struct FeedUi {
    pub open: bool,
    pub folded: BTreeSet<String>,
    loaded: bool,
    pub last_action: String,
}

impl FeedUi {
    /// State from the interface settings (`None` folded list: the data's default).
    pub fn from_settings(settings: Option<&Settings>, cfg: &FeedConfig) -> Self {
        let folded = settings
            .and_then(|s| s.event_feed_folded.clone())
            .unwrap_or_else(|| cfg.default_folded.clone());
        Self {
            open: settings.is_some_and(|s| s.event_feed_open),
            folded: folded.into_iter().collect(),
            loaded: true,
            last_action: String::new(),
        }
    }

    pub fn toggle_fold(&mut self, id: &str) {
        if !self.folded.remove(id) {
            self.folded.insert(id.to_owned());
        }
    }
}

/// Rows the unfolded list shows (headers of non-empty rubrics + entries of unfolded ones).
pub fn visible_rows(feed: &EventFeed, ui: &FeedUi, cfg: &FeedConfig) -> usize {
    cfg.categories
        .iter()
        .map(|c| {
            let n = feed.count(&c.id);
            if n == 0 {
                0
            } else if ui.folded.contains(&c.id) {
                1
            } else {
                1 + n
            }
        })
        .sum()
}

// ---- systems -------------------------------------------------------------------------------

#[derive(Component)]
struct FeedRoot;

#[derive(Resource)]
struct FeedData(FeedConfig);

pub fn plugin(app: &mut App) {
    app.insert_resource(FeedData(FeedConfig::load()))
        .init_resource::<EventFeed>()
        .init_resource::<FeedUi>()
        .register_dump(dump_feed)
        .add_systems(
            OnEnter(AppState::Campaign),
            (reset_feed, spawn_root).chain(),
        )
        .add_systems(
            Update,
            (record_turns, handle_buttons, rebuild, update_visibility)
                .chain()
                .in_set(HudSet::Ui)
                .run_if(in_state(AppState::Campaign)),
        );
}

fn reset_feed(
    mut feed: ResMut<EventFeed>,
    mut ui: ResMut<FeedUi>,
    cfg: Res<FeedData>,
    settings: Option<Res<Settings>>,
) {
    *feed = EventFeed::default();
    *ui = FeedUi::from_settings(settings.as_deref(), &cfg.0);
}

fn spawn_root(mut commands: Commands, theme: Res<UiTheme>, cfg: Res<FeedData>) {
    commands
        .spawn((
            Name::new("hud:feed"),
            FeedRoot,
            HudWidget,
            Hovered::default(),
            FitZone::Side,
            UiTier::Hud.index(),
            panel_note(&theme),
            DespawnOnExit(AppState::Campaign),
        ))
        .insert(Node {
            position_type: PositionType::Absolute,
            width: Val::Px(cfg.0.width),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(3.0),
            padding: UiRect::all(Val::Px(6.0)),
            ..default()
        });
}

fn record_turns(
    mut advanced: MessageReader<TurnAdvanced>,
    sim: Res<Sim>,
    settings: Option<Res<Settings>>,
    cfg: Res<FeedData>,
    mut feed: ResMut<EventFeed>,
) {
    let mode = settings
        .as_ref()
        .map_or(NewsMode::Interest, |s| NewsMode::parse(&s.news_filter));
    for turn in advanced.read() {
        let (Some(data), Some(state)) = (&sim.data, &sim.state) else {
            continue;
        };
        let journal = game_api::turn_journal(data, state, &sim.player, &turn.events, mode);
        feed.record(&journal, &cfg.0);
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_buttons(
    mut pressed: MessageReader<ButtonPressed>,
    mut ui: ResMut<FeedUi>,
    feed: Res<EventFeed>,
    sim: Res<Sim>,
    mut settings: Option<ResMut<Settings>>,
    mut file: Option<ResMut<SettingsFile>>,
    mut focus: MessageWriter<FocusCamera>,
    mut open_panel: MessageWriter<OpenPanel>,
    mut diplomacy: ResMut<DiplomacyUi>,
) {
    let mut persist = false;
    for press in pressed.read() {
        if press.name == TOGGLE {
            ui.open = !ui.open;
            persist = true;
        } else if let Some(id) = press.name.strip_prefix(FOLD_PREFIX) {
            ui.toggle_fold(id);
            persist = true;
        } else if let Some(id) = press.name.strip_prefix(ENTRY_PREFIX) {
            let Some(entry) = id
                .parse::<u32>()
                .ok()
                .and_then(|id| feed.entries.iter().find(|e| e.id == id))
            else {
                continue;
            };
            match entry_action(
                entry.province.as_deref(),
                entry.faction.as_deref(),
                sim.player.as_str(),
            ) {
                EntryAction::Province(province) => {
                    ui.last_action = format!("province:{province}");
                    focus.write(FocusCamera::Province(province));
                }
                EntryAction::Faction(faction) => {
                    ui.last_action = format!("faction:{faction}");
                    diplomacy.show(&faction);
                    if !diplomacy.open {
                        open_panel.write(OpenPanel::Diplomacy);
                    }
                }
                EntryAction::None => ui.last_action = "none".to_owned(),
            }
        }
    }
    if persist && let (Some(settings), Some(file)) = (settings.as_mut(), file.as_mut()) {
        let inner = settings.bypass_change_detection();
        inner.event_feed_open = ui.open;
        inner.event_feed_folded = Some(ui.folded.iter().cloned().collect());
        file.save(inner);
    }
}

#[allow(clippy::too_many_arguments)]
fn rebuild(
    mut commands: Commands,
    feed: Res<EventFeed>,
    ui: Res<FeedUi>,
    cfg: Res<FeedData>,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
    root: Query<Entity, With<FeedRoot>>,
    mut built: Local<Option<(u64, bool, BTreeSet<String>)>>,
) {
    let Ok(root) = root.single() else { return };
    let key = (feed.revision, ui.open, ui.folded.clone());
    if built.as_ref() == Some(&key) {
        return;
    }
    *built = Some(key);
    let cfg = &cfg.0;
    commands.entity(root).despawn_related::<Children>();
    let total = feed.entries.len();
    let title = if total == 0 {
        "Fil des événements".to_owned()
    } else {
        format!("Fil des événements ({total})")
    };
    let arrow = if ui.open {
        "▾ Replier"
    } else {
        "▸ Déplier"
    };
    commands.entity(root).with_children(|panel| {
        panel
            .spawn(Node {
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                column_gap: Val::Px(6.0),
                ..default()
            })
            .with_children(|header| {
                header.spawn(caption(&theme, &title));
                header.spawn(button_small(&theme, TOGGLE, arrow));
            });
        if !ui.open {
            return;
        }
        if total == 0 {
            panel.spawn(caption(&theme, "Rien à signaler."));
            return;
        }
        let rows = visible_rows(&feed, &ui, cfg) as f32;
        let height = (rows * cfg.row_height + 8.0).clamp(cfg.row_height * 2.0, cfg.list_max_height);
        panel
            .spawn(scroll_list(&theme, "feed:list", height))
            .with_children(|list| {
                for category in &cfg.categories {
                    let count = feed.count(&category.id);
                    if count == 0 {
                        continue;
                    }
                    let folded = ui.folded.contains(&category.id);
                    list.spawn(Node {
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(4.0),
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn(icon(&theme, &assets, &category.icon, ICON_SIZE));
                        let sign = if folded { "+" } else { "−" };
                        row.spawn(button_small(
                            &theme,
                            &format!("{FOLD_PREFIX}{}", category.id),
                            &format!("{sign} {} ({count})", category.label),
                        ));
                    });
                    if folded {
                        continue;
                    }
                    for entry in feed.in_category(&category.id) {
                        list.spawn(Node {
                            padding: UiRect::left(Val::Px(INDENT)),
                            ..default()
                        })
                        .with_children(|row| {
                            row.spawn(button_small(
                                &theme,
                                &format!("{ENTRY_PREFIX}{}", entry.id),
                                &shorten(&entry.text, cfg.max_chars),
                            ))
                            .insert(Tooltip::new(&format!(
                                "{}\n[i]{}[/i]",
                                escape(&entry.text),
                                escape(&entry.date)
                            )));
                        });
                    }
                }
            });
    });
}

/// Hidden while a window is open (windows dock on the same side) or when the HUD is not there.
fn update_visibility(stack: Option<Res<PanelStack>>, mut root: Query<&mut Node, With<FeedRoot>>) {
    let hidden = stack.is_some_and(|s| !s.visible().is_empty());
    let want = if hidden { Display::None } else { Display::Flex };
    for mut node in &mut root {
        if node.display != want {
            node.display = want;
        }
    }
}

fn dump_feed(world: &World) -> (String, Value) {
    let feed = world.resource::<EventFeed>();
    let ui = world.resource::<FeedUi>();
    let cfg = &world.resource::<FeedData>().0;
    let categories: Vec<Value> = cfg
        .categories
        .iter()
        .map(|c| {
            json!({
                "id": c.id,
                "label": c.label,
                "icon": c.icon,
                "icon_resolves": world
                    .get_resource::<UiTheme>()
                    .is_some_and(|t| t.icons.get(&c.icon).is_some()),
                "count": feed.count(&c.id),
                "folded": ui.folded.contains(&c.id),
                "entries": feed.in_category(&c.id).map(|e| json!({
                    "id": e.id, "kind": e.kind, "text": shorten(&e.text, cfg.max_chars),
                    "province": e.province,
                })).collect::<Vec<_>>(),
            })
        })
        .collect();
    let hidden = world
        .get_resource::<PanelStack>()
        .is_some_and(|s| !s.visible().is_empty());
    (
        "event_feed".to_owned(),
        json!({
            "open": ui.open,
            "total": feed.entries.len(),
            "hidden_by_panel": hidden,
            "rows": visible_rows(feed, ui, cfg),
            "last_action": ui.last_action,
            "folded": ui.folded.iter().collect::<Vec<_>>(),
            "categories": categories,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> FeedConfig {
        FeedConfig::from_json(
            &serde_json::from_str(include_str!("../../../../data/ui/event_feed.json")).unwrap(),
        )
    }

    fn entry(kind: &str, text: &str) -> JournalEntry {
        JournalEntry {
            kind: kind.into(),
            genre: String::new(),
            text: text.into(),
            faction: None,
            province: Some("prov_a".into()),
            relevance: "player".into(),
            relevance_label: String::new(),
            public: false,
            loss: false,
            style: None,
        }
    }

    fn journal(turn: u32, entries: Vec<JournalEntry>, world: Vec<JournalEntry>) -> TurnJournal {
        TurnJournal {
            turn,
            date_label: format!("Tour {turn}"),
            entries,
            world,
            letters: Vec::new(),
            expired: Vec::new(),
        }
    }

    #[test]
    fn every_journal_kind_has_a_rubric() {
        let cfg = cfg();
        let genres: Value =
            serde_json::from_str(include_str!("../../../../data/ui/journal_genres.json")).unwrap();
        for genre in genres["genres"].as_array().unwrap() {
            for kind in genre["kinds"].as_array().unwrap() {
                let kind = kind.as_str().unwrap();
                assert!(
                    cfg.categories
                        .iter()
                        .any(|c| c.kinds.iter().any(|k| k == kind)),
                    "kind {kind} of the journal has no feed rubric"
                );
            }
        }
        assert!(cfg.category(&cfg.other).is_some());
        assert!(cfg.category(&cfg.world_category).is_some());
        for id in &cfg.default_folded {
            assert!(cfg.category(id).is_some(), "{id}");
        }
    }

    #[test]
    fn kinds_map_to_the_expected_rubrics() {
        let cfg = cfg();
        assert_eq!(cfg.category_of("war_declared"), "war");
        assert_eq!(cfg.category_of("peace_signed"), "diplomacy");
        assert_eq!(cfg.category_of("recruited"), "armies");
        assert_eq!(cfg.category_of("building_completed"), "economy");
        assert_eq!(cfg.category_of("birth"), "people");
        assert_eq!(cfg.category_of("something_new"), cfg.other);
    }

    #[test]
    fn a_kind_is_in_one_rubric_only() {
        let cfg = cfg();
        let mut seen = std::collections::HashSet::new();
        for c in &cfg.categories {
            for k in &c.kinds {
                assert!(seen.insert(k.clone()), "{k} listed twice");
            }
        }
    }

    #[test]
    fn record_groups_world_news_and_trims_old_turns_and_overflow() {
        let cfg = FeedConfig {
            keep_turns: 2,
            max_per_category: 2,
            ..cfg()
        };
        let mut feed = EventFeed::default();
        feed.record(
            &journal(1, vec![entry("battle", "a")], vec![entry("battle", "far")]),
            &cfg,
        );
        assert_eq!(feed.count("war"), 1);
        assert_eq!(feed.count(&cfg.world_category), 1);
        feed.record(
            &journal(2, vec![entry("battle", "b"), entry("raid", "c")], vec![]),
            &cfg,
        );
        // War is capped to its two newest; turn 1's world news is still kept (keep_turns 2).
        let war: Vec<_> = feed.in_category("war").map(|e| e.text.as_str()).collect();
        assert_eq!(war, ["b", "c"]);
        assert_eq!(feed.count(&cfg.world_category), 1);
        feed.record(&journal(3, vec![], vec![]), &cfg);
        // Turn 1 is out, turn 2 stays.
        assert_eq!(feed.count(&cfg.world_category), 0);
        assert_eq!(feed.count("war"), 2);
        feed.record(&journal(4, vec![], vec![]), &cfg);
        assert_eq!(feed.entries.len(), 0);
    }

    #[test]
    fn fold_state_comes_from_settings_or_the_data_default() {
        let cfg = cfg();
        let fresh = FeedUi::from_settings(None, &cfg);
        assert!(!fresh.open);
        assert_eq!(
            fresh.folded.iter().cloned().collect::<Vec<_>>().len(),
            cfg.default_folded.len()
        );
        let settings = Settings {
            event_feed_open: true,
            event_feed_folded: Some(vec!["war".into()]),
            ..Settings::default()
        };
        let ui = FeedUi::from_settings(Some(&settings), &cfg);
        assert!(ui.open);
        assert_eq!(ui.folded.iter().collect::<Vec<_>>(), ["war"]);
        let mut ui = ui;
        ui.toggle_fold("war");
        ui.toggle_fold("economy");
        assert_eq!(ui.folded.iter().collect::<Vec<_>>(), ["economy"]);
    }

    #[test]
    fn rows_count_headers_and_unfolded_entries() {
        let cfg = cfg();
        let mut feed = EventFeed::default();
        feed.record(
            &journal(
                1,
                vec![
                    entry("battle", "a"),
                    entry("raid", "b"),
                    entry("income", "c"),
                ],
                vec![],
            ),
            &cfg,
        );
        let mut ui = FeedUi::default();
        // war (header + 2) + economy (header + 1).
        assert_eq!(visible_rows(&feed, &ui, &cfg), 5);
        ui.toggle_fold("war");
        assert_eq!(visible_rows(&feed, &ui, &cfg), 3);
    }

    #[test]
    fn entry_click_follows_the_letter_rule() {
        assert_eq!(
            entry_action(Some("prov"), Some("fac_x"), "fac_me"),
            EntryAction::Province("prov".into())
        );
        assert_eq!(
            entry_action(None, Some("fac_x"), "fac_me"),
            EntryAction::Faction("fac_x".into())
        );
        assert_eq!(
            entry_action(None, Some("fac_me"), "fac_me"),
            EntryAction::None
        );
        assert_eq!(entry_action(Some(""), None, "fac_me"), EntryAction::None);
    }

    #[test]
    fn long_texts_are_shortened_with_an_ellipsis() {
        assert_eq!(shorten("court", 10), "court");
        let cut = shorten("Bataille de Crécy remportée par le roi d’Angleterre", 20);
        assert_eq!(cut.chars().count(), 20);
        assert!(cut.ends_with('…'));
        assert_eq!(shorten("a\n  b", 10), "a b");
    }
}
