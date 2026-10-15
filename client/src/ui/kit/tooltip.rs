//! Rich tooltips with nested chains (game/scripts/ui/{tooltip_host,tooltip_view,rich_tooltip,
//! plain_tooltip_host}.gd, ADR 0109 « Infobulles en sections, chaîne de bulles à Alt maintenu »).
//!
//! - Put a [`Tooltip`] (BBCode) on any UI entity: hovering it for [`DELAY`] opens a parchment bubble
//!   next to the cursor, [`WIDTH`] px wide, clamped to the screen.
//! - **Alt** (held) freezes the bubble in place (locked). While Alt is held, hovering a `[url=key]`
//!   word inside a bubble opens its child bubble after [`CHILD_DELAY`] (already locked, placed on
//!   the side); hovering another word replaces the branch. Releasing Alt keeps the chain while the
//!   pointer is over a bubble; otherwise it closes. At most [`MAX_LEVELS`] bubbles; older ones
//!   shrink to their first line instead of closing. **Escape** closes everything.
//! - Link targets resolve through [`TooltipRegistry`] (`ib:rule:morale` -> BBCode); unknown keys show a
//!   neutral placeholder. Content (specs, texts) belongs to the screens; this module only renders.
//!
//! The chain logic ([`TooltipChain`]) and the placement ([`place`]) are pure and unit-tested.

use std::collections::HashMap;

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use serde_json::{Value, json};

use super::richtext::{self, RichLink, RichStyle};
use super::style::{self, UiTier};
use super::{UiTheme, rich_text};

/// Hover delay before a native tooltip opens (s).
pub const DELAY: f32 = 0.5;
/// Hover delay of a child bubble while Alt is held (s).
pub const CHILD_DELAY: f32 = 0.12;
/// Grace delay before an unpinned chain with no hovered bubble closes after Alt is released (s).
pub const RELEASE_GRACE: f32 = 0.15;
/// Bubble width (`RichTooltip.WIDTH`).
pub const WIDTH: f32 = 330.0;
/// Most bubbles in a chain.
pub const MAX_LEVELS: usize = 10;
/// Levels older than the last `FULL_LEVELS` shrink to their header.
pub const FULL_LEVELS: usize = 3;
/// Footer of a bubble that has explorable words.
pub const HINT: &str = "Alt : explorer";
const CURSOR_OFFSET: Vec2 = Vec2::new(14.0, 18.0);
const SIDE_GAP: f32 = 6.0;
const SCREEN_MARGIN: f32 = 4.0;

/// BBCode tooltip of a UI entity, shown after [`DELAY`] of hover.
#[derive(Component, Debug, Clone)]
#[require(Hovered)]
pub struct Tooltip {
    pub bbcode: String,
}

impl Tooltip {
    pub fn new(bbcode: &str) -> Self {
        Self {
            bbcode: bbcode.to_owned(),
        }
    }
}

/// Link target -> BBCode of its bubble (`ib:<kind>:<id>` / `cdx:<id>` keys).
#[derive(Resource, Default)]
pub struct TooltipRegistry {
    entries: HashMap<String, String>,
}

impl TooltipRegistry {
    pub fn register(&mut self, key: &str, bbcode: &str) {
        self.entries.insert(key.to_owned(), bbcode.to_owned());
    }

    /// BBCode of `key`, or a neutral placeholder.
    pub fn resolve(&self, key: &str) -> String {
        self.entries.get(key).cloned().unwrap_or_else(|| {
            format!(
                "[i]Aucune fiche pour « {} ».[/i]",
                key.replace(['[', ']'], "")
            )
        })
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

// ---- pure chain ----------------------------------------------------------------------------

/// How a bubble is placed.
#[derive(Debug, Clone, PartialEq)]
pub enum Anchor {
    /// Next to the cursor at open time.
    Cursor(Vec2),
    /// On the side of the bubble with this id.
    Side(u64),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Level {
    pub id: u64,
    pub bbcode: String,
    /// Link key this bubble explains (None for the root).
    pub target: Option<String>,
    pub locked: bool,
    pub collapsed: bool,
    pub anchor: Anchor,
}

/// The stack of open bubbles (root first). Pure data; [`tooltip_driver`] mirrors it into entities.
#[derive(Debug, Default, Clone)]
pub struct TooltipChain {
    pub levels: Vec<Level>,
    next_id: u64,
    /// Ids of bubbles removed since the last drain (their entities must go).
    removed: Vec<u64>,
    /// Ids whose content changed (collapse): their entity is rebuilt.
    dirty: Vec<u64>,
}

impl TooltipChain {
    fn new_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    pub fn is_empty(&self) -> bool {
        self.levels.is_empty()
    }

    /// Opens the root bubble, replacing any chain.
    pub fn open_root(&mut self, bbcode: &str, cursor: Vec2, locked: bool) {
        self.clear();
        let id = self.new_id();
        self.levels.push(Level {
            id,
            bbcode: bbcode.to_owned(),
            target: None,
            locked,
            collapsed: false,
            anchor: Anchor::Cursor(cursor),
        });
    }

    /// Opens the child of level `parent` for `target` (locked); the branch below `parent` is replaced.
    /// Returns false when `parent` does not exist or the branch already shows `target`.
    pub fn open_child(&mut self, parent: usize, target: &str, bbcode: &str) -> bool {
        if parent >= self.levels.len() {
            return false;
        }
        if self
            .levels
            .get(parent + 1)
            .is_some_and(|level| level.target.as_deref() == Some(target))
        {
            return false;
        }
        self.truncate(parent + 1);
        let id = self.new_id();
        let parent_id = self.levels[parent].id;
        self.levels.push(Level {
            id,
            bbcode: bbcode.to_owned(),
            target: Some(target.to_owned()),
            locked: true,
            collapsed: false,
            anchor: Anchor::Side(parent_id),
        });
        if self.levels.len() > MAX_LEVELS {
            let oldest = self.levels.remove(0);
            self.removed.push(oldest.id);
            // The new root keeps its position on the side of nothing: fix its anchor.
            if let Some(first) = self.levels.first_mut()
                && matches!(first.anchor, Anchor::Side(parent) if parent == oldest.id)
            {
                first.anchor = Anchor::Cursor(Vec2::ZERO);
            }
        }
        self.recollapse();
        true
    }

    /// Keeps the first `len` levels.
    pub fn truncate(&mut self, len: usize) {
        while self.levels.len() > len {
            let level = self.levels.pop().expect("non-empty");
            self.removed.push(level.id);
        }
    }

    pub fn clear(&mut self) {
        self.truncate(0);
    }

    /// Freezes every bubble in place.
    pub fn lock_all(&mut self) {
        for level in &mut self.levels {
            if !level.locked {
                level.locked = true;
                self.dirty.push(level.id);
            }
        }
    }

    pub fn locked(&self) -> bool {
        self.levels.first().is_some_and(|level| level.locked)
    }

    fn recollapse(&mut self) {
        let keep_from = self.levels.len().saturating_sub(FULL_LEVELS);
        for (index, level) in self.levels.iter_mut().enumerate() {
            let collapsed = index < keep_from;
            if level.collapsed != collapsed {
                level.collapsed = collapsed;
                self.dirty.push(level.id);
            }
        }
    }

    /// Index of the level with `id`.
    pub fn index_of(&self, id: u64) -> Option<usize> {
        self.levels.iter().position(|level| level.id == id)
    }

    fn drain_removed(&mut self) -> Vec<u64> {
        std::mem::take(&mut self.removed)
    }

    fn drain_dirty(&mut self) -> Vec<u64> {
        std::mem::take(&mut self.dirty)
    }
}

/// Top-left of a bubble of `size` for `anchor`, kept inside `screen` (all logical px).
///
/// `parent` is the rectangle of the parent bubble for [`Anchor::Side`].
pub fn place(anchor: &Anchor, parent: Option<Rect>, size: Vec2, screen: Vec2) -> Vec2 {
    let mut position = match (anchor, parent) {
        (Anchor::Side(_), Some(parent)) => {
            let right = parent.max.x + SIDE_GAP;
            if right + size.x + SCREEN_MARGIN <= screen.x {
                Vec2::new(right, parent.min.y)
            } else {
                Vec2::new(parent.min.x - SIDE_GAP - size.x, parent.min.y)
            }
        }
        (Anchor::Cursor(_) | Anchor::Side(_), _) => {
            let cursor = match anchor {
                Anchor::Cursor(cursor) => *cursor,
                Anchor::Side(_) => Vec2::ZERO,
            };
            let mut at = cursor + CURSOR_OFFSET;
            if at.x + size.x + SCREEN_MARGIN > screen.x {
                at.x = cursor.x - CURSOR_OFFSET.x - size.x;
            }
            if at.y + size.y + SCREEN_MARGIN > screen.y {
                at.y = cursor.y - CURSOR_OFFSET.y - size.y;
            }
            at
        }
    };
    let max = (screen - size - Vec2::splat(SCREEN_MARGIN)).max(Vec2::splat(SCREEN_MARGIN));
    position = position.clamp(Vec2::splat(SCREEN_MARGIN), max);
    position
}

/// First line of a BBCode text (the header kept by collapsed bubbles).
pub fn header_of(bbcode: &str) -> &str {
    bbcode.lines().next().unwrap_or("")
}

// ---- state and entities ----------------------------------------------------------------

#[derive(Resource, Default)]
pub struct TooltipState {
    pub chain: TooltipChain,
    /// Host being timed for the root bubble.
    pending_host: Option<(Entity, f32)>,
    /// Host whose hover keeps an unlocked root open.
    source: Option<Entity>,
    /// Link being timed for a child bubble: (parent level id, target, seconds).
    pending_child: Option<(u64, String, f32)>,
    alt_was_down: bool,
    release_timer: f32,
    /// Opens requested through the API (cursor, bbcode, locked).
    requests: Vec<Request>,
}

enum Request {
    Root {
        bbcode: String,
        cursor: Vec2,
        locked: bool,
    },
    Child {
        parent: usize,
        target: String,
    },
}

impl TooltipState {
    /// Opens a root bubble at `cursor` (API for screens and the gallery).
    pub fn open_at(&mut self, bbcode: &str, cursor: Vec2, locked: bool) {
        self.requests.push(Request::Root {
            bbcode: bbcode.to_owned(),
            cursor,
            locked,
        });
    }

    /// Opens the child bubble of level `parent` for link `target`.
    pub fn open_child_of(&mut self, parent: usize, target: &str) {
        self.requests.push(Request::Child {
            parent,
            target: target.to_owned(),
        });
    }
}

/// A bubble entity.
#[derive(Component, Debug)]
pub struct TooltipPanel {
    pub id: u64,
    pub placed: bool,
    pub rect: Rect,
}

pub struct TooltipPlugin;

impl Plugin for TooltipPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TooltipState>()
            .init_resource::<TooltipRegistry>()
            .add_systems(Update, (tooltip_driver, place_panels).chain());
    }
}

fn logical_cursor(window: &Query<&Window, With<PrimaryWindow>>) -> Option<Vec2> {
    window.single().ok().and_then(Window::cursor_position)
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn tooltip_driver(
    mut commands: Commands,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
    registry: Res<TooltipRegistry>,
    mut state: ResMut<TooltipState>,
    window: Query<&Window, With<PrimaryWindow>>,
    hosts: Query<(Entity, &Tooltip, &Hovered)>,
    links: Query<(Entity, &RichLink, &Hovered)>,
    parents: Query<&ChildOf>,
    panel_hover: Query<(&TooltipPanel, &Hovered)>,
    panel_ids: Query<&TooltipPanel>,
    panel_entities: Query<(Entity, &TooltipPanel)>,
) {
    let dt = time.delta_secs();
    let alt = keys.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]);
    let cursor = logical_cursor(&window).unwrap_or(Vec2::ZERO);
    let state = &mut *state;

    // API requests first.
    for request in std::mem::take(&mut state.requests) {
        match request {
            Request::Root {
                bbcode,
                cursor,
                locked,
            } => {
                state.chain.open_root(&bbcode, cursor, locked);
                state.source = None;
            }
            Request::Child { parent, target } => {
                let bbcode = registry.resolve(&target);
                state.chain.open_child(parent, &target, &bbcode);
            }
        }
    }

    if keys.just_pressed(KeyCode::Escape) && !state.chain.is_empty() {
        state.chain.clear();
    }

    if state.chain.is_empty() {
        // Time the hovered host, open its bubble after DELAY.
        match hosts.iter().find(|(_, _, hovered)| hovered.get()) {
            Some((entity, tooltip, _)) => {
                let elapsed = match state.pending_host {
                    Some((host, elapsed)) if host == entity => elapsed + dt,
                    _ => 0.0,
                };
                state.pending_host = Some((entity, elapsed));
                if elapsed >= DELAY {
                    state.chain.open_root(&tooltip.bbcode, cursor, alt);
                    state.source = Some(entity);
                    state.pending_host = None;
                }
            }
            None => state.pending_host = None,
        }
        state.pending_child = None;
        state.release_timer = 0.0;
    } else {
        // Alt freezes the bubble where it is.
        if alt && !state.alt_was_down {
            state.chain.lock_all();
        }
        let any_panel_hovered = panel_hover.iter().any(|(_, hovered)| hovered.get());
        if !state.chain.locked() {
            // An unlocked root follows its host: closes when the pointer leaves it.
            let still_hovered = state
                .source
                .and_then(|source| hosts.get(source).ok())
                .is_some_and(|(_, _, hovered)| hovered.get());
            if !still_hovered && !any_panel_hovered && state.source.is_some() {
                state.chain.clear();
            }
        } else if !alt && !any_panel_hovered {
            state.release_timer += dt;
            if state.release_timer >= RELEASE_GRACE {
                state.chain.clear();
            }
        } else {
            state.release_timer = 0.0;
        }
        // Child bubbles open on hover of a link inside a bubble, only while Alt is held.
        let hovered_link = if alt {
            links.iter().find_map(|(entity, link, hovered)| {
                if !hovered.get() {
                    return None;
                }
                let owner = std::iter::once(entity)
                    .chain(parents.iter_ancestors(entity))
                    .find_map(|ancestor| panel_ids.get(ancestor).ok())?;
                Some((owner.id, link.0.clone()))
            })
        } else {
            None
        };
        match hovered_link {
            Some((owner_id, target)) => {
                let elapsed = match &state.pending_child {
                    Some((id, pending, elapsed)) if *id == owner_id && *pending == target => {
                        elapsed + dt
                    }
                    _ => 0.0,
                };
                if elapsed >= CHILD_DELAY {
                    if let Some(parent) = state.chain.index_of(owner_id) {
                        let bbcode = registry.resolve(&target);
                        state.chain.open_child(parent, &target, &bbcode);
                    }
                    state.pending_child = Some((owner_id, target, elapsed));
                } else {
                    state.pending_child = Some((owner_id, target, elapsed));
                }
            }
            None => state.pending_child = None,
        }
    }
    state.alt_was_down = alt;

    // Mirror the chain into entities.
    let removed = state.chain.drain_removed();
    let dirty = state.chain.drain_dirty();
    let mut rebuilt: Vec<(u64, Rect)> = Vec::new();
    for (entity, panel) in &panel_entities {
        if removed.contains(&panel.id) {
            commands.entity(entity).despawn();
        } else if dirty.contains(&panel.id) {
            rebuilt.push((panel.id, panel.rect));
            commands.entity(entity).despawn();
        }
    }
    let existing: Vec<u64> = panel_entities
        .iter()
        .filter(|(_, panel)| !removed.contains(&panel.id) && !dirty.contains(&panel.id))
        .map(|(_, panel)| panel.id)
        .collect();
    for level in &state.chain.levels {
        if existing.contains(&level.id) {
            continue;
        }
        let reuse = rebuilt
            .iter()
            .find(|(id, _)| *id == level.id)
            .map(|(_, rect)| *rect);
        spawn_panel(&mut commands, &theme, &assets, level, reuse);
    }
}

fn spawn_panel(
    commands: &mut Commands,
    theme: &UiTheme,
    assets: &AssetServer,
    level: &Level,
    fixed: Option<Rect>,
) {
    let source = if level.collapsed {
        header_of(&level.bbcode).to_owned()
    } else {
        level.bbcode.clone()
    };
    let has_links = !richtext::parse(&source).links().is_empty();
    let style = RichStyle::caption(theme);
    let (left, top) = fixed.map_or((0.0, 0.0), |rect| (rect.min.x, rect.min.y));
    let mut panel = commands.spawn((
        Name::new(format!("Tooltip {}", level.id)),
        TooltipPanel {
            id: level.id,
            placed: fixed.is_some(),
            rect: fixed.unwrap_or(Rect::from_corners(Vec2::ZERO, Vec2::ZERO)),
        },
        Hovered::default(),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(left),
            top: Val::Px(top),
            max_width: Val::Px(WIDTH + 24.0),
            padding: theme.padding("tooltip"),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            ..default()
        },
        theme.sliced("tooltip"),
        UiTier::Tooltip.index(),
        if fixed.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        },
    ));
    panel.with_children(|parent| {
        parent.spawn(rich_text(theme, assets, &source, &style));
        if has_links && !level.collapsed && !level.locked {
            parent.spawn((
                Name::new("TooltipHint"),
                Text::new(HINT),
                theme.text_font(false, true, 13.0),
                TextColor(style::MUTED),
                Node {
                    align_self: AlignSelf::End,
                    ..default()
                },
            ));
        }
    });
}

/// Positions new bubbles once their size is known (after one layout pass).
#[allow(clippy::type_complexity)]
fn place_panels(
    state: Res<TooltipState>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut panels: Query<(&mut TooltipPanel, &mut Node, &mut Visibility, &ComputedNode)>,
) {
    let Ok(window) = window.single() else {
        return;
    };
    let screen = window.size();
    // Parent rectangles by id (placed bubbles only).
    let rects: HashMap<u64, Rect> = panels
        .iter()
        .filter(|(panel, ..)| panel.placed)
        .map(|(panel, ..)| (panel.id, panel.rect))
        .collect();
    for (mut panel, mut node, mut visibility, computed) in &mut panels {
        if panel.placed {
            continue;
        }
        let size = computed.size() * computed.inverse_scale_factor;
        if size.x <= 0.0 || size.y <= 0.0 {
            continue;
        }
        let Some(level) = state.chain.levels.iter().find(|level| level.id == panel.id) else {
            continue;
        };
        let parent = match level.anchor {
            Anchor::Side(parent_id) => rects.get(&parent_id).copied(),
            Anchor::Cursor(_) => None,
        };
        if matches!(level.anchor, Anchor::Side(_)) && parent.is_none() {
            continue; // the parent is not placed yet: wait a frame
        }
        let position = place(&level.anchor, parent, size, screen);
        node.left = Val::Px(position.x);
        node.top = Val::Px(position.y);
        panel.rect = Rect::from_corners(position, position + size);
        panel.placed = true;
        *visibility = Visibility::Inherited;
    }
}

/// Dump section part: open bubbles and chain state.
pub fn dump(world: &World) -> Value {
    let Some(state) = world.get_resource::<TooltipState>() else {
        return json!({});
    };
    let placed = world
        .iter_entities()
        .filter_map(|entity| entity.get::<TooltipPanel>())
        .filter(|panel| panel.placed)
        .count();
    json!({
        "open": state.chain.levels.len(),
        "placed": placed,
        "locked": state.chain.locked(),
        "levels": state.chain.levels.iter().map(|level| json!({
            "target": level.target,
            "collapsed": level.collapsed,
            "header": header_of(&level.bbcode),
        })).collect::<Vec<_>>(),
        "registry": world.get_resource::<TooltipRegistry>().map_or(0, TooltipRegistry::len),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Vec2 = Vec2::new(1280.0, 800.0);

    #[test]
    fn chain_opens_children_and_replaces_branch() {
        let mut chain = TooltipChain::default();
        chain.open_root("[b]Archer[/b]", Vec2::new(100.0, 100.0), false);
        assert!(!chain.locked());
        chain.lock_all();
        assert!(chain.open_child(0, "ib:rule:morale", "Moral"));
        assert!(chain.open_child(1, "ib:rule:fear", "Peur"));
        assert_eq!(chain.levels.len(), 3);
        // Hovering another word of the root replaces the whole branch below it.
        assert!(chain.open_child(0, "ib:rule:armor", "Armure"));
        assert_eq!(chain.levels.len(), 2);
        assert_eq!(chain.levels[1].target.as_deref(), Some("ib:rule:armor"));
        assert_eq!(chain.drain_removed().len(), 2);
        // Same word again: nothing changes.
        assert!(!chain.open_child(0, "ib:rule:armor", "Armure"));
        assert!(!chain.open_child(5, "x", "y"));
    }

    #[test]
    fn old_levels_shrink_and_depth_is_capped() {
        let mut chain = TooltipChain::default();
        chain.open_root("racine", Vec2::ZERO, true);
        for depth in 0..12 {
            chain.open_child(chain.levels.len() - 1, &format!("k{depth}"), "texte");
        }
        assert_eq!(chain.levels.len(), MAX_LEVELS);
        let collapsed = chain.levels.iter().filter(|level| level.collapsed).count();
        assert_eq!(collapsed, MAX_LEVELS - FULL_LEVELS);
        assert!(!chain.levels.last().unwrap().collapsed);
        // The oldest bubbles were dropped, the new first one is anchored to the screen.
        assert!(matches!(chain.levels[0].anchor, Anchor::Cursor(_)));
    }

    #[test]
    fn cursor_placement_flips_and_clamps() {
        let size = Vec2::new(330.0, 120.0);
        let normal = place(&Anchor::Cursor(Vec2::new(100.0, 100.0)), None, size, SCREEN);
        assert_eq!(normal, Vec2::new(114.0, 118.0));
        let flipped = place(
            &Anchor::Cursor(Vec2::new(1200.0, 750.0)),
            None,
            size,
            SCREEN,
        );
        assert!(flipped.x + size.x <= SCREEN.x && flipped.y + size.y <= SCREEN.y);
        assert!(flipped.x < 1200.0 && flipped.y < 750.0);
        let corner = place(&Anchor::Cursor(Vec2::new(-50.0, -50.0)), None, size, SCREEN);
        assert!(corner.x >= 0.0 && corner.y >= 0.0);
    }

    #[test]
    fn side_placement_goes_right_then_left() {
        let size = Vec2::new(330.0, 200.0);
        let parent = Rect::from_corners(Vec2::new(100.0, 50.0), Vec2::new(430.0, 250.0));
        let right = place(&Anchor::Side(1), Some(parent), size, SCREEN);
        assert_eq!(right, Vec2::new(436.0, 50.0));
        let near_edge = Rect::from_corners(Vec2::new(900.0, 50.0), Vec2::new(1230.0, 250.0));
        let left = place(&Anchor::Side(1), Some(near_edge), size, SCREEN);
        assert!(left.x + size.x <= near_edge.min.x);
        // Tall bubble near the bottom stays on screen.
        let low = Rect::from_corners(Vec2::new(100.0, 700.0), Vec2::new(430.0, 790.0));
        let clamped = place(&Anchor::Side(1), Some(low), size, SCREEN);
        assert!(clamped.y + size.y <= SCREEN.y);
    }

    #[test]
    fn registry_resolves_with_placeholder() {
        let mut registry = TooltipRegistry::default();
        registry.register("ib:rule:morale", "Le moral.");
        assert_eq!(registry.resolve("ib:rule:morale"), "Le moral.");
        assert!(registry.resolve("ib:rule:nope").contains("Aucune fiche"));
    }

    #[test]
    fn header_is_first_line() {
        assert_eq!(header_of("[b]Titre[/b]\ncorps"), "[b]Titre[/b]");
        assert_eq!(header_of(""), "");
    }
}
