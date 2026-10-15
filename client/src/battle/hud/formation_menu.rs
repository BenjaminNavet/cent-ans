//! Regiment formation menu (RJ-a, ADR 0174). Port of battle_formation_menu.gd and of the
//! « Formation » button of battle_hud.gd.
//!
//! A button above the leader's seal shows the formation of the selection (short label, a thin
//! bar while the men reform) and opens a menu with one row per formation of
//! `data/rules/unit_formations.json` (core catalogue, data order): historical name, greyed when
//! no selected regiment may take it, gold when it is the formation of the whole selection.
//! The tooltip of a row quotes the historical context and the figures of the data (signed
//! effects, reformation time and penalties). No rule here: the choice leaves as
//! `Command::Formation` through [`IssueBattleCommand`] for the selected regiments that may take
//! it; the core decides and makes the men walk (reformation).

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::Button;
use serde_json::{Value, json};
use sim_battle::{Command, Formation, FormationDef, FormationRules};

use super::{BattleHudRoot, HudData};
use crate::app::AppState;
use crate::battle::orders::BattleSelection;
use crate::battle::orders::input::IssueBattleCommand;
use crate::battle::scene::BattleSession;
use crate::ui::kit::style::{self, UiTier, UiType};
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::widgets::{caption, panel_page};
use crate::ui::kit::{ButtonPressed, UiTheme};

const BUTTON_W: f32 = 88.0;
const BUTTON_H: f32 = 40.0;
/// Above the seal (88 px + 6 px margin), under the orders bar.
const BOTTOM: f32 = 100.0;
const ROW_W: f32 = 190.0;
const ROW_H: f32 = 26.0;
const INK: Color = Color::srgb(0.22, 0.14, 0.07);
const GOLD_FILL: Color = Color::srgba(0.95, 0.75, 0.15, 0.45);
const BUTTON_ID: &str = "formation_menu";
const ROW_PREFIX: &str = "formation:";

/// What a row says about the selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RowState {
    /// At least one selected regiment may take it.
    pub able: bool,
    /// Every regiment that may take it already has it.
    pub current: bool,
}

/// `(key, state)` of every formation for the selected regiments (`menu_state`).
pub fn menu_state(catalog: &[String], data: &HudData, selected: &[u32]) -> Vec<(String, RowState)> {
    catalog
        .iter()
        .map(|key| {
            let (mut able, mut current) = (0, 0);
            for unit in data
                .own
                .iter()
                .filter(|u| selected.contains(&u.id) && u.present)
            {
                if !unit.status.allowed_formations.contains(&key.as_str()) {
                    continue;
                }
                able += 1;
                if unit.formation_key == *key {
                    current += 1;
                }
            }
            (
                key.clone(),
                RowState {
                    able: able > 0,
                    current: able > 0 && current == able,
                },
            )
        })
        .collect()
}

/// Selected regiments (present) that may take `key`.
pub fn takers(data: &HudData, selected: &[u32], key: &str) -> Vec<u32> {
    data.own
        .iter()
        .filter(|u| selected.contains(&u.id) && u.present)
        .filter(|u| u.status.allowed_formations.contains(&key))
        .map(|u| u.id)
        .collect()
}

/// Signed effect lines of a formation (`effect_lines`): `(text, +1 favourable / -1 unfavourable)`.
pub fn effect_lines(def: &FormationDef) -> Vec<(String, i8)> {
    let m = &def.modifiers;
    // (value, label, higher is better)
    let factors = [
        (m.speed, "Allure", true),
        (m.charge, "Charge", true),
        (m.shooting, "Pertes infligées au tir", true),
        (m.push_drive, "Poussée en mêlée", true),
        (m.push_resistance, "Tenue sous la poussée", true),
        (m.melee_taken, "Dégâts subis en mêlée", false),
        (m.missile_taken, "Pertes sous les traits", false),
        (m.morale_loss, "Moral perdu avec les pertes", false),
        (m.horse_blows, "Coups des cavaliers contre elle", false),
    ];
    let mut lines = Vec::new();
    for (value, label, higher_is_better) in factors {
        if (value - 1.0).abs() < 1e-9 {
            continue;
        }
        let percent = ((value - 1.0) * 100.0).round() as i64;
        let better = (value > 1.0) == higher_is_better;
        lines.push((
            format!("{label} {}{percent} %", if percent > 0 { "+" } else { "" }),
            if better { 1 } else { -1 },
        ));
    }
    if (m.fighting_ranks - 2.0).abs() > 1e-9 {
        lines.push((
            format!(
                "{} hommes combattent par file du front (2 d’ordinaire)",
                m.fighting_ranks.round() as i64
            ),
            if m.fighting_ranks > 2.0 { 1 } else { -1 },
        ));
    }
    if def.all_round {
        lines.push(("Ni flanc ni dos : tout coup porte de face".into(), 1));
    }
    if def.braced {
        lines.push((
            "Hérissée : la charge ne renverse personne ni n’ébranle le moral".into(),
            1,
        ));
    }
    lines
}

/// « gens de pied, tireurs » (`troops_label`).
pub fn troops_label(def: &FormationDef) -> String {
    use data_model::UnitCategory;
    let names: Vec<&str> = def
        .categories
        .iter()
        .map(|c| match c {
            UnitCategory::Infantry => "gens de pied",
            UnitCategory::Ranged => "tireurs",
            UnitCategory::Cavalry => "cavaliers",
            UnitCategory::Siege => "engins",
            #[allow(unreachable_patterns)]
            _ => "",
        })
        .collect();
    let mut text = names.join(", ");
    if def.foot_only {
        text += " (à pied)";
    } else if def.mounted_only {
        text += " (montés)";
    }
    text
}

fn factor_text(value: f64) -> String {
    crate::codex::rules::number((value * 100.0).round() / 100.0, None)
}

/// BBCode tooltip of a formation row: name, troops, historical context, figures, reformation.
pub fn tooltip_bbcode(def: &FormationDef, rules: &FormationRules) -> String {
    let mut lines = vec![
        format!("[b]{}[/b]", def.name),
        format!("[color=#6b5a45]Formation · {}[/color]", troops_label(def)),
    ];
    if !def.description.is_empty() {
        lines.push(format!("[i]{}[/i]", def.description));
    }
    let effects = effect_lines(def);
    if effects.is_empty() {
        lines.push("Aucun effet particulier : l’ordre de référence".into());
    }
    for (text, sign) in effects {
        let color = if sign > 0 { "#2f6b2a" } else { "#8a2a1a" };
        lines.push(format!("[color={color}]{text}[/color]"));
    }
    if def.reform_s > 0.0 {
        lines.push(format!(
            "Reformation : environ {} s pour cent hommes (plus long pour un grand régiment, plus court pour des vétérans) ; pendant ce temps, allure ×{} et dégâts subis en mêlée ×{}",
            def.reform_s.round() as i64,
            factor_text(rules.reform.speed),
            factor_text(rules.reform.melee_taken),
        ));
    }
    lines.join("\n")
}

/// Short label of the selection's formation, or `""` when they differ or nothing is selected.
pub fn selection_label(data: &HudData, selected: &[u32]) -> String {
    let mut keys = data
        .own
        .iter()
        .filter(|u| selected.contains(&u.id) && u.present)
        .map(|u| u.formation_key.as_str());
    let Some(first) = keys.next() else {
        return String::new();
    };
    if keys.all(|k| k == first) {
        Formation::all()
            .find(|f| f.key() == first)
            .map(|f| f.def().short.clone())
            .unwrap_or_default()
    } else {
        "variées".into()
    }
}

/// Least advanced reformation among the selected regiments (`None`: none reforming).
pub fn selection_progress(data: &HudData, selected: &[u32]) -> Option<f32> {
    data.own
        .iter()
        .filter(|u| selected.contains(&u.id) && u.present)
        .filter_map(|u| u.reform_progress)
        .reduce(f32::min)
}

/// Menu state.
#[derive(Resource, Default, Debug)]
pub struct FormationMenu {
    pub open: bool,
    /// Keys of the catalogue, data order.
    pub catalog: Vec<String>,
    pub state: Vec<(String, RowState)>,
    /// Formations ordered from the menu (dump).
    pub chosen: Vec<String>,
}

#[derive(Component)]
struct MenuButton;
#[derive(Component)]
struct MenuButtonLabel;
#[derive(Component)]
struct MenuButtonBar;
#[derive(Component)]
struct MenuPanel;
#[derive(Component)]
struct MenuRow(String);

pub struct FormationMenuPlugin;

impl Plugin for FormationMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FormationMenu>()
            .add_systems(OnEnter(AppState::Battle), reset)
            .add_systems(
                Update,
                (spawn, interact, refresh)
                    .chain()
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>),
            );
    }
}

fn reset(mut menu: ResMut<FormationMenu>) {
    *menu = FormationMenu::default();
}

fn spawn(
    mut commands: Commands,
    roots: Query<Entity, With<BattleHudRoot>>,
    existing: Query<(), With<MenuButton>>,
    mut menu: ResMut<FormationMenu>,
    theme: Res<UiTheme>,
) {
    let Ok(root) = roots.single() else { return };
    if !existing.is_empty() {
        return;
    }
    let rules = FormationRules::bundled();
    menu.catalog = rules.formations.iter().map(|f| f.key.clone()).collect();
    let button = commands
        .spawn((
            Name::new(BUTTON_ID),
            MenuButton,
            Button,
            Hovered::default(),
            Tooltip::new(
                "[b]Formation[/b]\nChoisir une formation dans le menu (noms historiques, effets) ; la touche T passe à la suivante. Doré : formation particulière ou reformation en cours.",
            ),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(8.0),
                bottom: Val::Px(BOTTOM),
                width: Val::Px(BUTTON_W),
                height: Val::Px(BUTTON_H),
                border: UiRect::all(Val::Px(1.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(style::PARCHMENT_LIGHT),
            BorderColor::all(style::INK_SOFT),
            ChildOf(root),
        ))
        .id();
    commands.spawn((
        MenuButtonLabel,
        Text::new("Formation"),
        theme.text_font(false, false, UiType::Caption.size()),
        TextColor(INK),
        TextLayout::justify(Justify::Center),
        Pickable::IGNORE,
        ChildOf(button),
    ));
    commands.spawn((
        MenuButtonBar,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(4.0),
            bottom: Val::Px(3.0),
            width: Val::Px(0.0),
            height: Val::Px(3.0),
            ..default()
        },
        BackgroundColor(style::GOLD),
        Pickable::IGNORE,
        ChildOf(button),
    ));
    let panel = commands
        .spawn((
            Name::new("FormationMenu"),
            MenuPanel,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(8.0),
                bottom: Val::Px(BOTTOM + BUTTON_H + 6.0),
                display: Display::None,
                ..default()
            },
            UiTier::Panel.index(),
            ChildOf(root),
        ))
        .id();
    let page = commands.spawn((panel_page(&theme), ChildOf(panel))).id();
    commands.spawn((caption(&theme, "Formation"), ChildOf(page)));
    for def in &rules.formations {
        let row = commands
            .spawn((
                Name::new(format!("{ROW_PREFIX}{}", def.key)),
                MenuRow(def.key.clone()),
                Button,
                Hovered::default(),
                Tooltip::new(&tooltip_bbcode(def, rules)),
                Node {
                    width: Val::Px(ROW_W),
                    height: Val::Px(ROW_H),
                    padding: UiRect::horizontal(Val::Px(6.0)),
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::NONE),
                ChildOf(page),
            ))
            .id();
        commands.spawn((
            Text::new(def.name.clone()),
            theme.text_font(false, false, UiType::Caption.size()),
            TextColor(INK),
            Pickable::IGNORE,
            ChildOf(row),
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn interact(
    mut pressed: MessageReader<ButtonPressed>,
    mut menu: ResMut<FormationMenu>,
    selection: Res<BattleSelection>,
    data: Res<HudData>,
    mut issue: MessageWriter<IssueBattleCommand>,
) {
    for press in pressed.read() {
        if press.name == BUTTON_ID {
            let able = menu.state.iter().any(|(_, s)| s.able);
            menu.open = !menu.open && able;
        } else if let Some(key) = press.name.strip_prefix(ROW_PREFIX) {
            let units = takers(&data, &selection.units, key);
            if units.is_empty() || !menu.catalog.iter().any(|k| k == key) {
                continue;
            }
            let Some(kind) = Formation::all().find(|f| f.key() == key) else {
                continue;
            };
            issue.write(IssueBattleCommand(Command::Formation { units, kind }));
            menu.chosen.push(key.to_owned());
            menu.open = false;
        }
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn refresh(
    mut menu: ResMut<FormationMenu>,
    selection: Res<BattleSelection>,
    data: Res<HudData>,
    mut panels: Query<&mut Node, (With<MenuPanel>, Without<MenuButtonBar>)>,
    mut rows: Query<(&MenuRow, &mut BackgroundColor), Without<MenuButton>>,
    mut labels: Query<&mut Text, With<MenuButtonLabel>>,
    mut bars: Query<&mut Node, (With<MenuButtonBar>, Without<MenuPanel>)>,
    mut buttons: Query<&mut BackgroundColor, With<MenuButton>>,
) {
    let state = menu_state(&menu.catalog, &data, &selection.units);
    // No selection able to reform: the menu closes.
    if !state.iter().any(|(_, s)| s.able) {
        menu.open = false;
    }
    if state != menu.state {
        menu.state = state;
    }
    for mut node in &mut panels {
        let display = if menu.open {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }
    for (row, mut background) in &mut rows {
        let state = menu
            .state
            .iter()
            .find(|(key, _)| *key == row.0)
            .map(|(_, s)| *s)
            .unwrap_or_default();
        let wanted = if !state.able {
            Color::srgba(0.85, 0.8, 0.75, 0.5)
        } else if state.current {
            GOLD_FILL
        } else {
            Color::NONE
        };
        if background.0 != wanted {
            background.0 = wanted;
        }
    }
    let label = selection_label(&data, &selection.units);
    let progress = selection_progress(&data, &selection.units);
    for mut text in &mut labels {
        let shown = if label.is_empty() {
            "Formation".to_owned()
        } else {
            label.clone()
        };
        if text.0 != shown {
            text.0 = shown;
        }
    }
    for mut node in &mut bars {
        node.width = Val::Px(progress.map_or(0.0, |p| (BUTTON_W - 8.0) * p.clamp(0.0, 1.0)));
    }
    let able = menu.state.iter().any(|(_, s)| s.able);
    for mut background in &mut buttons {
        let wanted = if !able {
            Color::srgba(0.85, 0.8, 0.75, 0.6)
        } else if menu.open || progress.is_some() {
            Color::srgb(0.93, 0.83, 0.5)
        } else {
            style::PARCHMENT_LIGHT
        };
        if background.0 != wanted {
            background.0 = wanted;
        }
    }
}

pub fn dump(world: &World) -> Value {
    let menu = world.resource::<FormationMenu>();
    json!({
        "open": menu.open,
        "formations": menu.catalog.len(),
        "able": menu.state.iter().filter(|(_, s)| s.able).map(|(k, _)| k).collect::<Vec<_>>(),
        "current": menu.state.iter().filter(|(_, s)| s.current).map(|(k, _)| k).collect::<Vec<_>>(),
        "chosen": menu.chosen,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::hud::view::{BattleRow, StatusView, UnitView};
    use data_model::UnitCategory;
    use sim_battle::{UnitFate, UnitState};

    fn unit(id: u32, formation: &str, allowed: &[&'static str]) -> UnitView {
        UnitView {
            id,
            unit_type: "x".into(),
            name: "x".into(),
            category: UnitCategory::Infantry,
            soldiers: 100,
            initial: 100,
            morale: 80.0,
            fatigue: 0.0,
            ammo: 0,
            max_ammo: 0,
            can_shoot: false,
            is_general: false,
            present: true,
            left_field: false,
            reserve: false,
            state: UnitState::Idle,
            formation: "x".into(),
            experience: 0,
            kills: 0,
            fate: UnitFate::Held,
            melee: 1,
            ranged: 0,
            armor: 1,
            x: 0.0,
            z: 0.0,
            row: BattleRow::Main,
            status: StatusView {
                allowed_formations: allowed.to_vec(),
                ..Default::default()
            },
            formation_key: formation.into(),
            reform_progress: None,
        }
    }

    fn data() -> HudData {
        HudData {
            own: vec![
                unit(1, "line", &["line", "column"]),
                unit(2, "column", &["line", "column", "wedge"]),
                unit(3, "line", &["line"]),
            ],
            ..Default::default()
        }
    }

    #[test]
    fn rows_are_greyed_without_a_taker_and_gold_when_all_have_it() {
        let catalog: Vec<String> = ["line", "column", "wedge"].map(String::from).to_vec();
        let state = menu_state(&catalog, &data(), &[1, 2]);
        assert_eq!(
            state[0].1,
            RowState {
                able: true,
                current: false
            }
        );
        assert_eq!(
            state[1].1,
            RowState {
                able: true,
                current: false
            }
        );
        assert_eq!(
            state[2].1,
            RowState {
                able: true,
                current: false
            }
        );
        let alone = menu_state(&catalog, &data(), &[3]);
        assert_eq!(
            alone[0].1,
            RowState {
                able: true,
                current: true
            }
        );
        assert!(!alone[1].1.able);
        assert!(
            menu_state(&catalog, &data(), &[])
                .iter()
                .all(|(_, s)| !s.able)
        );
    }

    #[test]
    fn only_able_selected_regiments_receive_the_order() {
        assert_eq!(takers(&data(), &[1, 2, 3], "wedge"), vec![2]);
        assert_eq!(takers(&data(), &[1, 2, 3], "line"), vec![1, 2, 3]);
        assert!(takers(&data(), &[], "line").is_empty());
    }

    #[test]
    fn catalogue_rows_quote_the_data() {
        let rules = FormationRules::bundled();
        assert!(!rules.formations.is_empty());
        for def in &rules.formations {
            let tip = tooltip_bbcode(def, rules);
            assert!(tip.contains(&def.name));
            assert!(tip.contains("Formation ·"));
        }
        // A formation with a favourable and an unfavourable figure signs them both ways.
        assert!(rules.formations.iter().any(|d| {
            let lines = effect_lines(d);
            lines.iter().any(|(_, s)| *s > 0) && lines.iter().any(|(_, s)| *s < 0)
        }));
    }

    #[test]
    fn selection_label_is_the_common_short_name() {
        let mut data = data();
        data.own[0].formation_key = "line".into();
        assert_eq!(
            selection_label(&data, &[3]),
            Formation::of("line").def().short
        );
        assert_eq!(selection_label(&data, &[1, 2]), "variées");
        assert_eq!(selection_label(&data, &[]), "");
        data.own[1].reform_progress = Some(0.4);
        data.own[0].reform_progress = Some(0.7);
        assert_eq!(selection_progress(&data, &[1, 2]), Some(0.4));
        assert_eq!(selection_progress(&data, &[3]), None);
    }
}
