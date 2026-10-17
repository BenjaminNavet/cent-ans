//! Campaign top bar: shield and name of the faction, figures with their icon, date, and the
//! launcher of the windows (round medallion buttons), then « Menu ».
//!
//! Ports the `TopBar` of `game/scenes/campaign_map.tscn` as decorated by `map_ui.gd`
//! (`_decorate_button`, `_apply_top_medallion`, `_add_keycap`) and `ui/top_bar_fit.gd`. Godot
//! shows a short caption beside each icon when the bar has room; here every launcher is an icon
//! with its shortcut letter, the name and key in the tooltip (the bar never wraps, from 1280 px
//! up). Improvement over Godot: one fixed height (`ui::kit::layout::TOP_BAR_HEIGHT`) that every
//! panel docks under, and figures that keep their place (icon + number, no « Trésor : » labels).

use bevy::picking::hover::Hovered;
use bevy::prelude::*;

use super::{HudField, HudWidget, PanelButtons, TopBarMark};
use crate::settings::{KeyMap, Settings};
use crate::ui::kit::UiTheme;
use crate::ui::kit::icons::{icon_tinted, medallion};
use crate::ui::kit::layout;
use crate::ui::kit::style::{self, UiType};
use crate::ui::kit::tooltip::Tooltip;

/// Diameter of a launcher button.
pub const LAUNCHER_SIZE: f32 = 32.0;
/// Size of a figure icon.
const FIGURE_ICON: f32 = 20.0;

/// Text of a top-bar figure.
#[derive(Component)]
pub(super) struct FieldText;

/// Image of a top-bar figure that changes with the state (shield, season).
#[derive(Component)]
pub(super) struct FieldImage;

/// A round launcher button of the top bar (`hud:open_*`, objectives, traditions, ransoms, menu).
#[derive(Component)]
pub struct LauncherButton;

/// Count badge of a launcher button (traditions to spend, captives…).
#[derive(Component)]
pub struct LauncherBadge;

/// Shortcut labels of the key map (`KeyMap::label` in the player's keyboard layout).
pub struct KeyHints<'a> {
    keymap: Option<&'a KeyMap>,
    azerty: bool,
}

impl<'a> KeyHints<'a> {
    pub fn new(keymap: Option<&'a KeyMap>, settings: Option<&Settings>) -> Self {
        Self {
            keymap,
            azerty: settings.is_none_or(|s| s.keyboard_layout != "qwerty"),
        }
    }

    /// Label of `action` (`F4`, `C`…), empty when unbound or unknown.
    pub fn label(&self, action: &str) -> String {
        if action.is_empty() {
            return String::new();
        }
        self.keymap.map_or_else(String::new, |map| {
            map.label(action, if self.azerty { "azerty" } else { "qwerty" })
        })
    }

    /// Replaces every `{key:ACTION}` of `text` with the key actually bound to the action (no
    /// key is ever written in the data: the player may have re-bound it). An unbound action
    /// leaves the word « (non assignée) ».
    pub fn expand(&self, text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut rest = text;
        while let Some(start) = rest.find("{key:") {
            out.push_str(&rest[..start]);
            let after = &rest[start + 5..];
            let Some(end) = after.find('}') else {
                out.push_str(&rest[start..]);
                return out;
            };
            let label = self.label(&after[..end]);
            out.push_str(if label.is_empty() {
                "non assignée"
            } else {
                &label
            });
            rest = &after[end + 1..];
        }
        out.push_str(rest);
        out
    }
}

/// One window of the launcher: button name, icon id, French name, tooltip line, key action.
pub struct Launcher {
    pub name: &'static str,
    pub icon: &'static str,
    pub title: &'static str,
    pub detail: &'static str,
    pub action: &'static str,
}

/// The fixed launchers, in Godot's order (court, techs, codex… then diplomacy, chronicle).
pub const LAUNCHERS: [Launcher; 9] = [
    Launcher {
        name: "hud:open_council",
        icon: "hud_court",
        title: "Conseil",
        detail: "Charges du royaume, cour et conseils.",
        action: "map_toggle_council",
    },
    Launcher {
        name: "hud:open_tech",
        icon: "hud_technologies",
        title: "Techniques",
        detail: "Arbre des techniques et recherche en cours.",
        action: "map_toggle_tech",
    },
    Launcher {
        name: "hud:open_diplomacy",
        icon: "hud_diplomacy",
        title: "Diplomatie",
        detail: "Relations, traités et religion.",
        action: "map_toggle_diplomacy",
    },
    Launcher {
        name: "hud:open_recruitment",
        icon: "recruit",
        title: "Recrutement",
        detail: "Lever des troupes dans la province de l’armée choisie, sinon à la capitale.",
        action: "",
    },
    Launcher {
        name: "hud:open_economy",
        icon: "hud_coinage",
        title: "Économie",
        detail: "Recettes, dépenses, commerce et monnaie.",
        action: "map_toggle_economy",
    },
    Launcher {
        name: "hud:open_faction",
        icon: "glyph_flag",
        title: "Ma faction",
        detail: "Impôts, édits, lois et état du royaume.",
        action: "",
    },
    Launcher {
        name: "hud:open_feudal",
        icon: "glyph_crown",
        title: "Féodalité",
        detail: "Titres, vassaux et suzerains.",
        action: "",
    },
    Launcher {
        name: "hud:open_chronicle",
        icon: "hud_chronicle",
        title: "Chronique",
        detail: "Récit de la partie et décisions de chronique.",
        action: "map_toggle_chronicle",
    },
    Launcher {
        name: "hud:open_codex",
        icon: "hud_codex",
        title: "Codex",
        detail: "Encyclopédie : règles, unités, lieux et personnages.",
        action: "",
    },
];

/// Tooltip of a launcher: bold name, key, one line.
pub fn launcher_tooltip(title: &str, key: &str, detail: &str) -> String {
    if key.is_empty() {
        format!("[b]{title}[/b]\n{detail}")
    } else {
        format!("[b]{title}[/b] ({key})\n{detail}")
    }
}

/// Round launcher button `name`: the medallion of `icon` (an ink icon on a parchment disc when no
/// medallion exists), the short key in a corner. Emits `ButtonPressed` like a kit button.
pub fn launcher_button(
    theme: &UiTheme,
    assets: &AssetServer,
    name: &str,
    icon: &str,
    key: &str,
) -> impl Bundle {
    round_button(theme, assets, name, icon, key, LAUNCHER_SIZE)
}

/// [`launcher_button`] of any diameter (map-mode buttons of the minimap).
pub fn round_button(
    theme: &UiTheme,
    assets: &AssetServer,
    name: &str,
    icon: &str,
    key: &str,
    size: f32,
) -> impl Bundle {
    let is_medallion = theme.icons.medallion(icon).is_some();
    let inner = if is_medallion { size } else { size * 0.66 };
    let key = (!key.is_empty() && key.chars().count() <= 3).then(|| key.to_owned());
    (
        Name::new(name.to_owned()),
        LauncherButton,
        HudWidget,
        bevy::ui_widgets::Button,
        Hovered::default(),
        Node {
            width: Val::Px(size),
            height: Val::Px(size),
            flex_shrink: 0.0,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(Val::Px(if is_medallion { 0.0 } else { 1.5 })),
            border_radius: BorderRadius::MAX,
            ..default()
        },
        BorderColor::all(style::GOLD),
        Outline::new(Val::Px(2.0), Val::Px(1.0), Color::NONE),
        BackgroundColor(if is_medallion {
            Color::NONE
        } else {
            style::PARCHMENT
        }),
        children![
            (
                medallion(theme, assets, icon, inner, style::INK),
                Pickable::IGNORE
            ),
            keycap(theme, key),
        ],
    )
}

/// Little key letter at the bottom right of a launcher (`MapUI._add_keycap`).
fn keycap(theme: &UiTheme, key: Option<String>) -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(-5.0),
            bottom: Val::Px(-4.0),
            padding: UiRect::axes(Val::Px(2.0), Val::Px(0.0)),
            display: if key.is_some() {
                Display::Flex
            } else {
                Display::None
            },
            ..default()
        },
        BackgroundColor(style::PARCHMENT_LIGHT),
        Outline::new(
            Val::Px(if key.is_some() { 1.0 } else { 0.0 }),
            Val::ZERO,
            style::INK_SOFT,
        ),
        Pickable::IGNORE,
        children![(
            Text::new(key.unwrap_or_default()),
            theme.text_font(true, false, 10.0),
            TextColor(style::INK),
            Pickable::IGNORE,
        )],
    )
}

/// Count badge on a launcher (`Traditions (n)` / captives), hidden at 0.
pub fn launcher_badge(theme: &UiTheme, count: usize) -> impl Bundle {
    (
        LauncherBadge,
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(-5.0),
            top: Val::Px(-4.0),
            min_width: Val::Px(14.0),
            padding: UiRect::axes(Val::Px(3.0), Val::Px(0.0)),
            justify_content: JustifyContent::Center,
            display: if count > 0 {
                Display::Flex
            } else {
                Display::None
            },
            border_radius: BorderRadius::MAX,
            ..default()
        },
        BackgroundColor(style::RUBRIC),
        Pickable::IGNORE,
        children![(
            Text::new(count.to_string()),
            theme.text_font(true, false, 10.0),
            TextColor(style::PARCHMENT_LIGHT),
            Pickable::IGNORE,
        )],
    )
}

fn separator() -> impl Bundle {
    (
        Node {
            width: Val::Px(1.0),
            height: Val::Px(22.0),
            margin: UiRect::horizontal(Val::Px(2.0)),
            ..default()
        },
        BackgroundColor(style::INK_FADED.with_alpha(0.5)),
        Pickable::IGNORE,
    )
}

fn spacer() -> impl Bundle {
    (
        Node {
            flex_grow: 1.0,
            ..default()
        },
        Pickable::IGNORE,
    )
}

/// A figure: `HudField` + tooltip on the chip, icon and text inside.
fn figure(
    bar: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    assets: &AssetServer,
    field: HudField,
    name: &str,
    icon_id: &str,
) {
    let mut chip = bar.spawn((
        Name::new(name.to_owned()),
        field,
        Tooltip::new(""),
        Hovered::default(),
        Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(4.0),
            flex_shrink: 0.0,
            ..default()
        },
    ));
    chip.with_children(|chip| {
        let mut image = chip.spawn((
            icon_tinted(theme, assets, icon_id, FIGURE_ICON, style::INK),
            Pickable::IGNORE,
        ));
        if field == HudField::Date {
            image.insert(FieldImage);
        }
        chip.spawn((
            FieldText,
            Text::new(""),
            theme.text_font(false, false, UiType::Body.size()),
            TextColor(style::INK),
            Pickable::IGNORE,
        ));
    });
    if field == HudField::Treasury {
        // A click on the treasury opens the economy window (Godot top bar).
        chip.observe(super::treasury_click);
    }
}

/// Builds the bar under `root`.
pub fn spawn_top_bar(
    root: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    assets: &AssetServer,
    keys: &KeyHints,
) {
    root.spawn((
        Name::new("hud:top_bar"),
        TopBarMark,
        HudWidget,
        Hovered::default(),
        // Above the full-screen dim of the central panels (`UiTier::Panel`), below banners and
        // modals: a launcher stays clickable while a window is open (click = switch / toggle).
        GlobalZIndex(crate::ui::kit::style::UiTier::Panel.z() + 5),
        crate::ui::kit::widgets::panel_inset(theme),
    ))
    .insert(Node {
        position_type: PositionType::Absolute,
        top: Val::Px(0.0),
        left: Val::Px(0.0),
        right: Val::Px(0.0),
        height: Val::Px(layout::TOP_BAR_HEIGHT),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: Val::Px(14.0),
        padding: UiRect::axes(Val::Px(12.0), Val::Px(3.0)),
        overflow: Overflow::clip(),
        ..default()
    })
    .with_children(|bar| {
        // Shield + name: a click opens the faction panel (`FactionSwatch` / `FactionLabel`).
        bar.spawn((
            Name::new("hud:faction"),
            HudField::Faction,
            Tooltip::new(""),
            bevy::ui_widgets::Button,
            Hovered::default(),
            Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(6.0),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .with_children(|chip| {
            chip.spawn((
                FieldImage,
                ImageNode::default(),
                Node {
                    width: Val::Px(22.0),
                    height: Val::Px(26.0),
                    ..default()
                },
                Pickable::IGNORE,
            ));
            chip.spawn((
                FieldText,
                Text::new(""),
                theme.title_font(UiType::Heading.size()),
                TextColor(style::INK_TITLE),
                Pickable::IGNORE,
            ));
        });
        bar.spawn(separator());
        figure(
            bar,
            theme,
            assets,
            HudField::Treasury,
            "hud:treasury",
            "hud_treasury",
        );
        figure(
            bar,
            theme,
            assets,
            HudField::Balance,
            "hud:balance",
            "hud_income",
        );
        figure(
            bar,
            theme,
            assets,
            HudField::Manpower,
            "hud:manpower",
            "hud_army",
        );
        figure(
            bar,
            theme,
            assets,
            HudField::Prestige,
            "hud:prestige",
            "glyph_star",
        );
        figure(
            bar,
            theme,
            assets,
            HudField::Unrest,
            "hud:unrest",
            "gauge_unrest",
        );
        bar.spawn(spacer());
        figure(
            bar,
            theme,
            assets,
            HudField::Date,
            "hud:date",
            "hud_season_spring",
        );
        bar.spawn(spacer());
        bar.spawn((
            Name::new("hud:launcher"),
            PanelButtons,
            Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(7.0),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|row| {
            for launcher in &LAUNCHERS {
                let key = keys.label(launcher.action);
                row.spawn((
                    launcher_button(theme, assets, launcher.name, launcher.icon, &key),
                    Tooltip::new(&launcher_tooltip(launcher.title, &key, launcher.detail)),
                ));
            }
        });
        bar.spawn(separator());
        let key = keys.label("campaign_pause");
        bar.spawn((
            launcher_button(theme, assets, "hud:menu", "hud_menu", ""),
            Tooltip::new(&launcher_tooltip(
                "Menu",
                &key,
                "Sauvegarder, charger, réglages, quitter.",
            )),
        ));
    });
}

/// Below this bar width (logical px: 4:3 screens, « Taille de l'interface » above 115 %) the bar
/// turns compact: the date chip (also on the seal) and the « / saison » of the balance go
/// (`TopBarFit.set_compact`).
pub const COMPACT_WIDTH: f32 = 1380.0;

/// Whether the bar is compact at `width`. Pure, tested.
pub fn compact_bar(width: f32) -> bool {
    width > 1.0 && width < COMPACT_WIDTH
}

/// Compact state of the bar (read by `update_fields`).
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BarCompact(pub bool);

/// Switches the bar compact from its laid-out width.
pub(super) fn fit_top_bar(
    bar: Query<&bevy::ui::ComputedNode, With<TopBarMark>>,
    mut compact: ResMut<BarCompact>,
    mut chips: Query<(&HudField, &mut Node)>,
) {
    let Ok(node) = bar.single() else { return };
    let wanted = compact_bar(node.size().x * node.inverse_scale_factor());
    if compact.0 != wanted {
        compact.0 = wanted;
    }
    for (field, mut chip) in &mut chips {
        if *field != HudField::Date {
            continue;
        }
        let display = if wanted { Display::None } else { Display::Flex };
        if chip.display != display {
            chip.display = display;
        }
    }
}

/// Hover / press look of the launchers: a gold halo, slightly darker when pressed.
#[allow(clippy::type_complexity)]
pub fn launcher_visuals(
    mut buttons: Query<(&Hovered, Has<bevy::ui::Pressed>, &mut Outline), With<LauncherButton>>,
) {
    for (hovered, pressed, mut outline) in &mut buttons {
        let color = match (hovered.get(), pressed) {
            (_, true) => style::GOLD,
            (true, false) => style::GOLD_PALE,
            _ => Color::NONE,
        };
        if outline.color != color {
            outline.color = color;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bar_turns_compact_on_narrow_logical_widths() {
        // 16:9 and 16:10 at 100 % (logical 1600 / 1440 wide): full bar.
        assert!(!compact_bar(1600.0));
        assert!(!compact_bar(1440.0));
        // 4:3 (1200) or 16:9 at 125 % (1280): compact.
        assert!(compact_bar(1200.0));
        assert!(compact_bar(1280.0));
        // Not laid out yet.
        assert!(!compact_bar(0.0));
    }

    #[test]
    fn tooltips_name_the_key() {
        assert_eq!(launcher_tooltip("Codex", "", "x"), "[b]Codex[/b]\nx");
        assert_eq!(
            launcher_tooltip("Techniques", "F4", "x"),
            "[b]Techniques[/b] (F4)\nx"
        );
    }
}

#[cfg(test)]
mod stacking_tests {
    use crate::ui::kit::style::UiTier;

    /// The bar sits above the full-screen dim of the central panels and below banners.
    #[test]
    fn top_bar_between_panels_and_banners() {
        let z = UiTier::Panel.z() + 5;
        assert!(z > UiTier::Panel.z() && z < UiTier::Banner.z());
    }
}

#[cfg(test)]
mod expand_tests {
    use super::*;

    #[test]
    fn key_placeholders_read_the_bound_key() {
        let map = KeyMap::defaults();
        let hints = KeyHints::new(Some(&map), None);
        assert_eq!(
            hints.expand("Technique ({key:map_toggle_tech}) ou {key:map_toggle_council}."),
            "Technique (F4) ou J."
        );
        assert_eq!(
            hints.expand("{key:nope} {key:inachevé"),
            "non assignée {key:inachevé"
        );
    }
}
