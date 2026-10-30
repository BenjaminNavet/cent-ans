//! « États et Parlement » tab of the faction panel (lot ECO-R1d, ADR 0813): the assemblies of the
//! player's house, their mood, the necessity invoked, the tier of demand and the form of the
//! subsidy, the forecast of the vote (grant, conditions or refusal with the named parts of the
//! agreement), the open session and the answer to the terms, the promises under way.
//!
//! Every figure comes from `game_api::assembly` (`assembly_panel`, `forecast_view`), which reads
//! `sim_campaign::assembly`; the client only picks a request and sends `ConveneAssembly` /
//! `AnswerAssembly` through `game_api::diplomacy::submit_order`. All texts live in
//! `data/ui/assembly_panel.json`.
//!
//! Widget names (agent `click`): `faction:tab:assembly`, `faction:asm:house:<id>`,
//! `faction:asm:necessity:<key>`, `faction:asm:ask:<id>`, `faction:asm:form:<id>`,
//! `faction:asm:permanent`, `faction:asm:convene:<id>`, `faction:asm:accept:<session>`,
//! `faction:asm:decline:<session>`; the Finances tab has `faction:fin:levy_assembly`
//! (« Convoquer les États »). Dump section `assembly_panel`.

use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use data_model::GameData;
use game_api::assembly::{
    AskView, AssemblyPanelView, AssemblyRow, ForecastView, FormView, NecessityView,
};
use serde_json::{Value, json};
use sim_campaign::{CampaignState, Order};

use super::FactionPanel;
use super::chart::season_label;
use super::finance::LastOrder;
use super::view::{Kids, ink, wrapped};
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::sim::Sim;
use crate::ui::kit::focus::FocusDefault;
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::{
    ButtonPressed, UiTheme, UiType, button_primary, button_small, caption, fr, heading, separator,
    style,
};

/// Key of the tab in the tabs of the faction panel.
pub const TAB: &str = "faction:tab:assembly";
const BAR_HEIGHT: f32 = 12.0;

/// Texts of `data/ui/assembly_panel.json`.
#[derive(Resource, Debug, Clone)]
pub struct AssemblyTexts(pub Value);

impl AssemblyTexts {
    pub fn parse(text: &str) -> Option<Self> {
        serde_json::from_str(text).ok().map(Self)
    }

    pub fn load() -> Self {
        crate::paths::find_root()
            .ok()
            .and_then(|root| std::fs::read_to_string(root.join("data/ui/assembly_panel.json")).ok())
            .and_then(|text| Self::parse(&text))
            .unwrap_or_else(|| {
                Self::parse(include_str!("../../../../../data/ui/assembly_panel.json"))
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

    /// Text at `path` with its `{key}` holes filled.
    pub fn f(&self, path: &str, values: &[(&str, String)]) -> String {
        values.iter().fold(self.s(path).to_owned(), |text, (k, v)| {
            text.replace(&format!("{{{k}}}"), v)
        })
    }

    /// Index and label of the mood band holding `mood`.
    fn mood_band(&self, mood: i32) -> (usize, &str) {
        let bands = self
            .at("mood.bands")
            .as_array()
            .map_or(&[][..], Vec::as_slice);
        let index = bands
            .iter()
            .position(|b| i64::from(mood) <= b["max"].as_i64().unwrap_or(i64::MAX))
            .unwrap_or(bands.len().saturating_sub(1));
        (
            index,
            bands
                .get(index)
                .map_or("", |b| b["label"].as_str().unwrap_or("")),
        )
    }
}

/// Choices of the tab kept across rebuilds.
#[derive(Resource, Debug, Default)]
pub struct AssemblyUi {
    pub assembly: Option<String>,
    pub necessity: Option<String>,
    pub ask: Option<String>,
    pub form: Option<String>,
    pub permanent: bool,
    pub last: Option<LastOrder>,
}

/// The request the choices of the player describe.
struct Choice<'a> {
    row: &'a AssemblyRow,
    necessity: Option<&'a NecessityView>,
    ask: &'a AskView,
    form: Option<&'a FormView>,
    permanent: bool,
}

/// The stored choices where they still exist in the view, else the first of each list.
fn resolve<'a>(view: &'a AssemblyPanelView, ui: &AssemblyUi) -> Option<Choice<'a>> {
    let row = ui
        .assembly
        .as_deref()
        .and_then(|id| view.assemblies.iter().find(|a| a.id == id))
        .or_else(|| view.assemblies.first())?;
    let necessity = ui
        .necessity
        .as_deref()
        .and_then(|key| view.necessities.iter().find(|n| n.key == key))
        .or_else(|| view.necessities.first());
    let ask = ui
        .ask
        .as_deref()
        .and_then(|id| view.asks.iter().find(|a| a.id == id))
        .or_else(|| view.asks.first())?;
    let form = ui
        .form
        .as_deref()
        .and_then(|id| row.forms.iter().find(|f| f.id == id))
        .or_else(|| row.forms.first());
    let permanent = ui.permanent && form.is_some_and(|f| f.permanent_possible);
    Some(Choice {
        row,
        necessity,
        ask,
        form,
        permanent,
    })
}

fn forecast_of(
    data: &GameData,
    state: &CampaignState,
    player: &data_model::FactionId,
    choice: &Choice<'_>,
) -> Option<Result<ForecastView, String>> {
    let necessity = choice
        .necessity
        .and_then(|n| game_api::assembly::necessity_of(&n.key))?;
    Some(game_api::assembly::forecast_view(
        data,
        state,
        player,
        &choice.row.id,
        necessity,
        &choice.ask.id,
        choice.form.map(|f| f.id.as_str()),
        choice.permanent,
    ))
}

pub struct AssemblyPlugin;

impl Plugin for AssemblyPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AssemblyTexts::load())
            .init_resource::<AssemblyUi>()
            .register_dump(dump_assembly)
            .add_systems(Update, handle_buttons.run_if(in_state(AppState::Campaign)));
    }
}

fn record(
    panel: &mut FactionPanel,
    ui: &mut AssemblyUi,
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
    texts: Res<AssemblyTexts>,
    mut ui: ResMut<AssemblyUi>,
    mut panel: ResMut<FactionPanel>,
    mut sim: ResMut<Sim>,
) {
    for press in pressed.read() {
        let Some(name) = press.name.strip_prefix("faction:asm:") else {
            continue;
        };
        if !panel.open {
            continue;
        }
        if let Some(id) = name.strip_prefix("house:") {
            ui.assembly = Some(id.to_owned());
            panel.dirty = true;
            continue;
        }
        if let Some(key) = name.strip_prefix("necessity:") {
            ui.necessity = Some(key.to_owned());
            panel.dirty = true;
            continue;
        }
        if let Some(id) = name.strip_prefix("ask:") {
            ui.ask = Some(id.to_owned());
            panel.dirty = true;
            continue;
        }
        if let Some(id) = name.strip_prefix("form:") {
            ui.form = Some(id.to_owned());
            panel.dirty = true;
            continue;
        }
        if name == "permanent" {
            ui.permanent = !ui.permanent;
            panel.dirty = true;
            continue;
        }
        let sim = &mut *sim;
        let player = sim.player.clone();
        let (Some(data), Some(state)) = (&sim.data, &mut sim.state) else {
            continue;
        };
        let turn = state.turn;
        let Some(view) = game_api::assembly::assembly_panel(data, state, &player) else {
            continue;
        };
        if let Some(id) = name.strip_prefix("convene:") {
            let Some(choice) = resolve(&view, &ui).filter(|c| c.row.id == id) else {
                continue;
            };
            let Some(necessity) = choice
                .necessity
                .and_then(|n| game_api::assembly::necessity_of(&n.key))
            else {
                continue;
            };
            let order = Order::ConveneAssembly {
                assembly: id.to_owned(),
                necessity,
                ask: choice.ask.id.clone(),
                form: choice.form.map(|f| f.id.clone()),
                permanent: choice.permanent,
            };
            let assembly_name = choice.row.name.clone();
            let detail = format!("{id}:{}:{}", necessity.key(), choice.ask.id);
            let result = game_api::diplomacy::submit_order(data, state, order);
            let message = if result.ok {
                texts.f("convene.done", &[("name", assembly_name)])
            } else {
                texts.f("convene.refused", &[("reason", result.message)])
            };
            record(
                &mut panel, &mut ui, "convene", detail, result.ok, message, turn,
            );
        } else if let Some((accept, session)) = name
            .strip_prefix("accept:")
            .map(|s| (true, s))
            .or_else(|| name.strip_prefix("decline:").map(|s| (false, s)))
        {
            let Ok(session) = session.parse::<u32>() else {
                continue;
            };
            let result = game_api::diplomacy::submit_order(
                data,
                state,
                Order::AnswerAssembly {
                    session,
                    accept_terms: accept,
                },
            );
            let message = if !result.ok {
                texts.f("convene.refused", &[("reason", result.message)])
            } else if accept {
                texts.s("session.accepted").to_owned()
            } else {
                texts.s("session.declined").to_owned()
            };
            record(
                &mut panel,
                &mut ui,
                if accept { "accept" } else { "decline" },
                session.to_string(),
                result.ok,
                message,
                turn,
            );
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

/// A choice button: primary when selected.
fn choice_button(row: &mut Kids, theme: &UiTheme, name: &str, label: &str, selected: bool) {
    if selected {
        row.spawn(button_primary(theme, name, label));
    } else {
        row.spawn(button_small(theme, name, label));
    }
}

fn mood_bar(pane: &mut Kids, theme: &UiTheme, texts: &AssemblyTexts, row: &AssemblyRow) {
    let (band, label) = texts.mood_band(row.mood);
    let colour = [style::POOR, style::FAIR, style::GOOD, style::GOOD]
        .get(band)
        .copied()
        .unwrap_or(style::GOOD);
    pane.spawn((
        Name::new("faction:asm:mood"),
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(BAR_HEIGHT),
            ..default()
        },
        BackgroundColor(style::PARCHMENT_DARK.with_alpha(0.35)),
        Tooltip::new(&format!(
            "[b]{}[/b]\n{}",
            texts.s("mood.label"),
            texts.s("mood.tip")
        )),
    ))
    .with_children(|gauge| {
        gauge.spawn((
            Node {
                width: Val::Percent(row.mood.clamp(0, 100) as f32),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(colour),
        ));
    });
    pane.spawn(ink(
        theme,
        &format!(
            "{} : {}",
            texts.s("mood.label"),
            texts.f(
                "mood.value",
                &[("label", label.to_owned()), ("mood", row.mood.to_string())]
            )
        ),
        UiType::Body,
        colour,
    ));
}

fn scope_lines(pane: &mut Kids, theme: &UiTheme, texts: &AssemblyTexts, row: &AssemblyRow) {
    pane.spawn(wrapped(caption(
        theme,
        &texts.f(
            "scope.provinces",
            &[("count", row.covered_provinces.to_string())],
        ),
    )));
    if !row.orders.is_empty() {
        pane.spawn(wrapped(caption(
            theme,
            &texts.f("scope.orders", &[("orders", row.orders.join(", "))]),
        )));
    }
    let running = if row.running_per_season > 0 {
        texts.f(
            "scope.running",
            &[("amount", fr::amount(row.running_per_season))],
        )
    } else {
        texts.s("scope.running_none").to_owned()
    };
    pane.spawn(wrapped(caption(theme, &running)));
}

fn session_section(
    pane: &mut Kids,
    theme: &UiTheme,
    texts: &AssemblyTexts,
    row: &AssemblyRow,
    session: &game_api::assembly::SessionView,
) {
    pane.spawn(heading(theme, texts.s("session.title")));
    if session.status == "proposed" {
        pane.spawn(wrapped(ink(
            theme,
            &texts.f(
                "session.proposed",
                &[
                    ("amount", fr::amount(session.per_season)),
                    ("term", session.term.to_string()),
                    ("accord", session.accord.to_string()),
                ],
            ),
            UiType::Body,
            style::INK,
        )));
        if let Some(last) = session.answer_last_turn {
            pane.spawn(wrapped(caption(
                theme,
                &texts.f("session.answer_by", &[("season", season_label(last))]),
            )));
        }
        pane.spawn(ink(
            theme,
            texts.s("session.terms_title"),
            UiType::Caption,
            style::RUBRIC,
        ));
        for concession in &session.concessions {
            pane.spawn((
                wrapped(caption(
                    theme,
                    &texts.f(
                        "forecast.term",
                        &[
                            ("name", concession.name.clone()),
                            ("text", concession.text.clone()),
                        ],
                    ),
                )),
                Tooltip::new(&texts.f(
                    "forecast.term_tip",
                    &[("cost", concession.cost.to_string())],
                )),
            ));
        }
        pane.spawn(button_row()).with_children(|buttons| {
            buttons.spawn((
                button_primary(
                    theme,
                    &format!("faction:asm:accept:{}", session.id),
                    texts.s("session.accept"),
                ),
                Tooltip::new(texts.s("session.accept_tip")),
                FocusDefault,
            ));
            buttons.spawn((
                button_small(
                    theme,
                    &format!("faction:asm:decline:{}", session.id),
                    texts.s("session.decline"),
                ),
                Tooltip::new(texts.s("session.decline_tip")),
            ));
        });
    } else {
        pane.spawn(wrapped(ink(
            theme,
            &texts.f(
                "session.pending",
                &[
                    ("name", row.name.clone()),
                    ("necessity", session.necessity_name.clone()),
                    ("ask", session.ask_name.clone()),
                    ("form", session.form_name.clone()),
                    ("season", season_label(session.convened_turn)),
                ],
            ),
            UiType::Body,
            style::INK,
        )));
    }
}

#[allow(clippy::too_many_arguments)]
fn request_section(
    pane: &mut Kids,
    theme: &UiTheme,
    texts: &AssemblyTexts,
    view: &AssemblyPanelView,
    choice: &Choice<'_>,
    forecast: Option<&Result<ForecastView, String>>,
) {
    pane.spawn(heading(theme, texts.s("necessity.title")));
    if view.necessities.is_empty() {
        pane.spawn(wrapped(ink(
            theme,
            texts.s("necessity.none"),
            UiType::Caption,
            style::POOR,
        )));
    } else {
        let chosen = choice.necessity.map(|n| n.key.as_str());
        pane.spawn(button_row()).with_children(|buttons| {
            for necessity in &view.necessities {
                choice_button(
                    buttons,
                    theme,
                    &format!("faction:asm:necessity:{}", necessity.key),
                    &necessity.name,
                    chosen == Some(necessity.key.as_str()),
                );
            }
        });
    }
    pane.spawn(heading(theme, texts.s("ask.title")));
    pane.spawn(button_row()).with_children(|buttons| {
        for ask in &view.asks {
            choice_button(
                buttons,
                theme,
                &format!("faction:asm:ask:{}", ask.id),
                &texts.f(
                    "ask.label",
                    &[
                        ("name", ask.name.clone()),
                        ("term", ask.term_seasons.to_string()),
                    ],
                ),
                choice.ask.id == ask.id,
            );
        }
    });
    pane.spawn(heading(theme, texts.s("form.title")));
    pane.spawn(button_row()).with_children(|buttons| {
        for form in &choice.row.forms {
            choice_button(
                buttons,
                theme,
                &format!("faction:asm:form:{}", form.id),
                &form.name,
                choice.form.is_some_and(|f| f.id == form.id),
            );
        }
    });
    if let Some(form) = choice.form {
        let label = if choice.permanent {
            texts.s("form.permanent_on")
        } else {
            texts.s("form.permanent")
        };
        pane.spawn(button_row()).with_children(|buttons| {
            let mut button = if choice.permanent {
                buttons.spawn(button_primary(theme, "faction:asm:permanent", label))
            } else {
                buttons.spawn(button_small(theme, "faction:asm:permanent", label))
            };
            if form.permanent_possible {
                button.insert(Tooltip::new(texts.s("form.permanent_tip")));
            } else {
                button.insert((
                    InteractionDisabled,
                    Tooltip::new(texts.s("form.permanent_unavailable")),
                ));
            }
        });
    }
    pane.spawn(separator(theme));
    forecast_section(
        pane,
        theme,
        texts,
        forecast,
        (view.grant_threshold, view.conditional_threshold),
    );
}

fn forecast_section(
    pane: &mut Kids,
    theme: &UiTheme,
    texts: &AssemblyTexts,
    forecast: Option<&Result<ForecastView, String>>,
    (grant, conditional): (i32, i32),
) {
    pane.spawn((
        heading(theme, texts.s("forecast.title")),
        Tooltip::new(texts.s("forecast.tip")),
    ));
    let Some(forecast) = forecast else {
        return;
    };
    let forecast = match forecast {
        Ok(forecast) => forecast,
        Err(reason) => {
            pane.spawn(wrapped(ink(
                theme,
                &texts.f("forecast.error", &[("reason", reason.clone())]),
                UiType::Caption,
                style::POOR,
            )));
            return;
        }
    };
    let colour = match forecast.outcome.as_str() {
        "grant" => style::GOOD,
        "conditions" => style::FAIR,
        _ => style::POOR,
    };
    pane.spawn((
        Name::new("faction:asm:forecast"),
        wrapped(ink(
            theme,
            texts.s(&format!("forecast.outcome.{}", forecast.outcome)),
            UiType::Body,
            colour,
        )),
    ));
    pane.spawn(wrapped(caption(
        theme,
        &texts.f(
            "forecast.accord",
            &[
                ("accord", forecast.accord.to_string()),
                ("grant", grant.to_string()),
                ("conditional", conditional.to_string()),
            ],
        ),
    )));
    if forecast.per_season > 0 {
        pane.spawn(wrapped(caption(
            theme,
            &texts.f(
                "forecast.amount",
                &[
                    ("amount", fr::amount(forecast.per_season)),
                    ("term", forecast.term.to_string()),
                ],
            ),
        )));
    } else {
        pane.spawn(wrapped(caption(theme, texts.s("forecast.no_amount"))));
    }
    if !forecast.terms.is_empty() {
        pane.spawn(ink(
            theme,
            texts.s("forecast.terms_title"),
            UiType::Caption,
            style::RUBRIC,
        ));
        for term in &forecast.terms {
            pane.spawn((
                wrapped(caption(
                    theme,
                    &texts.f(
                        "forecast.term",
                        &[("name", term.name.clone()), ("text", term.text.clone())],
                    ),
                )),
                Tooltip::new(&texts.f("forecast.term_tip", &[("cost", term.cost.to_string())])),
            ));
        }
    }
    pane.spawn(ink(
        theme,
        texts.s("forecast.parts_title"),
        UiType::Caption,
        style::RUBRIC,
    ));
    for part in forecast.parts.iter().filter(|p| p.points != 0) {
        let label = texts.s(&format!("forecast.parts.{}", part.key));
        pane.spawn((
            Name::new(format!("faction:asm:part:{}", part.key)),
            ink(
                theme,
                &texts.f(
                    "forecast.part",
                    &[
                        ("label", label.to_owned()),
                        ("points", fr::signed(i64::from(part.points))),
                    ],
                ),
                UiType::Caption,
                fr::color_of(i64::from(part.points)),
            ),
        ));
    }
}

fn convene_button(
    pane: &mut Kids,
    theme: &UiTheme,
    texts: &AssemblyTexts,
    choice: &Choice<'_>,
    has_forecast: bool,
) {
    let name = format!("faction:asm:convene:{}", choice.row.id);
    let label = texts.f("convene.button", &[("name", choice.row.name.clone())]);
    let blocked = choice
        .row
        .blocked
        .clone()
        .or_else(|| (!has_forecast).then(|| texts.s("necessity.none").to_owned()));
    let mut button = pane.spawn(button_primary(theme, &name, &label));
    match blocked {
        None => {
            button.insert(Tooltip::new(texts.s("convene.tip")));
        }
        Some(reason) => {
            button.insert((InteractionDisabled, Tooltip::new(&reason)));
            pane.spawn(wrapped(ink(
                theme,
                &texts.f("convene.blocked", &[("reason", reason)]),
                UiType::Caption,
                style::POOR,
            )));
        }
    }
}

fn promises_section(pane: &mut Kids, theme: &UiTheme, texts: &AssemblyTexts, row: &AssemblyRow) {
    pane.spawn(heading(theme, texts.s("promises.title")));
    if row.promises.is_empty() {
        pane.spawn(wrapped(caption(theme, texts.s("promises.none"))));
        return;
    }
    for promise in &row.promises {
        let values = [
            ("name", promise.concession.name.clone()),
            ("until", season_label(promise.until)),
            (
                "deadline",
                promise.deadline.map(season_label).unwrap_or_default(),
            ),
        ];
        let path = if promise.deadline.is_some() {
            "promises.deadline"
        } else {
            "promises.line"
        };
        pane.spawn((
            Name::new(format!("faction:asm:promise:{}", promise.concession.id)),
            wrapped(ink(
                theme,
                &texts.f(path, &values),
                UiType::Body,
                style::INK,
            )),
            Tooltip::new(&texts.f("promises.tip", &[("text", promise.concession.text.clone())])),
        ));
    }
}

/// Fills the « États et Parlement » pane.
pub(super) fn assembly_tab(
    pane: &mut Kids,
    theme: &UiTheme,
    texts: &AssemblyTexts,
    ui: &AssemblyUi,
    data: &GameData,
    state: &CampaignState,
    player: &data_model::FactionId,
) {
    pane.spawn(heading(theme, texts.s("title")));
    let Some(view) = game_api::assembly::assembly_panel(data, state, player) else {
        pane.spawn(wrapped(caption(theme, texts.s("none"))));
        return;
    };
    pane.spawn(wrapped(caption(theme, texts.s("intro"))));
    let Some(choice) = resolve(&view, ui) else {
        return;
    };
    if view.assemblies.len() > 1 {
        pane.spawn(button_row()).with_children(|buttons| {
            for row in &view.assemblies {
                let mut button = if row.id == choice.row.id {
                    buttons.spawn(button_primary(
                        theme,
                        &format!("faction:asm:house:{}", row.id),
                        &row.name,
                    ))
                } else {
                    buttons.spawn(button_small(
                        theme,
                        &format!("faction:asm:house:{}", row.id),
                        &row.name,
                    ))
                };
                button.insert(Tooltip::new(texts.s("pick.tip")));
            }
        });
    }
    pane.spawn(separator(theme));
    pane.spawn(heading(theme, &choice.row.name));
    mood_bar(pane, theme, texts, choice.row);
    scope_lines(pane, theme, texts, choice.row);
    pane.spawn(separator(theme));
    if let Some(session) = &choice.row.session {
        session_section(pane, theme, texts, choice.row, session);
    } else {
        let forecast = forecast_of(data, state, player, &choice);
        request_section(pane, theme, texts, &view, &choice, forecast.as_ref());
        convene_button(pane, theme, texts, &choice, forecast.is_some());
    }
    pane.spawn(separator(theme));
    promises_section(pane, theme, texts, choice.row);
    if let Some(last) = &ui.last {
        pane.spawn(wrapped(ink(
            theme,
            &last.message,
            UiType::Caption,
            if last.ok { style::GOOD } else { style::POOR },
        )));
    }
}

fn dump_assembly(world: &World) -> (String, Value) {
    let panel = world.resource::<FactionPanel>();
    let ui = world.resource::<AssemblyUi>();
    let sim = world.resource::<Sim>();
    let mut buttons = Vec::new();
    if let Some(mut query) = world.try_query::<(&Name, Option<&InteractionDisabled>)>() {
        for (name, disabled) in query.iter(world) {
            let name = name.as_str();
            if name.starts_with("faction:asm:")
                && !name.contains(":part:")
                && !name.contains(":promise:")
                && !name.ends_with(":mood")
                && !name.ends_with(":forecast")
            {
                buttons.push((name.to_owned(), disabled.is_none()));
            }
        }
    }
    buttons.sort();
    let mut body = json!({
        "open": panel.open && panel.tab == TAB,
        "tab": panel.tab,
        "buttons": buttons.iter().map(|(name, enabled)| json!({"name": name, "enabled": enabled})).collect::<Vec<_>>(),
        "last_order": ui.last.as_ref().map(|o| json!({
            "kind": o.kind, "detail": o.detail, "ok": o.ok, "message": o.message, "turn": o.turn,
        })),
    });
    if let (Some(data), Some(state)) = (&sim.data, &sim.state)
        && let Some(view) = game_api::assembly::assembly_panel(data, state, &sim.player)
    {
        let choice = resolve(&view, ui);
        let forecast = choice
            .as_ref()
            .and_then(|c| forecast_of(data, state, &sim.player, c));
        body["assemblies"] = json!(
            view.assemblies
                .iter()
                .map(|a| json!({
                    "id": a.id, "mood": a.mood, "covered_provinces": a.covered_provinces,
                    "blocked": a.blocked, "running_per_season": a.running_per_season,
                    "forms": a.forms.iter().map(|f| json!({"id": f.id, "permanent_possible": f.permanent_possible})).collect::<Vec<_>>(),
                    "session": a.session.as_ref().map(|s| json!({
                        "id": s.id, "status": s.status, "necessity": s.necessity, "accord": s.accord,
                        "per_season": s.per_season, "concessions": s.concessions.iter().map(|c| c.id.clone()).collect::<Vec<_>>(),
                    })),
                    "promises": a.promises.iter().map(|p| p.concession.id.clone()).collect::<Vec<_>>(),
                }))
                .collect::<Vec<_>>()
        );
        body["necessities"] = json!(
            view.necessities
                .iter()
                .map(|n| n.key.clone())
                .collect::<Vec<_>>()
        );
        body["asks"] = json!(view.asks.iter().map(|a| a.id.clone()).collect::<Vec<_>>());
        body["selected"] = json!(choice.as_ref().map(|c| json!({
            "assembly": c.row.id,
            "necessity": c.necessity.map(|n| n.key.clone()),
            "ask": c.ask.id,
            "form": c.form.map(|f| f.id.clone()),
            "permanent": c.permanent,
        })));
        body["forecast"] = match forecast {
            Some(Ok(f)) => json!({
                "outcome": f.outcome, "accord": f.accord, "per_season": f.per_season, "term": f.term,
                "terms": f.terms.iter().map(|t| t.id.clone()).collect::<Vec<_>>(),
                "parts": f.parts.iter().map(|p| json!({"key": p.key, "points": p.points})).collect::<Vec<_>>(),
            }),
            Some(Err(reason)) => json!({"error": reason}),
            None => Value::Null,
        };
    }
    ("assembly_panel".into(), body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::panels::faction::TABS;

    #[test]
    fn the_texts_parse_and_cover_every_part_of_the_agreement() {
        let texts = AssemblyTexts::load();
        assert!(!texts.s("title").is_empty());
        for key in [
            "assembly_mood",
            "assembly_prestige",
            "assembly_necessity",
            "assembly_ask",
            "assembly_subsidies",
            "assembly_unrest",
            "assembly_broken",
            "assembly_permanent",
        ] {
            assert!(
                !texts.s(&format!("forecast.parts.{key}")).is_empty(),
                "{key}"
            );
        }
        assert_eq!(texts.mood_band(10).1, "Hostile");
        assert_eq!(texts.mood_band(100).0, 3);
        assert_eq!(texts.s("finance.button"), "Convoquer les États");
    }

    #[test]
    fn the_tab_is_registered_with_the_panel() {
        assert!(TABS.contains(&TAB));
    }
}
