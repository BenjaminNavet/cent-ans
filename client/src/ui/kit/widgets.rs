//! Parchment widgets: sections, separators, buttons (4 states, primary, icon), toggles,
//! checkboxes, tabs, sliders, scroll lists, gauges, icon chips and the illuminated title.
//!
//! Replaces rich_button/rich_panel/rich_box/panel_section/list_menu/icon_chip/lettrine/
//! illuminated_title of game/scripts/ui. Every constructor returns a `Bundle` and tags it with
//! [`KitWidget`] so the `ui_kit` dump can count widgets by kind.
//!
//! Interaction goes through [`ButtonPressed`] (mouse, keyboard or agent `click <Name>`): toggles,
//! tabs and list items react to it and emit [`ToggleChanged`] / [`TabSelected`] /
//! [`ListSelected`]; sliders emit [`SliderChanged`].

use bevy::ecs::relationship::RelatedSpawner;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{IgnoreScroll, InteractionDisabled, Pressed};
use bevy::ui_widgets::{
    Button, ControlOrientation, ScrollArea, Scrollbar, ScrollbarThumb, Slider, SliderRange,
    SliderStep, SliderThumb, SliderValue, ValueChange,
};
use serde_json::{Value, json};

use super::richtext::{RichLink, RichText};
use super::style::{self, UiType};
use super::tooltip::Tooltip;
use super::{ButtonPressed, UiTheme};

/// Kind tag of every kit widget (`"button"`, `"checkbox"`, `"tab"`…), for the dump.
#[derive(Component, Debug, Clone, Copy)]
pub struct KitWidget(pub &'static str);

/// Button look.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    Normal,
    /// Primary action (`PrimaryButton` variation: dark, light text).
    Primary,
    Tab,
}

/// Visual state of a kit button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonVisual {
    Normal,
    Hover,
    Pressed,
    Disabled,
    /// Tabs only.
    Selected,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct KitButton {
    pub kind: ButtonKind,
    pub visual: ButtonVisual,
}

/// On/off state of a toggle button, a checkbox or a selected list item.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Toggled(pub bool);

#[derive(Component)]
struct CheckMark;

/// A tab; exactly one tab per `group` is selected.
#[derive(Component, Debug, Clone)]
pub struct TabButton {
    pub group: String,
    pub selected: bool,
}

/// Content shown only while tab `name` of `group` is selected.
#[derive(Component, Debug, Clone)]
pub struct TabPane {
    pub group: String,
    pub name: String,
}

/// A selectable row of a list; one selected item per `group`.
#[derive(Component, Debug, Clone)]
pub struct ListItem {
    pub group: String,
}

#[derive(Component)]
pub struct KitScrollList;

#[derive(Component, Clone, Copy)]
enum SliderPart {
    Fill,
    Thumb,
}

#[derive(Component)]
struct SliderLayout {
    width: f32,
}

/// A gauge fill; `ratio` 0..1.
#[derive(Component, Debug, Clone, Copy)]
pub struct Gauge(pub f32);

#[derive(Message, Debug, Clone)]
pub struct ToggleChanged {
    pub entity: Entity,
    pub name: String,
    pub on: bool,
}

#[derive(Message, Debug, Clone)]
pub struct TabSelected {
    pub entity: Entity,
    pub group: String,
    pub name: String,
}

#[derive(Message, Debug, Clone)]
pub struct ListSelected {
    pub entity: Entity,
    pub group: String,
    pub name: String,
}

#[derive(Message, Debug, Clone)]
pub struct SliderChanged {
    pub entity: Entity,
    pub name: String,
    pub value: f32,
    /// False while the thumb is being dragged.
    pub is_final: bool,
}

pub struct WidgetsPlugin;

impl Plugin for WidgetsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ToggleChanged>()
            .add_message::<TabSelected>()
            .add_message::<ListSelected>()
            .add_message::<SliderChanged>()
            .add_observer(on_slider_change)
            .add_systems(
                Update,
                (
                    handle_toggles,
                    handle_tabs,
                    handle_list_items,
                    button_visuals,
                    sync_checkmarks,
                    sync_tab_panes,
                    list_item_visuals,
                    sync_sliders,
                    sync_gauges,
                )
                    .chain(),
            )
            .add_systems(Update, attach_scrollbars);
    }
}

// ---- pure look rules ---------------------------------------------------------------------

/// Texture key and text colour of a button in a state (`parchment_theme.tres`).
pub fn visual_look(kind: ButtonKind, visual: ButtonVisual) -> (&'static str, Color) {
    use ButtonKind as K;
    use ButtonVisual as V;
    match (kind, visual) {
        (K::Normal, V::Normal) => ("button_normal", style::INK),
        (K::Normal, V::Hover) => ("button_hover", style::BUTTON_HOVER_INK),
        (K::Normal, V::Pressed | V::Selected) => ("button_pressed", style::BUTTON_PRESSED_INK),
        (K::Normal, V::Disabled) => ("button_disabled", style::INK_FADED),
        (K::Primary, V::Normal) => ("button_primary_normal", style::PRIMARY_INK),
        (K::Primary, V::Hover) => ("button_primary_hover", style::PRIMARY_HOVER_INK),
        (K::Primary, V::Pressed | V::Selected) => {
            ("button_primary_pressed", style::PRIMARY_PRESSED_INK)
        }
        (K::Primary, V::Disabled) => ("button_primary_disabled", style::PRIMARY_DISABLED_INK),
        (K::Tab, V::Selected | V::Pressed) => ("tab_ornate_selected", style::TAB_SELECTED_INK),
        (K::Tab, V::Disabled) => ("tab_ornate_unselected", style::INK_FADED),
        (K::Tab, V::Normal | V::Hover) => ("tab_ornate_unselected", style::TAB_UNSELECTED_INK),
    }
}

/// Visual state from pointer state: disabled beats pressed beats on/selected beats hover.
pub fn visual_state(hovered: bool, pressed: bool, disabled: bool, on: bool) -> ButtonVisual {
    if disabled {
        ButtonVisual::Disabled
    } else if pressed {
        ButtonVisual::Pressed
    } else if on {
        ButtonVisual::Selected
    } else if hovered {
        ButtonVisual::Hover
    } else {
        ButtonVisual::Normal
    }
}

/// Air between a panel's painted frame and its content.
const PANEL_GAP: f32 = 8.0;
/// Floor for framed boxes: a frame shorter than this is mostly border.
pub const MIN_FRAME_HEIGHT: f32 = 40.0;
/// Inner padding of the flat card (`card_box` has 4 px; more air reads better in Bevy text).
const CARD_PADDING: f32 = 6.0;
/// Width of the scrollbar track of a scroll list.
pub const SCROLLBAR_WIDTH: f32 = 8.0;

// ---- bundles -----------------------------------------------------------------------------

/// Section skeleton of a panel (`PanelSection`): named column with 4 px spacing.
pub fn section(name: &str) -> impl Bundle {
    (
        Name::new(name.to_owned()),
        KitWidget("section"),
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            align_self: AlignSelf::Stretch,
            ..default()
        },
    )
}

/// Section heading (Heading variation).
pub fn heading(theme: &UiTheme, content: &str) -> impl Bundle {
    (
        KitWidget("heading"),
        super::typed(theme, content, UiType::Heading),
    )
}

/// Caption text (Caption variation).
pub fn caption(theme: &UiTheme, content: &str) -> impl Bundle {
    (
        KitWidget("caption"),
        super::typed(theme, content, UiType::Caption),
    )
}

/// Refusal / error line in red, hidden by default (`PanelSection._add_error_label`).
pub fn error_line(theme: &UiTheme) -> impl Bundle {
    (
        Name::new("ErrorLine"),
        KitWidget("error_line"),
        Text::new(""),
        theme.text_font(false, false, UiType::Caption.size()),
        TextColor(style::ERROR),
        Visibility::Hidden,
    )
}

/// Page-style panel: the exact `panel_box` of Godot (painted frame, content margins only,
/// left-aligned column, 8 px gap). Use for dense panels; [`super::panel`] is the airy variant.
pub fn panel_page(theme: &UiTheme) -> impl Bundle {
    (
        KitWidget("panel"),
        Node {
            padding: theme.inner_padding("panel", PANEL_GAP),
            min_height: Val::Px(MIN_FRAME_HEIGHT),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.0),
            ..default()
        },
        theme.sliced("panel"),
    )
}

/// [`panel_page`] with a fixed width.
pub fn panel_sized(theme: &UiTheme, width: f32) -> impl Bundle {
    (
        KitWidget("panel"),
        Node {
            width: Val::Px(width),
            max_width: Val::Percent(96.0),
            padding: theme.inner_padding("panel", PANEL_GAP),
            min_height: Val::Px(MIN_FRAME_HEIGHT),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.0),
            ..default()
        },
        theme.sliced("panel"),
    )
}

/// Illuminated page (`panel_illuminated.png`: ivy scrolls, azure and gules rods, gold corners).
pub fn panel_illuminated(theme: &UiTheme) -> impl Bundle {
    (
        KitWidget("panel_illuminated"),
        Node {
            padding: theme.inner_padding("panel_illuminated", 6.0),
            min_height: Val::Px(MIN_FRAME_HEIGHT * 2.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(10.0),
            ..default()
        },
        theme.sliced("panel_illuminated"),
    )
}

/// Marginal note (tooltip frame, small windows).
pub fn panel_note(theme: &UiTheme) -> impl Bundle {
    (
        KitWidget("panel_note"),
        Node {
            padding: theme.inner_padding("tooltip", 4.0),
            min_height: Val::Px(MIN_FRAME_HEIGHT),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            ..default()
        },
        theme.sliced("tooltip"),
    )
}

/// Inset field (`inset.png`), for lists and data wells.
pub fn panel_inset(theme: &UiTheme) -> impl Bundle {
    (
        KitWidget("panel_inset"),
        Node {
            padding: theme.inner_padding("inset", 4.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            ..default()
        },
        theme.sliced("inset"),
    )
}

/// Flat card (`card_box`): vellum fill, thin ink border.
pub fn card(theme: &UiTheme) -> impl Bundle {
    let _ = theme;
    (
        KitWidget("card"),
        Node {
            padding: UiRect::axes(Val::Px(CARD_PADDING + 4.0), Val::Px(CARD_PADDING)),
            min_height: Val::Px(MIN_FRAME_HEIGHT),
            border: UiRect::all(Val::Px(1.0)),
            flex_direction: FlexDirection::Column,
            ..default()
        },
        BackgroundColor(style::PARCHMENT_LIGHT),
        BorderColor::all(style::INK_SOFT),
    )
}

/// Horizontal ornamental rule (`separator_h.png`).
pub fn separator(theme: &UiTheme) -> impl Bundle {
    (
        KitWidget("separator"),
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(12.0),
            ..default()
        },
        theme.sliced("separator_h"),
    )
}

fn button_node(kind: ButtonKind, min_width: f32) -> Node {
    let (px, py) = match kind {
        ButtonKind::Normal => (12.0, 5.0),
        ButtonKind::Primary => (14.0, 6.0),
        // Content margins of the ornate tab (`kit.json`: 14, 8, 14, 5).
        ButtonKind::Tab => (14.0, 6.5),
    };
    Node {
        min_width: Val::Px(min_width),
        padding: UiRect::axes(Val::Px(px), Val::Px(py)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        column_gap: Val::Px(6.0),
        ..default()
    }
}

fn kit_button(theme: &UiTheme, name: &str, kind: ButtonKind, min_width: f32) -> impl Bundle {
    let (texture, _) = visual_look(kind, ButtonVisual::Normal);
    (
        Name::new(name.to_owned()),
        KitWidget(match kind {
            ButtonKind::Normal => "button",
            ButtonKind::Primary => "button_primary",
            ButtonKind::Tab => "tab",
        }),
        KitButton {
            kind,
            visual: ButtonVisual::Normal,
        },
        Button,
        Hovered::default(),
        button_node(kind, min_width),
        theme.sliced(texture),
    )
}

fn button_text(theme: &UiTheme, caption: &str, kind: ButtonKind) -> impl Bundle {
    let (_, ink) = visual_look(kind, ButtonVisual::Normal);
    (
        Text::new(caption),
        theme.text_font(false, false, UiType::Body.size() + 1.0),
        TextColor(ink),
    )
}

/// Button named `name` (also its caption); press it by mouse or `click <name>` in a script.
pub fn button(theme: &UiTheme, name: &str) -> impl Bundle {
    button_captioned(theme, name, name)
}

/// Button named `name` showing `caption`.
pub fn button_captioned(theme: &UiTheme, name: &str, caption: &str) -> impl Bundle {
    (
        kit_button(theme, name, ButtonKind::Normal, 240.0),
        children![button_text(theme, caption, ButtonKind::Normal)],
    )
}

/// Compact button (no minimum width), for rows of actions.
pub fn button_small(theme: &UiTheme, name: &str, caption: &str) -> impl Bundle {
    (
        kit_button(theme, name, ButtonKind::Normal, 0.0),
        children![button_text(theme, caption, ButtonKind::Normal)],
    )
}

/// Caption of every panel close button (one wording across the campaign panels).
pub const CLOSE_CAPTION: &str = "Fermer";

/// Close button of a panel header: same caption and size everywhere.
pub fn close_button(theme: &UiTheme, name: &str) -> impl Bundle {
    button_small(theme, name, CLOSE_CAPTION)
}

/// Primary action button (`PrimaryButton`: « Confirmer », « Nouvelle partie »).
pub fn button_primary(theme: &UiTheme, name: &str, caption: &str) -> impl Bundle {
    (
        kit_button(theme, name, ButtonKind::Primary, 0.0),
        children![button_text(theme, caption, ButtonKind::Primary)],
    )
}

/// Button with an icon (path relative to `game/assets`) before its caption.
pub fn button_icon(
    theme: &UiTheme,
    assets: &AssetServer,
    name: &str,
    icon: &str,
    caption: &str,
) -> impl Bundle {
    (
        kit_button(theme, name, ButtonKind::Normal, 0.0),
        children![
            (
                Node {
                    width: Val::Px(20.0),
                    height: Val::Px(20.0),
                    ..default()
                },
                ImageNode::new(assets.load(icon.to_owned())),
            ),
            button_text(theme, caption, ButtonKind::Normal),
        ],
    )
}

/// Small button showing the library icon `icon_id` (or the `fallback` text when the library has
/// no PNG for it), like Godot's `BuildHammer`. Press it by mouse or `click <name>`.
pub fn button_glyph<'t>(
    theme: &'t UiTheme,
    assets: &AssetServer,
    name: &'t str,
    icon_id: &str,
    fallback: &str,
    size: f32,
) -> impl Bundle + use<'t> {
    let image = theme
        .icons
        .get(icon_id)
        .map(|entry| assets.load::<Image>(entry.path.clone()));
    let font = theme.text_font(false, false, UiType::Body.size());
    let fallback = fallback.to_owned();
    (
        kit_button(theme, name, ButtonKind::Normal, 0.0),
        Children::spawn(SpawnWith(move |parent: &mut ChildSpawner| {
            if let Some(image) = image {
                parent.spawn((
                    Node {
                        width: Val::Px(size),
                        height: Val::Px(size),
                        ..default()
                    },
                    ImageNode::new(image),
                ));
            } else {
                parent.spawn((Text::new(fallback), font, TextColor(style::INK)));
            }
        })),
    )
}

/// Toggle button: stays pressed while on. Emits [`ToggleChanged`].
pub fn toggle_button(theme: &UiTheme, name: &str, caption: &str, on: bool) -> impl Bundle {
    (
        kit_button(theme, name, ButtonKind::Normal, 0.0),
        Toggled(on),
        children![button_text(theme, caption, ButtonKind::Normal)],
    )
}

/// Marks a button disabled (greyed, no activation) or enabled again.
pub fn set_disabled(commands: &mut Commands, entity: Entity, disabled: bool) {
    if disabled {
        commands.entity(entity).insert(InteractionDisabled);
    } else {
        commands.entity(entity).remove::<InteractionDisabled>();
    }
}

/// Checkbox: inset box with a ✓ plus its label. Emits [`ToggleChanged`].
pub fn checkbox(theme: &UiTheme, name: &str, caption: &str, checked: bool) -> impl Bundle {
    (
        Name::new(name.to_owned()),
        KitWidget("checkbox"),
        Button,
        Hovered::default(),
        Toggled(checked),
        Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(8.0),
            ..default()
        },
        children![
            (
                Node {
                    width: Val::Px(22.0),
                    height: Val::Px(22.0),
                    border: UiRect::all(Val::Px(1.5)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                // An empty box must read as a box: dark outline over the inset texture.
                BorderColor::all(style::INK_SOFT),
                theme.sliced("inset"),
                children![(
                    CheckMark,
                    Text::new("✓"),
                    theme.text_font(true, false, 18.0),
                    TextColor(style::RUBRIC),
                    if checked {
                        Visibility::Inherited
                    } else {
                        Visibility::Hidden
                    },
                )],
            ),
            (
                Text::new(caption),
                theme.text_font(false, false, UiType::Body.size()),
                TextColor(style::INK),
            ),
        ],
    )
}

/// Row of tabs for `group`; `tabs` = `(name, caption)`, `selected` = index. A tab named `Name`
/// is also an agent click target; [`TabPane`]s of the same group follow the selection.
pub fn tab_bar(
    theme: &UiTheme,
    group: &str,
    tabs: &[(&str, &str)],
    selected: usize,
) -> impl Bundle {
    let theme = theme.clone();
    let group = group.to_owned();
    let tabs: Vec<(String, String)> = tabs
        .iter()
        .map(|(name, caption)| ((*name).to_owned(), (*caption).to_owned()))
        .collect();
    (
        Name::new(format!("Tabs {group}")),
        KitWidget("tab_bar"),
        Node {
            column_gap: Val::Px(2.0),
            align_items: AlignItems::End,
            ..default()
        },
        Children::spawn(SpawnWith(move |parent: &mut RelatedSpawner<ChildOf>| {
            for (index, (name, caption)) in tabs.iter().enumerate() {
                let on = index == selected;
                let kind = ButtonKind::Tab;
                let visual = if on {
                    ButtonVisual::Selected
                } else {
                    ButtonVisual::Normal
                };
                let (texture, ink) = visual_look(kind, visual);
                parent.spawn((
                    Name::new(name.clone()),
                    KitWidget("tab"),
                    KitButton { kind, visual },
                    TabButton {
                        group: group.clone(),
                        selected: on,
                    },
                    Button,
                    Hovered::default(),
                    button_node(kind, 0.0),
                    theme.sliced(texture),
                    children![(
                        Text::new(caption.clone()),
                        theme.text_font(on, false, UiType::Body.size()),
                        TextColor(ink),
                    )],
                ));
            }
        })),
    )
}

/// Content pane of tab `name` in `group`.
pub fn tab_pane(group: &str, name: &str, visible: bool) -> impl Bundle {
    (
        Name::new(format!("Pane {name}")),
        KitWidget("tab_pane"),
        TabPane {
            group: group.to_owned(),
            name: name.to_owned(),
        },
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            display: if visible {
                Display::Flex
            } else {
                Display::None
            },
            ..default()
        },
    )
}

/// Horizontal slider `min..=max` of `width` px; emits [`SliderChanged`].
pub fn slider(
    theme: &UiTheme,
    name: &str,
    min: f32,
    max: f32,
    value: f32,
    step: f32,
    width: f32,
) -> impl Bundle {
    let thumb = 22.0;
    let ratio = if max > min {
        (value - min) / (max - min)
    } else {
        0.0
    };
    (
        Name::new(name.to_owned()),
        KitWidget("slider"),
        Slider::default(),
        SliderValue(value),
        SliderRange::new(min, max),
        SliderStep(step),
        SliderLayout { width },
        Hovered::default(),
        Node {
            width: Val::Px(width),
            height: Val::Px(thumb),
            ..default()
        },
        children![
            (
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    top: Val::Px((thumb - 10.0) / 2.0),
                    height: Val::Px(10.0),
                    ..default()
                },
                theme.sliced("slider_track"),
            ),
            (
                SliderPart::Fill,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px((thumb - 10.0) / 2.0),
                    height: Val::Px(10.0),
                    width: Val::Px(ratio * (width - thumb) + thumb / 2.0),
                    ..default()
                },
                theme.sliced("gold_bar_h"),
            ),
            (
                SliderPart::Thumb,
                SliderThumb,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(ratio * (width - thumb)),
                    top: Val::Px(0.0),
                    width: Val::Px(thumb),
                    height: Val::Px(thumb),
                    ..default()
                },
                theme.image("slider_grabber"),
                Hovered::default(),
            ),
        ],
    )
}

/// Progress bar / gauge (`progress_frame` + `progress_fill`, filled with `gauge_color`).
pub fn gauge(theme: &UiTheme, name: &str, ratio: f32, width: f32) -> impl Bundle {
    let ratio = ratio.clamp(0.0, 1.0);
    (
        Name::new(name.to_owned()),
        KitWidget("gauge"),
        Gauge(ratio),
        Node {
            width: Val::Px(width),
            height: Val::Px(16.0),
            padding: UiRect::axes(Val::Px(3.0), Val::Px(4.0)),
            ..default()
        },
        theme.sliced("progress_frame"),
        children![(
            Node {
                width: Val::Percent(ratio * 100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            theme.sliced("progress_fill"),
            BackgroundColor(style::gauge_color(ratio).with_alpha(0.35)),
        )],
    )
}

/// Scrollable column of `height` px (mouse wheel / trackpad), inset background.
pub fn scroll_list(theme: &UiTheme, name: &str, height: f32) -> impl Bundle {
    scroll_list_with_height(theme, name, Val::Px(height))
}

/// [`tab_pane`] of `height` px that gives way down to `min_height` when its panel is capped by a
/// zone's `max_height`; its [`scroll_list_fill`] fills it and scrolls.
pub fn tab_pane_fit(
    group: &str,
    name: &str,
    visible: bool,
    height: f32,
    min_height: f32,
) -> impl Bundle {
    (
        Name::new(format!("Pane {name}")),
        KitWidget("tab_pane"),
        TabPane {
            group: group.to_owned(),
            name: name.to_owned(),
        },
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            height: Val::Px(height),
            min_height: Val::Px(min_height),
            display: if visible {
                Display::Flex
            } else {
                Display::None
            },
            ..default()
        },
    )
}

/// Entity command letting a flex child shrink down to `min` px (a scroll list that gives way when
/// its panel is capped by a zone's `max_height`): `entity.queue(shrinkable(120.0))`.
pub fn shrinkable(min: f32) -> impl FnOnce(EntityWorldMut) {
    move |mut entity: EntityWorldMut| {
        if let Some(mut node) = entity.get_mut::<Node>() {
            node.min_height = Val::Px(min);
            node.flex_shrink = 1.0;
        }
    }
}

/// [`scroll_list`] that fills its parent's height (the parent has a definite, shrinkable height).
pub fn scroll_list_fill(theme: &UiTheme, name: &str) -> impl Bundle {
    scroll_list_with_height(theme, name, Val::Percent(100.0))
}

fn scroll_list_with_height(theme: &UiTheme, name: &str, height: Val) -> impl Bundle {
    (
        Name::new(name.to_owned()),
        KitWidget("scroll_list"),
        KitScrollList,
        ScrollArea,
        Node {
            height,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(2.0),
            padding: {
                let mut padding = theme.inner_padding("inset", 4.0);
                padding.right = Val::Px(SCROLLBAR_WIDTH + 8.0);
                padding
            },
            overflow: Overflow::scroll_y(),
            ..default()
        },
        theme.sliced("inset"),
    )
}

/// Scrollable body of a dialog: takes the height the frame leaves (`flex_grow`, `min_height: 0`),
/// no inset background, the same wheel and scrollbar as [`scroll_list`]. Put the dialog's fixed
/// foot (buttons) after it, outside, so it never scrolls away.
pub fn scroll_body(name: &str) -> impl Bundle {
    (
        Name::new(name.to_owned()),
        KitWidget("scroll_body"),
        KitScrollList,
        ScrollArea,
        Node {
            flex_grow: 1.0,
            flex_shrink: 1.0,
            min_height: Val::Px(0.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.0),
            padding: UiRect::right(Val::Px(SCROLLBAR_WIDTH + 8.0)),
            overflow: Overflow::scroll_y(),
            ..default()
        },
    )
}

/// Thin parchment scrollbar for every new [`KitScrollList`]: a track pinned to the right edge of
/// the list (it ignores the list's own scroll) with a draggable thumb. Bevy's `Scrollbar` moves
/// the thumb with the scroll position and scrolls the list when the thumb is dragged; the wheel
/// is handled by `ScrollArea`.
fn attach_scrollbars(
    mut commands: Commands,
    theme: Res<UiTheme>,
    lists: Query<Entity, Added<KitScrollList>>,
) {
    for list in &lists {
        commands.entity(list).with_child((
            Name::new("ScrollbarTrack"),
            KitWidget("scrollbar"),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(2.0),
                bottom: Val::Px(2.0),
                right: Val::Px(2.0),
                width: Val::Px(SCROLLBAR_WIDTH),
                border_radius: BorderRadius::all(Val::Px(SCROLLBAR_WIDTH * 0.5)),
                ..default()
            },
            IgnoreScroll(BVec2::new(true, true)),
            BackgroundColor(style::PARCHMENT_DARK.with_alpha(0.45)),
            Scrollbar::new(list, ControlOrientation::Vertical, 24.0),
            children![(
                ScrollbarThumb::default(),
                // The painted gold bar of the theme's `VScrollBar` grabber (`gold_bar_v`).
                theme.sliced("gold_bar_v"),
            )],
        ));
    }
}

/// Selectable row of a list (`group` = one selection per group). Emits [`ListSelected`].
pub fn list_item(
    theme: &UiTheme,
    group: &str,
    name: &str,
    caption: &str,
    selected: bool,
) -> impl Bundle {
    (
        Name::new(name.to_owned()),
        KitWidget("list_item"),
        ListItem {
            group: group.to_owned(),
        },
        Toggled(selected),
        Button,
        Hovered::default(),
        Node {
            padding: UiRect::axes(Val::Px(8.0), Val::Px(3.0)),
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::NONE),
        children![(
            Text::new(caption),
            theme.text_font(false, false, UiType::Body.size()),
            TextColor(style::INK),
        )],
    )
}

/// Icon + label on one line with a rich tooltip over the whole chip (`IconChip`).
pub fn icon_chip(
    theme: &UiTheme,
    assets: &AssetServer,
    icon: &str,
    content: &str,
    tooltip_bbcode: &str,
) -> impl Bundle {
    (
        Name::new(format!("IconChip {content}")),
        KitWidget("icon_chip"),
        Tooltip::new(tooltip_bbcode),
        Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(4.0),
            ..default()
        },
        children![
            (
                Node {
                    width: Val::Px(20.0),
                    height: Val::Px(20.0),
                    ..default()
                },
                ImageNode::new(assets.load(icon.to_owned())),
            ),
            (
                Text::new(content),
                theme.text_font(false, false, UiType::Caption.size()),
                TextColor(style::INK),
            ),
        ],
    )
}

/// Illuminated window title (`Lettrine`): the initial in gold on an azure field with a double
/// gold rule, the rest of the title in IM Fell.
pub fn lettrine(theme: &UiTheme, content: &str, size: f32) -> impl Bundle {
    let mut chars = content.chars();
    let initial: String = chars.next().map(unaccent).unwrap_or(' ').to_string();
    let rest: String = chars.collect();
    let field = (size * 1.5).round();
    (
        Name::new("Lettrine"),
        KitWidget("lettrine"),
        Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(LETTRINE_GAP),
            ..default()
        },
        children![
            (
                Node {
                    width: Val::Px(field),
                    min_width: Val::Px(field),
                    height: Val::Px(field),
                    flex_shrink: 0.0,
                    border: UiRect::all(Val::Px(2.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(style::FE_AZURE),
                BorderColor::all(style::FE_GOLD),
                children![(
                    Text::new(initial),
                    theme.title_font(size * 1.15),
                    TextColor(style::FE_GOLD),
                )],
            ),
            (
                Node {
                    flex_shrink: 1.0,
                    ..default()
                },
                Text::new(rest),
                theme.title_font(size),
                TextColor(style::INK_TITLE),
            ),
        ],
    )
}

/// Space between the lettrine's field and the rest of the title (Godot `Lettrine.GAP` is 1 px
/// with its glyph drawn inside the field; here the rest of the title is its own text node).
const LETTRINE_GAP: f32 = 6.0;

/// Lettrines carry no accents (the accent would overflow the field).
pub fn unaccent(letter: char) -> char {
    match letter {
        'À' | 'Â' | 'Ä' => 'A',
        'Ç' => 'C',
        'É' | 'È' | 'Ê' | 'Ë' => 'E',
        'Î' | 'Ï' => 'I',
        'Ô' | 'Ö' => 'O',
        'Ù' | 'Û' | 'Ü' => 'U',
        'Ÿ' => 'Y',
        other => other,
    }
}

/// Front-end illuminated title: gold lettrine on azure, rest in vellum, italic subtitle.
pub fn illuminated_title(theme: &UiTheme, content: &str, subtitle: &str) -> impl Bundle {
    illuminated_title_colored(
        theme,
        content,
        subtitle,
        Color::srgb(0.97, 0.92, 0.80),
        Color::srgb(0.93, 0.86, 0.70),
    )
}

/// [`illuminated_title`] with its own text colours: the cream defaults are made for a dark
/// painted background, on parchment pass ink colours.
pub fn illuminated_title_colored(
    theme: &UiTheme,
    content: &str,
    subtitle: &str,
    title_color: Color,
    subtitle_color: Color,
) -> impl Bundle {
    let mut chars = content.chars();
    let initial: String = chars.next().map(unaccent).unwrap_or(' ').to_string();
    let rest: String = chars.collect();
    (
        Name::new("IlluminatedTitle"),
        KitWidget("illuminated_title"),
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.0),
            ..default()
        },
        children![
            (
                Node {
                    align_items: AlignItems::End,
                    column_gap: Val::Px(14.0),
                    ..default()
                },
                children![
                    (
                        Node {
                            width: Val::Px(132.0),
                            height: Val::Px(132.0),
                            border: UiRect::all(Val::Px(3.0)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(style::FE_AZURE),
                        BorderColor::all(style::FE_GOLD),
                        children![(
                            Text::new(initial),
                            theme.title_font(130.0),
                            TextColor(style::FE_GOLD),
                        )],
                    ),
                    (
                        Text::new(rest),
                        theme.title_font(92.0),
                        TextColor(title_color),
                    ),
                ],
            ),
            (
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(2.0),
                    ..default()
                },
                BackgroundColor(style::FE_GOLD.with_alpha(0.85)),
            ),
            (
                Text::new(subtitle),
                TextFont {
                    font: theme.font_title_italic.clone(),
                    font_size: 24.0.into(),
                    ..default()
                },
                TextColor(subtitle_color),
            ),
        ],
    )
}

// ---- systems -----------------------------------------------------------------------------

fn handle_toggles(
    mut pressed: MessageReader<ButtonPressed>,
    mut toggles: Query<(&mut Toggled, Has<ListItem>, Has<InteractionDisabled>)>,
    mut changed: MessageWriter<ToggleChanged>,
) {
    for press in pressed.read() {
        let Ok((mut toggled, is_list_item, disabled)) = toggles.get_mut(press.entity) else {
            continue;
        };
        if is_list_item || disabled {
            continue;
        }
        toggled.0 = !toggled.0;
        changed.write(ToggleChanged {
            entity: press.entity,
            name: press.name.clone(),
            on: toggled.0,
        });
    }
}

fn handle_tabs(
    mut pressed: MessageReader<ButtonPressed>,
    mut tabs: Query<(Entity, &mut TabButton, &Name)>,
    mut selected: MessageWriter<TabSelected>,
) {
    for press in pressed.read() {
        let Ok((_, tab, _)) = tabs.get(press.entity) else {
            continue;
        };
        let group = tab.group.clone();
        for (entity, mut other, _) in &mut tabs {
            if other.group == group {
                let now = entity == press.entity;
                if other.selected != now {
                    other.selected = now;
                }
            }
        }
        selected.write(TabSelected {
            entity: press.entity,
            group,
            name: press.name.clone(),
        });
    }
}

fn handle_list_items(
    mut pressed: MessageReader<ButtonPressed>,
    mut items: Query<(Entity, &ListItem, &mut Toggled)>,
    mut selected: MessageWriter<ListSelected>,
) {
    for press in pressed.read() {
        let Ok((_, item, _)) = items.get(press.entity) else {
            continue;
        };
        let group = item.group.clone();
        for (entity, other, mut toggled) in &mut items {
            if other.group == group {
                toggled.0 = entity == press.entity;
            }
        }
        selected.write(ListSelected {
            entity: press.entity,
            group,
            name: press.name.clone(),
        });
    }
}

/// Applies texture and ink of the current visual state to every kit button.
#[allow(clippy::type_complexity)]
fn button_visuals(
    theme: Res<UiTheme>,
    mut buttons: Query<(
        &mut KitButton,
        &Hovered,
        Has<Pressed>,
        Has<InteractionDisabled>,
        Option<&Toggled>,
        Option<&TabButton>,
        &mut ImageNode,
        Option<&Children>,
    )>,
    mut inks: Query<(&mut TextColor, &mut TextFont)>,
) {
    for (mut kit, hovered, pressed, disabled, toggled, tab, mut image, children) in &mut buttons {
        let on = toggled.is_some_and(|t| t.0) || tab.is_some_and(|t| t.selected);
        let visual = visual_state(hovered.get(), pressed, disabled, on);
        if visual == kit.visual {
            continue;
        }
        kit.visual = visual;
        let (texture, ink) = visual_look(kit.kind, visual);
        if let Some(handle) = theme.textures.get(texture) {
            image.image = handle.clone();
        }
        for child in children.into_iter().flatten() {
            if let Ok((mut color, mut font)) = inks.get_mut(*child) {
                color.0 = ink;
                if kit.kind == ButtonKind::Tab {
                    font.weight = bevy::text::FontWeight(if on { 650 } else { 470 });
                }
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn sync_checkmarks(
    boxes: Query<(&Toggled, &Children), (Changed<Toggled>, With<Button>, Without<KitButton>)>,
    children_of: Query<&Children>,
    mut marks: Query<&mut Visibility, With<CheckMark>>,
) {
    for (toggled, children) in &boxes {
        for child in children.iter() {
            let Ok(grandchildren) = children_of.get(child) else {
                continue;
            };
            for grandchild in grandchildren.iter() {
                if let Ok(mut visibility) = marks.get_mut(grandchild) {
                    *visibility = if toggled.0 {
                        Visibility::Inherited
                    } else {
                        Visibility::Hidden
                    };
                }
            }
        }
    }
}

fn sync_tab_panes(
    tabs: Query<(&TabButton, &Name), Changed<TabButton>>,
    mut panes: Query<(&TabPane, &mut Node)>,
) {
    for (tab, name) in &tabs {
        if !tab.selected {
            continue;
        }
        for (pane, mut node) in &mut panes {
            if pane.group == tab.group {
                node.display = if pane.name == name.as_str() {
                    Display::Flex
                } else {
                    Display::None
                };
            }
        }
    }
}

fn list_item_visuals(mut items: Query<(&Toggled, &Hovered, &mut BackgroundColor), With<ListItem>>) {
    for (toggled, hovered, mut fill) in &mut items {
        let target = if toggled.0 {
            style::GOLD_PALE.with_alpha(0.55)
        } else if hovered.get() {
            style::PARCHMENT_DARK.with_alpha(0.35)
        } else {
            Color::NONE
        };
        if fill.0 != target {
            fill.0 = target;
        }
    }
}

fn on_slider_change(
    change: On<ValueChange<f32>>,
    mut commands: Commands,
    names: Query<&Name>,
    mut writer: MessageWriter<SliderChanged>,
) {
    commands
        .entity(change.source)
        .insert(SliderValue(change.value));
    writer.write(SliderChanged {
        entity: change.source,
        name: names
            .get(change.source)
            .map(|name| name.to_string())
            .unwrap_or_default(),
        value: change.value,
        is_final: change.is_final,
    });
}

#[allow(clippy::type_complexity)]
fn sync_sliders(
    sliders: Query<(&SliderValue, &SliderRange, &SliderLayout, &Children), Changed<SliderValue>>,
    mut parts: Query<(&SliderPart, &mut Node)>,
) {
    let thumb = 22.0;
    for (value, range, layout, children) in &sliders {
        let span = range.end() - range.start();
        let ratio = if span > 0.0 {
            ((value.0 - range.start()) / span).clamp(0.0, 1.0)
        } else {
            0.0
        };
        for child in children.iter() {
            let Ok((part, mut node)) = parts.get_mut(child) else {
                continue;
            };
            match part {
                SliderPart::Thumb => node.left = Val::Px(ratio * (layout.width - thumb)),
                SliderPart::Fill => {
                    node.width = Val::Px(ratio * (layout.width - thumb) + thumb / 2.0)
                }
            }
        }
    }
}

fn sync_gauges(
    gauges: Query<(&Gauge, &Children), Changed<Gauge>>,
    mut fills: Query<(&mut Node, &mut BackgroundColor)>,
) {
    for (gauge, children) in &gauges {
        for child in children.iter() {
            if let Ok((mut node, mut fill)) = fills.get_mut(child) {
                node.width = Val::Percent(gauge.0.clamp(0.0, 1.0) * 100.0);
                fill.0 = style::gauge_color(gauge.0).with_alpha(0.35);
            }
        }
    }
}

/// Widget counts by kind plus rich text / link / button state counts, for the dump.
pub fn count_widgets(world: &World) -> Value {
    let mut by_kind: std::collections::BTreeMap<&'static str, usize> = Default::default();
    let mut rich_texts = 0;
    let mut links = 0;
    let mut disabled = 0;
    let mut toggled_on = Vec::new();
    let mut selected_tabs = Vec::new();
    let mut sliders = Vec::new();
    for entity in world.iter_entities() {
        let name = entity.get::<Name>().map(|name| name.to_string());
        if entity.get::<Toggled>().is_some_and(|t| t.0)
            && let Some(name) = &name
        {
            toggled_on.push(name.clone());
        }
        if entity.get::<TabButton>().is_some_and(|t| t.selected)
            && let Some(name) = &name
        {
            selected_tabs.push(name.clone());
        }
        if let (Some(value), Some(name)) = (entity.get::<SliderValue>(), &name) {
            sliders.push(json!({"name": name, "value": value.0}));
        }
        if let Some(kind) = entity.get::<KitWidget>() {
            *by_kind.entry(kind.0).or_default() += 1;
        }
        rich_texts += usize::from(entity.contains::<RichText>());
        links += usize::from(entity.contains::<RichLink>());
        disabled += usize::from(entity.contains::<InteractionDisabled>());
    }
    json!({
        "by_kind": by_kind,
        "total": by_kind.values().sum::<usize>(),
        "rich_texts": rich_texts,
        "links": links,
        "disabled": disabled,
        "toggled_on": toggled_on,
        "selected_tabs": selected_tabs,
        "sliders": sliders,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_priority() {
        assert_eq!(visual_state(true, true, true, true), ButtonVisual::Disabled);
        assert_eq!(visual_state(true, true, false, true), ButtonVisual::Pressed);
        assert_eq!(
            visual_state(true, false, false, true),
            ButtonVisual::Selected
        );
        assert_eq!(visual_state(true, false, false, false), ButtonVisual::Hover);
        assert_eq!(
            visual_state(false, false, false, false),
            ButtonVisual::Normal
        );
    }

    #[test]
    fn looks_name_existing_kit_textures() {
        let names = [
            "button_normal",
            "button_hover",
            "button_pressed",
            "button_disabled",
            "button_primary_normal",
            "button_primary_hover",
            "button_primary_pressed",
            "button_primary_disabled",
            "tab_ornate_selected",
            "tab_ornate_unselected",
        ];
        for kind in [ButtonKind::Normal, ButtonKind::Primary, ButtonKind::Tab] {
            for visual in [
                ButtonVisual::Normal,
                ButtonVisual::Hover,
                ButtonVisual::Pressed,
                ButtonVisual::Disabled,
                ButtonVisual::Selected,
            ] {
                assert!(names.contains(&visual_look(kind, visual).0));
            }
        }
    }

    #[test]
    fn lettrine_drops_accents() {
        assert_eq!(unaccent('É'), 'E');
        assert_eq!(unaccent('Ç'), 'C');
        assert_eq!(unaccent('D'), 'D');
    }
}
