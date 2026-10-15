//! End-of-battle screen: verdict banner (Victoire, Défaite, Victoire à la Pyrrhus), balance of
//! engaged / lost / surviving, notable facts, then for each side the regiment cards (losses,
//! kills, hero) and a table (engaged, losses, kills, fate); « Retour à la campagne » sends
//! [`LeaveBattle`]. Port of battle_result_screen.gd (UB1) and roster_card.gd (« bilan » mode).
//!
//! Everything shown comes from [`sim_battle::BattleSim::outcome`] and the final [`UnitView`]s;
//! the verdict is only a label from the loss ratios (display thresholds, as in Godot).
//!
//! Not ported: aftermath cards (captives, ransoms, experience, loot: `BattleAftermath` of the
//! campaign, flow lot), historical card, replay button, painted backdrop (`ArtPlates`).

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;
use serde_json::{Value, json};
use sim_battle::BattleEnd;
use sim_battle::{BattleOutcome, SideId};

use super::view::{UnitView, distinct_names, fate_text, number};
use super::{BuildCtx, HudData};
use crate::app::AppState;
use crate::battle::scene::BattleSession;
use crate::flow::LeaveBattle;
use crate::paths;
use crate::ui::kit::icons::icon;
use crate::ui::kit::style::{self, UiTier, UiType};
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::widgets::{button_primary, illuminated_title, panel_illuminated, separator};
use crate::ui::kit::{ButtonPressed, UiTheme};

/// Loss shares that qualify the verdict (`DECISIVE_*`, `PYRRHIC_OWN`).
pub const PYRRHIC_OWN: f32 = 0.3;
const CARD_W: f32 = 54.0;
const CARD_H: f32 = 80.0;
const RETURN_BUTTON: &str = "result:return";

/// « Victoire », « Victoire à la Pyrrhus » or « Défaite » (`banner_title`).
pub fn banner_title(won: bool, own_ratio: f32, enemy_ratio: f32) -> &'static str {
    if !won {
        "Défaite"
    } else if own_ratio >= PYRRHIC_OWN && own_ratio > enemy_ratio {
        "Victoire à la Pyrrhus"
    } else {
        "Victoire"
    }
}

/// « de France », « d’Angleterre » (`BattleScene.de`).
pub fn de(name: &str) -> String {
    let vowel = name
        .chars()
        .next()
        .is_some_and(|c| "aeiouyhAEIOUYHÀÂÉÈÊÎÔÛ".contains(c));
    if vowel {
        format!("d’{name}")
    } else {
        format!("de {name}")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SideSummary {
    pub name: String,
    pub engaged: u32,
    pub losses: u32,
    pub ratio: f32,
}

impl SideSummary {
    pub fn survivors(&self) -> u32 {
        self.engaged.saturating_sub(self.losses)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegimentRow {
    pub id: u32,
    pub unit_type: String,
    pub name: String,
    pub is_general: bool,
    pub initial: u32,
    pub soldiers: u32,
    pub lost: u32,
    pub kills: u32,
    pub fate: &'static str,
    pub hero: bool,
}

/// Everything the screen shows (`[own, enemy]` order).
#[derive(Debug, Clone, PartialEq)]
pub struct ResultView {
    pub title: &'static str,
    pub won: bool,
    pub subtitle: String,
    pub sides: [SideSummary; 2],
    pub facts: Vec<String>,
    pub rows: [Vec<RegimentRow>; 2],
}

fn rows_of(units: &[UnitView]) -> Vec<RegimentRow> {
    let names = distinct_names(units);
    let hero = units
        .iter()
        .filter(|u| u.kills > 0)
        .max_by_key(|u| u.kills)
        .map(|u| u.id);
    units
        .iter()
        .zip(names)
        .map(|(unit, (_, name))| RegimentRow {
            id: unit.id,
            unit_type: unit.unit_type.clone(),
            name,
            is_general: unit.is_general,
            initial: unit.initial,
            soldiers: unit.soldiers,
            lost: unit.initial.saturating_sub(unit.soldiers),
            kills: unit.kills,
            fate: fate_text(unit.fate),
            hero: Some(unit.id) == hero,
        })
        .collect()
}

/// Notable facts of the battle (`mentions`, without the standards and historical lines).
pub fn facts(
    outcome: &BattleOutcome,
    player: SideId,
    names: &[String; 2],
    rows: &[Vec<RegimentRow>; 2],
) -> Vec<String> {
    let enemy = match player {
        SideId::Attacker => SideId::Defender,
        SideId::Defender => SideId::Attacker,
    };
    let winner_index = usize::from(outcome.winner != player);
    let loser_index = 1 - winner_index;
    let mut lines = Vec::new();
    match outcome.end {
        BattleEnd::Refused => lines.push(format!(
            "Bataille refusée : personne n’a engagé le combat. L’ost {} renonce et se retire, l’ost {} garde le champ.",
            de(&names[loser_index]),
            de(&names[winner_index])
        )),
        BattleEnd::Broken => lines.push(format!(
            "L’armée {}, trop entamée, s’est brisée : déroute générale.",
            de(&names[loser_index])
        )),
        BattleEnd::Withdrawal => lines.push(format!(
            "Retraite ordonnée : l’ost {} a quitté le champ en bon ordre, l’ost {} le garde.",
            de(&names[loser_index]),
            de(&names[winner_index])
        )),
        BattleEnd::Lull => lines.push(format!(
            "Le combat est retombé : l’ost {} a cédé le terrain à l’ost {}.",
            de(&names[loser_index]),
            de(&names[winner_index])
        )),
        _ => {}
    }
    for (index, side) in [player, enemy].into_iter().enumerate() {
        let result = outcome.side(side);
        let name = &names[index];
        let destroyed: Vec<&str> = rows[index]
            .iter()
            .filter(|r| r.soldiers == 0)
            .map(|r| r.name.as_str())
            .collect();
        if !destroyed.is_empty() {
            lines.push(format!(
                "{name} : {} anéanti{} ({}).",
                if destroyed.len() == 1 {
                    "un régiment".to_owned()
                } else {
                    format!("{} régiments", destroyed.len())
                },
                if destroyed.len() == 1 { "" } else { "s" },
                destroyed.join(", ")
            ));
        }
        if result.general_killed {
            lines.push(format!(
                "Le chef de l’ost {} est tombé sur le champ.",
                de(name)
            ));
        } else if result.general_captured {
            lines.push(format!(
                "Le chef de l’ost {} est aux mains de l’ost {} : rançon à attendre.",
                de(name),
                de(&names[1 - index])
            ));
        }
        if result.no_quarter {
            lines.push(format!(
                "{name} a déployé l’étendard du « pas de quartier » : aucun prisonnier, aucune rançon."
            ));
        }
        let ended_quietly = matches!(
            outcome.end,
            BattleEnd::Refused | BattleEnd::Lull | BattleEnd::Withdrawal
        );
        if !ended_quietly {
            if result.withdrew {
                lines.push(format!(
                    "L’ost {} a sonné la retraite et quitté le champ en bon ordre.",
                    de(name)
                ));
            } else if result.routed {
                lines.push(format!("L’ost {} a été mis en déroute.", de(name)));
            }
        }
    }
    if lines.is_empty() {
        lines.push(
            "Aucun fait d’armes notable : les deux osts se sont séparés en bon ordre.".to_owned(),
        );
    }
    lines
}

/// Builds the view from the outcome and the final regiments.
pub fn build_view(outcome: &BattleOutcome, player: SideId, data: &HudData) -> ResultView {
    let enemy = match player {
        SideId::Attacker => SideId::Defender,
        SideId::Defender => SideId::Attacker,
    };
    let rows = [rows_of(&data.own), rows_of(&data.enemy)];
    let sides = [player, enemy].map(|side| (side, if side == player { 0 } else { 1 }));
    let summary = sides.map(|(side, index)| {
        let engaged: u32 = rows[index].iter().map(|r| r.initial).sum();
        let losses = outcome.side(side).total_losses;
        SideSummary {
            name: data.faction_names[index].clone(),
            engaged,
            losses,
            ratio: losses as f32 / engaged.max(1) as f32,
        }
    });
    let won = outcome.winner == player;
    let title = banner_title(won, summary[0].ratio, summary[1].ratio);
    let minutes = outcome.duration as u64 / 60;
    let seconds = outcome.duration as u64 % 60;
    let winner_name = &summary[usize::from(!won)].name;
    let names = [summary[0].name.clone(), summary[1].name.clone()];
    let nuance = if outcome.end == BattleEnd::Refused {
        "Bataille refusée · "
    } else {
        ""
    };
    ResultView {
        title,
        won,
        subtitle: format!("{nuance}{minutes} min {seconds:02} s · vainqueur : {winner_name}"),
        facts: facts(outcome, player, &names, &rows),
        sides: summary,
        rows,
    }
}

/// The screen currently shown.
#[derive(Resource, Default, Debug)]
pub struct ResultState {
    pub view: Option<ResultView>,
    pub returned: bool,
}

#[derive(Component)]
struct ResultOverlay;

pub struct ResultPlugin;

impl Plugin for ResultPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ResultState>()
            .add_systems(OnEnter(AppState::Battle), reset)
            .add_systems(
                Update,
                (show_result, result_clicks)
                    .chain()
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>),
            );
    }
}

fn reset(mut state: ResMut<ResultState>) {
    *state = ResultState::default();
}

fn show_result(
    mut commands: Commands,
    mut session: ResMut<BattleSession>,
    data: Res<HudData>,
    mut state: ResMut<ResultState>,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
) {
    if state.view.is_some() || !session.snapshot.finished || !data.finished {
        return;
    }
    let player = session.player.unwrap_or(SideId::Attacker);
    let Some(outcome) = session.sim_mut().outcome() else {
        return;
    };
    let view = build_view(&outcome, player, &data);
    let ctx = BuildCtx {
        theme: &theme,
        assets: &assets,
        data: &data,
    };
    spawn_screen(&mut commands, &ctx, &view);
    state.view = Some(view);
}

fn result_clicks(
    mut pressed: MessageReader<ButtonPressed>,
    mut state: ResMut<ResultState>,
    mut leave: MessageWriter<LeaveBattle>,
) {
    for press in pressed.read() {
        if press.name == RETURN_BUTTON && state.view.is_some() && !state.returned {
            state.returned = true;
            leave.write(LeaveBattle { abandoned: false });
        }
    }
}

fn text_node(
    commands: &mut Commands,
    theme: &UiTheme,
    parent: Entity,
    content: &str,
    size: f32,
    color: Color,
    width: Option<f32>,
) {
    commands.spawn((
        Node {
            width: width.map_or(Val::Auto, Val::Px),
            ..default()
        },
        Text::new(content),
        theme.text_font(false, false, size),
        TextColor(color),
        ChildOf(parent),
    ));
}

fn illustration_exists(unit_type: &str) -> bool {
    paths::find_root()
        .map(|root| {
            paths::asset_root(&root)
                .join(format!("illustrations/{unit_type}.jpg"))
                .exists()
        })
        .unwrap_or(false)
}

/// Regiment card of the end screen (`RosterCard` in « bilan » mode): illustration or colour +
/// class icon, soldiers in large, the fallen share veiled in red, « −losses », hero star.
fn roster_card(
    commands: &mut Commands,
    ctx: &BuildCtx,
    parent: Entity,
    row: &RegimentRow,
    color: Color,
) {
    let tooltip = format!(
        "[b]{}{}[/b]\n{} · {} sur {} engagés · −{} · {} ennemis abattus",
        row.name,
        if row.is_general { " ★" } else { "" },
        row.fate,
        number(f64::from(row.soldiers)),
        number(f64::from(row.initial)),
        number(f64::from(row.lost)),
        row.kills
    );
    let card = commands
        .spawn((
            Name::new(format!("rostercard:{}", row.id)),
            Hovered::default(),
            Tooltip::new(&tooltip),
            Node {
                width: Val::Px(CARD_W),
                height: Val::Px(CARD_H),
                border: UiRect::all(Val::Px(2.0)),
                overflow: Overflow::clip(),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(color.darker(0.35)),
            BorderColor::all(if row.hero {
                style::GOLD_PALE
            } else {
                style::INK_SOFT
            }),
            ChildOf(parent),
        ))
        .id();
    if illustration_exists(&row.unit_type) {
        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            ImageNode::new(
                ctx.assets
                    .load(format!("illustrations/{}.jpg", row.unit_type)),
            ),
            ChildOf(card),
        ));
    } else {
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    padding: UiRect::top(Val::Px(14.0)),
                    ..default()
                },
                ChildOf(card),
            ))
            .with_children(|center| {
                let id = if ctx.theme.icons.get(&row.unit_type).is_some() {
                    row.unit_type.clone()
                } else {
                    "unit_category_infantry".to_owned()
                };
                center.spawn(icon(ctx.theme, ctx.assets, &id, 32.0));
            });
    }
    // Fallen share, veiled in red from the top.
    let lost_share = row.lost as f32 / row.initial.max(1) as f32;
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(lost_share.clamp(0.0, 1.0) * 100.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.7, 0.05, 0.03, 0.45)),
        ChildOf(card),
    ));
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Px(4.0),
            ..default()
        },
        BackgroundColor(color),
        ChildOf(card),
    ));
    let shade = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                bottom: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            ChildOf(card),
        ))
        .id();
    commands.spawn((
        Text::new(row.soldiers.to_string()),
        ctx.theme.text_font(true, false, 17.0),
        TextColor(Color::srgb(1.0, 0.97, 0.88)),
        ChildOf(shade),
    ));
    if row.lost > 0 {
        commands.spawn((
            Text::new(format!("−{}", row.lost)),
            ctx.theme.text_font(false, false, 12.0),
            TextColor(Color::srgb(1.0, 0.6, 0.55)),
            ChildOf(shade),
        ));
    }
    if row.hero || row.is_general {
        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(2.0),
                top: Val::Px(4.0),
                ..default()
            },
            Text::new("★"),
            ctx.theme.text_font(true, false, 15.0),
            TextColor(style::GOLD_PALE),
            ChildOf(card),
        ));
    }
}

fn spawn_screen(commands: &mut Commands, ctx: &BuildCtx, view: &ResultView) {
    let theme = ctx.theme;
    let overlay = commands
        .spawn((
            Name::new("BattleResult"),
            ResultOverlay,
            DespawnOnExit(AppState::Battle),
            UiTier::Banner.index(),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(24.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.05, 0.02, 0.6)),
        ))
        .id();
    let holder = commands
        .spawn((
            Name::new("ResultScroll"),
            Node {
                width: Val::Px(1000.0),
                max_width: Val::Percent(100.0),
                max_height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::scroll_y(),
                ..default()
            },
            ChildOf(overlay),
        ))
        .id();
    let framed = commands
        .spawn((panel_illuminated(theme), ChildOf(holder)))
        .id();
    let page = commands
        .spawn((
            Name::new("ResultPage"),
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(8.0),
                padding: UiRect::all(Val::Px(20.0)),
                ..default()
            },
            ChildOf(framed),
        ))
        .id();
    commands.spawn((
        Name::new("Verdict"),
        illuminated_title(theme, view.title, &view.subtitle),
        ChildOf(page),
    ));
    // Balance grid.
    let balance = commands
        .spawn((
            Name::new("Balance"),
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                ..default()
            },
            ChildOf(page),
        ))
        .id();
    let head = commands
        .spawn((
            Node {
                column_gap: Val::Px(28.0),
                ..default()
            },
            ChildOf(balance),
        ))
        .id();
    for (text, width) in [
        ("", 160.0),
        ("Engagés", 110.0),
        ("Pertes", 140.0),
        ("Survivants", 110.0),
    ] {
        text_node(commands, theme, head, text, 14.0, style::MUTED, Some(width));
    }
    for summary in &view.sides {
        let row = commands
            .spawn((
                Node {
                    column_gap: Val::Px(28.0),
                    ..default()
                },
                ChildOf(balance),
            ))
            .id();
        text_node(
            commands,
            theme,
            row,
            &summary.name,
            16.0,
            style::INK,
            Some(160.0),
        );
        text_node(
            commands,
            theme,
            row,
            &number(f64::from(summary.engaged)),
            16.0,
            style::INK,
            Some(110.0),
        );
        text_node(
            commands,
            theme,
            row,
            &format!(
                "{} ({} %)",
                number(f64::from(summary.losses)),
                (summary.ratio * 100.0).round() as i32
            ),
            16.0,
            style::RUBRIC,
            Some(140.0),
        );
        text_node(
            commands,
            theme,
            row,
            &number(f64::from(summary.survivors())),
            16.0,
            style::GOOD,
            Some(110.0),
        );
    }
    commands.spawn((separator(theme), ChildOf(page)));
    // Notable facts.
    let facts = commands
        .spawn((
            Name::new("Mentions"),
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(1.0),
                align_self: AlignSelf::Stretch,
                ..default()
            },
            ChildOf(page),
        ))
        .id();
    text_node(
        commands,
        theme,
        facts,
        "Faits notables",
        UiType::Body.size(),
        style::INK_TITLE,
        None,
    );
    for fact in &view.facts {
        text_node(
            commands,
            theme,
            facts,
            &format!("• {fact}"),
            14.0,
            style::INK,
            None,
        );
    }
    commands.spawn((separator(theme), ChildOf(page)));
    // One column per side.
    let columns = commands
        .spawn((
            Name::new("Sides"),
            Node {
                column_gap: Val::Px(18.0),
                align_self: AlignSelf::Stretch,
                ..default()
            },
            ChildOf(page),
        ))
        .id();
    for (index, summary) in view.sides.iter().enumerate() {
        let column = commands
            .spawn((
                Name::new(format!("Side_{index}")),
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    flex_grow: 1.0,
                    flex_basis: Val::Px(0.0),
                    ..default()
                },
                ChildOf(columns),
            ))
            .id();
        text_node(
            commands,
            theme,
            column,
            &format!("Régiments — {}", summary.name),
            UiType::Body.size(),
            style::INK_TITLE,
            None,
        );
        let cards = commands
            .spawn((
                Name::new("RosterCards"),
                Node {
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(3.0),
                    row_gap: Val::Px(3.0),
                    ..default()
                },
                ChildOf(column),
            ))
            .id();
        for row in &view.rows[index] {
            roster_card(commands, ctx, cards, row, ctx.data.colors[index]);
        }
        let table = commands
            .spawn((
                Name::new(format!("Losses_{index}")),
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(1.0),
                    ..default()
                },
                ChildOf(column),
            ))
            .id();
        let head = commands
            .spawn((
                Node {
                    column_gap: Val::Px(8.0),
                    ..default()
                },
                ChildOf(table),
            ))
            .id();
        for (text, width) in [
            ("Régiment", 150.0),
            ("Engagés", 62.0),
            ("Pertes", 54.0),
            ("Tués", 46.0),
            ("Sort", 100.0),
        ] {
            text_node(commands, theme, head, text, 14.0, style::MUTED, Some(width));
        }
        for row in &view.rows[index] {
            let line = commands
                .spawn((
                    Node {
                        column_gap: Val::Px(8.0),
                        ..default()
                    },
                    ChildOf(table),
                ))
                .id();
            let name = format!("{}{}", row.name, if row.is_general { " ★" } else { "" });
            text_node(commands, theme, line, &name, 15.0, style::INK, Some(150.0));
            text_node(
                commands,
                theme,
                line,
                &row.initial.to_string(),
                15.0,
                style::INK,
                Some(62.0),
            );
            let lost_color = if row.lost > row.initial / 2 {
                style::RUBRIC
            } else {
                style::INK
            };
            text_node(
                commands,
                theme,
                line,
                &row.lost.to_string(),
                15.0,
                lost_color,
                Some(54.0),
            );
            let kill_color = if row.kills > 0 {
                style::GOOD
            } else {
                style::MUTED
            };
            text_node(
                commands,
                theme,
                line,
                &row.kills.to_string(),
                15.0,
                kill_color,
                Some(46.0),
            );
            let fate_color = if row.fate == "anéanti" || row.fate == "en déroute" {
                style::RUBRIC
            } else {
                style::MUTED
            };
            text_node(
                commands,
                theme,
                line,
                row.fate,
                15.0,
                fate_color,
                Some(100.0),
            );
        }
    }
    commands.spawn((
        button_primary(theme, RETURN_BUTTON, "Retour à la campagne"),
        ChildOf(page),
    ));
}

pub fn dump(world: &World) -> Value {
    let state = world.resource::<ResultState>();
    let Some(view) = &state.view else {
        return json!({ "shown": false });
    };
    json!({
        "shown": true,
        "title": view.title,
        "won": view.won,
        "subtitle": view.subtitle,
        "sides": view.sides.iter().map(|s| json!({"name": s.name, "engaged": s.engaged, "losses": s.losses, "survivors": s.survivors()})).collect::<Vec<_>>(),
        "facts": view.facts,
        "regiments": [view.rows[0].len(), view.rows[1].len()],
        "returned": state.returned,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdict_labels() {
        assert_eq!(banner_title(false, 0.1, 0.9), "Défaite");
        assert_eq!(banner_title(true, 0.1, 0.9), "Victoire");
        assert_eq!(banner_title(true, 0.4, 0.3), "Victoire à la Pyrrhus");
        assert_eq!(banner_title(true, 0.4, 0.6), "Victoire");
    }

    #[test]
    fn elision() {
        assert_eq!(de("France"), "de France");
        assert_eq!(de("Angleterre"), "d’Angleterre");
        assert_eq!(de("Bourgogne"), "de Bourgogne");
    }

    #[test]
    fn survivors_never_underflow() {
        let side = SideSummary {
            name: "X".into(),
            engaged: 10,
            losses: 12,
            ratio: 1.2,
        };
        assert_eq!(side.survivors(), 0);
    }

    fn unit(id: u32, initial: u32, soldiers: u32, kills: u32) -> UnitView {
        UnitView {
            id,
            unit_type: "unit_x".into(),
            name: "Piquiers".into(),
            category: data_model::UnitCategory::Infantry,
            soldiers,
            initial,
            morale: 50.0,
            fatigue: 0.0,
            ammo: 0,
            max_ammo: 0,
            can_shoot: false,
            is_general: false,
            present: soldiers > 0,
            left_field: false,
            reserve: false,
            state: sim_battle::UnitState::Idle,
            formation: "Ligne".into(),
            experience: 0,
            kills,
            fate: if soldiers > 0 {
                sim_battle::UnitFate::Held
            } else {
                sim_battle::UnitFate::Destroyed
            },
            melee: 1,
            ranged: 0,
            armor: 1,
            x: 0.0,
            z: 0.0,
            row: crate::battle::hud::view::BattleRow::Main,
        }
    }

    #[test]
    fn view_of_a_pyrrhic_victory() {
        let data = HudData {
            own: vec![unit(1, 100, 40, 90), unit(2, 100, 0, 0)],
            enemy: vec![unit(3, 100, 0, 0)],
            faction_names: ["France".into(), "Angleterre".into()],
            ..default()
        };
        let mut outcome = BattleOutcome {
            winner: SideId::Attacker,
            attacker: sim_battle::SideResult::default(),
            defender: sim_battle::SideResult::default(),
            duration: 754.0,
            end: BattleEnd::Rout,
        };
        outcome.attacker.total_losses = 160;
        outcome.defender.total_losses = 60;
        outcome.defender.routed = true;
        let view = build_view(&outcome, SideId::Attacker, &data);
        assert_eq!(view.title, "Victoire à la Pyrrhus");
        assert!(view.subtitle.contains("12 min 34 s"));
        assert!(view.subtitle.contains("France"));
        assert_eq!(view.sides[0].engaged, 200);
        assert_eq!(view.sides[0].survivors(), 40);
        assert!(view.rows[0][0].hero);
        assert!(view.facts.iter().any(|f| f.contains("anéanti")));
        assert!(view.facts.iter().any(|f| f.contains("déroute")));
    }
}
