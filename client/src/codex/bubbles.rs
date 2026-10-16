//! Hover bubbles of Codex keywords (port of game/scripts/codex/codex_bubbles.gd, simplified).
//!
//! - Hovering a `cdx:` link of any text for 0,35 s opens a bubble near the pointer (title,
//!   category, summary) through the kit's tooltip chain ([`TooltipState`]); the pointer may
//!   enter the bubble; leaving the link and the bubbles closes them after 0,4 s of grace;
//!   Escape closes them (kit). Alt chains child bubbles (kit), whose content comes from the
//!   [`TooltipRegistry`] keys `cdx:<id>` filled here for every entry.
//! - A bubble opened discovers its entry.
//! - A left click on a `cdx:` link of a bubble (or of any text outside the Codex screen) opens
//!   the full sheet with [`OpenCodex`].
//!
//! - A child bubble of a `cdx:` link sits beside its parent on the link's line
//!   ([`super::bubble_layout::place_beside`], `keyword_y`), below any other open bubble it would
//!   cover; when nothing fits it keeps the kit's place.
//!
//! - `ib:<kind>:<id>` links (entities and rules, [`text::ib_key`]) open the bubble registered under
//!   their key in the [`TooltipRegistry`] (nothing when unknown); every Codex entry that describes
//!   a unit, building, technology, resource, trait or skill registers one (unless a screen already
//!   registered a richer one). A click opens the sheet of the entity (`entry_for_entity`).
//!
//! Pinned bubbles (right click, T) are in [`super::pinned`], the decor bubble of ADR 0219 in
//! [`super::decor`].

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::text::{ComputedTextBlock, TextLayoutInfo};
use bevy::ui::UiGlobalTransform;
use bevy::window::PrimaryWindow;
use serde_json::{Value, json};

use super::pinned::AnyBubble;
use super::store::CodexStore;
use super::{CodexOverlay, CodexRoot, OpenCodex, text};
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::ui::kit::RichLink;
use crate::ui::kit::tooltip::{Anchor, TooltipPanel, TooltipRegistry, TooltipState};

/// Seconds of hover before a bubble opens.
pub const HOVER_DELAY: f32 = 0.35;
/// Seconds outside every link and bubble before the bubbles close.
pub const CLOSE_GRACE: f32 = 0.4;

/// Registry key of the bubble a link target opens: `cdx:<id>` or `ib:<kind>:<id>`.
pub fn bubble_key(target: &str) -> Option<&str> {
    text::target_id(target)
        .map(|_| target)
        .or_else(|| text::ib_key(target))
}

/// BBCode of the bubble of `id`: title, category (and era), summary with its links.
pub fn bubble_bbcode(store: &CodexStore, id: &str) -> Option<String> {
    let entry = store.entry(id)?;
    let mut meta = entry.category_label();
    let era = entry.era_label();
    if !era.is_empty() {
        meta = format!("{meta} · {era}");
    }
    let summary = text::format(store, &entry.summary);
    Some(format!("[b]{}[/b]\n[i]{meta}[/i]\n{summary}", entry.title))
}

#[derive(Resource, Default)]
pub(super) struct BubbleState {
    /// Link being timed: (target, seconds).
    pending: Option<(String, f32)>,
    /// Target of the bubble this system opened, and seconds spent outside.
    open: Option<(String, f32)>,
    opened_total: usize,
    clicked_total: usize,
    last_opened: String,
    registered: usize,
    /// Key of the bubble opened by the proof hook (`CENT_ANS_CODEX_PROOF`).
    proof_key: Option<String>,
    /// `ib:` entity bubbles registered from the Codex entries.
    registered_ib: usize,
    /// `ib:` bubbles opened (hover), keys read (links lose their underline).
    ib_opened: usize,
    ib_seen: std::collections::BTreeSet<String>,
    /// Child bubbles moved beside their link's line, and the last line used.
    aligned_total: usize,
    last_keyword_y: Option<f32>,
}

impl BubbleState {
    /// Key of the hover bubble this module opened, while it is up.
    pub(super) fn open_key(&self) -> Option<&str> {
        self.open
            .as_ref()
            .map(|(key, _)| key.as_str())
            .or(self.proof_key.as_deref())
    }

    /// The hover bubble was closed or pinned elsewhere.
    pub(super) fn forget_open(&mut self) {
        self.open = None;
        self.proof_key = None;
        self.pending = None;
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<BubbleState>()
        .register_dump(dump)
        .add_systems(
            Update,
            (register_bubbles, hover_bubbles, click_links).chain(),
        )
        .add_systems(Update, proof_open.run_if(in_state(AppState::Campaign)))
        // After the kit placed the new bubbles (Update), before this frame's layout.
        .add_systems(
            PostUpdate,
            align_child_bubbles.before(bevy::ui::UiSystems::Layout),
        );
}

/// Proof hook: `CENT_ANS_CODEX_PROOF=<cdx id or ib key>` opens that bubble at (320, 180) once the
/// campaign runs (scripts cannot hover); a `key KeyT` of the script then pins it.
fn proof_open(
    mut done: Local<bool>,
    mut frames: Local<u32>,
    registry: Res<TooltipRegistry>,
    store: Res<CodexStore>,
    mut tooltips: ResMut<TooltipState>,
    mut state: ResMut<BubbleState>,
) {
    if *done || store.is_empty() {
        return;
    }
    let Ok(wanted) = std::env::var("CENT_ANS_CODEX_PROOF") else {
        *done = true;
        return;
    };
    *frames += 1;
    if *frames < 30 {
        return;
    }
    *done = true;
    let key = if wanted.starts_with(text::IB_PREFIX) || wanted.starts_with(text::META_PREFIX) {
        wanted
    } else {
        format!("{}{wanted}", text::META_PREFIX)
    };
    tooltips.open_at(&registry.resolve(&key), Vec2::new(320.0, 180.0), false);
    state.proof_key = Some(key.clone());
    state.last_opened = key;
    state.opened_total += 1;
}

/// Fills the registry with the bubble of every entry (colours follow the read state).
fn register_bubbles(
    store: Res<CodexStore>,
    mut registry: ResMut<TooltipRegistry>,
    mut state: ResMut<BubbleState>,
) {
    if !store.is_changed() || store.is_empty() {
        return;
    }
    let mut count = 0;
    let mut ib_count = 0;
    for id in store.ids_in_family(None) {
        let Some(bbcode) = bubble_bbcode(&store, id) else {
            continue;
        };
        registry.register(&format!("{}{id}", text::META_PREFIX), &bbcode);
        count += 1;
        let entity = store.entry(id).map_or("", |entry| entry.entity.as_str());
        if let Some(kind) = text::entity_kind(entity) {
            let key = format!("{}{kind}:{entity}", text::IB_PREFIX);
            if !registry.contains(&key) {
                registry.register(&key, &bbcode);
                ib_count += 1;
            }
        }
    }
    state.registered = count;
    state.registered_ib += ib_count;
}

fn in_panel(entity: Entity, parents: &Query<&ChildOf>, panels: &Query<(), AnyBubble>) -> bool {
    std::iter::once(entity)
        .chain(parents.iter_ancestors(entity))
        .any(|ancestor| panels.contains(ancestor))
}

#[allow(clippy::too_many_arguments)]
fn hover_bubbles(
    time: Res<Time<Real>>,
    registry: Res<TooltipRegistry>,
    mut store: ResMut<CodexStore>,
    mut tooltips: ResMut<TooltipState>,
    mut state: ResMut<BubbleState>,
    window: Query<&Window, With<PrimaryWindow>>,
    links: Query<(Entity, &RichLink, &Hovered)>,
    parents: Query<&ChildOf>,
    panels: Query<(), AnyBubble>,
    panel_hover: Query<&Hovered, AnyBubble>,
) {
    let dt = time.delta_secs();
    let hovered_link = links.iter().find_map(|(entity, link, hovered)| {
        let key = bubble_key(&link.0)?;
        (hovered.get() && !in_panel(entity, &parents, &panels)).then(|| key.to_owned())
    });
    let on_panel = panel_hover.iter().any(Hovered::get);
    let state = &mut *state;

    // Open after the delay (only when no bubble is up yet).
    match (&hovered_link, tooltips.chain.is_empty()) {
        (Some(target), true) => {
            let elapsed = match &state.pending {
                Some((pending, elapsed)) if pending == target => elapsed + dt,
                _ => 0.0,
            };
            let known = text::target_id(target).is_some() || registry.contains(target);
            if elapsed >= HOVER_DELAY && !known {
                // An `ib:` link nobody registered: no bubble (as in Godot), try again after a move.
                state.pending = Some((target.clone(), f32::NEG_INFINITY));
            } else if elapsed >= HOVER_DELAY {
                let cursor = window
                    .single()
                    .ok()
                    .and_then(Window::cursor_position)
                    .unwrap_or(Vec2::ZERO);
                tooltips.open_at(&registry.resolve(target), cursor, false);
                if let Some(id) = text::target_id(target) {
                    store.discover(id);
                } else {
                    state.ib_seen.insert(target.clone());
                    state.ib_opened += 1;
                }
                state.open = Some((target.clone(), 0.0));
                state.opened_total += 1;
                state.last_opened.clone_from(target);
                state.pending = None;
            } else {
                state.pending = Some((target.clone(), elapsed));
            }
        }
        _ => state.pending = None,
    }

    // Close after the grace once the pointer left the link and the bubbles.
    if let Some((target, outside)) = state.open.take()
        && !tooltips.chain.is_empty()
    {
        let near = on_panel || hovered_link.as_deref() == Some(target.as_str());
        let outside = if near { 0.0 } else { outside + dt };
        if outside >= CLOSE_GRACE {
            tooltips.chain.clear();
        } else {
            state.open = Some((target, outside));
        }
    }
}

/// Left click on a `cdx:` link of a bubble, or outside the Codex screen: opens the sheet.
#[allow(clippy::too_many_arguments)]
fn click_links(
    mouse: Res<ButtonInput<MouseButton>>,
    state_now: Res<State<AppState>>,
    overlay: Res<CodexOverlay>,
    store: Res<CodexStore>,
    links: Query<(Entity, &RichLink, &Hovered)>,
    parents: Query<&ChildOf>,
    panels: Query<(), AnyBubble>,
    roots: Query<(), With<CodexRoot>>,
    mut tooltips: ResMut<TooltipState>,
    mut state: ResMut<BubbleState>,
    mut requests: MessageWriter<OpenCodex>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let screen = *state_now.get() == AppState::Codex || overlay.open;
    let clicked = links.iter().find_map(|(entity, link, hovered)| {
        let key = bubble_key(&link.0)?;
        if !hovered.get() {
            return None;
        }
        // The sheet behind the link: a Codex entry, or the entry of the entity of an `ib:` link.
        let sheet = text::target_id(key).map(str::to_owned).or_else(|| {
            let entity = text::ib_entity(key)?;
            store.entry_for_entity(entity).map(str::to_owned)
        })?;
        let bubble = in_panel(entity, &parents, &panels);
        let on_screen = std::iter::once(entity)
            .chain(parents.iter_ancestors(entity))
            .any(|ancestor| roots.contains(ancestor));
        // The Codex screen handles its own links; bubbles and other texts open the sheet.
        (bubble || !(on_screen || screen)).then_some(sheet)
    });
    if let Some(id) = clicked {
        tooltips.chain.clear();
        state.open = None;
        state.clicked_total += 1;
        requests.write(OpenCodex { entry: Some(id) });
    }
}

/// Top of the first line of `span` inside its text block, in logical pixels (screen space).
fn link_line_y(
    span: Entity,
    parents: &Query<&ChildOf>,
    texts: &Query<(
        &ComputedTextBlock,
        &TextLayoutInfo,
        &ComputedNode,
        &UiGlobalTransform,
    )>,
) -> Option<f32> {
    let root = std::iter::once(span)
        .chain(parents.iter_ancestors(span))
        .find(|entity| texts.contains(*entity))?;
    let (block, layout, node, transform) = texts.get(root).ok()?;
    let index = block.entities().iter().position(|e| e.entity == span)? as u32;
    let run = layout
        .run_geometry
        .iter()
        .filter(|run| run.section_index == index)
        .min_by(|a, b| a.bounds.min.y.total_cmp(&b.bounds.min.y))?;
    let top = transform.translation.y - node.size().y * 0.5;
    Some((top + run.bounds.min.y) * node.inverse_scale_factor)
}

/// Child bubbles of Codex links: beside the parent, on the line of the link that opened them.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn align_child_bubbles(
    tooltips: Res<TooltipState>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut panels: Query<(Entity, &mut TooltipPanel, &mut Node)>,
    links: Query<(Entity, &RichLink)>,
    parents: Query<&ChildOf>,
    texts: Query<(
        &ComputedTextBlock,
        &TextLayoutInfo,
        &ComputedNode,
        &UiGlobalTransform,
    )>,
    mut state: ResMut<BubbleState>,
    mut done: Local<std::collections::HashSet<u64>>,
) {
    done.retain(|id| tooltips.chain.levels.iter().any(|level| level.id == *id));
    let Ok(window) = window.single() else {
        return;
    };
    let margin = 8.0;
    let area = Rect::from_corners(Vec2::splat(margin), window.size() - Vec2::splat(margin));
    for level in &tooltips.chain.levels {
        let (Anchor::Side(parent_id), Some(target)) = (&level.anchor, level.target.as_deref())
        else {
            continue;
        };
        if done.contains(&level.id) || bubble_key(target).is_none() {
            continue;
        }
        let placed: Vec<(Entity, u64, Rect)> = panels
            .iter()
            .filter(|(_, panel, _)| panel.placed)
            .map(|(entity, panel, _)| (entity, panel.id, panel.rect))
            .collect();
        let Some(&(parent_entity, _, parent_rect)) =
            placed.iter().find(|(_, id, _)| id == parent_id)
        else {
            continue;
        };
        let Some(&(child_entity, _, child_rect)) = placed.iter().find(|(_, id, _)| *id == level.id)
        else {
            continue;
        };
        done.insert(level.id);
        let keyword_y = links
            .iter()
            .filter(|(_, link)| link.0 == target)
            .filter(|(entity, _)| {
                parents
                    .iter_ancestors(*entity)
                    .any(|ancestor| ancestor == parent_entity)
            })
            .find_map(|(entity, _)| link_line_y(entity, &parents, &texts))
            .unwrap_or(parent_rect.min.y);
        let avoid: Vec<Rect> = placed
            .iter()
            .filter(|(_, id, _)| id != parent_id && *id != level.id)
            .map(|(_, _, rect)| *rect)
            .collect();
        let Some(position) = super::bubble_layout::place_beside(
            child_rect.size(),
            parent_rect,
            keyword_y,
            area,
            &avoid,
            super::bubble_layout::GAP,
        ) else {
            continue;
        };
        if let Ok((_, mut panel, mut node)) = panels.get_mut(child_entity) {
            node.left = Val::Px(position.x);
            node.top = Val::Px(position.y);
            panel.rect = Rect::from_corners(position, position + child_rect.size());
            state.aligned_total += 1;
            state.last_keyword_y = Some(keyword_y);
        }
    }
}

fn dump(world: &World) -> (String, Value) {
    let state = world.resource::<BubbleState>();
    let tooltips = world.resource::<TooltipState>();
    (
        "codex_bubbles".to_owned(),
        json!({
            "registered": state.registered,
            "registered_ib": state.registered_ib,
            "ib_opened": state.ib_opened,
            "ib_seen": state.ib_seen,
            "opened": state.opened_total,
            "clicked": state.clicked_total,
            "last_opened": state.last_opened,
            "open_now": !tooltips.chain.is_empty(),
            "children_aligned": state.aligned_total,
            "last_keyword_y": state.last_keyword_y,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bubble_has_title_category_era_and_linked_summary() {
        let store = CodexStore::parse(
            r#"{"entries":[
              {"id":"cdx_a","title":"Crécy","category":"bataille","era":{"from":"1346","to":"1346"},"summary":"Victoire des [[cdx_b|archers]]."},
              {"id":"cdx_b","title":"Archers","category":"unite","summary":"s"}]}"#
                .as_bytes(),
        )
        .unwrap();
        let bubble = bubble_bbcode(&store, "cdx_a").unwrap();
        assert!(bubble.starts_with("[b]Crécy[/b]\n[i]"), "{bubble}");
        assert!(bubble.contains("1346"));
        assert!(bubble.contains("[url=cdx:cdx_b]"));
        assert!(bubble_bbcode(&store, "cdx_zz").is_none());
    }
}
