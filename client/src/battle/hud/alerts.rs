//! Alerts column, top left: at most five typed alerts (`sim_battle::alerts`), a glyph and a short
//! French caption, cleared after `duration_s`, one line per kind with a « ×n » counter; a click
//! recentres the battle camera on the place. Port of battle_alerts_column.gd; importance,
//! duration and merge radius come from `data/rules/battle_alerts.json` through
//! [`sim_battle::alerts::AlertRules`].

use bevy::prelude::*;
use bevy::ui_widgets::Button;
use serde_json::{Value, json};
use sim_battle::alerts::{AlertKind, AlertRules, BattleAlert};

use super::BuildCtx;
use crate::app::AppState;
use crate::battle::scene::camera::BattleCameraState;
use crate::ui::kit::icons::icon;
use crate::ui::kit::style::{self, UiType};
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::widgets::panel_note;
use crate::ui::kit::{ButtonPressed, UiTheme};

/// A typed alert drained from the simulation (for the audio lot: cries, pings).
#[derive(Message, Debug, Clone)]
pub struct BattleAlertRaised(pub BattleAlert);

/// Caption of an alert kind (`LABELS`).
pub fn label(kind: AlertKind) -> &'static str {
    match kind {
        AlertKind::Rout => "Unité en déroute",
        AlertKind::GeneralDown => "Général tombé ou capturé",
        AlertKind::Flanked => "Flanc ou dos attaqué",
        AlertKind::Reinforcements => "Renforts en approche",
        AlertKind::AmmoOut => "Munitions épuisées",
        AlertKind::WallBreached => "Muraille rompue",
        AlertKind::GateDestroyed => "Porte détruite",
        AlertKind::SquareThreatened => "La place est menacée",
        AlertKind::Rallied => "Unité ralliée",
    }
}

/// « (×3) » counter, capped at « ×9+ ».
pub fn count_label(count: usize) -> String {
    match count {
        0 | 1 => String::new(),
        2..=9 => format!(" (×{count})"),
        _ => " (×9+)".to_owned(),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub kind: AlertKind,
    pub x: f64,
    pub z: f64,
    pub unit: Option<u32>,
    pub last_time: f64,
    pub remaining: f64,
}

/// One line of the column: a kind, how many units, where the latest happened.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub kind: AlertKind,
    pub count: usize,
    pub x: f64,
    pub z: f64,
}

/// Pure state of the column (unit-tested).
#[derive(Resource, Debug, Clone)]
pub struct AlertColumn {
    pub entries: Vec<Entry>,
    pub duration_s: f64,
    pub merge_radius_m: f64,
    pub max_shown: usize,
    pub importance: [u32; 9],
    /// Bumped whenever the visible rows may have changed.
    pub revision: u64,
    /// Total alerts received (dump).
    pub received: u64,
    pub clicks: u32,
}

const KINDS: [AlertKind; 9] = [
    AlertKind::Rout,
    AlertKind::GeneralDown,
    AlertKind::Flanked,
    AlertKind::Reinforcements,
    AlertKind::AmmoOut,
    AlertKind::WallBreached,
    AlertKind::GateDestroyed,
    AlertKind::SquareThreatened,
    AlertKind::Rallied,
];

fn kind_index(kind: AlertKind) -> usize {
    KINDS.iter().position(|k| *k == kind).unwrap_or(0)
}

impl AlertColumn {
    pub fn from_rules(rules: &AlertRules) -> Self {
        Self {
            entries: Vec::new(),
            duration_s: rules.duration_s,
            merge_radius_m: rules.merge_radius_m,
            max_shown: rules.max_shown as usize,
            importance: KINDS.map(|kind| rules.importance(kind)),
            revision: 0,
            received: 0,
            clicks: 0,
        }
    }

    /// Fallback values when the rules are not loaded (tests).
    pub fn fallback() -> Self {
        Self {
            entries: Vec::new(),
            duration_s: 10.0,
            merge_radius_m: 60.0,
            max_shown: 5,
            importance: [70, 100, 60, 50, 40, 80, 90, 95, 35],
            revision: 0,
            received: 0,
            clicks: 0,
        }
    }

    /// Adds an alert or refreshes the entry of the same kind: same unit, or (no unit) the same
    /// zone.
    pub fn push(&mut self, alert: &BattleAlert) {
        self.received += 1;
        self.revision += 1;
        for entry in &mut self.entries {
            if entry.kind != alert.kind {
                continue;
            }
            let same = match alert.unit {
                Some(unit) => entry.unit == Some(unit),
                None => {
                    entry.unit.is_none()
                        && (entry.x - alert.x).hypot(entry.z - alert.z) <= self.merge_radius_m
                }
            };
            if same {
                entry.last_time = alert.time;
                entry.x = alert.x;
                entry.z = alert.z;
                entry.remaining = self.duration_s;
                return;
            }
        }
        self.entries.push(Entry {
            kind: alert.kind,
            x: alert.x,
            z: alert.z,
            unit: alert.unit,
            last_time: alert.time,
            remaining: self.duration_s,
        });
    }

    /// Ages the entries; true when one expired.
    pub fn tick(&mut self, dt: f64) -> bool {
        let before = self.entries.len();
        for entry in &mut self.entries {
            entry.remaining -= dt;
        }
        self.entries.retain(|entry| entry.remaining > 0.0);
        let changed = self.entries.len() != before;
        if changed {
            self.revision += 1;
        }
        changed
    }

    /// Rows shown, most important first, at most `max_shown`.
    pub fn rows(&self) -> Vec<Row> {
        let mut rows: Vec<(Row, f64)> = Vec::new();
        for entry in &self.entries {
            match rows.iter_mut().find(|(row, _)| row.kind == entry.kind) {
                None => rows.push((
                    Row {
                        kind: entry.kind,
                        count: 1,
                        x: entry.x,
                        z: entry.z,
                    },
                    entry.last_time,
                )),
                Some((row, last)) => {
                    row.count += 1;
                    if entry.last_time >= *last {
                        *last = entry.last_time;
                        row.x = entry.x;
                        row.z = entry.z;
                    }
                }
            }
        }
        rows.sort_by(|(a, ta), (b, tb)| {
            let (ia, ib) = (
                self.importance[kind_index(a.kind)],
                self.importance[kind_index(b.kind)],
            );
            ib.cmp(&ia).then(tb.total_cmp(ta))
        });
        rows.into_iter()
            .map(|(row, _)| row)
            .take(self.max_shown)
            .collect()
    }
}

#[derive(Component)]
struct AlertFrame;

#[derive(Component)]
struct AlertList;

#[derive(Component)]
struct AlertRow {
    x: f32,
    z: f32,
}

pub struct AlertsPlugin;

impl Plugin for AlertsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<BattleAlertRaised>()
            .insert_resource(AlertColumn::fallback())
            .add_systems(OnEnter(AppState::Battle), reset_column)
            .add_systems(
                Update,
                (collect_alerts, rebuild_rows, alert_clicks)
                    .chain()
                    .run_if(in_state(AppState::Battle)),
            );
    }
}

fn reset_column(mut column: ResMut<AlertColumn>) {
    // The rules are installed by the battle scene when the battle opens (its OnEnter runs in the
    // same schedule; fall back on the defaults if not yet).
    *column = AlertColumn::from_rules(AlertRules::bundled());
}

fn collect_alerts(
    mut raised: MessageReader<BattleAlertRaised>,
    mut column: ResMut<AlertColumn>,
    time: Res<Time<Real>>,
) {
    for BattleAlertRaised(alert) in raised.read() {
        column.push(alert);
    }
    if !column.entries.is_empty() {
        column.tick(f64::from(time.delta_secs()));
    }
}

/// Spawns the (initially hidden) column.
pub fn spawn_column(commands: &mut Commands, root: Entity, ctx: &BuildCtx) {
    let wrapper = commands
        .spawn((
            Name::new("BattleAlertsColumn"),
            AlertFrame,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(8.0),
                top: Val::Px(8.0),
                width: Val::Px(260.0),
                display: Display::None,
                ..default()
            },
            ChildOf(root),
        ))
        .id();
    let outer = commands
        .spawn((panel_note(ctx.theme), ChildOf(wrapper)))
        .id();
    let frame = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                width: Val::Percent(100.0),
                ..default()
            },
            ChildOf(outer),
        ))
        .id();
    commands.spawn((
        AlertList,
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(2.0),
            width: Val::Percent(100.0),
            ..default()
        },
        ChildOf(frame),
    ));
}

#[allow(clippy::too_many_arguments)]
fn rebuild_rows(
    mut commands: Commands,
    column: Res<AlertColumn>,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
    list: Query<(Entity, Option<&Children>), With<AlertList>>,
    mut frame: Query<&mut Node, With<AlertFrame>>,
    mut seen: Local<u64>,
) {
    let Ok((list_entity, children)) = list.single() else {
        return;
    };
    if *seen == column.revision && children.is_some() {
        return;
    }
    *seen = column.revision;
    for child in children.into_iter().flatten() {
        commands.entity(*child).despawn();
    }
    let rows = column.rows();
    if let Ok(mut node) = frame.single_mut() {
        node.display = if rows.is_empty() {
            Display::None
        } else {
            Display::Flex
        };
    }
    for row in rows {
        let key = row.kind.key();
        commands
            .spawn((
                Name::new(format!("alert:{key}")),
                AlertRow {
                    x: row.x as f32,
                    z: row.z as f32,
                },
                Button,
                bevy::picking::hover::Hovered::default(),
                Tooltip::new("Cliquer : centrer la caméra sur le lieu."),
                Node {
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(6.0),
                    min_height: Val::Px(26.0),
                    padding: UiRect::axes(Val::Px(4.0), Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.8, 0.7, 0.5, 0.25)),
                ChildOf(list_entity),
            ))
            .with_children(|parent| {
                parent.spawn(icon(&theme, &assets, &format!("battle_alert_{key}"), 22.0));
                parent.spawn((
                    Text::new(format!("{}{}", label(row.kind), count_label(row.count))),
                    theme.text_font(false, false, UiType::Caption.size()),
                    TextColor(style::INK),
                ));
            });
    }
}

fn alert_clicks(
    mut pressed: MessageReader<ButtonPressed>,
    rows: Query<&AlertRow>,
    mut column: ResMut<AlertColumn>,
    camera: Option<ResMut<BattleCameraState>>,
) {
    let mut camera = camera;
    for press in pressed.read() {
        if !press.name.starts_with("alert:") {
            continue;
        }
        if let Ok(row) = rows.get(press.entity) {
            column.clicks += 1;
            if let Some(camera) = camera.as_deref_mut() {
                camera.target.x = row.x;
                camera.target.z = row.z;
            }
        }
    }
}

pub fn dump(world: &World) -> Value {
    let column = world.resource::<AlertColumn>();
    json!({
        "received": column.received,
        "entries": column.entries.len(),
        "rows": column.rows().iter().map(|r| json!({"kind": r.kind.key(), "count": r.count})).collect::<Vec<_>>(),
        "clicks": column.clicks,
        "duration_s": column.duration_s,
        "max_shown": column.max_shown,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_battle::SideId;

    fn alert(kind: AlertKind, unit: Option<u32>, x: f64, time: f64) -> BattleAlert {
        BattleAlert {
            kind,
            time,
            x,
            z: 0.0,
            side: Some(SideId::Attacker),
            unit,
        }
    }

    #[test]
    fn same_unit_refreshes_instead_of_counting() {
        let mut column = AlertColumn::fallback();
        column.push(&alert(AlertKind::Rout, Some(1), 0.0, 1.0));
        column.push(&alert(AlertKind::Rout, Some(1), 5.0, 2.0));
        assert_eq!(column.entries.len(), 1);
        column.push(&alert(AlertKind::Rout, Some(2), 9.0, 3.0));
        let rows = column.rows();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].count, 2);
        assert_eq!(rows[0].x, 9.0);
    }

    #[test]
    fn unitless_alerts_merge_by_zone() {
        let mut column = AlertColumn::fallback();
        column.push(&alert(AlertKind::WallBreached, None, 0.0, 1.0));
        column.push(&alert(AlertKind::WallBreached, None, 30.0, 2.0));
        column.push(&alert(AlertKind::WallBreached, None, 500.0, 3.0));
        assert_eq!(column.entries.len(), 2);
    }

    #[test]
    fn rows_sorted_by_importance_and_capped() {
        let mut column = AlertColumn::fallback();
        column.push(&alert(AlertKind::Rallied, Some(1), 0.0, 1.0));
        column.push(&alert(AlertKind::GeneralDown, Some(2), 0.0, 2.0));
        column.push(&alert(AlertKind::Rout, Some(3), 0.0, 3.0));
        let rows = column.rows();
        assert_eq!(rows[0].kind, AlertKind::GeneralDown);
        assert_eq!(rows[2].kind, AlertKind::Rallied);
        column.max_shown = 2;
        assert_eq!(column.rows().len(), 2);
    }

    #[test]
    fn entries_expire() {
        let mut column = AlertColumn::fallback();
        column.push(&alert(AlertKind::Rout, Some(1), 0.0, 1.0));
        assert!(!column.tick(5.0));
        assert!(column.tick(6.0));
        assert!(column.rows().is_empty());
    }

    #[test]
    fn counter_label() {
        assert_eq!(count_label(1), "");
        assert_eq!(count_label(3), " (×3)");
        assert_eq!(count_label(12), " (×9+)");
    }

    #[test]
    fn bundled_rules_feed_the_column() {
        let column = AlertColumn::from_rules(AlertRules::bundled());
        assert!(column.duration_s > 0.0);
        assert!(column.max_shown > 0);
    }
}
