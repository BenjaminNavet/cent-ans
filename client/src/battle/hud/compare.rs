//! Face-to-face comparison, bottom left: opens when exactly one regiment of the player is
//! selected and an enemy is hovered ([`HoveredBattleUnits`], written by the orders lot). Nine
//! lines straight from [`sim_battle::BattleSim::compare`] (hover.rs): the side with the net
//! advantage in green, the other in red, nothing when even. Port of battle_compare_panel.gd.

use bevy::prelude::*;
use serde_json::{Value, json};
use sim_battle::{Advantage, Compare, CompareSide};

use super::{BuildCtx, HoveredBattleUnits, HudData};
use crate::app::AppState;
use crate::battle::orders::BattleSelection;
use crate::battle::scene::BattleSession;
use crate::ui::kit::style::{self, UiType};
use crate::ui::kit::widgets::panel_note;

/// Lines of the comparison: key, caption, suffix (`LINES`).
pub const LINES: [(&str, &str, &str); 9] = [
    ("soldiers", "Effectif", ""),
    ("melee", "Mêlée", ""),
    ("defense", "Défense", ""),
    ("charge", "Charge", ""),
    ("ranged", "Tir", ""),
    ("range", "Portée", " m"),
    ("morale", "Moral", ""),
    ("fatigue", "Fatigue", ""),
    ("bonus_vs", "Bonus contre", " %"),
];
/// Seconds between two queries of the core while the hover lasts.
const REFRESH_S: f32 = 0.25;
const NONE: &str = "—";

/// Value of `line` on `side`.
pub fn side_value(side: &CompareSide, line: &str) -> f64 {
    match line {
        "soldiers" => f64::from(side.soldiers),
        "melee" => side.melee,
        "defense" => side.defense,
        "charge" => side.charge,
        "ranged" => side.ranged,
        "range" => side.range,
        "morale" => side.morale,
        "fatigue" => side.fatigue,
        "bonus_vs" => side.bonus_vs,
        _ => 0.0,
    }
}

/// Text of a value: rounded French integer, « — » for no shot / no range (`value_text`).
pub fn value_text(line: &str, value: f64, suffix: &str) -> String {
    if (line == "range" || line == "ranged") && value <= 0.0 {
        return NONE.to_owned();
    }
    format!("{}{suffix}", super::view::number(value))
}

/// Advantage flag of `line` in `compare`.
pub fn advantage_of(compare: &Compare, line: &str) -> Advantage {
    compare
        .advantages
        .iter()
        .find(|(key, _)| key == line)
        .map_or(Advantage::Even, |(_, advantage)| *advantage)
}

/// Ink of a value by the advantage flag of its line.
pub fn value_color(advantage: Advantage, is_ours: bool) -> Color {
    match advantage {
        Advantage::Even => style::INK,
        _ => {
            if (advantage == Advantage::Ours) == is_ours {
                style::GOOD
            } else {
                style::RUBRIC
            }
        }
    }
}

/// True when the panel must open.
pub fn wanted(selected: usize, enemy: Option<u32>) -> bool {
    selected == 1 && enemy.is_some()
}

/// First hovered regiment that is an enemy and present.
pub fn hovered_enemy(data: &HudData, hovered: &[u32]) -> Option<u32> {
    hovered
        .iter()
        .copied()
        .find(|id| data.enemy.iter().any(|u| u.id == *id && u.present))
}

/// What the panel shows now.
#[derive(Resource, Default, Debug)]
pub struct CompareState {
    pub enemy: Option<u32>,
    pub ours: Option<u32>,
    pub compare: Option<Compare>,
    pub ours_name: String,
    pub theirs_name: String,
    /// Queries made to the core (dump, throttling proof).
    pub core_calls: u32,
    key: Option<(u32, u32)>,
    last_call: f32,
    /// Bumped when the content changes.
    revision: u64,
}

impl CompareState {
    pub fn is_open(&self) -> bool {
        self.compare.is_some()
    }

    fn close(&mut self) {
        if self.compare.is_some() || self.key.is_some() {
            self.revision += 1;
        }
        self.enemy = None;
        self.ours = None;
        self.compare = None;
        self.key = None;
    }
}

#[derive(Component)]
struct ComparePanel;

#[derive(Component)]
struct CompareTitle(bool);

#[derive(Component)]
struct CompareCell {
    line: &'static str,
    ours: bool,
}

pub struct ComparePlugin;

impl Plugin for ComparePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CompareState>()
            .add_systems(OnEnter(AppState::Battle), reset)
            .add_systems(
                Update,
                (query_core, show_compare)
                    .chain()
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>),
            );
    }
}

fn reset(mut state: ResMut<CompareState>) {
    *state = CompareState::default();
}

/// Spawns the (hidden) panel.
pub fn spawn_panel(commands: &mut Commands, root: Entity, ctx: &BuildCtx) {
    let wrapper = commands
        .spawn((
            Name::new("ComparePanel"),
            ComparePanel,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(12.0),
                bottom: Val::Px(176.0),
                display: Display::None,
                ..default()
            },
            Pickable::IGNORE,
            ChildOf(root),
        ))
        .id();
    let frame = commands
        .spawn((
            panel_note(ctx.theme),
            Node {
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(10.0)),
                row_gap: Val::Px(1.0),
                min_height: Val::Px(40.0),
                ..default()
            },
            ChildOf(wrapper),
        ))
        .id();
    let cell = |commands: &mut Commands,
                parent: Entity,
                marker: Option<(bool, Option<&'static str>)>,
                width: f32,
                align: Justify,
                size: f32,
                color: Color,
                text: &str| {
        let mut entity = commands.spawn((
            Node {
                width: Val::Px(width),
                ..default()
            },
            Text::new(text),
            TextLayout::justify(align),
            ctx.theme.text_font(false, false, size),
            TextColor(color),
            Pickable::IGNORE,
            ChildOf(parent),
        ));
        match marker {
            Some((ours, None)) => {
                entity.insert(CompareTitle(ours));
            }
            Some((ours, Some(line))) => {
                entity.insert(CompareCell { line, ours });
            }
            None => {}
        }
    };
    let head = commands
        .spawn((
            Node {
                column_gap: Val::Px(14.0),
                ..default()
            },
            ChildOf(frame),
        ))
        .id();
    cell(
        commands,
        head,
        Some((true, None)),
        130.0,
        Justify::Right,
        UiType::Caption.size(),
        style::INK,
        "",
    );
    cell(
        commands,
        head,
        None,
        90.0,
        Justify::Center,
        13.0,
        style::INK_FADED,
        "contre",
    );
    cell(
        commands,
        head,
        Some((false, None)),
        130.0,
        Justify::Left,
        UiType::Caption.size(),
        style::RUBRIC,
        "",
    );
    for (line, caption, _) in LINES {
        let row = commands
            .spawn((
                Node {
                    column_gap: Val::Px(14.0),
                    ..default()
                },
                ChildOf(frame),
            ))
            .id();
        cell(
            commands,
            row,
            Some((true, Some(line))),
            130.0,
            Justify::Right,
            UiType::Caption.size(),
            style::INK,
            "",
        );
        cell(
            commands,
            row,
            None,
            90.0,
            Justify::Center,
            14.0,
            style::INK_SOFT,
            caption,
        );
        cell(
            commands,
            row,
            Some((false, Some(line))),
            130.0,
            Justify::Left,
            UiType::Caption.size(),
            style::INK,
            "",
        );
    }
}

fn query_core(
    time: Res<Time<Real>>,
    data: Res<HudData>,
    selection: Res<BattleSelection>,
    hovered: Res<HoveredBattleUnits>,
    mut session: ResMut<BattleSession>,
    mut state: ResMut<CompareState>,
) {
    let enemy = hovered_enemy(&data, &hovered.0);
    if !wanted(selection.units.len(), enemy) {
        state.close();
        return;
    }
    let (Some(enemy), Some(ours)) = (enemy, selection.units.first().copied()) else {
        return;
    };
    let now = time.elapsed_secs();
    if state.key == Some((ours, enemy)) && now - state.last_call < REFRESH_S {
        return;
    }
    state.key = Some((ours, enemy));
    state.last_call = now;
    let sim = session.sim_mut();
    let (Some(mine), Some(theirs)) = (
        sim.units().iter().find(|u| u.id == ours),
        sim.units().iter().find(|u| u.id == enemy),
    ) else {
        state.close();
        return;
    };
    let compare = sim.compare(mine, theirs);
    let theirs_name = theirs.name.to_string();
    state.core_calls += 1;
    state.enemy = Some(enemy);
    state.ours = Some(ours);
    state.ours_name = data.name_of(ours);
    state.theirs_name = theirs_name;
    state.compare = Some(compare);
    state.revision += 1;
}

fn show_compare(
    state: Res<CompareState>,
    mut panel: Query<&mut Node, With<ComparePanel>>,
    mut titles: Query<(&CompareTitle, &mut Text), Without<CompareCell>>,
    mut cells: Query<(&CompareCell, &mut Text, &mut TextColor), Without<CompareTitle>>,
    mut seen: Local<u64>,
) {
    if *seen == state.revision {
        return;
    }
    *seen = state.revision;
    if let Ok(mut node) = panel.single_mut() {
        node.display = if state.is_open() {
            Display::Flex
        } else {
            Display::None
        };
    }
    let Some(compare) = &state.compare else {
        return;
    };
    for (title, mut text) in &mut titles {
        text.0 = if title.0 {
            state.ours_name.clone()
        } else {
            state.theirs_name.clone()
        };
    }
    for (cell, mut text, mut color) in &mut cells {
        let suffix = LINES
            .iter()
            .find(|(line, _, _)| *line == cell.line)
            .map_or("", |(_, _, suffix)| suffix);
        let side = if cell.ours {
            &compare.ours
        } else {
            &compare.theirs
        };
        text.0 = value_text(cell.line, side_value(side, cell.line), suffix);
        *color = TextColor(value_color(advantage_of(compare, cell.line), cell.ours));
    }
}

pub fn dump(world: &World) -> Value {
    let state = world.resource::<CompareState>();
    json!({
        "open": state.is_open(),
        "ours": state.ours,
        "enemy": state.enemy,
        "core_calls": state.core_calls,
        "lines": state.compare.as_ref().map(|c| LINES.iter().map(|(line, _, suffix)| json!({
            "line": line,
            "ours": value_text(line, side_value(&c.ours, line), suffix),
            "theirs": value_text(line, side_value(&c.theirs, line), suffix),
            "advantage": advantage_of(c, line).key(),
        })).collect::<Vec<_>>()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn side(soldiers: u32, range: f64) -> CompareSide {
        CompareSide {
            unit: 1,
            soldiers,
            melee: 6.0,
            defense: 4.0,
            charge: 0.0,
            ranged: 0.0,
            range,
            morale: 55.4,
            fatigue: 10.0,
            bonus_vs: 100.0,
        }
    }

    #[test]
    fn opens_only_with_one_selected_and_an_enemy() {
        assert!(wanted(1, Some(3)));
        assert!(!wanted(2, Some(3)));
        assert!(!wanted(1, None));
    }

    #[test]
    fn values_are_rounded_and_empty_range_is_a_dash() {
        assert_eq!(value_text("morale", 55.4, ""), "55");
        assert_eq!(value_text("range", 0.0, " m"), "—");
        assert_eq!(value_text("ranged", 0.0, ""), "—");
        assert_eq!(value_text("bonus_vs", 100.0, " %"), "100 %");
        assert!(value_text("soldiers", 1234.0, "").contains("234"));
    }

    #[test]
    fn colours_follow_the_advantage_flag() {
        assert_eq!(value_color(Advantage::Even, true), style::INK);
        assert_eq!(value_color(Advantage::Ours, true), style::GOOD);
        assert_eq!(value_color(Advantage::Ours, false), style::RUBRIC);
        assert_eq!(value_color(Advantage::Theirs, false), style::GOOD);
    }

    #[test]
    fn line_values_map_to_the_core_fields() {
        let s = side(120, 80.0);
        assert_eq!(side_value(&s, "soldiers"), 120.0);
        assert_eq!(side_value(&s, "range"), 80.0);
        assert_eq!(side_value(&s, "unknown"), 0.0);
    }
}
