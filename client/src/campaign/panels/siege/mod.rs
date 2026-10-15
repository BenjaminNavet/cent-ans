//! Siege panel: state of a siege (supplies, breach, walls), garrison, besieging armies, engines
//! built on the spot, and the actions: assault, summon, sortie, lift the siege, wait.
//!
//! Ports: game/scripts/map/siege_controller.gd (status line, engines line, « Donner l'assaut »,
//! « Sommer la garnison »), extended to the whole siege view (garrison, besiegers) and to the
//! defender's sortie. Data view: `game_api::siege`; every order goes through
//! `CampaignState::submit_order_outcome` exactly as `godot-bridge` does (`assault`,
//! `demand_surrender`, `sortie`, `set_stance`). No rule lives here.
//!
//! Opens on `OpenPanel::Siege(settlement or province id)`; closes with Escape, « Fermer »,
//! `CloseAll` and the central screens. « Attendre » closes the panel without an order: the
//! siege goes on at the end of the turn. Dev hooks: `CENT_ANS_OPEN_SIEGE=<id>` opens it on the
//! first campaign frame, `CENT_ANS_STAGE_SIEGE=<id>` first puts the player's strongest army in
//! siege of that settlement/province (script proofs only).
//! Button names: `siege:close|assault|summon|sortie|lift|wait`.

mod view;

use bevy::prelude::*;
use game_api::siege::{SiegeRole, SiegeSheet};
use serde_json::{Value, json};
use sim_campaign::{ArmyId, ArmyPosition, Order, Place, SiegeState, Stance};

use super::OpenPanel;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::menu::KeyEdges;
use crate::sim::Sim;
use crate::ui::kit::ButtonPressed;
use crate::ui::kit::modal::ToastRequest;

const ENV_OPEN: &str = "CENT_ANS_OPEN_SIEGE";
const ENV_STAGE: &str = "CENT_ANS_STAGE_SIEGE";

/// Root of the siege panel tree.
#[derive(Component)]
pub struct SiegePanelRoot;

/// Last order the panel submitted.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OrderRecord {
    pub kind: String,
    pub ok: bool,
    pub error: String,
}

/// What the panel remembers between rebuilds.
#[derive(Resource, Default)]
pub struct SiegePanel {
    /// Settlement or province id the panel was opened on.
    pub place: Option<String>,
    pub dirty: bool,
    pub issued: u32,
    pub accepted: u32,
    pub last: Option<OrderRecord>,
    pub error: String,
    summary: Summary,
}

/// Figures shown at the last rebuild (dump).
#[derive(Default, Clone)]
struct Summary {
    settlement: String,
    role: String,
    supplies: u8,
    breach: u8,
    turns_left: u32,
    walls_stand: bool,
    garrison_regiments: usize,
    garrison_men: u32,
    besiegers: usize,
    besieger_men: u32,
    engines: Vec<(String, bool, u32)>,
    assault_odds: Option<u32>,
    assault_blocker: String,
    surrender_chance: u32,
    actions: Vec<&'static str>,
}

pub struct PanelSiegePlugin;

impl Plugin for PanelSiegePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SiegePanel>()
            .register_dump(dump_panel)
            .add_systems(
                Update,
                (intake, escape, handle_buttons, rebuild)
                    .chain()
                    .before(crate::menu::pause::pause_escape)
                    .run_if(in_state(AppState::Campaign)),
            )
            .add_systems(OnExit(AppState::Campaign), reset);
    }
}

fn reset(mut panel: ResMut<SiegePanel>) {
    *panel = SiegePanel::default();
}

// ---- opening -----------------------------------------------------------------------------

fn open(panel: &mut SiegePanel, place: &str) {
    if panel.place.as_deref() != Some(place) {
        panel.place = Some(place.to_owned());
        panel.error.clear();
    }
    panel.dirty = true;
}

fn close(panel: &mut SiegePanel) {
    if panel.place.take().is_some() {
        panel.dirty = true;
    }
    panel.error.clear();
}

/// Dev only: the player's most numerous army besieges `place` (no war check, no battle).
fn stage_siege(sim: &mut Sim, place: &str) {
    let Some(state) = sim.state.as_mut() else {
        return;
    };
    let target = game_api::siege::besieged_settlement(state, place)
        .or_else(|| game_api::recruit::settlement_or_city(state, place));
    let player = state.player_faction().clone();
    let army = state
        .armies
        .iter()
        .filter(|(_, a)| a.faction == player)
        .max_by_key(|(_, a)| a.units.len())
        .map(|(id, _)| id.clone());
    let (Some(target), Some(army)) = (target, army) else {
        warn!("{ENV_STAGE}: no settlement or army for `{place}`");
        return;
    };
    if let Some(a) = state.armies.get_mut(&army) {
        a.position = ArmyPosition::Settlement(target.clone());
        a.clear_plan();
        a.stance = Stance::Siege;
    }
    if let Some(s) = state.settlements.get_mut(&target)
        && s.siege.is_none()
    {
        s.siege = Some(SiegeState {
            attacker: player,
            turns_left: 4,
            turns_elapsed: 1,
            supplies: 55,
            breach: 20,
            started_turn: state.turn,
            engine_work: 0,
        });
    }
}

fn intake(
    mut messages: MessageReader<OpenPanel>,
    mut sim: ResMut<Sim>,
    mut panel: ResMut<SiegePanel>,
    mut hook: Local<bool>,
) {
    if sim.state.is_none() {
        return;
    }
    if !*hook {
        *hook = true;
        if let Ok(place) = std::env::var(ENV_STAGE) {
            stage_siege(&mut sim, &place);
        }
        if let Ok(place) = std::env::var(ENV_OPEN) {
            open(&mut panel, &place);
        }
    }
    for message in messages.read() {
        match message {
            OpenPanel::Siege(place) => open(&mut panel, place),
            // Docked panels of the map keep company with the siege panel.
            OpenPanel::Province(_) | OpenPanel::Army(_) | OpenPanel::Recruitment(_) => {}
            _ => close(&mut panel),
        }
    }
}

fn escape(mut edges: ResMut<KeyEdges>, mut panel: ResMut<SiegePanel>) {
    if edges.escape && panel.place.is_some() {
        edges.escape = false;
        close(&mut panel);
    }
}

// ---- orders ------------------------------------------------------------------------------

fn submit(sim: &mut Sim, order: Order) -> Result<String, String> {
    let data = sim.data.clone().ok_or("données non chargées")?;
    let state = sim.state.as_mut().ok_or("aucune campagne en cours")?;
    state
        .submit_order_outcome(&data, order)
        .map_err(|e| e.to_string())?;
    Ok(state
        .pending_events
        .last()
        .map(|e| e.text_fr.clone())
        .unwrap_or_default())
}

fn order(
    panel: &mut SiegePanel,
    sim: &mut Sim,
    toasts: &mut MessageWriter<ToastRequest>,
    kind: &str,
    order: Order,
    default_toast: &str,
) -> bool {
    panel.issued += 1;
    let result = submit(sim, order);
    panel.last = Some(OrderRecord {
        kind: kind.to_owned(),
        ok: result.is_ok(),
        error: result.clone().err().unwrap_or_default(),
    });
    panel.dirty = true;
    match result {
        Ok(text) => {
            panel.accepted += 1;
            panel.error.clear();
            toasts.write(ToastRequest::new(if text.is_empty() {
                default_toast
            } else {
                &text
            }));
            true
        }
        Err(message) => {
            panel.error = message;
            false
        }
    }
}

fn current_sheet(sim: &Sim, panel: &SiegePanel) -> Option<SiegeSheet> {
    game_api::siege_sheet(
        sim.data.as_deref()?,
        sim.state.as_ref()?,
        panel.place.as_deref()?,
    )
}

fn handle_buttons(
    mut pressed: MessageReader<ButtonPressed>,
    mut sim: ResMut<Sim>,
    mut panel: ResMut<SiegePanel>,
    mut toasts: MessageWriter<ToastRequest>,
) {
    for press in pressed.read() {
        let Some(action) = press.name.strip_prefix("siege:") else {
            continue;
        };
        if panel.place.is_none() {
            continue;
        }
        match action {
            // « Attendre » = no order: the siege goes on when the turn ends.
            "close" | "wait" => close(&mut panel),
            "assault" | "summon" | "sortie" | "lift" => {
                let Some(sheet) = current_sheet(&sim, &panel) else {
                    continue;
                };
                action_order(action, &sheet, &mut panel, &mut sim, &mut toasts);
            }
            _ => {}
        }
    }
}

fn action_order(
    action: &str,
    sheet: &SiegeSheet,
    panel: &mut SiegePanel,
    sim: &mut Sim,
    toasts: &mut MessageWriter<ToastRequest>,
) {
    let Ok(settlement) = data_model::SettlementId::new(sheet.settlement.as_str()) else {
        return;
    };
    match action {
        "assault" => {
            let Some(army) = sheet.assault_army.as_deref().and_then(ArmyId::parse) else {
                return;
            };
            order(
                panel,
                sim,
                toasts,
                "assault",
                Order::Assault { army },
                "L'assaut est donné.",
            );
        }
        "summon" => {
            order(
                panel,
                sim,
                toasts,
                "demand_surrender",
                Order::DemandSurrender {
                    settlement: Place::Settlement(settlement),
                },
                "La sommation est faite.",
            );
        }
        "sortie" => {
            order(
                panel,
                sim,
                toasts,
                "sortie",
                Order::Sortie {
                    settlement: Place::Settlement(settlement),
                },
                "La garnison fait une sortie.",
            );
        }
        "lift" => {
            let mut lifted = 0;
            for army in sheet
                .liftable_armies
                .iter()
                .filter_map(|a| ArmyId::parse(a))
            {
                let done = order(
                    panel,
                    sim,
                    toasts,
                    "set_stance",
                    Order::SetStance {
                        army,
                        stance: Stance::Normal,
                    },
                    "Le siège sera levé à la fin du tour.",
                );
                lifted += usize::from(done);
            }
            if lifted == 0 && panel.error.is_empty() {
                panel.error = "Aucune armée ne tient ce siège.".to_owned();
            }
        }
        _ => {}
    }
}

// ---- building ----------------------------------------------------------------------------

fn rebuild(
    mut commands: Commands,
    sim: Res<Sim>,
    theme: Res<crate::ui::kit::UiTheme>,
    mut panel: ResMut<SiegePanel>,
    roots: Query<Entity, With<SiegePanelRoot>>,
) {
    let open = panel.place.is_some();
    if !(panel.dirty || (sim.is_changed() && (open || !roots.is_empty()))) {
        return;
    }
    panel.dirty = false;
    for root in &roots {
        commands.entity(root).despawn();
    }
    panel.summary = Summary::default();
    if !open {
        return;
    }
    match current_sheet(&sim, &panel) {
        Some(sheet) => {
            panel.summary = summarize(&sheet);
            view::spawn_siege_panel(&mut commands, &theme, &sheet, &panel.error);
        }
        None => {
            // The siege ended (taken, lifted, abandoned): say so once, then close.
            let name = panel.place.clone().unwrap_or_default();
            view::spawn_ended_panel(&mut commands, &theme, &name);
            panel.summary.role = "ended".to_owned();
        }
    }
}

fn summarize(sheet: &SiegeSheet) -> Summary {
    let attacker = sheet.role == SiegeRole::Attacker;
    let mut actions = vec!["wait"];
    if attacker {
        actions.push("assault");
        actions.push("summon");
        if !sheet.liftable_armies.is_empty() {
            actions.push("lift");
        }
    }
    if sheet.can_sortie {
        actions.push("sortie");
    }
    Summary {
        settlement: sheet.settlement.clone(),
        role: serde_json::to_value(sheet.role)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default(),
        supplies: sheet.supplies,
        breach: sheet.breach,
        turns_left: sheet.turns_left,
        walls_stand: sheet.walls_stand,
        garrison_regiments: sheet.garrison.len(),
        garrison_men: sheet.garrison_men,
        besiegers: sheet.besiegers.len(),
        besieger_men: sheet.besieger_men,
        engines: sheet
            .engines
            .iter()
            .map(|e| (e.id.clone(), e.ready, e.turns_left))
            .collect(),
        assault_odds: sheet.assault_odds,
        assault_blocker: sheet.assault_blocker.clone(),
        surrender_chance: sheet.surrender_chance,
        actions,
    }
}

// ---- dump --------------------------------------------------------------------------------

fn dump_panel(world: &World) -> (String, Value) {
    let panel = world.resource::<SiegePanel>();
    let s = &panel.summary;
    let value = json!({
        "open": panel.place,
        "settlement": s.settlement,
        "role": s.role,
        "supplies": s.supplies,
        "breach": s.breach,
        "turns_left": s.turns_left,
        "walls_stand": s.walls_stand,
        "garrison": { "regiments": s.garrison_regiments, "men": s.garrison_men },
        "besiegers": { "armies": s.besiegers, "men": s.besieger_men },
        "engines": s.engines.iter().map(|(id, ready, left)| json!({"id": id, "ready": ready, "turns_left": left})).collect::<Vec<_>>(),
        "assault_odds": s.assault_odds,
        "assault_blocker": s.assault_blocker,
        "surrender_chance": s.surrender_chance,
        "actions": s.actions,
        "orders": {
            "issued": panel.issued, "accepted": panel.accepted,
            "last": panel.last.as_ref().map(|l| json!({"kind": l.kind, "ok": l.ok, "error": l.error})),
        },
        "error": panel.error,
    });
    ("panel_siege".to_owned(), value)
}
