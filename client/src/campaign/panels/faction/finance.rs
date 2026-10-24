//! « Finances » tab of the faction panel (lot CREDIT-UI, ADR 0780): revenue and expenses by
//! item as bars, expected balance, debt by banker, borrow / repay buttons, extraordinary taxes,
//! bankruptcy-risk gauge, last eight seasons. Plus the advisor alert when the risk passes a
//! threshold and the notice of a royal default (`royal_default`, ADR 0777).
//!
//! Every figure comes from `game_api::finance::finance_view` and `game_api::hud::EconomyView`;
//! the orders `Borrow`, `Repay` and `LevyExtraordinary` go through
//! `game_api::diplomacy::submit_order` like the other panels. The client only chooses an amount
//! among the steps of `data/ui/finance_panel.json` (all texts and display thresholds live there).
//!
//! Widget names (agent `click`): `faction:tab:finance`, `faction:fin:lender:<id>`,
//! `faction:fin:borrow_step:<n>`, `faction:fin:borrow`, `faction:fin:repay_step:<n>`,
//! `faction:fin:repay`, `faction:fin:levy:<id>`. Key `campaign_toggle_finances` (Maj+J) opens the
//! panel on this tab. Dump section `finance_panel`.

use std::collections::HashMap;

use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use data_model::GameData;
use game_api::faction_panel::FactionPanelView;
use game_api::finance::{FinanceView, LenderView};
use serde_json::{Value, json};
use sim_campaign::Order;

use super::chart::season_label;
use super::view::{Kids, ink, pair, wrapped};
use super::{FactionPanel, TABS};
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::hud::flow::TurnAdvanced;
use crate::campaign::panels::OpenPanel;
use crate::settings::KeyMap;
use crate::sim::Sim;
use crate::ui::advisor_band::{
    AdvisorAction, AdvisorActionKind, AdvisorPriority, AdvisorWithdraw, BandSay,
};
use crate::ui::kit::focus::FocusDefault;
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::{
    ButtonPressed, UiTheme, UiType, button_primary, button_small, caption, fr, heading, separator,
    style,
};

/// Key of the tab in [`TABS`].
pub const TAB: &str = "faction:tab:finance";
/// Action of the key map that opens the tab.
pub const SHORTCUT: &str = "campaign_toggle_finances";
const ADVISOR_KEY: &str = "finance:risk";
const LABEL_WIDTH: f32 = 150.0;
const VALUE_WIDTH: f32 = 96.0;
const BAR_HEIGHT: f32 = 12.0;

/// Texts and thresholds of `data/ui/finance_panel.json`.
#[derive(Resource, Debug, Clone)]
pub struct FinanceTexts(pub Value);

impl FinanceTexts {
    pub fn parse(text: &str) -> Option<Self> {
        serde_json::from_str(text).ok().map(Self)
    }

    pub fn load() -> Self {
        crate::paths::find_root()
            .ok()
            .and_then(|root| std::fs::read_to_string(root.join("data/ui/finance_panel.json")).ok())
            .and_then(|text| Self::parse(&text))
            .unwrap_or_else(|| {
                Self::parse(include_str!("../../../../../data/ui/finance_panel.json"))
                    .unwrap_or(Self(Value::Null))
            })
    }

    fn at(&self, path: &str) -> &Value {
        path.split('.')
            .try_fold(&self.0, |node, key| node.get(key))
            .unwrap_or(&Value::Null)
    }

    /// Text at `path` (empty when absent).
    pub fn s(&self, path: &str) -> &str {
        self.at(path).as_str().unwrap_or("")
    }

    fn n(&self, path: &str) -> i64 {
        self.at(path).as_i64().unwrap_or(0)
    }

    fn list(&self, path: &str) -> &[Value] {
        self.at(path).as_array().map_or(&[], Vec::as_slice)
    }

    /// Text at `path` with its `{key}` holes filled.
    fn f(&self, path: &str, values: &[(&str, String)]) -> String {
        fill(self.s(path), values)
    }

    fn steps(&self, path: &str) -> Vec<(String, String, i64)> {
        self.list(path)
            .iter()
            .map(|step| {
                (
                    step["id"].as_str().unwrap_or("").to_owned(),
                    step["label"].as_str().unwrap_or("").to_owned(),
                    step["percent"].as_i64().unwrap_or(0),
                )
            })
            .collect()
    }

    /// Label of the gauge band holding `percent`.
    fn band(&self, percent: i64) -> (usize, &str) {
        let bands = self.list("risk.bands");
        let index = bands
            .iter()
            .position(|b| percent <= b["max"].as_i64().unwrap_or(i64::MAX))
            .unwrap_or(bands.len().saturating_sub(1));
        (
            index,
            bands
                .get(index)
                .map_or("", |b| b["label"].as_str().unwrap_or("")),
        )
    }
}

fn fill(template: &str, values: &[(&str, String)]) -> String {
    values.iter().fold(template.to_owned(), |text, (k, v)| {
        text.replace(&format!("{{{k}}}"), v)
    })
}

/// Number with one decimal at most, French comma.
fn num1(value: f64) -> String {
    if (value - value.round()).abs() < 0.05 {
        format!("{}", value.round() as i64)
    } else {
        format!("{value:.1}").replace('.', ",")
    }
}

/// The last order sent from the tab, for the player and the dump.
#[derive(Debug, Clone)]
pub struct LastOrder {
    pub kind: &'static str,
    pub detail: String,
    pub ok: bool,
    pub message: String,
    pub turn: u32,
}

/// Choices of the tab kept across rebuilds.
#[derive(Resource, Debug, Default)]
pub struct FinanceUi {
    pub lender: Option<String>,
    pub borrow_step: usize,
    pub repay_step: usize,
    pub last: Option<LastOrder>,
}

/// A bar of the revenue / expense chart (`key` of the budget line).
#[derive(Component)]
pub struct FinanceBar {
    pub key: String,
    pub value: i64,
}

/// Fill of the bankruptcy-risk gauge.
#[derive(Component)]
pub struct FinanceGauge {
    pub percent: i64,
}

/// What the borrow button can do now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BorrowPlan {
    Ready { lender: String, amount: i64 },
    Frozen(u32),
    NoRoom(String),
    BelowMinimum { lender: String, min: i64 },
    NoLender,
}

/// Amount of the step `percent` of the room of `lender`: rounded down to 50 livres, never
/// under the minimum loan nor over the room; `None` when the room is under the minimum.
pub fn borrow_amount(lender: &LenderView, percent: i64) -> Option<i64> {
    if lender.room < lender.min_loan || lender.room <= 0 {
        return None;
    }
    let raw = lender.room * percent / 100;
    let rounded = if percent >= 100 { raw } else { raw / 50 * 50 };
    Some(rounded.clamp(lender.min_loan, lender.room))
}

/// Amount of the repay step `percent`: a share of what can be paid now (the debt, capped by
/// the treasury).
pub fn repay_amount(debt: i64, treasury: i64, percent: i64) -> i64 {
    let base = debt.min(treasury.max(0));
    if base <= 0 {
        return 0;
    }
    if percent >= 100 {
        return base;
    }
    (base * percent / 100).max(1)
}

/// Lender chosen in the tab: the stored one if it still exists, else the first.
fn chosen_lender<'a>(finance: &'a FinanceView, ui: &FinanceUi) -> Option<&'a LenderView> {
    ui.lender
        .as_deref()
        .and_then(|id| finance.lenders.iter().find(|l| l.id == id))
        .or_else(|| finance.lenders.first())
}

/// The state of the borrow button for the current choices.
pub fn borrow_plan(
    texts: &FinanceTexts,
    finance: &FinanceView,
    ui: &FinanceUi,
    turn: u32,
) -> BorrowPlan {
    let Some(lender) = chosen_lender(finance, ui) else {
        return BorrowPlan::NoLender;
    };
    if finance.frozen_until > turn {
        return BorrowPlan::Frozen(finance.frozen_until);
    }
    if lender.room <= 0 {
        return BorrowPlan::NoRoom(lender.name.clone());
    }
    let steps = texts.steps("borrow.steps");
    let percent = steps.get(ui.borrow_step).map_or(0, |s| s.2);
    match borrow_amount(lender, percent) {
        Some(amount) => BorrowPlan::Ready {
            lender: lender.id.clone(),
            amount,
        },
        None => BorrowPlan::BelowMinimum {
            lender: lender.name.clone(),
            min: lender.min_loan,
        },
    }
}

fn repay_plan(texts: &FinanceTexts, finance: &FinanceView, treasury: i64, ui: &FinanceUi) -> i64 {
    let steps = texts.steps("repay.steps");
    let percent = steps.get(ui.repay_step).map_or(100, |s| s.2);
    repay_amount(finance.debt, treasury, percent)
}

pub struct FinancePlugin;

impl Plugin for FinancePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(FinanceTexts::load())
            .init_resource::<FinanceUi>()
            .register_dump(dump_finance)
            .add_systems(
                Update,
                (shortcut, handle_buttons, advisor_alerts, advisor_go)
                    .run_if(in_state(AppState::Campaign)),
            );
    }
}

/// Maj+J (`campaign_toggle_finances`) opens the faction panel on the Finances tab, switches to
/// it when the panel is open elsewhere and closes it when already there.
fn shortcut(
    keymap: Res<KeyMap>,
    keys: Res<ButtonInput<KeyCode>>,
    sim: Res<Sim>,
    mut panel: ResMut<FactionPanel>,
    mut open: MessageWriter<OpenPanel>,
    focus: Res<crate::ui::kit::text_field::TextInputFocus>,
    flow: Res<crate::campaign::hud::flow::TurnFlow>,
) {
    if flow.running() || focus.is_active() || !keymap.just_pressed(SHORTCUT, &keys) {
        return;
    }
    let player = sim.player.as_str().to_owned();
    if panel.open && panel.faction.as_deref() == Some(player.as_str()) {
        if panel.tab == TAB {
            open.write(OpenPanel::Faction(player));
        } else {
            panel.tab = TAB.to_owned();
            panel.dirty = true;
        }
    } else {
        panel.tab = TAB.to_owned();
        open.write(OpenPanel::Faction(player));
    }
}

fn record(
    panel: &mut FactionPanel,
    ui: &mut FinanceUi,
    kind: &'static str,
    detail: String,
    ok: bool,
    message: String,
    turn: u32,
) {
    panel.result = Some((ok, message.clone()));
    panel.dirty = true;
    ui.last = Some(LastOrder {
        kind,
        detail,
        ok,
        message,
        turn,
    });
}

fn handle_buttons(
    mut pressed: MessageReader<ButtonPressed>,
    texts: Res<FinanceTexts>,
    mut ui: ResMut<FinanceUi>,
    mut panel: ResMut<FactionPanel>,
    mut sim: ResMut<Sim>,
) {
    for press in pressed.read() {
        let Some(name) = press.name.strip_prefix("faction:fin:") else {
            continue;
        };
        if !panel.open {
            continue;
        }
        if let Some(id) = name.strip_prefix("lender:") {
            ui.lender = Some(id.to_owned());
            panel.dirty = true;
            continue;
        }
        if let Some(step) = name.strip_prefix("borrow_step:") {
            ui.borrow_step = step.parse().unwrap_or(0);
            panel.dirty = true;
            continue;
        }
        if let Some(step) = name.strip_prefix("repay_step:") {
            ui.repay_step = step.parse().unwrap_or(0);
            panel.dirty = true;
            continue;
        }
        let sim = &mut *sim;
        let player = sim.player.clone();
        let (Some(data), Some(state)) = (&sim.data, &mut sim.state) else {
            continue;
        };
        let Some(finance) = game_api::finance::finance_view(data, state, &player) else {
            continue;
        };
        let turn = state.turn;
        let treasury = state.factions.get(&player).map_or(0, |f| f.treasury);
        if name == "borrow" {
            let BorrowPlan::Ready { lender, amount } = borrow_plan(&texts, &finance, &ui, turn)
            else {
                continue;
            };
            let lender_name = finance
                .lenders
                .iter()
                .find(|l| l.id == lender)
                .map_or_else(|| lender.clone(), |l| l.name.clone());
            let result = game_api::diplomacy::submit_order(
                data,
                state,
                Order::Borrow {
                    lender: lender.clone(),
                    amount,
                },
            );
            let message = if result.ok {
                texts.f(
                    "borrow.done",
                    &[("amount", fr::amount(amount)), ("name", lender_name)],
                )
            } else {
                texts.f("result.refused", &[("reason", result.message)])
            };
            record(
                &mut panel,
                &mut ui,
                "borrow",
                format!("{lender}:{amount}"),
                result.ok,
                message,
                turn,
            );
        } else if name == "repay" {
            let amount = repay_plan(&texts, &finance, treasury, &ui);
            if amount <= 0 {
                continue;
            }
            let result = game_api::diplomacy::submit_order(data, state, Order::Repay { amount });
            let message = if result.ok {
                texts.f("repay.done", &[("amount", fr::amount(amount))])
            } else {
                texts.f("result.refused", &[("reason", result.message)])
            };
            record(
                &mut panel,
                &mut ui,
                "repay",
                amount.to_string(),
                result.ok,
                message,
                turn,
            );
        } else if let Some(id) = name.strip_prefix("levy:") {
            let Some(levy) = finance.levies.iter().find(|l| l.id == id) else {
                continue;
            };
            let (levy_name, gain) = (levy.name.clone(), levy.yield_livres);
            let result = game_api::diplomacy::submit_order(
                data,
                state,
                Order::LevyExtraordinary {
                    levy: id.to_owned(),
                },
            );
            let message = if result.ok {
                texts.f(
                    "levies.done",
                    &[("name", levy_name), ("amount", fr::amount(gain))],
                )
            } else {
                texts.f("result.refused", &[("reason", result.message)])
            };
            record(
                &mut panel,
                &mut ui,
                "levy",
                id.to_owned(),
                result.ok,
                message,
                turn,
            );
        }
    }
}

/// Advisor alerts: the risk of default over its threshold (standing message withdrawn when it
/// falls back) and the notice of every royal default, the player's or an AI's.
#[allow(clippy::too_many_arguments)]
fn advisor_alerts(
    sim: Res<Sim>,
    texts: Res<FinanceTexts>,
    mut advanced: MessageReader<TurnAdvanced>,
    mut says: MessageWriter<BandSay>,
    mut withdraws: MessageWriter<AdvisorWithdraw>,
    mut defaults: Local<HashMap<String, u32>>,
    mut shown: Local<bool>,
    mut last_turn: Local<Option<u32>>,
) {
    let (Some(data), Some(state)) = (&sim.data, &sim.state) else {
        return;
    };
    let turn_changed = advanced.read().count() > 0 || *last_turn != Some(state.turn);
    if !turn_changed {
        return;
    }
    *last_turn = Some(state.turn);
    let first_sight = defaults.is_empty();
    for (id, faction) in &state.factions {
        let count = faction.finance.defaults;
        let known = defaults.insert(id.as_str().to_owned(), count).unwrap_or(0);
        if !first_sight && count > known {
            let text = if id == &sim.player {
                texts.s("advisor.default_player").to_owned()
            } else {
                texts.f("advisor.default_other", &[("name", data.faction_label(id))])
            };
            says.write(BandSay {
                key: format!("finance:default:{}:{count}", id.as_str()),
                text,
                priority: if id == &sim.player {
                    AdvisorPriority::Urgent
                } else {
                    AdvisorPriority::Info
                },
                seconds: None,
                action: None,
                standing: false,
            });
        }
    }
    let warn = texts.n("risk.warn_percent");
    let risk = game_api::finance::finance_view(data, state, &sim.player)
        .map_or(0, |f| f.default_risk_percent);
    if risk >= warn && warn > 0 {
        *shown = true;
        says.write(BandSay {
            key: ADVISOR_KEY.to_owned(),
            text: format!(
                "{}\n{}",
                texts.s("advisor.risk_title"),
                texts.f("advisor.risk", &[("percent", risk.to_string())])
            ),
            priority: AdvisorPriority::Warning,
            seconds: None,
            action: Some(texts.s("advisor.risk_action").to_owned()),
            standing: true,
        });
    } else if *shown {
        *shown = false;
        withdraws.write(AdvisorWithdraw(ADVISOR_KEY.to_owned()));
    }
}

/// « Voir les finances » opens the tab.
fn advisor_go(
    mut actions: MessageReader<AdvisorAction>,
    sim: Res<Sim>,
    mut panel: ResMut<FactionPanel>,
    mut open: MessageWriter<OpenPanel>,
) {
    for action in actions.read() {
        if action.key != ADVISOR_KEY || action.kind != AdvisorActionKind::Go {
            continue;
        }
        panel.tab = TAB.to_owned();
        if panel.open && panel.faction.as_deref() == Some(sim.player.as_str()) {
            panel.dirty = true;
        } else {
            open.write(OpenPanel::Faction(sim.player.as_str().to_owned()));
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------------------------

fn button_row() -> impl Bundle {
    Node {
        flex_wrap: FlexWrap::Wrap,
        column_gap: Val::Px(6.0),
        row_gap: Val::Px(4.0),
        align_items: AlignItems::Center,
        ..default()
    }
}

/// A step button: primary when selected.
fn step_button(
    row: &mut Kids,
    theme: &UiTheme,
    name: &str,
    label: &str,
    selected: bool,
    disabled: bool,
) {
    let mut entity = if selected {
        row.spawn(button_primary(theme, name, label))
    } else {
        row.spawn(button_small(theme, name, label))
    };
    if disabled {
        entity.insert(InteractionDisabled);
    }
}

fn bar_row(
    pane: &mut Kids,
    theme: &UiTheme,
    texts: &FinanceTexts,
    key: &str,
    value: i64,
    scale: i64,
    tip: &str,
    colour: Color,
) {
    let label = texts.s(&format!("rubrics.{key}.label"));
    let label = if label.is_empty() { key } else { label };
    let share = if scale > 0 {
        (value.abs() as f32 / scale as f32).clamp(0.0, 1.0) * 100.0
    } else {
        0.0
    };
    pane.spawn((
        Name::new(format!("faction:fin:bar:{key}")),
        FinanceBar {
            key: key.to_owned(),
            value,
        },
        Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(6.0),
            width: Val::Percent(100.0),
            ..default()
        },
        Tooltip::new(tip),
    ))
    .with_children(|row| {
        row.spawn((
            ink(theme, label, UiType::Caption, style::INK),
            Node {
                width: Val::Px(LABEL_WIDTH),
                ..default()
            },
        ));
        row.spawn((
            Node {
                flex_grow: 1.0,
                height: Val::Px(BAR_HEIGHT),
                ..default()
            },
            BackgroundColor(style::PARCHMENT_DARK.with_alpha(0.35)),
            children![(
                Node {
                    width: Val::Percent(share.max(if value != 0 { 1.5 } else { 0.0 })),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(colour),
            )],
        ));
        row.spawn((
            ink(theme, &fr::signed(value), UiType::Caption, colour),
            Node {
                width: Val::Px(VALUE_WIDTH),
                justify_content: JustifyContent::FlexEnd,
                ..default()
            },
        ));
    });
}

fn bars_section(pane: &mut Kids, theme: &UiTheme, texts: &FinanceTexts, view: &FactionPanelView) {
    pane.spawn(heading(theme, texts.s("bars.title")));
    let economy = &view.economy;
    let has_past = economy.net_change.is_some();
    let known = |key: &str| !texts.s(&format!("rubrics.{key}.label")).is_empty();
    let lines: Vec<_> = economy
        .budget_lines
        .iter()
        .filter(|l| known(&l.key) && (l.key == "receipts" || l.charge))
        .collect();
    if lines.is_empty() {
        pane.spawn(caption(theme, texts.s("bars.empty")));
        return;
    }
    let scale = lines.iter().map(|l| l.projected.abs()).max().unwrap_or(0);
    let total = |receipts: bool| -> i64 {
        lines
            .iter()
            .filter(|l| (l.key == "receipts") == receipts)
            .map(|l| l.projected.abs())
            .sum()
    };
    for (receipts, title) in [(true, "bars.receipts"), (false, "bars.charges")] {
        pane.spawn(ink(theme, texts.s(title), UiType::Caption, style::RUBRIC));
        let sum = total(receipts).max(1);
        for line in lines.iter().filter(|l| (l.key == "receipts") == receipts) {
            let share = line.projected.abs() * 100 / sum;
            let last = match (has_past, line.last) {
                (true, Some(last)) => fr::signed(last),
                _ => "—".to_owned(),
            };
            let template = if has_past && line.last.is_some() {
                "bars.tip"
            } else {
                "bars.tip_no_past"
            };
            let side = texts.s(if receipts {
                "bars.side_receipts"
            } else {
                "bars.side_charges"
            });
            let body = texts.f(
                template,
                &[
                    ("value", fr::signed(line.projected)),
                    ("share", share.to_string()),
                    ("side", side.to_owned()),
                    ("last", last),
                ],
            );
            let label = texts.s(&format!("rubrics.{}.label", line.key));
            let tip = format!(
                "[b]{label}[/b]\n{}\n{body}",
                texts.s(&format!("rubrics.{}.tip", line.key))
            );
            bar_row(
                pane,
                theme,
                texts,
                &line.key,
                line.projected,
                scale,
                &tip,
                if receipts { style::GOOD } else { style::POOR },
            );
        }
    }
}

fn risk_section(pane: &mut Kids, theme: &UiTheme, texts: &FinanceTexts, finance: &FinanceView) {
    pane.spawn(heading(theme, texts.s("risk.title")));
    let percent = finance.default_risk_percent.max(0);
    let (band, label) = texts.band(percent);
    let colour = [style::GOOD, style::FAIR, style::POOR, style::WAX]
        .get(band)
        .copied()
        .unwrap_or(style::WAX);
    let warn = texts.n("risk.warn_percent");
    pane.spawn((
        Name::new("faction:fin:gauge"),
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(16.0),
            ..default()
        },
        BackgroundColor(style::PARCHMENT_DARK.with_alpha(0.35)),
        Tooltip::new(&format!(
            "[b]{}[/b]\n{}",
            texts.s("risk.title"),
            texts.s("risk.tip")
        )),
    ))
    .with_children(|gauge| {
        gauge.spawn((
            FinanceGauge { percent },
            Node {
                width: Val::Percent(percent.min(100) as f32),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(colour),
        ));
        gauge.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(warn as f32),
                width: Val::Px(2.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(style::INK),
        ));
    });
    pane.spawn(ink(
        theme,
        &format!(
            "{label} — {}",
            texts.f("risk.value", &[("percent", percent.to_string())])
        ),
        UiType::Body,
        colour,
    ));
    pane.spawn(caption(
        theme,
        &texts.f("risk.rating", &[("rating", finance.rating.to_string())]),
    ));
    if finance.defaults > 0 {
        pane.spawn(caption(
            theme,
            &texts.f("risk.defaults", &[("count", finance.defaults.to_string())]),
        ));
    }
    if finance.ransom_owed > 0 {
        pane.spawn(caption(
            theme,
            &texts.f(
                "risk.ransom",
                &[("amount", fr::amount(finance.ransom_owed))],
            ),
        ));
    }
    if finance.grievance_unrest > 0.0 {
        pane.spawn(wrapped(caption(
            theme,
            &texts.f(
                "risk.grievance",
                &[("unrest", num1(finance.grievance_unrest))],
            ),
        )));
    }
}

fn debt_section(pane: &mut Kids, theme: &UiTheme, texts: &FinanceTexts, finance: &FinanceView) {
    pane.spawn(heading(theme, texts.s("loans.title")));
    if finance.loans.is_empty() {
        pane.spawn(caption(theme, texts.s("loans.none")));
    }
    for loan in &finance.loans {
        pane.spawn(wrapped(ink(
            theme,
            &texts.f(
                "loans.line",
                &[
                    ("name", loan.lender_name.clone()),
                    ("principal", fr::amount(loan.principal)),
                    ("rate", num1(loan.rate_permille / 10.0)),
                    ("interest", fr::amount(loan.interest)),
                ],
            ),
            UiType::Body,
            style::INK,
        )));
    }
    if !finance.obligations.is_empty() {
        pane.spawn(ink(
            theme,
            texts.s("loans.obligation_title"),
            UiType::Caption,
            style::RUBRIC,
        ));
        for obligation in &finance.obligations {
            let kind = texts.s(&format!("loans.kinds.{}", obligation.kind));
            pane.spawn(wrapped(caption(
                theme,
                &texts.f(
                    "loans.obligation",
                    &[
                        ("kind", kind.to_owned()),
                        ("amount", fr::amount(obligation.per_season)),
                        ("seasons", obligation.seasons_left.to_string()),
                    ],
                ),
            )));
        }
    }
}

fn borrow_section(
    pane: &mut Kids,
    theme: &UiTheme,
    texts: &FinanceTexts,
    ui: &FinanceUi,
    finance: &FinanceView,
    turn: u32,
) {
    pane.spawn(heading(theme, texts.s("borrow.title")));
    if finance.frozen_until > turn {
        pane.spawn(wrapped(ink(
            theme,
            &texts.f(
                "lenders.frozen",
                &[("turn", finance.frozen_until.to_string())],
            ),
            UiType::Caption,
            style::POOR,
        )));
    }
    let chosen = chosen_lender(finance, ui).map(|l| l.id.clone());
    for (index, lender) in finance.lenders.iter().enumerate() {
        let selected = chosen.as_deref() == Some(lender.id.as_str());
        pane.spawn(Node {
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .with_children(|block| {
            block.spawn(button_row()).with_children(|row| {
                let name = format!("faction:fin:lender:{}", lender.id);
                let mut button = if selected {
                    row.spawn(button_primary(theme, &name, &lender.name))
                } else {
                    row.spawn(button_small(theme, &name, &lender.name))
                };
                button.insert(Tooltip::new(&format!(
                    "[b]{}[/b]\n{}",
                    lender.name,
                    texts.f(
                        "lenders.tip",
                        &[
                            ("name", lender.name.clone()),
                            ("room", fr::amount(lender.room)),
                            ("rate", num1(lender.rate_permille / 10.0)),
                        ]
                    )
                )));
                if selected || (chosen.is_none() && index == 0) {
                    button.insert(FocusDefault);
                }
            });
            block.spawn(wrapped(caption(
                theme,
                &texts.f(
                    "lenders.terms",
                    &[
                        ("rate", num1(lender.rate_permille / 10.0)),
                        ("room", fr::amount(lender.room)),
                        ("min", fr::amount(lender.min_loan)),
                    ],
                ),
            )));
        });
    }
    let plan = borrow_plan(texts, finance, ui, turn);
    pane.spawn(button_row()).with_children(|row| {
        for (index, (_, label, _)) in texts.steps("borrow.steps").iter().enumerate() {
            step_button(
                row,
                theme,
                &format!("faction:fin:borrow_step:{index}"),
                label,
                ui.borrow_step == index,
                false,
            );
        }
    });
    let (caption_text, tip, reason) = match &plan {
        BorrowPlan::Ready { lender, amount } => {
            let found = finance.lenders.iter().find(|l| &l.id == lender);
            let name = found.map_or(String::new(), |l| l.name.clone());
            let interest = found.map_or(0, |l| {
                (*amount as f64 * l.rate_permille / 1000.0).round() as i64
            });
            (
                texts.f("borrow.go", &[("amount", fr::amount(*amount))]),
                texts.f(
                    "borrow.go_tip",
                    &[
                        ("amount", fr::amount(*amount)),
                        ("name", name),
                        ("interest", fr::amount(interest)),
                    ],
                ),
                String::new(),
            )
        }
        other => {
            let reason = match other {
                BorrowPlan::Frozen(turn) => texts.f("borrow.frozen", &[("turn", turn.to_string())]),
                BorrowPlan::NoRoom(name) => texts.f("borrow.no_room", &[("name", name.clone())]),
                BorrowPlan::BelowMinimum { lender, min } => texts.f(
                    "borrow.below_min",
                    &[("name", lender.clone()), ("min", fr::amount(*min))],
                ),
                _ => texts.s("borrow.no_lender").to_owned(),
            };
            (
                texts.f("borrow.go", &[("amount", "…".to_owned())]),
                reason.clone(),
                reason,
            )
        }
    };
    let mut button = pane.spawn(button_primary(theme, "faction:fin:borrow", &caption_text));
    button.insert(Tooltip::new(&tip));
    if !reason.is_empty() {
        button.insert(InteractionDisabled);
        pane.spawn(wrapped(ink(theme, &reason, UiType::Caption, style::POOR)));
    }
}

fn repay_section(
    pane: &mut Kids,
    theme: &UiTheme,
    texts: &FinanceTexts,
    ui: &FinanceUi,
    finance: &FinanceView,
    treasury: i64,
) {
    pane.spawn(heading(theme, texts.s("repay.title")));
    let amount = repay_plan(texts, finance, treasury, ui);
    let reason = if finance.debt <= 0 {
        texts.s("repay.no_debt").to_owned()
    } else if amount <= 0 {
        texts.s("repay.no_cash").to_owned()
    } else {
        String::new()
    };
    pane.spawn(button_row()).with_children(|row| {
        for (index, (_, label, _)) in texts.steps("repay.steps").iter().enumerate() {
            step_button(
                row,
                theme,
                &format!("faction:fin:repay_step:{index}"),
                label,
                ui.repay_step == index,
                !reason.is_empty(),
            );
        }
    });
    let shown = if reason.is_empty() { amount } else { 0 };
    pane.spawn(wrapped(caption(
        theme,
        &texts.f("repay.amount", &[("amount", fr::amount(shown))]),
    )));
    let mut button = pane.spawn(button_primary(
        theme,
        "faction:fin:repay",
        &texts.f("repay.go", &[("amount", fr::amount(shown))]),
    ));
    if reason.is_empty() {
        button.insert(Tooltip::new(
            &texts.f("repay.go_tip", &[("amount", fr::amount(amount))]),
        ));
    } else {
        button.insert((InteractionDisabled, Tooltip::new(&reason)));
        pane.spawn(wrapped(ink(theme, &reason, UiType::Caption, style::POOR)));
    }
}

fn levies_section(
    pane: &mut Kids,
    theme: &UiTheme,
    texts: &FinanceTexts,
    finance: &FinanceView,
    data: &GameData,
    year: i32,
) {
    pane.spawn(heading(theme, texts.s("levies.title")));
    pane.spawn(wrapped(caption(theme, texts.s("levies.intro"))));
    for rule in &data.finance_rules.levies {
        let live = finance.levies.iter().find(|l| l.id == rule.id);
        let name = format!("faction:fin:levy:{}", rule.id);
        pane.spawn(Node {
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .with_children(|block| {
            let Some(levy) = live else {
                let values = [
                    ("name", rule.name.clone()),
                    ("year", rule.from_year.to_string()),
                ];
                let line = texts.f("levies.early", &values);
                block.spawn(button_row()).with_children(|row| {
                    row.spawn((
                        button_small(theme, &name, &line),
                        InteractionDisabled,
                        Tooltip::new(&texts.f("levies.early_tip", &values)),
                    ));
                });
                debug_assert!(year < rule.from_year || live.is_some());
                return;
            };
            let gain = fr::amount(levy.yield_livres);
            let values = [
                ("name", levy.name.clone()),
                ("amount", gain),
                ("unrest", num1(levy.unrest)),
                ("turns", levy.unrest_turns.to_string()),
                ("cooldown", levy.cooldown_turns.to_string()),
            ];
            if levy.ready_in == 0 {
                block.spawn(button_row()).with_children(|row| {
                    row.spawn((
                        button_small(theme, &name, &texts.f("levies.button", &values)),
                        Tooltip::new(&format!(
                            "[b]{}[/b]\n{}",
                            levy.name,
                            texts.f("levies.tip", &values)
                        )),
                    ));
                });
                block.spawn(wrapped(caption(theme, &texts.f("levies.line", &values))));
            } else {
                let wait = [
                    ("name", levy.name.clone()),
                    ("turns", levy.ready_in.to_string()),
                ];
                block.spawn(button_row()).with_children(|row| {
                    row.spawn((
                        button_small(theme, &name, &texts.f("levies.wait", &wait)),
                        InteractionDisabled,
                        Tooltip::new(&texts.f("levies.wait_tip", &wait)),
                    ));
                });
                block.spawn(wrapped(caption(theme, &texts.f("levies.line", &values))));
            }
        });
    }
}

fn history_section(
    pane: &mut Kids,
    theme: &UiTheme,
    texts: &FinanceTexts,
    view: &FactionPanelView,
) {
    pane.spawn(heading(theme, texts.s("history.title")));
    let count = texts.n("history.count").max(1) as usize;
    let records: Vec<_> = view
        .economy
        .budget_history
        .iter()
        .rev()
        .take(count)
        .collect();
    if records.is_empty() {
        pane.spawn(caption(theme, texts.s("history.empty")));
        return;
    }
    let head = texts.list("history.head");
    let cell = |row: &mut Kids, text: &str, colour: Color, wide: bool| {
        row.spawn((
            ink(theme, text, UiType::Caption, colour),
            Node {
                width: Val::Px(if wide { 120.0 } else { VALUE_WIDTH }),
                justify_content: if wide {
                    JustifyContent::FlexStart
                } else {
                    JustifyContent::FlexEnd
                },
                ..default()
            },
        ));
    };
    pane.spawn(Node {
        column_gap: Val::Px(6.0),
        ..default()
    })
    .with_children(|row| {
        for (index, title) in head.iter().enumerate() {
            cell(
                row,
                title.as_str().unwrap_or(""),
                style::INK_SOFT,
                index == 0,
            );
        }
    });
    for record in records {
        pane.spawn((
            Name::new(format!("faction:fin:history:{}", record.turn)),
            Node {
                column_gap: Val::Px(6.0),
                ..default()
            },
        ))
        .with_children(|row| {
            cell(row, &season_label(record.turn), style::INK, true);
            cell(row, &fr::amount(record.treasury), style::INK, false);
            cell(
                row,
                &fr::signed(record.net),
                fr::color_of(record.net),
                false,
            );
            cell(
                row,
                &fr::signed(record.change),
                fr::color_of(record.change),
                false,
            );
        });
    }
}

/// Fills the Finances pane.
pub(super) fn finance_tab(
    pane: &mut Kids,
    theme: &UiTheme,
    texts: &FinanceTexts,
    ui: &FinanceUi,
    view: &FactionPanelView,
    data: &GameData,
    turn: u32,
    year: i32,
    shortcut_label: &str,
) {
    let economy = &view.economy;
    pane.spawn(heading(theme, texts.s("title")));
    if !shortcut_label.is_empty() {
        pane.spawn(caption(theme, shortcut_label));
    }
    pair(
        pane,
        theme,
        texts.s("summary.treasury"),
        &fr::amount(economy.treasury),
        fr::color_of(economy.treasury),
    );
    pane.spawn(Tooltip::new(&format!(
        "[b]{}[/b]\n{}",
        texts.s("summary.net"),
        texts.s("summary.net_tip")
    )))
    .with_children(|row| {
        pair(
            row,
            theme,
            texts.s("summary.net"),
            &fr::signed(economy.net_income),
            fr::color_of(economy.net_income),
        );
    });
    let forecast = economy.treasury + economy.net_income;
    pane.spawn(Tooltip::new(&format!(
        "[b]{}[/b]\n{}",
        texts.s("summary.forecast"),
        texts.s("summary.forecast_tip")
    )))
    .with_children(|row| {
        pair(
            row,
            theme,
            texts.s("summary.forecast"),
            &fr::signed(forecast),
            fr::color_of(forecast),
        );
    });
    bars_section(pane, theme, texts, view);
    let Some(finance) = &view.finance else {
        return;
    };
    pane.spawn(separator(theme));
    pair(
        pane,
        theme,
        texts.s("summary.debt"),
        &fr::amount(finance.debt),
        if finance.debt > 0 {
            style::POOR
        } else {
            style::INK
        },
    );
    pair(
        pane,
        theme,
        texts.s("summary.interest"),
        &fr::amount(finance.interest),
        style::INK,
    );
    risk_section(pane, theme, texts, finance);
    pane.spawn(separator(theme));
    debt_section(pane, theme, texts, finance);
    pane.spawn(separator(theme));
    borrow_section(pane, theme, texts, ui, finance, turn);
    pane.spawn(separator(theme));
    repay_section(pane, theme, texts, ui, finance, economy.treasury);
    pane.spawn(separator(theme));
    levies_section(pane, theme, texts, finance, data, year);
    pane.spawn(separator(theme));
    history_section(pane, theme, texts, view);
}

fn dump_finance(world: &World) -> (String, Value) {
    let panel = world.resource::<FactionPanel>();
    let ui = world.resource::<FinanceUi>();
    let texts = world.resource::<FinanceTexts>();
    let mut buttons = Vec::new();
    if let Some(mut query) = world.try_query::<(&Name, Option<&InteractionDisabled>)>() {
        for (name, disabled) in query.iter(world) {
            if name.as_str().starts_with("faction:fin:") && !name.as_str().contains(":bar:") {
                buttons.push((name.as_str().to_owned(), disabled.is_none()));
            }
        }
    }
    buttons.sort();
    let mut bars = Vec::new();
    if let Some(mut query) = world.try_query::<&FinanceBar>() {
        bars = query
            .iter(world)
            .map(|b| json!({"key": b.key, "value": b.value}))
            .collect();
    }
    let gauge = world
        .try_query::<&FinanceGauge>()
        .and_then(|mut q| q.iter(world).next().map(|g| g.percent));
    let finance = panel.view.as_ref().and_then(|v| v.finance.as_ref());
    let sim = world.resource::<Sim>();
    let plan = finance.map(|f| {
        let turn = sim.state.as_ref().map_or(0, |s| s.turn);
        format!("{:?}", borrow_plan(texts, f, ui, turn))
    });
    (
        "finance_panel".into(),
        json!({
            "open": panel.open && panel.tab == TAB,
            "tab": panel.tab,
            "treasury": panel.view.as_ref().map(|v| v.economy.treasury),
            "net_income": panel.view.as_ref().map(|v| v.economy.net_income),
            "history_rows": panel.view.as_ref().map(|v| v.economy.budget_history.len().min(texts.n("history.count").max(1) as usize)),
            "debt": finance.map(|f| f.debt),
            "interest": finance.map(|f| f.interest),
            "risk_percent": finance.map(|f| f.default_risk_percent),
            "gauge_percent": gauge,
            "frozen_until": finance.map(|f| f.frozen_until),
            "lenders": finance.map(|f| f.lenders.iter().map(|l| json!({
                "id": l.id, "rate_permille": l.rate_permille, "room": l.room, "min_loan": l.min_loan,
            })).collect::<Vec<_>>()),
            "loans": finance.map(|f| f.loans.iter().map(|l| json!({
                "lender": l.lender, "principal": l.principal, "interest": l.interest,
            })).collect::<Vec<_>>()),
            "levies": finance.map(|f| f.levies.iter().map(|l| json!({
                "id": l.id, "yield": l.yield_livres, "unrest": l.unrest, "ready_in": l.ready_in,
            })).collect::<Vec<_>>()),
            "selected_lender": finance.and_then(|f| chosen_lender(f, ui)).map(|l| l.id.clone()),
            "borrow_step": ui.borrow_step,
            "repay_step": ui.repay_step,
            "borrow_plan": plan,
            "bars": bars,
            "buttons": buttons.iter().map(|(name, enabled)| json!({"name": name, "enabled": enabled})).collect::<Vec<_>>(),
            "last_order": ui.last.as_ref().map(|o| json!({
                "kind": o.kind, "detail": o.detail, "ok": o.ok, "message": o.message, "turn": o.turn,
            })),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lender(room: i64, min_loan: i64) -> LenderView {
        LenderView {
            id: "lombards".into(),
            name: "Lombards".into(),
            rate_permille: 45.0,
            room,
            min_loan,
        }
    }

    #[test]
    fn borrow_steps_round_down_and_respect_the_limits() {
        let l = lender(10_030, 500);
        assert_eq!(borrow_amount(&l, 0), Some(500));
        assert_eq!(borrow_amount(&l, 25), Some(2_500));
        assert_eq!(borrow_amount(&l, 50), Some(5_000));
        assert_eq!(borrow_amount(&l, 100), Some(10_030));
        assert_eq!(borrow_amount(&lender(300, 500), 100), None);
        assert_eq!(borrow_amount(&lender(0, 500), 100), None);
    }

    #[test]
    fn repay_steps_are_capped_by_the_treasury() {
        assert_eq!(repay_amount(8_000, 2_000, 100), 2_000);
        assert_eq!(repay_amount(8_000, 20_000, 50), 4_000);
        assert_eq!(repay_amount(8_000, -5, 100), 0);
        assert_eq!(repay_amount(0, 100, 25), 0);
        assert_eq!(repay_amount(3, 100, 25), 1);
    }

    #[test]
    fn the_texts_parse_and_the_risk_bands_cover_every_value() {
        let texts = FinanceTexts::load();
        assert!(!texts.s("title").is_empty());
        assert_eq!(texts.band(0).1, "Maîtrisé");
        assert_eq!(texts.band(45).1, "À surveiller");
        assert_eq!(texts.band(99).1, "Critique");
        assert_eq!(texts.band(100_000_000).0, 3);
        assert_eq!(texts.steps("borrow.steps").len(), 4);
        assert_eq!(
            texts.f(
                "borrow.done",
                &[("amount", "1".into()), ("name", "x".into())]
            ),
            "Emprunt de 1 auprès de x."
        );
    }

    #[test]
    fn numbers_print_with_a_french_comma() {
        assert_eq!(num1(4.0), "4");
        assert_eq!(num1(2.5), "2,5");
    }

    #[test]
    fn the_tab_is_registered_with_the_panel() {
        assert!(TABS.contains(&TAB));
    }
}
