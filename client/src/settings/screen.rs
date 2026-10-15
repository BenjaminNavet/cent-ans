//! The Réglages screen: a modal panel with tabs, built from [`ROWS`] with the UI kit.
//!
//! Changes apply live to the [`Settings`] resource; « Valider » writes the file, « Annuler »
//! (or Échap) restores the settings as they were when the screen opened, « Défaut » resets.

use bevy::prelude::*;
use bevy::ui::FocusPolicy;
use bevy::ui_widgets::SliderValue;

use super::model::Settings;
use super::rows::{Kind, ROWS, Row, Tab, row};
use super::{OpenSettings, SettingsFile};
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::widgets::{
    ListSelected, SliderChanged, TabSelected, ToggleChanged, button_primary, button_small,
    checkbox, list_item, scroll_list, slider, tab_bar, tab_pane,
};
use crate::ui::kit::{ButtonPressed, UiTheme, UiTier, UiType, typed};

const PANEL_WIDTH: f32 = 800.0;
const BODY_HEIGHT: f32 = 400.0;
const TAB_GROUP: &str = "settings";

#[derive(Resource, Default)]
pub struct SettingsScreen {
    pub root: Option<Entity>,
    /// Settings when the screen opened (restored by « Annuler »).
    pub snapshot: Option<Settings>,
    pub tab: Option<Tab>,
    /// Set by F10 / Échap while open: handled as « Annuler ».
    pub cancel_requested: bool,
}

/// Text showing the value of the slider row `key`.
#[derive(Component)]
pub struct ValueLabel(&'static str);

fn widget_name(key: &str) -> String {
    format!("settings:{key}")
}

pub fn open_and_close(
    mut commands: Commands,
    mut opens: MessageReader<OpenSettings>,
    keys: Res<ButtonInput<KeyCode>>,
    theme: Res<UiTheme>,
    settings: Res<Settings>,
    mut screen: ResMut<SettingsScreen>,
    mut was_down: Local<(bool, bool)>,
) {
    let requested = opens.read().count() > 0;
    // Edge detection on `pressed`: a key pushed by an agent script is cleared of `just_pressed`
    // before `Update` runs.
    let (f10, esc) = (keys.pressed(KeyCode::F10), keys.pressed(KeyCode::Escape));
    let toggle = f10 && !was_down.0;
    let escape = esc && !was_down.1;
    *was_down = (f10, esc);
    if screen.root.is_some() {
        if toggle || escape {
            screen.cancel_requested = true;
        }
    } else if requested || toggle {
        screen.snapshot = Some(settings.clone());
        let tab = screen.tab.unwrap_or(Tab::Display);
        screen.root = Some(spawn(&mut commands, &theme, &settings, tab));
    }
}

/// Builds the whole screen on the current `settings`; returns its root.
pub fn spawn(commands: &mut Commands, theme: &UiTheme, settings: &Settings, tab: Tab) -> Entity {
    let mut current = settings.clone();
    commands
        .spawn((
            Name::new("Settings"),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45)),
            UiTier::Modal.index(),
            FocusPolicy::Block,
        ))
        .with_children(|root| {
            root.spawn((
                theme.sliced("panel"),
                Node {
                    width: Val::Px(PANEL_WIDTH),
                    padding: theme.padding("panel"),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(10.0),
                    ..default()
                },
            ))
            .with_children(|panel| {
                panel.spawn(typed(theme, "Réglages", UiType::Title));
                let tabs: Vec<(String, &str)> = Tab::ALL
                    .iter()
                    .map(|tab| (widget_name(&format!("tab_{}", tab.id())), tab.caption()))
                    .collect();
                let tab_refs: Vec<(&str, &str)> = tabs
                    .iter()
                    .map(|(name, caption)| (name.as_str(), *caption))
                    .collect();
                let selected = Tab::ALL.iter().position(|t| *t == tab).unwrap_or(0);
                panel.spawn(tab_bar(theme, TAB_GROUP, &tab_refs, selected));
                panel
                    .spawn(scroll_list(theme, &widget_name("scroll"), BODY_HEIGHT))
                    .with_children(|body| {
                        for pane_tab in Tab::ALL {
                            body.spawn(tab_pane(
                                TAB_GROUP,
                                &widget_name(&format!("tab_{}", pane_tab.id())),
                                pane_tab == tab,
                            ))
                            .with_children(|pane| {
                                for row in ROWS.iter().filter(|row| row.tab == pane_tab) {
                                    spawn_row(pane, theme, row, &mut current);
                                }
                                if pane_tab == Tab::Controls {
                                    spawn_controls_footer(pane, theme, settings);
                                }
                            });
                        }
                    });
                panel
                    .spawn(Node {
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|bar| {
                        bar.spawn(button_small(
                            theme,
                            &widget_name("default"),
                            "Valeurs par défaut",
                        ));
                        bar.spawn(Node {
                            column_gap: Val::Px(10.0),
                            ..default()
                        })
                        .with_children(|group| {
                            group.spawn(button_small(theme, &widget_name("cancel"), "Annuler"));
                            group.spawn(button_primary(theme, &widget_name("ok"), "Valider"));
                        });
                    });
            });
        })
        .id()
}

fn spawn_row(
    pane: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    row: &'static Row,
    current: &mut Settings,
) {
    let mut line = pane.spawn(Node {
        width: Val::Percent(100.0),
        justify_content: JustifyContent::SpaceBetween,
        align_items: AlignItems::Center,
        column_gap: Val::Px(16.0),
        padding: UiRect::vertical(Val::Px(2.0)),
        ..default()
    });
    if !row.tip.is_empty() {
        line.insert(Tooltip::new(row.tip));
    }
    line.with_children(|line| match &row.kind {
        Kind::Check(field) => {
            let on = *field(current);
            line.spawn(checkbox(theme, &widget_name(row.key), row.label, on));
        }
        Kind::Slider {
            min,
            max,
            step,
            format,
            field,
        } => {
            let value = *field(current);
            line.spawn(typed(theme, row.label, UiType::Body));
            line.spawn(Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                ..default()
            })
            .with_children(|group| {
                group.spawn(slider(
                    theme,
                    &widget_name(row.key),
                    *min,
                    *max,
                    value,
                    *step,
                    240.0,
                ));
                group.spawn((
                    ValueLabel(row.key),
                    Node {
                        min_width: Val::Px(54.0),
                        ..default()
                    },
                    typed(theme, &format.show(value), UiType::Caption),
                ));
            });
        }
        Kind::Choice { labels, get, .. } => {
            let selected = get(current);
            line.spawn(typed(theme, row.label, UiType::Body));
            line.spawn(Node {
                max_width: Val::Px(440.0),
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::FlexEnd,
                column_gap: Val::Px(4.0),
                ..default()
            })
            .with_children(|group| {
                let list = format!("choice:{}", row.key);
                for (index, label) in labels.iter().enumerate() {
                    group.spawn(list_item(
                        theme,
                        &list,
                        &widget_name(&format!("{}:{index}", row.key)),
                        label,
                        selected == Some(index),
                    ));
                }
            });
        }
    });
}

fn spawn_controls_footer(pane: &mut ChildSpawnerCommands, theme: &UiTheme, settings: &Settings) {
    let custom = settings.bindings.len() + settings.battle_bindings.len();
    pane.spawn((
        ValueLabel("bindings_count"),
        typed(
            theme,
            &format!("Touches réaffectées : {custom}"),
            UiType::Caption,
        ),
    ));
    pane.spawn(button_small(
        theme,
        &widget_name("restore_keys"),
        "Rétablir les touches par défaut",
    ));
}

/// Applies widget messages to `Settings` and handles the bottom buttons.
#[allow(clippy::too_many_arguments)]
pub fn handle_widgets(
    mut commands: Commands,
    theme: Res<UiTheme>,
    mut settings: ResMut<Settings>,
    mut screen: ResMut<SettingsScreen>,
    mut file: ResMut<SettingsFile>,
    mut toggles: MessageReader<ToggleChanged>,
    mut lists: MessageReader<ListSelected>,
    mut sliders: MessageReader<SliderChanged>,
    mut tabs: MessageReader<TabSelected>,
    mut presses: MessageReader<ButtonPressed>,
) {
    if screen.root.is_none() {
        // Drain so nothing leaks into the next opening.
        toggles.clear();
        lists.clear();
        sliders.clear();
        tabs.clear();
        presses.clear();
        screen.cancel_requested = false;
        return;
    }
    for toggle in toggles.read() {
        if let Some(Row {
            kind: Kind::Check(field),
            ..
        }) = key_of(&toggle.name).and_then(row)
        {
            *field(&mut settings) = toggle.on;
        }
    }
    for list in lists.read() {
        let Some((key, index)) = key_of(&list.name).and_then(|rest| rest.split_once(':')) else {
            continue;
        };
        if let (
            Some(Row {
                kind: Kind::Choice { set, labels, .. },
                ..
            }),
            Ok(index),
        ) = (row(key), index.parse::<usize>())
            && index < labels.len()
        {
            set(&mut settings, index);
        }
    }
    for change in sliders.read() {
        if let Some(Row {
            kind: Kind::Slider { field, .. },
            ..
        }) = key_of(&change.name).and_then(row)
        {
            let target = field(&mut settings);
            if (*target - change.value).abs() > f32::EPSILON {
                *target = change.value;
            }
        }
    }
    for tab in tabs.read() {
        if tab.group == TAB_GROUP {
            screen.tab = Tab::ALL
                .into_iter()
                .find(|t| tab.name == widget_name(&format!("tab_{}", t.id())));
        }
    }
    let mut rebuild = false;
    let mut close = false;
    let mut cancel = std::mem::take(&mut screen.cancel_requested);
    let mut save = false;
    for press in presses.read() {
        match press.name.as_str() {
            "settings:ok" => {
                save = true;
                close = true;
            }
            "settings:cancel" => cancel = true,
            "settings:default" => {
                let keep = std::mem::take(&mut settings.bindings);
                let keep_battle = std::mem::take(&mut settings.battle_bindings);
                *settings = Settings {
                    bindings: keep,
                    battle_bindings: keep_battle,
                    ..Settings::default()
                };
                rebuild = true;
            }
            "settings:restore_keys" => {
                settings.bindings.clear();
                settings.battle_bindings.clear();
                settings.keyboard_layout = Settings::default().keyboard_layout;
                rebuild = true;
            }
            name => {
                // A click on a slider by name (agent scripts): advance it by a tenth of its range.
                if let Some(Row {
                    kind:
                        Kind::Slider {
                            min,
                            max,
                            step,
                            field,
                            ..
                        },
                    ..
                }) = key_of(name).and_then(row)
                {
                    let mut value = *field(&mut settings) + (max - min) * 0.1;
                    if value > *max + 1e-4 {
                        value = *min;
                    }
                    value = (value / step).round() * step;
                    value = value.clamp(*min, *max);
                    *field(&mut settings) = value;
                    commands.entity(press.entity).insert(SliderValue(value));
                }
            }
        }
    }
    if cancel {
        if let Some(snapshot) = screen.snapshot.clone() {
            *settings = snapshot;
        }
        close = true;
    }
    if save {
        file.save(&settings);
    }
    if close || rebuild {
        if let Some(root) = screen.root.take() {
            commands.entity(root).despawn();
        }
        if rebuild && !close {
            let tab = screen.tab.unwrap_or(Tab::Display);
            screen.root = Some(spawn(&mut commands, &theme, &settings, tab));
        } else {
            screen.snapshot = None;
        }
    }
}

fn key_of(name: &str) -> Option<&str> {
    name.strip_prefix("settings:")
}

/// Keeps the value texts of the sliders in step with `Settings`.
pub fn refresh_value_labels(settings: Res<Settings>, mut labels: Query<(&ValueLabel, &mut Text)>) {
    if !settings.is_changed() {
        return;
    }
    let mut scratch = settings.clone();
    for (label, mut text) in &mut labels {
        let shown = if label.0 == "bindings_count" {
            format!(
                "Touches réaffectées : {}",
                settings.bindings.len() + settings.battle_bindings.len()
            )
        } else if let Some(Row {
            kind: Kind::Slider { format, field, .. },
            ..
        }) = row(label.0)
        {
            format.show(*field(&mut scratch))
        } else {
            continue;
        };
        if text.0 != shown {
            text.0 = shown;
        }
    }
}
