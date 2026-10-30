//! « Livre à onglets » (XP-R3, ADR 0818): the reusable frame of the five books.
//!
//! A book is data (`data/ui/books.fr.json`): a title and a row of tabs, each tab pointing at an
//! existing view (a panel and one of its inner tabs). This module holds the pure part: the
//! [`Books`] table (parsing, lookups, alias resolution, the tab to land on) and [`book_bar`], the
//! row of tabs a hosting panel draws in place of its own tab bar. How a tab opens its panel lives
//! in `campaign/panels/books.rs`. No game rule here.

use bevy::prelude::*;
use serde_json::Value;

use super::{UiTheme, widgets};

/// Width shared by the panels that host a book, so that the page does not jump between tabs.
pub const BOOK_WIDTH: f32 = 720.0;

/// One tab of a book.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookTab {
    pub key: String,
    /// Name of the tab button (agent `click`), kept from the old windows.
    pub name: String,
    pub label: String,
    /// Hosting panel (`faction`, `economy`, `feudal`, `tech`, `chronicle`, `roster`,
    /// `recruitment`, `ransom`, `court`, `diplomacy`).
    pub panel: String,
    /// Inner tab of that panel (empty when it has none).
    pub inner: String,
    /// Condition under which the tab exists (`assembly_house`), if any.
    pub needs: Option<String>,
}

/// One book.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookDef {
    pub id: String,
    pub title: String,
    pub action: String,
    pub launcher: String,
    pub icon: String,
    pub detail: String,
    pub tabs: Vec<BookTab>,
}

impl BookDef {
    pub fn tab(&self, key: &str) -> Option<&BookTab> {
        self.tabs.iter().find(|tab| tab.key == key)
    }

    /// The tab showing `inner` of `panel`.
    pub fn tab_of(&self, panel: &str, inner: &str) -> Option<&BookTab> {
        self.tabs
            .iter()
            .find(|tab| tab.panel == panel && tab.inner == inner)
    }
}

/// An old button name that now opens a tab of a book.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookAlias {
    pub name: String,
    pub book: String,
    pub tab: String,
}

/// The table of books (resource).
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct Books {
    pub books: Vec<BookDef>,
    pub aliases: Vec<BookAlias>,
    pub registry_empty: String,
}

fn text(node: &Value, key: &str) -> String {
    node.get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

impl Books {
    pub fn parse(source: &str) -> Option<Self> {
        let root: Value = serde_json::from_str(source).ok()?;
        let books = root
            .get("books")?
            .as_array()?
            .iter()
            .map(|book| BookDef {
                id: text(book, "id"),
                title: text(book, "title"),
                action: text(book, "action"),
                launcher: text(book, "launcher"),
                icon: text(book, "icon"),
                detail: text(book, "detail"),
                tabs: book
                    .get("tabs")
                    .and_then(Value::as_array)
                    .map(|tabs| {
                        tabs.iter()
                            .map(|tab| BookTab {
                                key: text(tab, "key"),
                                name: text(tab, "name"),
                                label: text(tab, "label"),
                                panel: text(tab, "panel"),
                                inner: text(tab, "inner"),
                                needs: tab.get("needs").and_then(Value::as_str).map(str::to_owned),
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            })
            .collect();
        let aliases = root
            .get("aliases")
            .and_then(Value::as_array)
            .map(|list| {
                list.iter()
                    .map(|alias| BookAlias {
                        name: text(alias, "name"),
                        book: text(alias, "book"),
                        tab: text(alias, "tab"),
                    })
                    .collect()
            })
            .unwrap_or_default();
        Some(Self {
            books,
            aliases,
            registry_empty: root
                .get("registry")
                .map(|r| text(r, "empty"))
                .unwrap_or_default(),
        })
    }

    /// Reads `data/ui/books.fr.json` (the repository copy when found, else the one compiled in).
    pub fn load() -> Self {
        crate::paths::find_root()
            .ok()
            .and_then(|root| std::fs::read_to_string(root.join("data/ui/books.fr.json")).ok())
            .and_then(|source| Self::parse(&source))
            .or_else(|| Self::parse(include_str!("../../../../data/ui/books.fr.json")))
            .unwrap_or_default()
    }

    pub fn book(&self, id: &str) -> Option<&BookDef> {
        self.books.iter().find(|book| book.id == id)
    }

    /// The book and tab showing `inner` of `panel` (a panel may host tabs of two books).
    pub fn hosting_tab(&self, panel: &str, inner: &str) -> Option<(&BookDef, &BookTab)> {
        self.books
            .iter()
            .find_map(|book| book.tab_of(panel, inner).map(|tab| (book, tab)))
    }

    /// The first book hosting `panel`, if any.
    pub fn hosting(&self, panel: &str) -> Option<&BookDef> {
        self.books
            .iter()
            .find(|book| book.tabs.iter().any(|tab| tab.panel == panel))
    }

    /// The book and tab a button name stands for: an old launcher alias, or a tab button name.
    pub fn resolve_name(&self, name: &str) -> Option<(&BookDef, &BookTab)> {
        if let Some(alias) = self.aliases.iter().find(|alias| alias.name == name) {
            let book = self.book(&alias.book)?;
            return Some((book, book.tab(&alias.tab)?));
        }
        self.books.iter().find_map(|book| {
            book.tabs
                .iter()
                .find(|tab| tab.name == name)
                .map(|tab| (book, tab))
        })
    }

    /// True when `name` is an old launcher kept as an alias (it has no button any more).
    pub fn is_alias(&self, name: &str) -> bool {
        self.aliases.iter().any(|alias| alias.name == name)
    }

    /// The book whose key action is `action`.
    pub fn by_action(&self, action: &str) -> Option<&BookDef> {
        self.books.iter().find(|book| book.action == action)
    }
}

/// Row of tabs of `book`, `active` being the name of the selected tab; tabs whose `needs`
/// condition is not in `have` are left out; `relabel` replaces the label of a tab by name (a
/// count, say). The row wraps when it is wider than the page.
pub fn book_bar(
    parent: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    book: &BookDef,
    active: &str,
    have: &[&str],
    relabel: &[(&str, String)],
) {
    let shown: Vec<&BookTab> = book
        .tabs
        .iter()
        .filter(|tab| tab.needs.as_deref().is_none_or(|need| have.contains(&need)))
        .collect();
    let pairs: Vec<(&str, &str)> = shown
        .iter()
        .map(|tab| {
            let label = relabel
                .iter()
                .find(|(name, _)| *name == tab.name)
                .map_or(tab.label.as_str(), |(_, label)| label.as_str());
            (tab.name.as_str(), label)
        })
        .collect();
    let selected = shown.iter().position(|tab| tab.name == active).unwrap_or(0);
    parent.spawn(widgets::tab_bar_wrapped(
        theme,
        &format!("book:{}", book.id),
        &pairs,
        selected,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn books() -> Books {
        Books::parse(include_str!("../../../../data/ui/books.fr.json")).expect("books parse")
    }

    #[test]
    fn data_parses_with_five_books() {
        let books = books();
        assert_eq!(books.books.len(), 5);
        assert!(books.book("kingdom").is_some_and(|b| b.tabs.len() == 10));
        assert!(books.book("court").is_some_and(|b| b.tabs.len() == 9));
        assert!(books.book("chronicle").is_some_and(|b| b.tabs.len() == 5));
    }

    #[test]
    fn tab_names_are_unique_and_hosted() {
        let books = books();
        let mut names: Vec<&str> = books
            .books
            .iter()
            .flat_map(|b| b.tabs.iter().map(|t| t.name.as_str()))
            .collect();
        let total = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), total, "duplicate tab name");
        for book in &books.books {
            for tab in &book.tabs {
                assert_eq!(
                    books
                        .hosting_tab(&tab.panel, &tab.inner)
                        .map(|(b, _)| &b.id),
                    Some(&book.id)
                );
            }
        }
    }

    #[test]
    fn old_launchers_resolve_to_tabs() {
        let books = books();
        for (old, key) in [
            ("hud:open_faction", "sheet"),
            ("hud:open_economy", "overview"),
            ("hud:open_feudal", "feudal"),
        ] {
            assert!(books.is_alias(old));
            let (book, tab) = books.resolve_name(old).expect(old);
            assert_eq!((book.id.as_str(), tab.key.as_str()), ("kingdom", key));
        }
        let (book, tab) = books.resolve_name("economy:tab:trade").unwrap();
        assert_eq!(
            (book.id.as_str(), tab.panel.as_str()),
            ("kingdom", "economy")
        );
        assert!(books.resolve_name("nothing").is_none());
    }

    #[test]
    fn tab_of_finds_the_inner_tab() {
        let books = books();
        let book = books.book("chronicle").unwrap();
        assert_eq!(book.tab_of("chronicle", "journal").unwrap().key, "journal");
        assert!(book.tab_of("faction", "x").is_none());
    }
}
