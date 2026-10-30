//! Books of the campaign UI (XP-R3, ADR 0818): the wiring between the book table
//! (`ui/kit/book.rs`, `data/ui/books.fr.json`) and the panels that host a book's tabs.
//!
//! - A click on a tab button, on the launcher of a book (`hud:open_kingdom`, `hud:open_chronicle`),
//!   on an old launcher kept as an alias (`hud:open_faction`, `hud:open_economy`,
//!   `hud:open_feudal`) or a press of a book key (R: Royaume, H or F5: Chronique) becomes an
//!   [`OpenBook`].
//! - [`OpenBook`] either closes the book (a launcher or key while the book is open) or sends
//!   [`OpenPanel::Hosted`] to the panel that owns the tab, which opens on the right inner tab and
//!   sets the other central panels aside.
//! - The last tab of each book is remembered; a key or launcher reopens on it.
//!
//! Dump section `books`: the open book and tab, the remembered tabs. Dev command: the message
//! `OpenBook` has no script form; scripts `click` the tab buttons (old names kept).

use std::collections::HashMap;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use serde_json::{Value, json};

use super::OpenPanel;
use super::army;
use super::chronicle::ChroniclePanel;
use super::court::{CourtUi, Screen};
use super::diplomacy::DiplomacyUi;
use super::economy::EconomyPanel;
use super::faction::FactionPanel;
use super::feudal::FeudalUi;
use super::tech::TechPanel;
use crate::campaign::armies::roster::Roster;
use crate::campaign::hud::ransom::Ransoms;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::hud::keys::action_down;
use crate::settings::KeyMap;
use crate::sim::Sim;
use crate::ui::kit::ButtonPressed;
use crate::ui::kit::book::{BookDef, Books};
use crate::ui::kit::text_field::{TextInputFocus, keyboard_free};

/// Request to show a book. `tab` is a tab key; without one the remembered (else first) tab is
/// used. With `toggle`, the request closes the book when it is already open on that tab (or, with
/// no tab, on any tab).
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct OpenBook {
    pub book: String,
    pub tab: Option<String>,
    pub toggle: bool,
}

/// Last tab shown in each book (key of the tab).
#[derive(Resource, Default, Debug)]
pub struct BookState {
    pub last: HashMap<String, String>,
}

/// The hosting panels, to know which one is open and on which inner tab.
#[derive(SystemParam)]
struct Hosted<'w> {
    faction: Res<'w, FactionPanel>,
    economy: Res<'w, EconomyPanel>,
    feudal: Res<'w, FeudalUi>,
    chronicle: Res<'w, ChroniclePanel>,
    tech: Res<'w, TechPanel>,
    roster: Res<'w, Roster>,
    recruit: Res<'w, army::PanelState>,
    ransoms: Res<'w, Ransoms>,
    court: Res<'w, CourtUi>,
    diplomacy: Res<'w, DiplomacyUi>,
    sim: Res<'w, Sim>,
}

impl Hosted<'_> {
    /// `(panel, inner tab)` of the open hosting panel, if any.
    fn current(&self) -> Option<(&'static str, String)> {
        current_panel(
            &Refs {
                faction: &self.faction,
                economy: &self.economy,
                feudal: &self.feudal,
                chronicle: &self.chronicle,
                tech: &self.tech,
                roster: &self.roster,
                recruit: &self.recruit,
                ransoms: &self.ransoms,
                court: &self.court,
                diplomacy: &self.diplomacy,
            },
            self.sim.player.as_str(),
        )
    }
}

/// Borrowed states of the hosting panels (shared by the system and the dump).
struct Refs<'a> {
    faction: &'a FactionPanel,
    economy: &'a EconomyPanel,
    feudal: &'a FeudalUi,
    chronicle: &'a ChroniclePanel,
    tech: &'a TechPanel,
    roster: &'a Roster,
    recruit: &'a army::PanelState,
    ransoms: &'a Ransoms,
    court: &'a CourtUi,
    diplomacy: &'a DiplomacyUi,
}

/// The open hosting panel and its inner tab. The central screens exclude each other, so the
/// first one found is the book on view; the docked recruitment screen comes after them.
fn current_panel(refs: &Refs, player: &str) -> Option<(&'static str, String)> {
    if refs.faction.open && refs.faction.faction.as_deref() == Some(player) {
        Some(("faction", refs.faction.tab.clone()))
    } else if refs.economy.open {
        Some(("economy", refs.economy.tab.clone()))
    } else if refs.feudal.open {
        Some(("feudal", String::new()))
    } else if refs.tech.open {
        Some(("tech", String::new()))
    } else if refs.chronicle.open {
        Some(("chronicle", refs.chronicle.tab_key().to_owned()))
    } else if refs.court.screen != Screen::None {
        let tab = if refs.court.screen == Screen::Sheet {
            &refs.court.sheet_tab
        } else {
            &refs.court.council_tab
        };
        Some(("court", tab.clone()))
    } else if refs.diplomacy.open {
        Some(("diplomacy", refs.diplomacy.tab.clone()))
    } else if refs.roster.open {
        Some(("roster", String::new()))
    } else if refs.ransoms.open {
        Some(("ransom", String::new()))
    } else if refs.recruit.recruit.is_some() {
        Some(("recruitment", String::new()))
    } else {
        None
    }
}

pub struct BooksPlugin;

impl Plugin for BooksPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Books::load())
            .init_resource::<BookState>()
            .add_message::<OpenBook>()
            .register_dump(dump_books)
            .add_systems(
                Update,
                (presses, keys, remember, open_book)
                    .chain()
                    .run_if(in_state(AppState::Campaign)),
            );
    }
}

/// Button presses -> `OpenBook`: launchers, old launchers (aliases) and tab buttons.
fn presses(
    mut pressed: MessageReader<ButtonPressed>,
    books: Res<Books>,
    mut open: MessageWriter<OpenBook>,
) {
    for press in pressed.read() {
        let name = press.name.as_str();
        if let Some(book) = books.books.iter().find(|book| book.launcher == name) {
            open.write(OpenBook {
                book: book.id.clone(),
                tab: None,
                toggle: true,
            });
        } else if let Some((book, tab)) = books.resolve_name(name) {
            open.write(OpenBook {
                book: book.id.clone(),
                tab: Some(tab.key.clone()),
                toggle: books.is_alias(name),
            });
        }
    }
}

/// Book keys (R, H / F5): the edge is taken on `pressed` so that scripted keys work too.
fn keys(
    books: Res<Books>,
    keymap: Option<Res<KeyMap>>,
    key_state: Res<ButtonInput<KeyCode>>,
    focus: Option<Res<TextInputFocus>>,
    paused: Option<Res<crate::menu::pause::GamePaused>>,
    mut held: Local<HashMap<String, bool>>,
    mut open: MessageWriter<OpenBook>,
) {
    let free = keyboard_free(focus) && !paused.is_some_and(|paused| paused.0);
    for book in &books.books {
        let down = free
            && action_down(
                keymap.as_deref(),
                &book.action,
                KeyCode::F24,
                &key_state,
                false,
            );
        let was = held.insert(book.id.clone(), down).unwrap_or(false);
        if down && !was {
            open.write(OpenBook {
                book: book.id.clone(),
                tab: None,
                toggle: true,
            });
        }
    }
}

/// Remembers the tab of the open book.
fn remember(books: Res<Books>, hosted: Hosted, mut state: ResMut<BookState>) {
    let Some((panel, inner)) = hosted.current() else {
        return;
    };
    if let Some((book, tab)) = books.hosting_tab(panel, &inner)
        && state.last.get(&book.id) != Some(&tab.key)
    {
        state.last.insert(book.id.clone(), tab.key.clone());
    }
}

/// The legacy message that toggles `panel` closed.
fn close_message(panel: &str, player: &str) -> Option<OpenPanel> {
    Some(match panel {
        "faction" => OpenPanel::Faction(player.to_owned()),
        "economy" => OpenPanel::Economy,
        "feudal" => OpenPanel::Feudal,
        "chronicle" => OpenPanel::Chronicle,
        // The other hosts have no toggle message of their own.
        other => OpenPanel::Close(other.to_owned()),
    })
}

/// Pure choice of what an [`OpenBook`] does: `Err(panel)` closes that open panel, `Ok((panel,
/// inner, key))` opens the tab.
pub fn plan<'a>(
    book: &'a BookDef,
    request: &OpenBook,
    current: Option<(&str, &str)>,
    last: Option<&str>,
) -> Option<Result<(&'a str, &'a str, &'a str), String>> {
    let open_tab = current.and_then(|(panel, inner)| book.tab_of(panel, inner));
    let wanted = request
        .tab
        .as_deref()
        .and_then(|key| book.tab(key))
        .or_else(|| last.and_then(|key| book.tab(key)))
        .or_else(|| book.tabs.first())?;
    if request.toggle
        && let Some(open) = open_tab
        && (request.tab.is_none() || open.key == wanted.key)
    {
        return Some(Err(open.panel.clone()));
    }
    Some(Ok((&wanted.panel, &wanted.inner, &wanted.key)))
}

fn open_book(
    mut requests: MessageReader<OpenBook>,
    books: Res<Books>,
    hosted: Hosted,
    mut state: ResMut<BookState>,
    mut panels: MessageWriter<OpenPanel>,
) {
    for request in requests.read() {
        let Some(book) = books.book(&request.book) else {
            continue;
        };
        let current = hosted.current();
        let last = state.last.get(&book.id).cloned();
        let Some(step) = plan(
            book,
            request,
            current.as_ref().map(|(p, i)| (*p, i.as_str())),
            last.as_deref(),
        ) else {
            continue;
        };
        match step {
            Err(panel) => {
                if let Some(message) = close_message(&panel, hosted.sim.player.as_str()) {
                    panels.write(message);
                }
            }
            Ok((panel, inner, key)) => {
                panels.write(OpenPanel::Hosted {
                    panel: panel.to_owned(),
                    tab: inner.to_owned(),
                });
                state.last.insert(book.id.clone(), key.to_owned());
            }
        }
    }
}

fn dump_books(world: &World) -> (String, Value) {
    let books = world.resource::<Books>();
    let state = world.resource::<BookState>();
    let refs = Refs {
        faction: world.resource::<FactionPanel>(),
        economy: world.resource::<EconomyPanel>(),
        feudal: world.resource::<FeudalUi>(),
        chronicle: world.resource::<ChroniclePanel>(),
        tech: world.resource::<TechPanel>(),
        roster: world.resource::<Roster>(),
        recruit: world.resource::<army::PanelState>(),
        ransoms: world.resource::<Ransoms>(),
        court: world.resource::<CourtUi>(),
        diplomacy: world.resource::<DiplomacyUi>(),
    };
    let player = world.resource::<Sim>().player.as_str().to_owned();
    let open = current_panel(&refs, &player).and_then(|(panel, inner)| {
        books.hosting_tab(panel, &inner).map(
            |(book, tab)| json!({ "book": book.id, "tab": tab.key, "panel": panel }),
        )
    });
    let mut last: Vec<(&String, &String)> = state.last.iter().collect();
    last.sort();
    (
        "books".to_owned(),
        json!({
            "open": open,
            "last": last.iter().map(|(book, tab)| json!({ "book": book, "tab": tab })).collect::<Vec<_>>(),
            "books": books.books.iter().map(|book| json!({
                "id": book.id,
                "title": book.title,
                "launcher": book.launcher,
                "tabs": book.tabs.iter().map(|tab| tab.name.clone()).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kingdom() -> BookDef {
        Books::parse(include_str!("../../../../data/ui/books.fr.json"))
            .and_then(|books| books.book("kingdom").cloned())
            .expect("kingdom book")
    }

    fn request(tab: Option<&str>, toggle: bool) -> OpenBook {
        OpenBook {
            book: "kingdom".to_owned(),
            tab: tab.map(str::to_owned),
            toggle,
        }
    }

    #[test]
    fn key_opens_on_last_tab_then_closes() {
        let book = kingdom();
        let opened = plan(&book, &request(None, true), None, Some("trade")).unwrap();
        assert_eq!(opened, Ok(("economy", "trade", "trade")));
        let closed = plan(
            &book,
            &request(None, true),
            Some(("economy", "trade")),
            Some("trade"),
        )
        .unwrap();
        assert_eq!(closed, Err("economy".to_owned()));
    }

    #[test]
    fn key_without_memory_opens_the_first_tab() {
        let book = kingdom();
        let step = plan(&book, &request(None, true), None, None).unwrap();
        assert_eq!(step, Ok(("faction", "faction:tab:sheet", "sheet")));
    }

    #[test]
    fn tab_click_switches_panel_without_toggling() {
        let book = kingdom();
        let step = plan(
            &book,
            &request(Some("feudal"), false),
            Some(("faction", "faction:tab:sheet")),
            None,
        )
        .unwrap();
        assert_eq!(step, Ok(("feudal", "", "feudal")));
    }

    #[test]
    fn old_launcher_toggles_only_its_own_tab() {
        let book = kingdom();
        let same = plan(
            &book,
            &request(Some("sheet"), true),
            Some(("faction", "faction:tab:sheet")),
            None,
        )
        .unwrap();
        assert_eq!(same, Err("faction".to_owned()));
        let other = plan(
            &book,
            &request(Some("sheet"), true),
            Some(("faction", "faction:tab:budget")),
            None,
        )
        .unwrap();
        assert_eq!(other, Ok(("faction", "faction:tab:sheet", "sheet")));
    }

    /// The launchers of the bar are the five books and the Codex, with the data's names, keys
    /// and titles (no window of the bar escapes the books).
    #[test]
    fn bar_launchers_match_the_books() {
        use crate::campaign::hud::bar::LAUNCHERS;
        let books = Books::parse(include_str!("../../../../data/ui/books.fr.json")).unwrap();
        assert_eq!(LAUNCHERS.len(), books.books.len() + 1);
        for book in &books.books {
            let launcher = LAUNCHERS
                .iter()
                .find(|launcher| launcher.name == book.launcher)
                .unwrap_or_else(|| panic!("no launcher for {}", book.id));
            assert_eq!(launcher.action, book.action, "{}", book.id);
            assert_eq!(launcher.title, book.title, "{}", book.id);
        }
    }

    /// Every tab of every book lands on a panel the book knows how to close.
    #[test]
    fn every_hosting_panel_can_be_closed() {
        let books = Books::parse(include_str!("../../../../data/ui/books.fr.json")).unwrap();
        for book in &books.books {
            for tab in &book.tabs {
                assert!(close_message(&tab.panel, "p").is_some(), "{}", tab.panel);
            }
        }
    }

    /// The book keys (R, H or F5) open the book, but not while a text field has the keyboard.
    #[test]
    fn key_is_ignored_while_typing() {
        let mut app = App::new();
        app.insert_resource(Books::load())
            .insert_resource(KeyMap::defaults())
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<TextInputFocus>()
            .add_message::<OpenBook>()
            .add_systems(Update, keys);
        let field = app.world_mut().spawn_empty().id();
        app.world_mut().resource_mut::<TextInputFocus>().0 = Some(field);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F5);
        app.update();
        assert_eq!(app.world().resource::<Messages<OpenBook>>().len(), 0);
        // Focus dropped and the key released then pressed again: the chronicle book opens.
        app.world_mut().resource_mut::<TextInputFocus>().0 = None;
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::F5);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F5);
        app.update();
        let messages = app.world().resource::<Messages<OpenBook>>();
        assert_eq!(messages.len(), 1);
        assert_eq!(
            messages.iter_current_update_messages().next().unwrap().book,
            "chronicle"
        );
    }
}
