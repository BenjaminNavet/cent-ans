//! L'Audience de saison (XP-AUDIENCE, R1): the single channel that speaks to the player.
//!
//! `game_api::agenda` ranks every source (bell alerts, chronicle decisions, mission offer, the
//! ambitions to choose, the treasurer's balance) into at most three Affairs, a short « À
//! surveiller » list and folded news. This module only draws it: a card with the councillors'
//! portraits at the start of a season (non-blocking, folds into the Registre), the Registre
//! (watch list + folded news) behind the bell's counter, and the buttons that send the player to
//! the right panel. No rule and no text here: the lines come from `data/ui/audience.fr.json`.

use std::collections::BTreeSet;

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use game_api::agenda::{Affair, Agenda, AgendaAction};
use serde_json::{Value, json};

use super::attention::AttentionLog;
use super::decision::OpenDecision;
use super::flow::TurnFlow;
use super::missions::OpenMissionOffer;
use super::{AdvisorColumn, HudData, HudSet, HudWidget};
use crate::agent::dump::DumpAppExt;
use crate::campaign::armies::SelectedArmy;
use crate::campaign::panels::OpenPanel;
use crate::campaign::panels::chronicle::CausesRequest;
use crate::campaign::panels::court::portrait::{self, PortraitBank};
use crate::campaign::progress::OpenObjectives;
use crate::campaign::progress::objectives::ObjectivesPanel;
use crate::campaign::provinces::picker::SelectProvince;
use crate::sim::Sim;
use crate::ui::kit::style::{self, UiType};
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::widgets::{button_small, caption, panel_note};
use crate::ui::kit::{ButtonPressed, UiTheme};
use sim_campaign::ArmyId;

const CARD_WIDTH: f32 = 340.0;
const PORTRAIT: f32 = 52.0;

/// Name of the toggle that the bell's counter pill sends.
pub const REGISTRY_TOGGLE: &str = "hud:alert:registry";
const FOLD: &str = "audience:fold";
const OPEN: &str = "audience:open";
const REGISTRY: &str = "audience:registry";
const ACT: &str = "audience:act:";
/// Stable name of the button of the blocking Affair (scripts, smoke tests).
pub const DECIDE: &str = "audience:decide";
const WHY: &str = "audience:why:";

/// How much of the Audience is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AudienceView {
    /// The Affairs, with portraits (pushed once per season and per new Affair).
    Card,
    /// The Affairs, the watch list and the folded news.
    Registry,
    /// One line.
    #[default]
    Folded,
}

impl AudienceView {
    fn key(self) -> &'static str {
        match self {
            AudienceView::Card => "card",
            AudienceView::Registry => "registry",
            AudienceView::Folded => "folded",
        }
    }
}

#[derive(Resource, Default)]
pub struct Audience {
    pub agenda: Option<Agenda>,
    pub view: AudienceView,
    /// Cards pushed in front of the player so far.
    pub pushes: u32,
    /// Seconds the card has been open.
    card_age: f32,
    seen_turn: Option<u32>,
    seen: BTreeSet<String>,
    built_revision: Option<u64>,
    /// Last button handled, for the dump.
    pub last_action: String,
}

impl Audience {
    /// Affairs and watch lines: what the bell's counter shows.
    pub fn count(&self) -> usize {
        self.agenda
            .as_ref()
            .map_or(0, |a| a.affairs.len() + a.watch.len())
    }
}

#[derive(Component)]
struct AudienceRoot;

pub fn plugin(app: &mut App) {
    app.init_resource::<Audience>()
        .register_dump(dump)
        .add_systems(
            Update,
            (refresh, auto_fold, handle_buttons, rebuild, seal_tooltip)
                .chain()
                .in_set(HudSet::Ui),
        );
}

/// Recomputes the agenda after each refresh of the HUD views; pushes the card when the season
/// is new or when an Affair appears that the player has not seen.
fn refresh(
    hud: Res<HudData>,
    sim: Res<Sim>,
    flow: Res<TurnFlow>,
    mut audience: ResMut<Audience>,
    mut log: ResMut<AttentionLog>,
) {
    if audience.built_revision == Some(hud.revision) {
        return;
    }
    let (Some(data), Some(state)) = (sim.data.as_ref(), sim.state.as_ref()) else {
        return;
    };
    audience.built_revision = Some(hud.revision);
    let agenda = game_api::agenda(data, state, &sim.player, &flow.last_events);
    let new_season = audience.seen_turn != Some(agenda.turn);
    if new_season {
        audience.seen_turn = Some(agenda.turn);
        audience.seen.clear();
    }
    let fresh = agenda
        .affairs
        .iter()
        .any(|affair| !audience.seen.contains(&affair.id));
    audience
        .seen
        .extend(agenda.affairs.iter().map(|a| a.id.clone()));
    if fresh {
        audience.view = AudienceView::Card;
        audience.card_age = 0.0;
        audience.pushes += 1;
        log.audience_cards += 1;
    } else if agenda.affairs.is_empty() && audience.view == AudienceView::Card {
        audience.view = AudienceView::Folded;
    }
    if audience.agenda.as_ref() != Some(&agenda) {
        audience.agenda = Some(agenda);
    }
}

/// The card folds into the Registre counter after `audience_seconds` (non-blocking Audience).
fn auto_fold(time: Res<Time>, sim: Res<Sim>, mut audience: ResMut<Audience>) {
    if audience.view != AudienceView::Card {
        return;
    }
    audience.card_age += time.delta_secs();
    let seconds = sim
        .data
        .as_ref()
        .map_or(25.0, |data| data.agenda_rules.audience_seconds);
    if audience.card_age >= seconds {
        audience.view = AudienceView::Folded;
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_buttons(
    mut pressed: MessageReader<ButtonPressed>,
    mut audience: ResMut<Audience>,
    mut panels: MessageWriter<OpenPanel>,
    mut select_province: MessageWriter<SelectProvince>,
    mut decisions: MessageWriter<OpenDecision>,
    mut offers: MessageWriter<OpenMissionOffer>,
    mut objectives: MessageWriter<OpenObjectives>,
    objectives_panel: Res<ObjectivesPanel>,
    mut selected_army: ResMut<SelectedArmy>,
    mut causes: ResMut<CausesRequest>,
) {
    for press in pressed.read() {
        let name = press.name.as_str();
        if name == FOLD {
            audience.view = AudienceView::Folded;
        } else if name == OPEN {
            audience.view = AudienceView::Card;
            audience.card_age = 0.0;
        } else if name == REGISTRY || name == REGISTRY_TOGGLE {
            audience.view = if audience.view == AudienceView::Registry {
                AudienceView::Folded
            } else {
                AudienceView::Registry
            };
        } else if let Some(id) = name.strip_prefix(WHY) {
            if let Some(cause) = audience
                .agenda
                .as_ref()
                .and_then(|a| a.affairs.iter().find(|x| x.id == id))
                .and_then(|x| x.cause_id)
            {
                causes.0 = Some(Some(cause));
                audience.last_action = format!("why:{id}");
            }
        } else if let Some(id) = (name == DECIDE)
            .then(|| {
                audience
                    .agenda
                    .as_ref()
                    .and_then(|a| a.affairs.iter().find(|x| x.blocking))
                    .map(|x| x.id.clone())
            })
            .flatten()
            .or_else(|| name.strip_prefix(ACT).map(str::to_owned))
        {
            let Some(action) = audience.agenda.as_ref().and_then(|a| {
                a.affairs
                    .iter()
                    .map(|x| (&x.id, &x.action))
                    .chain(a.watch.iter().map(|x| (&x.id, &x.action)))
                    .find(|(found, _)| found.as_str() == id.as_str())
                    .map(|(_, action)| action.clone())
            }) else {
                continue;
            };
            audience.last_action = format!("{}:{id}", action.kind);
            match action.kind.as_str() {
                "open_chronicle" => {
                    match id.strip_prefix("chronicle:").and_then(|n| n.parse().ok()) {
                        Some(decision) => {
                            decisions.write(OpenDecision::Id(decision));
                        }
                        None => {
                            panels.write(OpenPanel::Chronicle);
                        }
                    }
                }
                "open_offer" => {
                    offers.write(OpenMissionOffer);
                }
                "open_ambitions" | "open_objectives" => {
                    if !objectives_panel.open {
                        objectives.write(OpenObjectives);
                    }
                }
                "center_province" => {
                    if let Some(province) = action.target {
                        select_province.write(SelectProvince(Some(province.clone())));
                        panels.write(OpenPanel::Province(province));
                    }
                }
                "center_army" => centre_army(
                    &action,
                    &mut panels,
                    &mut select_province,
                    &mut selected_army,
                ),
                "open_finance" => {
                    panels.write(OpenPanel::Economy);
                }
                "open_diplomacy" => {
                    panels.write(OpenPanel::Diplomacy);
                }
                "open_research" => {
                    panels.write(OpenPanel::Tech);
                }
                _ => {}
            }
        }
    }
}

/// The seal of the end of the season says which Affair blocks it (XP-AUDIENCE).
fn seal_tooltip(
    audience: Res<Audience>,
    mut seal: Query<&mut Tooltip, With<super::EndTurnButton>>,
    mut base: Local<Option<String>>,
) {
    let Ok(mut tip) = seal.single_mut() else {
        return;
    };
    let base = base.get_or_insert_with(|| tip.bbcode.clone()).clone();
    let want = match audience.agenda.as_ref().and_then(|a| a.blocking.as_ref()) {
        Some(blocking) => format!("{base}\n[b]{blocking}[/b]"),
        None => base,
    };
    if tip.bbcode != want {
        tip.bbcode = want;
    }
}

fn centre_army(
    action: &AgendaAction,
    panels: &mut MessageWriter<OpenPanel>,
    select_province: &mut MessageWriter<SelectProvince>,
    selected_army: &mut SelectedArmy,
) {
    let Some(target) = action.target.clone() else {
        return;
    };
    if action.target_kind == "army" {
        if let Some(id) = ArmyId::parse(&target) {
            selected_army.0 = Some(id);
        }
        panels.write(OpenPanel::Army(target));
    } else {
        select_province.write(SelectProvince(Some(target.clone())));
        panels.write(OpenPanel::Province(target));
    }
}

#[allow(clippy::too_many_arguments)]
fn rebuild(
    mut commands: Commands,
    audience: Res<Audience>,
    sim: Res<Sim>,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
    bank: Option<Res<PortraitBank>>,
    column: Query<Entity, With<AdvisorColumn>>,
    existing: Query<Entity, With<AudienceRoot>>,
    mut built: Local<Option<(Option<Agenda>, AudienceView)>>,
) {
    let Ok(column) = column.single() else { return };
    let want = (audience.agenda.clone(), audience.view);
    let existing = existing.single().ok();
    if built.as_ref() == Some(&want) && existing.is_some() {
        return;
    }
    if let Some(entity) = existing {
        commands.entity(entity).despawn();
    }
    *built = Some(want.clone());
    let (Some(agenda), view) = want else { return };
    let bank = bank.as_deref().cloned().unwrap_or_default();
    let root = commands
        .spawn((
            Name::new("hud:audience"),
            AudienceRoot,
            ChildOf(column),
            HudWidget,
            Hovered::default(),
            panel_note(&theme),
        ))
        .insert(Node {
            width: Val::Px(CARD_WIDTH),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            padding: UiRect::all(Val::Px(6.0)),
            ..default()
        })
        .id();
    commands.entity(root).with_children(|card| {
        if view == AudienceView::Folded {
            let count = agenda.affairs.len();
            card.spawn(button_small(
                &theme,
                OPEN,
                &format!("▸ {} ({count})", agenda.title),
            ));
            return;
        }
        card.spawn(Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            column_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|head| {
            head.spawn((
                Text::new(agenda.title.clone()),
                theme.text_font(true, false, UiType::Body.size()),
                TextColor(style::INK_TITLE),
            ));
            head.spawn(Node {
                column_gap: Val::Px(4.0),
                ..default()
            })
            .with_children(|buttons| {
                buttons.spawn(button_small(
                    &theme,
                    REGISTRY,
                    &game_api::agenda::ui_text("open_registry"),
                ));
                buttons.spawn(button_small(
                    &theme,
                    FOLD,
                    &game_api::agenda::ui_text("close"),
                ));
            });
        });
        registry_body(
            card,
            &theme,
            &assets,
            &bank,
            &sim,
            &agenda,
            view == AudienceView::Registry,
            CARD_WIDTH,
        );
    });
}

/// The Affairs and, with `lists`, the watch list and the folded news: the body of the Registre,
/// drawn in the advisor column and as the « Registre » tab of the Chronique book (XP-R3).
#[allow(clippy::too_many_arguments)]
pub fn registry_body(
    card: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    assets: &AssetServer,
    bank: &PortraitBank,
    sim: &Sim,
    agenda: &Agenda,
    lists: bool,
    width: f32,
) {
    if agenda.affairs.is_empty() {
        card.spawn((
            caption(theme, &agenda.quiet),
            Node {
                width: Val::Px(width - 16.0),
                ..default()
            },
        ));
    }
    for affair in &agenda.affairs {
        spawn_affair(card, theme, assets, bank, sim, affair);
    }
    if lists {
        card.spawn((
            Text::new(agenda.watch_heading.clone()),
            theme.text_font(true, false, UiType::Body.size()),
            TextColor(style::INK_TITLE),
        ));
        for item in &agenda.watch {
            card.spawn(Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                column_gap: Val::Px(6.0),
                ..default()
            })
            .with_children(|row| {
                row.spawn((
                    caption(theme, &item.text),
                    Node {
                        width: Val::Px(width - 110.0),
                        ..default()
                    },
                ));
                if !item.action.label.is_empty() && item.action.kind != "none" {
                    row.spawn(button_small(
                        theme,
                        &format!("{ACT}{}", item.id),
                        &item.action.label,
                    ));
                }
            });
        }
        card.spawn((
            Text::new(agenda.news_heading.clone()),
            theme.text_font(true, false, UiType::Body.size()),
            TextColor(style::INK_TITLE),
        ));
        card.spawn((
            caption(theme, &agenda.news_summary),
            Node {
                width: Val::Px(width - 16.0),
                ..default()
            },
        ));
    }
}

fn spawn_affair(
    card: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    assets: &AssetServer,
    bank: &PortraitBank,
    sim: &Sim,
    affair: &Affair,
) {
    card.spawn((
        Name::new(format!("audience:affair:{}", affair.id)),
        Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(6.0),
            align_items: AlignItems::FlexStart,
            ..default()
        },
    ))
    .with_children(|row| {
        let sheet = match (&sim.data, &sim.state) {
            (Some(data), Some(state)) if !affair.speaker.character_id.is_empty() => {
                data_model::CharacterId::new(affair.speaker.character_id.clone())
                    .ok()
                    .and_then(|id| game_api::court::character_sheet(data, state, &id))
            }
            _ => None,
        };
        match &sheet {
            Some(sheet) => {
                portrait::spawn_portrait(
                    row,
                    theme,
                    assets,
                    bank,
                    &crate::campaign::panels::court::sheet_subject(sheet),
                    PORTRAIT,
                );
            }
            None => {
                row.spawn(Node {
                    width: Val::Px(PORTRAIT),
                    height: Val::Px(PORTRAIT),
                    flex_shrink: 0.0,
                    ..default()
                });
            }
        }
        row.spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(2.0),
            width: Val::Px(CARD_WIDTH - PORTRAIT - 28.0),
            ..default()
        })
        .with_children(|col| {
            col.spawn((
                Text::new(format!(
                    "{}, {}",
                    affair.speaker.name,
                    affair.speaker.role.to_lowercase()
                )),
                theme.text_font(true, false, UiType::Caption.size()),
                TextColor(style::INK_TITLE),
            ));
            col.spawn(caption(theme, &affair.text));
            col.spawn(Node {
                column_gap: Val::Px(4.0),
                ..default()
            })
            .with_children(|buttons| {
                if affair.action.kind != "none" && !affair.action.label.is_empty() {
                    let name = if affair.blocking {
                        DECIDE.to_owned()
                    } else {
                        format!("{ACT}{}", affair.id)
                    };
                    buttons.spawn(button_small(theme, &name, &affair.action.label));
                }
                if affair.cause_id.is_some() {
                    buttons.spawn((
                        button_small(
                            theme,
                            &format!("{WHY}{}", affair.id),
                            &game_api::agenda::ui_text("why_button"),
                        ),
                        Tooltip::new(&game_api::causes::ui_text("open_causes_button")),
                    ));
                }
            });
        });
    });
}

/// Dump section `agenda`.
fn dump(world: &World) -> (String, Value) {
    let audience = world.resource::<Audience>();
    let value = audience.agenda.as_ref().map_or(Value::Null, |a| {
        json!({
            "turn": a.turn,
            "title": a.title,
            "affairs": a.affairs.iter().map(|x| json!({
                "id": x.id, "kind": x.kind, "speaker": x.speaker.name, "role": x.speaker.role,
                "text": x.text, "action": x.action.kind, "button": x.action.label,
                "blocking": x.blocking, "cause_id": x.cause_id,
            })).collect::<Vec<_>>(),
            "watch": a.watch.iter().map(|x| json!({
                "id": x.id, "kind": x.kind, "text": x.text,
            })).collect::<Vec<_>>(),
            "news": {
                "total": a.news.total, "letters": a.news.letters, "world": a.news.world,
                "overflow": a.news.overflow, "summary": a.news_summary,
            },
            "blocking": a.blocking,
        })
    });
    (
        "agenda".to_owned(),
        json!({
            "agenda": value,
            "view": audience.view.key(),
            "pushes": audience.pushes,
            "last_action": audience.last_action,
            "count": audience.count(),
        }),
    )
}
