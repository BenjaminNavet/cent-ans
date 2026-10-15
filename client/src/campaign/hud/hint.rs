//! « Que faire maintenant » card (ports game/scripts/ui/next_hint.gd, next_hint_card.gd and
//! game/scripts/map/next_hint_controller.gd). One hint at a time, the first of
//! `data/ui/next_hints.json` whose condition holds; a click performs its action, the cross hides
//! it until the next season. The conditions only read what the core already exposes (alerts,
//! idle army, treasury, building sites); no rule here.

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use game_api::Alert;
use serde_json::{Value, json};

use super::flow::EndTurnRequested;
use super::{HudData, HudSet, HudWidget, RightColumn};
use crate::agent::dump::DumpAppExt;
use crate::campaign::armies::SelectedArmy;
use crate::campaign::panels::OpenPanel;
use crate::campaign::provinces::picker::SelectProvince;
use crate::sim::Sim;
use crate::ui::kit::style::{self, UiType};
use crate::ui::kit::widgets::{button_small, panel_note};
use crate::ui::kit::{ButtonPressed, UiTheme, fr};
use sim_campaign::ArmyId;

/// Conditions served by an alert of the same kind.
const ALERT_KINDS: [&str; 5] = [
    "chronicle_decision",
    "siege",
    "enemy_army",
    "debt",
    "research_idle",
];

#[derive(Debug, Clone, PartialEq)]
pub struct HintDef {
    pub id: String,
    pub title: String,
    pub text: String,
    pub action: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HintConfig {
    pub idle_treasury_min: i64,
    pub idle_treasury_income_seasons: i64,
    pub hints: Vec<HintDef>,
}

impl HintConfig {
    pub fn parse(text: &str) -> Option<Self> {
        let tree: Value = serde_json::from_str(text).ok()?;
        let s = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_owned();
        Some(Self {
            idle_treasury_min: tree
                .get("idle_treasury_min")
                .and_then(Value::as_i64)
                .unwrap_or(2000),
            idle_treasury_income_seasons: tree
                .get("idle_treasury_income_seasons")
                .and_then(Value::as_i64)
                .unwrap_or(3),
            hints: tree
                .get("hints")?
                .as_array()?
                .iter()
                .map(|h| HintDef {
                    id: s(h, "id"),
                    title: s(h, "title"),
                    text: s(h, "text"),
                    action: s(h, "action"),
                })
                .collect(),
        })
    }

    pub fn load() -> Self {
        crate::paths::find_root()
            .ok()
            .and_then(|root| std::fs::read_to_string(root.join("data/ui/next_hints.json")).ok())
            .and_then(|text| Self::parse(&text))
            .unwrap_or(Self {
                idle_treasury_min: 2000,
                idle_treasury_income_seasons: 3,
                hints: Vec::new(),
            })
    }
}

/// What the conditions read.
#[derive(Debug, Clone, Default)]
pub struct HintInput {
    pub alerts: Vec<Alert>,
    /// `(province id, display name)` per alert province, for `{province}`.
    pub province_names: Vec<(String, String)>,
    pub idle_army: Option<(String, String)>,
    pub treasury: i64,
    pub income: i64,
    pub constructions: usize,
    pub can_build: bool,
    pub dismissed: Vec<String>,
}

/// The chosen hint with its filled texts.
#[derive(Debug, Clone, PartialEq)]
pub struct Hint {
    pub id: String,
    pub title: String,
    pub text: String,
    pub action: String,
    pub alert: Option<Alert>,
    pub army: Option<String>,
}

fn fill(template: &str, values: &[(&str, String)]) -> String {
    values.iter().fold(template.to_owned(), |text, (k, v)| {
        text.replace(&format!("{{{k}}}"), v)
    })
}

/// `idle_treasury`: the treasury passes the threshold while nothing is being built.
pub fn idle_treasury(input: &HintInput, config: &HintConfig) -> bool {
    if !input.can_build || input.constructions > 0 {
        return false;
    }
    let threshold = config
        .idle_treasury_min
        .max(input.income * config.idle_treasury_income_seasons);
    input.treasury >= threshold
}

/// The highest-priority hint that holds (`NextHint.choose`).
pub fn choose(input: &HintInput, config: &HintConfig) -> Option<Hint> {
    for def in &config.hints {
        if input.dismissed.contains(&def.id) {
            continue;
        }
        let mut alert = None;
        let mut army = None;
        let values: Vec<(&str, String)> = if ALERT_KINDS.contains(&def.id.as_str()) {
            let Some(found) = input.alerts.iter().find(|a| a.kind == def.id) else {
                continue;
            };
            let province = found
                .province
                .as_ref()
                .and_then(|id| input.province_names.iter().find(|(p, _)| p == id))
                .map(|(_, name)| name.clone())
                .unwrap_or_else(|| "une de vos provinces".to_owned());
            alert = Some(found.clone());
            vec![("province", province)]
        } else {
            match def.id.as_str() {
                "army_idle" => {
                    let Some((id, name)) = input.idle_army.clone() else {
                        continue;
                    };
                    army = Some(id);
                    vec![(
                        "army",
                        if name.is_empty() {
                            "votre armée".to_owned()
                        } else {
                            name
                        },
                    )]
                }
                "idle_treasury" if idle_treasury(input, config) => {
                    vec![("treasury", fr::amount(input.treasury))]
                }
                "end_turn" => Vec::new(),
                _ => continue,
            }
        };
        return Some(Hint {
            id: def.id.clone(),
            title: fill(&def.title, &values),
            text: fill(&def.text, &values),
            action: def.action.clone(),
            alert,
            army,
        });
    }
    None
}

#[derive(Resource)]
pub struct NextHint {
    pub config: HintConfig,
    pub current: Option<Hint>,
    dismissed: Vec<String>,
    dismissed_turn: u32,
    pub activated: u32,
    built: Option<Option<Hint>>,
}

impl Default for NextHint {
    fn default() -> Self {
        Self {
            config: HintConfig::load(),
            current: None,
            dismissed: Vec::new(),
            dismissed_turn: 0,
            activated: 0,
            built: None,
        }
    }
}

#[derive(Component)]
struct HintCard;

pub fn plugin(app: &mut App) {
    app.init_resource::<NextHint>()
        .register_dump(dump_hint)
        .add_systems(
            Update,
            (refresh, rebuild_card, handle_buttons)
                .chain()
                .in_set(HudSet::Ui),
        );
}

/// Player option `interface/next_hint` (on without settings).
pub fn hint_enabled(settings: Option<&crate::settings::Settings>) -> bool {
    settings.is_none_or(|s| s.next_hint)
}

fn refresh(
    hud: Res<HudData>,
    sim: Res<Sim>,
    mut hint: ResMut<NextHint>,
    settings: Option<Res<crate::settings::Settings>>,
) {
    let settings_changed = settings.as_ref().is_some_and(|s| s.is_changed());
    if !hud.is_changed() && !settings_changed {
        return;
    }
    if !hint_enabled(settings.as_deref()) {
        if hint.current.is_some() {
            hint.current = None;
        }
        return;
    }
    let (Some(top), Some(readiness), Some(data), Some(state)) = (
        hud.top.as_ref(),
        hud.readiness.as_ref(),
        sim.data.as_ref(),
        sim.state.as_ref(),
    ) else {
        return;
    };
    if hint.dismissed_turn != top.turn {
        hint.dismissed.clear();
        hint.dismissed_turn = top.turn;
    }
    let idle_army = readiness.idle_armies.first().map(|id| {
        let name = hud
            .armies
            .iter()
            .find(|a| &a.id == id)
            .map_or_else(String::new, |a| a.name.clone());
        (id.clone(), name)
    });
    let province_names = readiness
        .alerts
        .iter()
        .filter_map(|a| a.province.clone())
        .filter_map(|id| {
            let pid = data_model::ProvinceId::new(id.clone()).ok()?;
            Some((id, data.province_name(&pid)))
        })
        .collect();
    let player = &sim.player;
    let constructions = state
        .settlements
        .values()
        .filter(|s| &s.controller == player && s.construction.is_some())
        .count();
    let input = HintInput {
        alerts: readiness.alerts.clone(),
        province_names,
        idle_army,
        treasury: top.treasury,
        income: top.net_income,
        constructions,
        can_build: true,
        dismissed: hint.dismissed.clone(),
    };
    let chosen = choose(&input, &hint.config);
    if hint.current != chosen {
        hint.current = chosen;
    }
}

fn rebuild_card(
    mut commands: Commands,
    mut hint: ResMut<NextHint>,
    theme: Res<UiTheme>,
    column: Query<Entity, With<RightColumn>>,
    card: Query<Entity, With<HintCard>>,
) {
    let Ok(column) = column.single() else { return };
    let existing = card.single().ok();
    if existing.is_some() && hint.built.as_ref() == Some(&hint.current) {
        return;
    }
    if existing.is_none() && hint.built.as_ref() == Some(&hint.current) && hint.current.is_none() {
        return;
    }
    hint.built = Some(hint.current.clone());
    if let Some(entity) = existing {
        commands.entity(entity).despawn();
    }
    let Some(current) = hint.current.clone() else {
        return;
    };
    commands
        .spawn((
            Name::new("hud:hint"),
            HintCard,
            ChildOf(column),
            HudWidget,
            Hovered::default(),
            panel_note(&theme),
        ))
        .insert(Node {
            width: Val::Px(280.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(3.0),
            padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
            ..default()
        })
        .with_children(|card| {
            card.spawn((
                Text::new(format!("Conseil : {}", current.title)),
                theme.text_font(true, false, UiType::Caption.size()),
                TextColor(style::RUBRIC),
            ));
            card.spawn((
                Text::new(current.text.clone()),
                theme.text_font(false, false, UiType::Caption.size()),
                TextColor(style::INK),
            ));
            card.spawn(Node {
                column_gap: Val::Px(4.0),
                ..default()
            })
            .with_children(|row| {
                row.spawn(button_small(&theme, "hud:hint_go", "Y aller"));
                row.spawn(button_small(&theme, "hud:hint_hide", "Masquer"));
            });
        });
}

#[allow(clippy::too_many_arguments)]
fn handle_buttons(
    mut pressed: MessageReader<ButtonPressed>,
    mut hint: ResMut<NextHint>,
    sim: Res<Sim>,
    mut panels: MessageWriter<OpenPanel>,
    mut select_province: MessageWriter<SelectProvince>,
    mut decisions: MessageWriter<super::decision::OpenDecision>,
    mut selected_army: ResMut<SelectedArmy>,
    mut end_turn: MessageWriter<EndTurnRequested>,
) {
    for press in pressed.read() {
        let Some(current) = hint.current.clone() else {
            continue;
        };
        match press.name.as_str() {
            "hud:hint_hide" => {
                hint.dismissed.push(current.id);
            }
            "hud:hint_go" => {
                hint.activated += 1;
                match current.action.as_str() {
                    "alert" => {
                        if let Some(alert) = &current.alert {
                            super::activate_alert(
                                alert,
                                &mut panels,
                                &mut select_province,
                                &mut decisions,
                                &mut selected_army,
                            );
                        }
                    }
                    "select_army" => {
                        if let Some(id) = current.army.clone() {
                            if let Some(parsed) = ArmyId::parse(&id) {
                                selected_army.0 = Some(parsed);
                            }
                            panels.write(OpenPanel::Army(id));
                        }
                    }
                    "open_city" => {
                        let player = &sim.player;
                        let province = sim.state.as_ref().and_then(|state| {
                            state
                                .settlements
                                .values()
                                .find(|s| &s.controller == player)
                                .map(|s| s.province.as_str().to_owned())
                        });
                        if let Some(province) = province {
                            select_province.write(SelectProvince(Some(province.clone())));
                            panels.write(OpenPanel::Province(province));
                        }
                    }
                    "end_turn" => {
                        end_turn.write(EndTurnRequested::default());
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

fn dump_hint(world: &World) -> (String, Value) {
    let hint = world.resource::<NextHint>();
    (
        "next_hint".to_owned(),
        json!({
            "current": hint.current.as_ref().map(|h| json!({
                "id": h.id, "title": h.title, "text": h.text, "action": h.action,
            })),
            "dismissed": hint.dismissed,
            "activated": hint.activated,
            "configured": hint.config.hints.len(),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_api::turn::Severity;

    fn config() -> HintConfig {
        HintConfig::parse(
            r#"{"idle_treasury_min": 2000, "idle_treasury_income_seasons": 2, "hints": [
            {"id":"siege","title":"Défendez {province}","text":"{province} est assiégée","action":"alert"},
            {"id":"army_idle","title":"Armée","text":"Sélectionnez {army}","action":"select_army"},
            {"id":"idle_treasury","title":"Construisez","text":"Trésor {treasury}","action":"open_city"},
            {"id":"end_turn","title":"Fin","text":"Entrée","action":"end_turn"}]}"#,
        )
        .expect("config")
    }

    fn siege_alert() -> Alert {
        Alert {
            id: "siege:1".into(),
            kind: "siege".into(),
            severity: Severity::Danger,
            text: "x".into(),
            province: Some("prov_paris".into()),
            army: None,
            blocking: false,
        }
    }

    #[test]
    fn priority_follows_the_file_order() {
        let mut input = HintInput {
            can_build: true,
            ..Default::default()
        };
        assert_eq!(choose(&input, &config()).expect("hint").id, "end_turn");
        input.idle_army = Some(("army_1".into(), "Ost royal".into()));
        let hint = choose(&input, &config()).expect("hint");
        assert_eq!(hint.id, "army_idle");
        assert_eq!(hint.text, "Sélectionnez Ost royal");
        input.alerts = vec![siege_alert()];
        input.province_names = vec![("prov_paris".into(), "Paris".into())];
        let hint = choose(&input, &config()).expect("hint");
        assert_eq!(hint.id, "siege");
        assert_eq!(hint.title, "Défendez Paris");
    }

    #[test]
    fn dismissed_hints_are_skipped_and_treasury_needs_idle_sites() {
        let mut input = HintInput {
            can_build: true,
            treasury: 5000,
            income: 100,
            ..Default::default()
        };
        assert_eq!(choose(&input, &config()).expect("hint").id, "idle_treasury");
        input.constructions = 1;
        assert_eq!(choose(&input, &config()).expect("hint").id, "end_turn");
        input.constructions = 0;
        input.dismissed = vec!["idle_treasury".into()];
        assert_eq!(choose(&input, &config()).expect("hint").id, "end_turn");
    }

    #[test]
    fn next_hint_option_switches_the_card_off() {
        let mut settings = crate::settings::Settings::default();
        assert!(hint_enabled(Some(&settings)) && hint_enabled(None));
        settings.next_hint = false;
        assert!(!hint_enabled(Some(&settings)));
    }

    #[test]
    fn threshold_scales_with_income() {
        let cfg = config();
        let input = HintInput {
            can_build: true,
            treasury: 2500,
            income: 2000,
            ..Default::default()
        };
        assert!(!idle_treasury(&input, &cfg));
    }
}
