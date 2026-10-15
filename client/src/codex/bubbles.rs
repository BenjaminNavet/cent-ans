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
//! Not ported: pinned bubbles (right click, T), `ib:` entity bubbles, the decor bubble of ADR 0219.

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use serde_json::{Value, json};

use super::store::CodexStore;
use super::{CodexOverlay, CodexRoot, OpenCodex, text};
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::ui::kit::RichLink;
use crate::ui::kit::tooltip::{TooltipPanel, TooltipRegistry, TooltipState};

/// Seconds of hover before a bubble opens.
pub const HOVER_DELAY: f32 = 0.35;
/// Seconds outside every link and bubble before the bubbles close.
pub const CLOSE_GRACE: f32 = 0.4;

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
struct BubbleState {
    /// Link being timed: (target, seconds).
    pending: Option<(String, f32)>,
    /// Target of the bubble this system opened, and seconds spent outside.
    open: Option<(String, f32)>,
    opened_total: usize,
    clicked_total: usize,
    last_opened: String,
    registered: usize,
}

pub fn plugin(app: &mut App) {
    app.init_resource::<BubbleState>()
        .register_dump(dump)
        .add_systems(
            Update,
            (register_bubbles, hover_bubbles, click_links).chain(),
        );
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
    for id in store.ids_in_family(None) {
        if let Some(bbcode) = bubble_bbcode(&store, id) {
            registry.register(&format!("{}{id}", text::META_PREFIX), &bbcode);
            count += 1;
        }
    }
    state.registered = count;
}

fn in_panel(
    entity: Entity,
    parents: &Query<&ChildOf>,
    panels: &Query<(), With<TooltipPanel>>,
) -> bool {
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
    panels: Query<(), With<TooltipPanel>>,
    panel_hover: Query<&Hovered, With<TooltipPanel>>,
) {
    let dt = time.delta_secs();
    let hovered_link = links.iter().find_map(|(entity, link, hovered)| {
        let target = text::target_id(&link.0)?;
        (hovered.get() && !in_panel(entity, &parents, &panels)).then(|| target.to_owned())
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
            if elapsed >= HOVER_DELAY {
                let cursor = window
                    .single()
                    .ok()
                    .and_then(Window::cursor_position)
                    .unwrap_or(Vec2::ZERO);
                let key = format!("{}{target}", text::META_PREFIX);
                tooltips.open_at(&registry.resolve(&key), cursor, false);
                store.discover(target);
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
    links: Query<(Entity, &RichLink, &Hovered)>,
    parents: Query<&ChildOf>,
    panels: Query<(), With<TooltipPanel>>,
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
        let target = text::target_id(&link.0)?;
        if !hovered.get() {
            return None;
        }
        let bubble = in_panel(entity, &parents, &panels);
        let on_screen = std::iter::once(entity)
            .chain(parents.iter_ancestors(entity))
            .any(|ancestor| roots.contains(ancestor));
        // The Codex screen handles its own links; bubbles and other texts open the sheet.
        (bubble || !(on_screen || screen)).then(|| target.to_owned())
    });
    if let Some(id) = clicked {
        tooltips.chain.clear();
        state.open = None;
        state.clicked_total += 1;
        requests.write(OpenCodex { entry: Some(id) });
    }
}

fn dump(world: &World) -> (String, Value) {
    let state = world.resource::<BubbleState>();
    let tooltips = world.resource::<TooltipState>();
    (
        "codex_bubbles".to_owned(),
        json!({
            "registered": state.registered,
            "opened": state.opened_total,
            "clicked": state.clicked_total,
            "last_opened": state.last_opened,
            "open_now": !tooltips.chain.is_empty(),
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
