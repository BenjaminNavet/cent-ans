//! « Que faire maintenant » card (ports game/scripts/ui/next_hint.gd, next_hint_card.gd and
//! game/scripts/map/next_hint_controller.gd). One hint at a time, the first of
//! `data/ui/next_hints.json` whose condition holds; a click performs its action, the cross hides
//! it until the next season. The conditions only read what the core already exposes (alerts,
//! idle army, treasury, building sites); no rule here.

use bevy::prelude::*;
use game_api::Alert;
use serde_json::{Value, json};

use super::flow::EndTurnRequested;
use super::{HudData, HudSet};
use crate::agent::dump::DumpAppExt;
use crate::campaign::armies::SelectedArmy;
use crate::campaign::panels::OpenPanel;
use crate::campaign::provinces::picker::SelectProvince;
use crate::sim::Sim;
use crate::ui::advisor_band::{
    AdvisorAction, AdvisorActionKind, AdvisorPriority, AdvisorWithdraw, BandSay,
};
use crate::ui::kit::fr;
use sim_campaign::ArmyId;

/// Conditions served by an alert of the same kind.
const ALERT_KINDS: [&str; 6] = [
    "chronicle_decision",
    "siege",
    "enemy_army",
    "debt",
    "research_idle",
    // GAMEPLAY2 (ADR 0789): the counsellor's season advice; `text` = title, then body.
    "advice",
];

#[derive(Debug, Clone, PartialEq)]
pub struct HintDef {
    pub id: String,
    pub title: String,
    pub text: String,
    /// Wording when the alert's province belongs to someone else (a neighbour of ours): no
    /// garrison of ours to reinforce (lot HIST4). Falls back to `text`.
    pub text_foreign: Option<String>,
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
                    text_foreign: h
                        .get("text_foreign")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
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
    /// Alert provinces held by another faction.
    pub foreign_provinces: Vec<String>,
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
        let mut foreign = false;
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
            foreign = found
                .province
                .as_ref()
                .is_some_and(|id| input.foreign_provinces.contains(id));
            alert = Some(found.clone());
            let (title, body) = found
                .text
                .split_once('\n')
                .unwrap_or(("", found.text.as_str()));
            vec![
                ("province", province),
                ("title", title.to_owned()),
                ("text", body.to_owned()),
            ]
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
            text: fill(
                def.text_foreign
                    .as_deref()
                    .filter(|_| foreign)
                    .unwrap_or(&def.text),
                &values,
            ),
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
    /// What the band was last told (`Some(None)` = told that there is nothing).
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

pub fn plugin(app: &mut App) {
    app.init_resource::<NextHint>()
        .register_dump(dump_hint)
        .add_systems(
            Update,
            (refresh, publish, handle_buttons)
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
    let foreign_provinces = readiness
        .alerts
        .iter()
        .filter_map(|a| a.province.clone())
        .filter(|id| {
            data_model::ProvinceId::new(id.clone())
                .ok()
                .and_then(|pid| state.province_controller(&pid).cloned())
                .is_some_and(|controller| &controller != player)
        })
        .collect();
    let constructions = state
        .settlements
        .values()
        .filter(|s| &s.controller == player && s.construction.is_some())
        .count();
    let input = HintInput {
        alerts: readiness.alerts.clone(),
        province_names,
        foreign_provinces,
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

/// Key of the band message of hint `id`.
fn band_key(id: &str) -> String {
    format!("hint:{id}")
}

/// Sends the current hint to the single advisor band (ADR 0752) and withdraws the one that is
/// gone. A hint is a standing message: after the reading time it folds into the band's chip.
fn publish(
    mut hint: ResMut<NextHint>,
    mut says: MessageWriter<BandSay>,
    mut withdraws: MessageWriter<AdvisorWithdraw>,
    keymap: Option<Res<crate::settings::KeyMap>>,
    settings: Option<Res<crate::settings::Settings>>,
) {
    if hint.built.as_ref() == Some(&hint.current) {
        return;
    }
    let now = hint.current.clone();
    let previous = hint.built.replace(now).flatten();
    if let Some(previous) = previous
        && hint.current.as_ref().map(|h| h.id.as_str()) != Some(previous.id.as_str())
    {
        withdraws.write(AdvisorWithdraw(band_key(&previous.id)));
    }
    let Some(current) = hint.current.clone() else {
        return;
    };
    // `{key:ACTION}` reads the key actually bound (never a letter written in the data).
    let keys = super::bar::KeyHints::new(keymap.as_deref(), settings.as_deref());
    let title = keys.expand(&current.title);
    let text = keys.expand(&current.text);
    let priority = if matches!(current.id.as_str(), "siege" | "enemy_army") {
        AdvisorPriority::Warning
    } else {
        AdvisorPriority::Advice
    };
    says.write(BandSay {
        key: band_key(&current.id),
        text: format!("{title}\n{text}"),
        priority,
        seconds: None,
        action: Some("Y aller".to_owned()),
        standing: true,
    });
}

#[allow(clippy::too_many_arguments)]
fn handle_buttons(
    mut actions: MessageReader<AdvisorAction>,
    mut hint: ResMut<NextHint>,
    sim: Res<Sim>,
    mut panels: MessageWriter<OpenPanel>,
    mut select_province: MessageWriter<SelectProvince>,
    mut decisions: MessageWriter<super::decision::OpenDecision>,
    mut selected_army: ResMut<SelectedArmy>,
    mut end_turn: MessageWriter<EndTurnRequested>,
) {
    for action in actions.read() {
        let Some(current) = hint.current.clone() else {
            continue;
        };
        if action.key != band_key(&current.id) {
            continue;
        }
        match action.kind {
            AdvisorActionKind::Later => {
                hint.dismissed.push(current.id);
            }
            AdvisorActionKind::Go => {
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
    fn an_enemy_at_a_foreign_province_does_not_ask_for_a_garrison() {
        let config = HintConfig::parse(
            r#"{"hints": [{"id":"enemy_army","title":"Ennemi","text":"Renforcez la garnison de {province}",
            "text_foreign":"Couvrez vos marches près de {province}","action":"alert"}]}"#,
        )
        .expect("config");
        let mut alert = siege_alert();
        alert.kind = "enemy_army".into();
        let mut input = HintInput {
            alerts: vec![alert],
            province_names: vec![("prov_paris".into(), "Paris".into())],
            ..Default::default()
        };
        assert_eq!(
            choose(&input, &config).expect("hint").text,
            "Renforcez la garnison de Paris"
        );
        input.foreign_provinces = vec!["prov_paris".into()];
        assert_eq!(
            choose(&input, &config).expect("hint").text,
            "Couvrez vos marches près de Paris"
        );
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
