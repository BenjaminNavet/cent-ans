//! Pinned Codex bubbles (port of the pinning half of game/scripts/codex/codex_bubbles.gd).
//!
//! A pinned bubble is an entity of its own, independent of the kit's tooltip chain: it stays open
//! when the pointer leaves, while ordinary hover bubbles keep opening and closing around it.
//!
//! - **Right click** on a `cdx:` / `ib:` link opens its bubble pinned at the pointer; on an open
//!   hover bubble it pins the whole chain where it stands; on a pinned bubble it detaches
//!   (closes) it.
//! - **T** (`codex_pin_tooltip`) pins, by priority: the bubble of the link being hovered (not yet
//!   open), the open hover chain, the plain tooltip of the hovered control (text converted with
//!   auto-links). The key is consumed only when something was pinned (T also opens the tech tree).
//! - **Left click** on the body of a hover or pinned bubble opens the sheet of its entry; links
//!   inside open their own sheet (see [`super::bubbles`]).
//! - Pinned bubbles close with the screen change (campaign, battle, Codex, menu).
//!
//! Not ported: parent tracking between pinned bubbles (detaching a bubble does not detach its
//! descendants), the breadcrumb and the collapse of old ancestors.

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use serde_json::{Value, json};

use super::bubbles::{BubbleState, bubble_key};
use super::store::CodexStore;
use super::{OpenCodex, text};
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::settings::KeyMap;
use crate::ui::kit::style::{self, UiTier};
use crate::ui::kit::tooltip::{Tooltip, TooltipPanel, TooltipRegistry, TooltipState, WIDTH};
use crate::ui::kit::{RichLink, RichStyle, UiTheme, rich_text};

/// Footer of a pinned bubble.
pub const FOOTER_SHEET: &str = "Clic : lire la fiche · Clic droit : détacher";
pub const FOOTER_FREE: &str = "Clic droit : détacher";
/// Offset of a bubble pinned at the pointer.
const CURSOR_OFFSET: Vec2 = Vec2::new(14.0, 18.0);
const SCREEN_MARGIN: f32 = 4.0;

/// Any bubble of the screen: a hover bubble of the kit or a pinned one.
pub(super) type AnyBubble = Or<(With<TooltipPanel>, With<PinnedBubble>)>;

/// A pinned bubble: the registry key it shows (empty for a free text) and the Codex sheet a
/// click on it opens.
#[derive(Component, Debug)]
pub struct PinnedBubble {
    pub key: String,
    pub sheet: Option<String>,
    /// Positioned once its size is known.
    pub placed: bool,
    /// Where it should go (clamped to the screen at placement).
    pub at: Vec2,
}

#[derive(Resource, Default)]
struct PinState {
    pinned_total: usize,
    detached_total: usize,
    last_key: String,
    last_source: &'static str,
}

pub fn plugin(app: &mut App) {
    app.init_resource::<PinState>()
        .register_dump(dump)
        .add_systems(PreUpdate, pin_key.after(bevy::input::InputSystems))
        .add_systems(
            Update,
            (
                pin_with_right_click,
                click_bubble_body,
                place_pinned,
                clear_on_screen_change,
            ),
        );
}

/// Codex sheet a registry key leads to: the entry itself, or the entry of the entity.
pub fn sheet_of_key(store: &CodexStore, key: &str) -> Option<String> {
    if let Some(id) = text::target_id(key) {
        return store.has(id).then(|| id.to_owned());
    }
    store
        .entry_for_entity(text::ib_entity(key)?)
        .map(str::to_owned)
}

/// Spawns a pinned bubble showing `bbcode`.
fn spawn_pinned(
    commands: &mut Commands,
    theme: &UiTheme,
    assets: &AssetServer,
    bbcode: &str,
    key: &str,
    sheet: Option<String>,
    at: Vec2,
) {
    let style = RichStyle::caption(theme);
    let footer = if sheet.is_some() {
        FOOTER_SHEET
    } else {
        FOOTER_FREE
    };
    commands
        .spawn((
            Name::new(format!("PinnedBubble {key}")),
            PinnedBubble {
                key: key.to_owned(),
                sheet,
                placed: false,
                at,
            },
            Hovered::default(),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(at.x),
                top: Val::Px(at.y),
                max_width: Val::Px(WIDTH + 24.0),
                padding: theme.padding("tooltip"),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            },
            theme.sliced("tooltip"),
            UiTier::Tooltip.index(),
            Visibility::Hidden,
        ))
        .with_children(|parent| {
            parent.spawn(rich_text(theme, assets, bbcode, &style));
            parent.spawn((
                Name::new("PinnedFooter"),
                Text::new(footer),
                theme.text_font(false, true, 13.0),
                TextColor(style::MUTED),
                Node {
                    align_self: AlignSelf::End,
                    ..default()
                },
            ));
        });
}

/// Moves a fresh pinned bubble inside the screen and shows it.
fn place_pinned(
    window: Query<&Window, With<PrimaryWindow>>,
    mut bubbles: Query<(&mut PinnedBubble, &mut Node, &mut Visibility, &ComputedNode)>,
) {
    let Ok(window) = window.single() else {
        return;
    };
    for (mut bubble, mut node, mut visibility, computed) in &mut bubbles {
        if bubble.placed {
            continue;
        }
        let size = computed.size() * computed.inverse_scale_factor;
        if size.x <= 0.0 || size.y <= 0.0 {
            continue;
        }
        let max =
            (window.size() - size - Vec2::splat(SCREEN_MARGIN)).max(Vec2::splat(SCREEN_MARGIN));
        let at = bubble.at.clamp(Vec2::splat(SCREEN_MARGIN), max);
        node.left = Val::Px(at.x);
        node.top = Val::Px(at.y);
        bubble.placed = true;
        *visibility = Visibility::Inherited;
    }
}

fn in_bubble(entity: Entity, parents: &Query<&ChildOf>, bubbles: &Query<(), AnyBubble>) -> bool {
    std::iter::once(entity)
        .chain(parents.iter_ancestors(entity))
        .any(|ancestor| bubbles.contains(ancestor))
}

/// The link under the pointer outside any bubble: its registry key.
fn hovered_link_key(
    links: &Query<(Entity, &RichLink, &Hovered)>,
    parents: &Query<&ChildOf>,
    bubbles: &Query<(), AnyBubble>,
) -> Option<String> {
    links.iter().find_map(|(entity, link, hovered)| {
        let key = bubble_key(&link.0)?;
        (hovered.get() && !in_bubble(entity, parents, bubbles)).then(|| key.to_owned())
    })
}

/// Rectangle of every placed hover bubble of the kit chain, by level order.
fn chain_rects(
    tooltips: &TooltipState,
    panels: &Query<&TooltipPanel>,
) -> Vec<(Option<String>, String, Rect)> {
    tooltips
        .chain
        .levels
        .iter()
        .filter_map(|level| {
            let panel = panels
                .iter()
                .find(|panel| panel.id == level.id && panel.placed)?;
            Some((level.target.clone(), level.bbcode.clone(), panel.rect))
        })
        .collect()
}

/// Pins the whole open hover chain where it stands and closes the chain.
#[allow(clippy::too_many_arguments)]
fn pin_chain(
    commands: &mut Commands,
    theme: &UiTheme,
    assets: &AssetServer,
    store: &CodexStore,
    tooltips: &mut TooltipState,
    bubbles: &mut BubbleState,
    panels: &Query<&TooltipPanel>,
    pins: &mut PinState,
    existing: &Query<&PinnedBubble>,
) -> bool {
    let levels = chain_rects(tooltips, panels);
    if levels.is_empty() {
        return false;
    }
    let root_key = bubbles.open_key().map(str::to_owned);
    for (index, (target, bbcode, rect)) in levels.iter().enumerate() {
        let key = target
            .clone()
            .or_else(|| (index == 0).then(|| root_key.clone()).flatten())
            .unwrap_or_default();
        if !key.is_empty() && existing.iter().any(|pinned| pinned.key == key) {
            continue;
        }
        let sheet = sheet_of_key(store, &key);
        spawn_pinned(commands, theme, assets, bbcode, &key, sheet, rect.min);
        pins.pinned_total += 1;
        pins.last_key = key;
        pins.last_source = "chain";
    }
    tooltips.chain.clear();
    bubbles.forget_open();
    true
}

/// T: pins what the pointer is on, in priority order (see the module header).
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn pin_key(
    mut commands: Commands,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    keymap: Option<Res<KeyMap>>,
    (theme, assets, store, registry): (
        Option<Res<UiTheme>>,
        Res<AssetServer>,
        Res<CodexStore>,
        Res<TooltipRegistry>,
    ),
    (mut tooltips, mut bubbles, mut pins): (
        ResMut<TooltipState>,
        ResMut<BubbleState>,
        ResMut<PinState>,
    ),
    window: Query<&Window, With<PrimaryWindow>>,
    (panels, existing, hosts): (
        Query<&TooltipPanel>,
        Query<&PinnedBubble>,
        Query<(&Tooltip, &Hovered)>,
    ),
    (links, parents, bubble_nodes): (
        Query<(Entity, &RichLink, &Hovered)>,
        Query<&ChildOf>,
        Query<(), AnyBubble>,
    ),
) {
    let pressed = match &keymap {
        Some(map) => map.just_pressed("codex_pin_tooltip", &keys),
        None => keys.just_pressed(KeyCode::KeyT),
    };
    let Some(theme) = theme else {
        return;
    };
    if !pressed {
        return;
    }
    let cursor = window
        .single()
        .ok()
        .and_then(Window::cursor_position)
        .unwrap_or(Vec2::ZERO);
    let done = if let Some(key) = hovered_link_key(&links, &parents, &bubble_nodes)
        .filter(|key| tooltips.chain.is_empty() && registry.contains(key))
    {
        pin_key_at(
            &mut commands,
            &theme,
            &assets,
            &store,
            &registry,
            &existing,
            &mut pins,
            &key,
            cursor + CURSOR_OFFSET,
            "link",
        );
        true
    } else if pin_chain(
        &mut commands,
        &theme,
        &assets,
        &store,
        &mut tooltips,
        &mut bubbles,
        &panels,
        &mut pins,
        &existing,
    ) {
        true
    } else if let Some((tooltip, _)) = hosts
        .iter()
        .filter(|(tooltip, hovered)| hovered.get() && !tooltip.bbcode.is_empty())
        .last()
    {
        let bbcode = text::auto_link(&store, &tooltip.bbcode);
        spawn_pinned(
            &mut commands,
            &theme,
            &assets,
            &bbcode,
            "",
            None,
            cursor + CURSOR_OFFSET,
        );
        pins.pinned_total += 1;
        pins.last_key.clear();
        pins.last_source = "tooltip";
        true
    } else {
        false
    };
    if done {
        // T also opens the tech tree: nothing else may see this press.
        match &keymap {
            Some(map) => map.chords("codex_pin_tooltip").iter().for_each(|chord| {
                keys.clear_just_pressed(chord.key);
            }),
            None => {
                keys.clear_just_pressed(KeyCode::KeyT);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn pin_key_at(
    commands: &mut Commands,
    theme: &UiTheme,
    assets: &AssetServer,
    store: &CodexStore,
    registry: &TooltipRegistry,
    existing: &Query<&PinnedBubble>,
    pins: &mut PinState,
    key: &str,
    at: Vec2,
    source: &'static str,
) {
    if existing.iter().any(|pinned| pinned.key == key) {
        return;
    }
    let sheet = sheet_of_key(store, key);
    spawn_pinned(
        commands,
        theme,
        assets,
        &registry.resolve(key),
        key,
        sheet,
        at,
    );
    pins.pinned_total += 1;
    pins.last_key = key.to_owned();
    pins.last_source = source;
}

/// Right click: link -> pinned bubble; hover bubble -> pin the chain; pinned bubble -> detach.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn pin_with_right_click(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    (theme, assets, store, registry): (
        Option<Res<UiTheme>>,
        Res<AssetServer>,
        Res<CodexStore>,
        Res<TooltipRegistry>,
    ),
    (mut tooltips, mut bubbles, mut pins): (
        ResMut<TooltipState>,
        ResMut<BubbleState>,
        ResMut<PinState>,
    ),
    window: Query<&Window, With<PrimaryWindow>>,
    (panels, panel_hover, existing, pinned_hover): (
        Query<&TooltipPanel>,
        Query<&Hovered, With<TooltipPanel>>,
        Query<&PinnedBubble>,
        Query<(Entity, &Hovered), With<PinnedBubble>>,
    ),
    (links, parents, bubble_nodes): (
        Query<(Entity, &RichLink, &Hovered)>,
        Query<&ChildOf>,
        Query<(), AnyBubble>,
    ),
) {
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    let Some(theme) = theme else {
        return;
    };
    let cursor = window
        .single()
        .ok()
        .and_then(Window::cursor_position)
        .unwrap_or(Vec2::ZERO);
    if let Some(key) =
        hovered_link_key(&links, &parents, &bubble_nodes).filter(|key| registry.contains(key))
    {
        // The hover bubble the link opened is replaced by the pinned one.
        tooltips.chain.clear();
        bubbles.forget_open();
        pin_key_at(
            &mut commands,
            &theme,
            &assets,
            &store,
            &registry,
            &existing,
            &mut pins,
            &key,
            cursor + CURSOR_OFFSET,
            "right_click",
        );
    } else if let Some((entity, _)) = pinned_hover.iter().find(|(_, hovered)| hovered.get()) {
        commands.entity(entity).despawn();
        pins.detached_total += 1;
    } else if panel_hover.iter().any(Hovered::get) {
        pin_chain(
            &mut commands,
            &theme,
            &assets,
            &store,
            &mut tooltips,
            &mut bubbles,
            &panels,
            &mut pins,
            &existing,
        );
    }
}

/// Left click on the body of a bubble (hover or pinned): the sheet of its entry. Clicks on links
/// are handled by [`super::bubbles`].
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn click_bubble_body(
    mouse: Res<ButtonInput<MouseButton>>,
    store: Res<CodexStore>,
    mut tooltips: ResMut<TooltipState>,
    mut bubbles: ResMut<BubbleState>,
    links: Query<&Hovered, With<RichLink>>,
    pinned: Query<(&PinnedBubble, &Hovered)>,
    panels: Query<(&TooltipPanel, &Hovered)>,
    mut requests: MessageWriter<OpenCodex>,
) {
    if !mouse.just_pressed(MouseButton::Left) || links.iter().any(Hovered::get) {
        return;
    }
    let sheet = pinned
        .iter()
        .find(|(_, hovered)| hovered.get())
        .and_then(|(bubble, _)| bubble.sheet.clone())
        .or_else(|| {
            let (panel, _) = panels.iter().find(|(_, hovered)| hovered.get())?;
            let level = tooltips
                .chain
                .levels
                .iter()
                .find(|level| level.id == panel.id)?;
            let key = level
                .target
                .clone()
                .or_else(|| bubbles.open_key().map(str::to_owned))?;
            sheet_of_key(&store, &key)
        });
    if let Some(id) = sheet {
        tooltips.chain.clear();
        bubbles.forget_open();
        requests.write(OpenCodex { entry: Some(id) });
    }
}

/// Pinned bubbles do not outlive the screen they were pinned on.
fn clear_on_screen_change(
    mut commands: Commands,
    state: Res<State<AppState>>,
    bubbles: Query<Entity, With<PinnedBubble>>,
) {
    if state.is_changed() {
        for entity in &bubbles {
            commands.entity(entity).despawn();
        }
    }
}

fn dump(world: &World) -> (String, Value) {
    let pins = world.resource::<PinState>();
    let mut alive = Vec::new();
    for entity in world.iter_entities() {
        if let Some(bubble) = entity.get::<PinnedBubble>() {
            alive.push(json!({"key": bubble.key, "sheet": bubble.sheet, "placed": bubble.placed}));
        }
    }
    (
        "codex_pinned".to_owned(),
        json!({
            "pinned_total": pins.pinned_total,
            "detached_total": pins.detached_total,
            "last_key": pins.last_key,
            "last_source": pins.last_source,
            "alive": alive,
        }),
    )
}
