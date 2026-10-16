//! Modal dialogs with a priority queue, toasts and the slide-in motion.
//!
//! Ports game/scripts/ui/{modal_queue,confirm_dialog,confirm_panel,ui_motion}.gd.
//!
//! - [`ModalQueue`] (resource): one modal visible at a time; decisions ([`PRIORITY_DECISION`]) go
//!   before reports ([`PRIORITY_REPORT`]); the rest waits for the active one to close. A request
//!   with the key of the active modal refreshes its content; with the key of a pending request it
//!   replaces it. Result: [`ModalResult`] (confirmed or cancelled; Escape = cancel).
//! - [`ToastRequest`] (message): short notices stacked at the top right, auto-dismissed.
//! - [`SlideIn`]: 0.12 s slide of 8 px (Bevy UI has no per-node alpha, so no fade), skipped when
//!   [`ReduceMotion`] is set (« Réduire les animations »).

use bevy::prelude::*;
use serde_json::{Value, json};

use super::richtext::RichStyle;
use super::style::{self, UiTier, UiType};
use super::widgets::{KitWidget, button_primary, button_small};
use super::{ButtonPressed, UiTheme, rich_text, typed};

pub const PRIORITY_DECISION: i32 = 0;
pub const PRIORITY_REPORT: i32 = 1;
/// Seconds a toast stays.
pub const TOAST_SECONDS: f32 = 4.0;
/// Most toasts shown at once.
pub const MAX_TOASTS: usize = 5;
const SLIDE_SECONDS: f32 = 0.12;
const SLIDE_PIXELS: f32 = 8.0;

/// Content of a confirm / cancel dialog.
#[derive(Debug, Clone, PartialEq)]
pub struct ModalSpec {
    pub title: String,
    /// BBCode body.
    pub text: String,
    pub yes_label: String,
    /// Empty = no cancel button (information dialog).
    pub no_label: String,
    pub width: f32,
    /// Illuminated picture above the text, cover-cropped to the card width (season report).
    pub art: Option<ModalArt>,
    /// Escape neither answers nor holds back the pause menu: it closes this dialog as confirmed
    /// and passes through (the campaign briefing, which Godot never closed on Escape).
    pub escape_passes: bool,
}

/// Picture of a modal: asset path (relative to `game/assets`), aspect ratio (width / height) and
/// a title shown under it.
#[derive(Debug, Clone, PartialEq)]
pub struct ModalArt {
    pub path: String,
    pub aspect: f32,
    pub caption: String,
}

impl ModalSpec {
    pub fn confirm(title: &str, text: &str) -> Self {
        Self {
            title: title.to_owned(),
            text: text.to_owned(),
            yes_label: "Confirmer".to_owned(),
            no_label: "Renoncer".to_owned(),
            width: 440.0,
            art: None,
            escape_passes: false,
        }
    }

    pub fn info(title: &str, text: &str) -> Self {
        Self {
            no_label: String::new(),
            yes_label: "Fermer".to_owned(),
            ..Self::confirm(title, text)
        }
    }
}

#[derive(Debug, Clone)]
struct Entry {
    key: String,
    priority: i32,
    spec: ModalSpec,
    seq: u64,
}

/// Queue of modal dialogs. Pure logic (unit-tested); the systems below show the active one.
#[derive(Resource, Default, Debug)]
pub struct ModalQueue {
    pending: Vec<Entry>,
    active: Option<Entry>,
    seq: u64,
    /// The active modal's content changed: rebuild its entities.
    refresh: bool,
}

impl ModalQueue {
    /// True while a modal is active or waiting.
    pub fn is_busy(&self) -> bool {
        self.active.is_some() || !self.pending.is_empty()
    }

    /// True while a modal that holds Escape back is active or waiting: the pause menu opens over
    /// a lone dialog that lets Escape through ([`ModalSpec::escape_passes`]).
    pub fn blocks_escape(&self) -> bool {
        !self.pending.is_empty()
            || self
                .active
                .as_ref()
                .is_some_and(|entry| !entry.spec.escape_passes)
    }

    pub fn active_key(&self) -> &str {
        self.active.as_ref().map_or("", |entry| entry.key.as_str())
    }

    /// Keys waiting, in order of passage.
    pub fn pending_keys(&self) -> Vec<&str> {
        let mut ordered: Vec<&Entry> = self.pending.iter().collect();
        ordered.sort_by_key(|entry| (entry.priority, entry.seq));
        ordered
            .into_iter()
            .map(|entry| entry.key.as_str())
            .collect()
    }

    /// Queues `spec` under `key` (see the module doc for same-key behaviour).
    pub fn request(&mut self, key: &str, priority: i32, spec: ModalSpec) {
        if let Some(active) = self.active.as_mut()
            && active.key == key
        {
            active.spec = spec;
            self.refresh = true;
            return;
        }
        if let Some(entry) = self.pending.iter_mut().find(|entry| entry.key == key) {
            entry.priority = priority;
            entry.spec = spec;
            return;
        }
        self.seq += 1;
        self.pending.push(Entry {
            key: key.to_owned(),
            priority,
            spec,
            seq: self.seq,
        });
        self.pump();
    }

    /// Drops a pending request (a report of a past turn, say).
    pub fn cancel(&mut self, key: &str) {
        self.pending.retain(|entry| entry.key != key);
    }

    /// Forgets everything (screen change, game load).
    pub fn clear(&mut self) {
        self.pending.clear();
        self.active = None;
    }

    /// Closes the active modal and shows the next; returns the closed key.
    pub fn finish(&mut self) -> Option<String> {
        let closed = self.active.take().map(|entry| entry.key);
        self.pump();
        closed
    }

    fn pump(&mut self) {
        if self.active.is_some() || self.pending.is_empty() {
            return;
        }
        self.pending
            .sort_by_key(|entry| (entry.priority, entry.seq));
        self.active = Some(self.pending.remove(0));
        self.refresh = true;
    }
}

/// A modal was answered (button or Escape).
#[derive(Message, Debug, Clone)]
pub struct ModalResult {
    pub key: String,
    pub confirmed: bool,
}

/// Short notice at the top right.
#[derive(Message, Debug, Clone)]
pub struct ToastRequest {
    pub text: String,
    pub seconds: f32,
}

impl ToastRequest {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_owned(),
            seconds: TOAST_SECONDS,
        }
    }
}

/// « Réduire les animations »: the settings screen sets it.
#[derive(Resource, Default)]
pub struct ReduceMotion(pub bool);

/// Slide-in of a panel (0.12 s, 8 px up). Add to a freshly spawned node.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct SlideIn {
    pub elapsed: f32,
}

#[derive(Component)]
struct ModalRoot;

#[derive(Component)]
struct ModalButton {
    confirm: bool,
}

#[derive(Component)]
struct ToastStack;

#[derive(Component)]
struct Toast {
    remaining: f32,
}

pub struct ModalPlugin;

impl Plugin for ModalPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ModalQueue>()
            .init_resource::<ReduceMotion>()
            .add_message::<ModalResult>()
            .add_message::<ToastRequest>()
            .add_systems(
                Update,
                (
                    sync_modal,
                    handle_modal_buttons,
                    modal_escape,
                    spawn_toasts,
                    expire_toasts,
                    slide_in,
                )
                    .chain(),
            );
    }
}

fn sync_modal(
    mut commands: Commands,
    mut queue: ResMut<ModalQueue>,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
    roots: Query<Entity, With<ModalRoot>>,
) {
    let has_root = !roots.is_empty();
    let wants = queue.active.is_some();
    if has_root && (!wants || queue.refresh) {
        for root in &roots {
            commands.entity(root).despawn();
        }
    }
    if wants && (!has_root || queue.refresh) {
        queue.refresh = false;
        let Some(entry) = queue.active.clone() else {
            return;
        };
        spawn_modal(&mut commands, &theme, &assets, &entry);
    }
}

fn spawn_modal(commands: &mut Commands, theme: &UiTheme, assets: &AssetServer, entry: &Entry) {
    let spec = &entry.spec;
    let body = RichStyle::caption(theme);
    let body = RichStyle {
        size: UiType::Body.size(),
        ..body
    };
    commands
        .spawn((
            Name::new(format!("Modal {}", entry.key)),
            ModalRoot,
            KitWidget("modal"),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexStart,
                padding: UiRect::top(Val::Px(110.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45)),
            UiTier::Modal.index(),
            // Blocks the pointer below.
            bevy::ui::FocusPolicy::Block,
        ))
        .with_children(|root| {
            root.spawn((
                KitWidget("panel"),
                theme.sliced("panel"),
                SlideIn::default(),
                UiTransform::default(),
                Node {
                    width: Val::Px(spec.width),
                    // Never wider / taller than the window (long text wraps, then clips).
                    max_width: Val::Percent(94.0),
                    max_height: Val::Percent(88.0),
                    // Inner padding keeps title and text off the painted frame.
                    padding: theme.inner_padding("panel", 8.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(10.0),
                    ..default()
                },
            ))
            .with_children(|card| {
                if !spec.title.is_empty() {
                    card.spawn(typed(theme, &spec.title, UiType::Heading));
                }
                if let Some(art) = &spec.art {
                    card.spawn((
                        Name::new("modal:art"),
                        Node {
                            width: Val::Percent(100.0),
                            aspect_ratio: Some(art.aspect),
                            flex_shrink: 0.0,
                            ..default()
                        },
                        ImageNode::new(assets.load(art.path.clone())),
                        crate::menu::CoverFit::new(Vec2::splat(0.5)),
                    ));
                    if !art.caption.is_empty() {
                        card.spawn((
                            Name::new("modal:art_caption"),
                            Node {
                                flex_shrink: 0.0,
                                ..default()
                            },
                            Text::new(art.caption.clone()),
                            theme.text_font(false, true, UiType::Caption.size()),
                            TextColor(style::RUBRIC),
                        ));
                    }
                }
                if !spec.text.is_empty() {
                    card.spawn((
                        // Full width of the card's content box: the text gets a definite width
                        // to wrap in (it used to run past the right edge of the frame).
                        Node {
                            width: Val::Percent(100.0),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Stretch,
                            ..default()
                        },
                        children![rich_text(theme, assets, &spec.text, &body)],
                    ));
                }
                card.spawn(Node {
                    column_gap: Val::Px(12.0),
                    margin: UiRect::top(Val::Px(4.0)),
                    ..default()
                })
                .with_children(|row| {
                    if !spec.no_label.is_empty() {
                        row.spawn((
                            button_small(theme, &spec.no_label, &spec.no_label),
                            ModalButton { confirm: false },
                        ));
                    }
                    row.spawn((
                        button_primary(theme, &spec.yes_label, &spec.yes_label),
                        ModalButton { confirm: true },
                    ));
                });
            });
        });
}

fn handle_modal_buttons(
    mut pressed: MessageReader<ButtonPressed>,
    buttons: Query<&ModalButton>,
    mut queue: ResMut<ModalQueue>,
    mut results: MessageWriter<ModalResult>,
) {
    for press in pressed.read() {
        let Ok(button) = buttons.get(press.entity) else {
            continue;
        };
        if let Some(key) = queue.finish() {
            results.write(ModalResult {
                key,
                confirmed: button.confirm,
            });
        }
    }
}

fn modal_escape(
    keys: Res<ButtonInput<KeyCode>>,
    mut queue: ResMut<ModalQueue>,
    mut results: MessageWriter<ModalResult>,
) {
    if keys.just_pressed(KeyCode::Escape)
        && let Some(entry) = queue.active.as_ref()
    {
        // Information dialogs close as "confirmed"; confirmations cancel.
        let confirmed = entry.spec.no_label.is_empty() || entry.spec.escape_passes;
        if let Some(key) = queue.finish() {
            results.write(ModalResult { key, confirmed });
        }
    }
}

fn spawn_toasts(
    mut commands: Commands,
    mut requests: MessageReader<ToastRequest>,
    theme: Res<UiTheme>,
    stacks: Query<Entity, With<ToastStack>>,
    toasts: Query<Entity, With<Toast>>,
) {
    let requests: Vec<ToastRequest> = requests.read().cloned().collect();
    if requests.is_empty() {
        return;
    }
    let stack = match stacks.single() {
        Ok(stack) => stack,
        Err(_) => commands
            .spawn((
                Name::new("ToastStack"),
                ToastStack,
                // Centred under the campaign top bar (Godot `Toast`: offset_top 52, centred),
                // clear of the launcher and of the docked side panels.
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    top: Val::Px(super::layout::PANEL_TOP + 6.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.0),
                    align_items: AlignItems::Center,
                    ..default()
                },
                Pickable::IGNORE,
                UiTier::Toast.index(),
            ))
            .id(),
    };
    let shown = toasts.iter().count();
    for request in requests.into_iter().take(MAX_TOASTS.saturating_sub(shown)) {
        commands.entity(stack).with_children(|parent| {
            parent.spawn((
                Name::new("Toast"),
                KitWidget("toast"),
                Toast {
                    remaining: request.seconds,
                },
                SlideIn::default(),
                UiTransform::default(),
                Node {
                    max_width: Val::Px(360.0),
                    padding: theme.padding("tooltip"),
                    ..default()
                },
                theme.sliced("tooltip"),
                children![(
                    Text::new(request.text),
                    theme.text_font(false, false, UiType::Caption.size()),
                    TextColor(style::INK),
                )],
            ));
        });
    }
}

fn expire_toasts(mut commands: Commands, time: Res<Time>, mut toasts: Query<(Entity, &mut Toast)>) {
    for (entity, mut toast) in &mut toasts {
        toast.remaining -= time.delta_secs();
        if toast.remaining <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

fn slide_in(
    time: Res<Time>,
    reduce: Res<ReduceMotion>,
    mut nodes: Query<(&mut SlideIn, &mut UiTransform)>,
) {
    for (mut slide, mut transform) in &mut nodes {
        if slide.elapsed >= SLIDE_SECONDS || reduce.0 {
            if transform.translation != Val2::ZERO {
                transform.translation = Val2::ZERO;
            }
            slide.elapsed = SLIDE_SECONDS;
            continue;
        }
        slide.elapsed += time.delta_secs();
        let remaining = 1.0 - (slide.elapsed / SLIDE_SECONDS).min(1.0);
        transform.translation = Val2::px(0.0, SLIDE_PIXELS * remaining);
    }
}

/// Dump section part: queue and toasts.
pub fn dump(world: &World) -> Value {
    let Some(queue) = world.get_resource::<ModalQueue>() else {
        return json!({});
    };
    let toasts = world
        .iter_entities()
        .filter(|entity| entity.contains::<Toast>())
        .count();
    json!({
        "active": queue.active_key(),
        "pending": queue.pending_keys(),
        "queue_length": queue.pending.len() + usize::from(queue.active.is_some()),
        "toasts": toasts,
        "art": queue.active.as_ref().and_then(|e| e.spec.art.as_ref()).map(|art| json!({
            "path": art.path, "aspect": art.aspect, "caption": art.caption,
            // `CoverFit` has cropped the source rectangle once the image is loaded.
            "cropped": world.iter_entities().any(|entity| {
                entity.get::<Name>().is_some_and(|n| n.as_str() == "modal:art")
                    && entity.get::<ImageNode>().is_some_and(|image| image.rect.is_some())
            }),
        })),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> ModalSpec {
        ModalSpec::confirm("t", "x")
    }

    #[test]
    fn escape_passing_modal_does_not_hold_back_the_pause_menu() {
        let mut queue = ModalQueue::default();
        assert!(!queue.blocks_escape());
        let mut spec = spec();
        spec.escape_passes = true;
        queue.request("briefing", PRIORITY_DECISION, spec);
        assert!(queue.is_busy() && !queue.blocks_escape());
        queue.request("other", PRIORITY_REPORT, self::spec());
        assert!(queue.blocks_escape());
    }

    #[test]
    fn one_at_a_time_decisions_first() {
        let mut queue = ModalQueue::default();
        queue.request("season_report", PRIORITY_REPORT, spec());
        assert_eq!(queue.active_key(), "season_report");
        queue.request("report_b", PRIORITY_REPORT, spec());
        queue.request("decision", PRIORITY_DECISION, spec());
        assert_eq!(queue.pending_keys(), vec!["decision", "report_b"]);
        assert_eq!(queue.finish().as_deref(), Some("season_report"));
        assert_eq!(queue.active_key(), "decision");
        queue.finish();
        queue.finish();
        assert!(!queue.is_busy());
    }

    #[test]
    fn same_key_refreshes_or_replaces() {
        let mut queue = ModalQueue::default();
        queue.request("a", PRIORITY_REPORT, spec());
        queue.refresh = false;
        queue.request("a", PRIORITY_REPORT, ModalSpec::info("autre", "y"));
        assert!(queue.refresh);
        assert_eq!(queue.active.as_ref().unwrap().spec.title, "autre");
        queue.request("b", PRIORITY_REPORT, spec());
        queue.request("b", PRIORITY_DECISION, ModalSpec::info("b2", "z"));
        assert_eq!(queue.pending.len(), 1);
        assert_eq!(queue.pending[0].priority, PRIORITY_DECISION);
    }

    #[test]
    fn cancel_and_clear() {
        let mut queue = ModalQueue::default();
        queue.request("a", 0, spec());
        queue.request("b", 1, spec());
        queue.request("c", 1, spec());
        queue.cancel("b");
        assert_eq!(queue.pending_keys(), vec!["c"]);
        queue.clear();
        assert!(!queue.is_busy());
    }
}
