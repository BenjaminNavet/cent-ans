//! War declaration with confirmation and escalation preview.
//!
//! Ports `map/war_declaration_dialog.gd` and `ui/escalation_preview.gd`: before a declaration of
//! war (the « Déclarer la guerre » action of the sheet, or an attack on an army / a place of a
//! faction the player is not at war with) a modal lists the consequences (casus belli,
//! reputation, perjury, allies' answers, prestige: `game_api::diplomacy::preview_order`) and the
//! « Qui peut entrer en guerre » chain of suzerains (`game_api::diplomacy::war_escalation`).
//! Confirming sends `Order::DeclareWar`, then the optional follow-up order (the attack).
//!
//! Other lots (army orders on the map) open the dialog with an [`AskWarDeclaration`] message; no
//! rule is decided here.

use bevy::prelude::*;
use data_model::{FactionId, GameData};
use game_api::diplomacy::{EscalationView, faction_list, preview_order, war_escalation};
use serde_json::{Value, json};
use sim_campaign::{CampaignState, Order};

use super::format;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::sim::Sim;
use crate::ui::kit::modal::{ModalQueue, ModalResult, ModalSpec, PRIORITY_DECISION, ToastRequest};

/// Key of the confirmation in the [`ModalQueue`].
pub const WAR_KEY: &str = "war_declaration";

/// Asks the player to confirm a declaration of war on `target`. `then` is sent right after the
/// declaration (the attack that triggered the dialog); `target_label` names what is attacked
/// (« l'armée de X »), empty for a plain declaration.
#[derive(Message, Debug, Clone)]
pub struct AskWarDeclaration {
    pub target: String,
    pub target_label: String,
    pub then: Option<Order>,
    /// Army that storms the place at once if `then` (a march to it) ends in a siege
    /// (`army_movement_controller.order_attack_settlement`).
    pub assault_after: Option<sim_campaign::ArmyId>,
}

/// The declaration waiting for the player's answer, and what the dialog last said (dump).
#[derive(Resource, Default)]
pub struct WarDialog {
    pending: Option<AskWarDeclaration>,
    pub last_text: String,
    pub last_steps: Vec<EscalationView>,
    pub confirmed: u32,
    pub cancelled: u32,
}

pub struct WarDialogPlugin;

impl Plugin for WarDialogPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WarDialog>()
            .add_message::<AskWarDeclaration>()
            .register_dump(dump_war)
            .add_systems(
                Update,
                (open_dialog, answer_dialog)
                    .chain()
                    .run_if(in_state(AppState::Campaign)),
            );
    }
}

/// Whether an attack on `faction` first needs a declaration of war: the player is not at war with
/// it but holds a peace, a truce or a vassal / suzerain link (`army_movement_controller.relation_to`
/// = "peace"). Allies, the player's own faction and unknown ones cannot be attacked.
pub fn needs_declaration(
    data: &GameData,
    state: &CampaignState,
    player: &FactionId,
    faction: &FactionId,
) -> bool {
    if faction == player || state.is_at_war(faction, player) {
        return false;
    }
    faction_list(data, state, player)
        .into_iter()
        .find(|row| row.id == faction.as_str())
        .is_some_and(|row| {
            matches!(
                row.status.as_str(),
                "peace" | "truce" | "vassal" | "suzerain"
            )
        })
}

/// BBCode of the escalation chain (`EscalationPreview.bbcode`).
pub fn escalation_bbcode(steps: &[EscalationView]) -> String {
    if steps.is_empty() {
        return String::new();
    }
    let parts: Vec<String> = steps
        .iter()
        .map(|step| {
            format!(
                "{} ([color={}]{}[/color] : {})",
                step.name,
                likelihood_color(&step.likelihood),
                step.likelihood_label,
                step.reason
            )
        })
        .collect();
    format!(
        "[b]Qui peut entrer en guerre :[/b] {}",
        parts.join(" \u{2192} ")
    )
}

fn likelihood_color(key: &str) -> &'static str {
    match key {
        "likely" => "#8b1a1a",
        "uncertain" => "#8a6a10",
        _ => "#2a6a2a",
    }
}

/// Title and BBCode body of the confirmation; `None` for an unknown or dead faction.
pub fn dialog_content(
    data: &GameData,
    state: &CampaignState,
    player: &FactionId,
    ask: &AskWarDeclaration,
) -> Option<(String, String, Vec<EscalationView>)> {
    let target = FactionId::new(ask.target.clone()).ok()?;
    state.factions.get(&target)?;
    let name = data.faction_name(&target);
    let mut lines = Vec::new();
    lines.push(if ask.target_label.is_empty() {
        format!("Déclarer la guerre à {name}.")
    } else {
        format!(
            "Attaquer {} nous met en guerre avec {name}.",
            ask.target_label
        )
    });
    let status = faction_list(data, state, player)
        .into_iter()
        .find(|row| row.id == ask.target)
        .map(|row| row.status)
        .unwrap_or_default();
    match status.as_str() {
        "vassal" => lines.push(format!(
            "{name} est notre vassal : la guerre rompt son hommage."
        )),
        "suzerain" => lines.push(format!(
            "{name} est notre suzerain : la guerre rompt notre hommage."
        )),
        _ => {}
    }
    let preview = preview_order(
        data,
        state,
        player,
        &Order::DeclareWar {
            target: target.clone(),
        },
    );
    let parts: Vec<String> = preview
        .reasons
        .iter()
        .map(|reason| {
            if reason.value == 0 {
                reason.text.clone()
            } else {
                format::reason_tooltip(reason)
            }
        })
        .collect();
    if !parts.is_empty() {
        lines.push(format!("[b]Conséquences :[/b] {}", parts.join(" \u{b7} ")));
    }
    let steps = war_escalation(data, state, player, &target);
    let chain = escalation_bbcode(&steps);
    if !chain.is_empty() {
        lines.push(chain);
    }
    Some((
        format!("Déclarer la guerre à {name} ?"),
        lines.join("\n"),
        steps,
    ))
}

fn open_dialog(
    mut asks: MessageReader<AskWarDeclaration>,
    sim: Res<Sim>,
    mut dialog: ResMut<WarDialog>,
    mut modals: ResMut<ModalQueue>,
) {
    for ask in asks.read() {
        let (Some(data), Some(state)) = (&sim.data, &sim.state) else {
            continue;
        };
        let Some((title, text, steps)) = dialog_content(data, state, &sim.player, ask) else {
            continue;
        };
        dialog.last_text = text.clone();
        dialog.last_steps = steps;
        dialog.pending = Some(ask.clone());
        let mut spec = ModalSpec::confirm(&title, &text);
        spec.yes_label = "Déclarer la guerre".to_owned();
        spec.width = 520.0;
        modals.request(WAR_KEY, PRIORITY_DECISION, spec);
    }
}

fn answer_dialog(
    mut results: MessageReader<ModalResult>,
    mut sim: ResMut<Sim>,
    mut dialog: ResMut<WarDialog>,
    mut toasts: MessageWriter<ToastRequest>,
) {
    for result in results.read() {
        if result.key != WAR_KEY {
            continue;
        }
        let Some(ask) = dialog.pending.take() else {
            continue;
        };
        if !result.confirmed {
            dialog.cancelled += 1;
            continue;
        }
        dialog.confirmed += 1;
        let Ok(target) = FactionId::new(ask.target.clone()) else {
            continue;
        };
        let (Some(data), Some(state)) = (sim.data.clone(), sim.state.as_mut()) else {
            continue;
        };
        let declared =
            game_api::diplomacy::submit_order(&data, state, Order::DeclareWar { target });
        if !declared.ok {
            toasts.write(ToastRequest::new(&declared.message));
            continue;
        }
        toasts.write(ToastRequest::new("La guerre est déclarée."));
        if let Some(order) = ask.then {
            let followed = game_api::diplomacy::submit_order(&data, state, order);
            if !followed.ok {
                toasts.write(ToastRequest::new(&followed.message));
            } else if let Some(army) = ask.assault_after {
                // A siege opened by the march: the assault follows; a refusal leaves the siege.
                let besieging = state.army(&army).is_some_and(|a| a.settlement().is_some());
                if besieging {
                    let _ = state.submit_order_outcome(&data, Order::Assault { army });
                }
            }
        }
    }
}

fn dump_war(world: &World) -> (String, Value) {
    let dialog = world.resource::<WarDialog>();
    // Factions an attack on whose armies first asks for a declaration (orders.rs).
    let sim = world.resource::<Sim>();
    let needing: Vec<String> = match (&sim.data, &sim.state) {
        (Some(data), Some(state)) => state
            .factions
            .keys()
            .filter(|f| needs_declaration(data, state, &sim.player, f))
            .map(|f| f.as_str().to_owned())
            .collect(),
        _ => Vec::new(),
    };
    (
        "war_dialog".to_owned(),
        json!({
            "attack_needs_declaration": needing.len(),
            "attack_needs_declaration_sample": needing.iter().take(5).collect::<Vec<_>>(),
            "pending": dialog.pending.as_ref().map(|p| p.target.clone()),
            "text": dialog.last_text,
            "escalation": dialog.last_steps,
            "confirmed": dialog.confirmed,
            "cancelled": dialog.cancelled,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(name: &str, likelihood: &str) -> EscalationView {
        EscalationView {
            faction: "fac_x".into(),
            name: name.into(),
            likelihood: likelihood.into(),
            likelihood_label: "probable".into(),
            reason: "raison".into(),
        }
    }

    #[test]
    fn bbcode_chain_joins_links() {
        assert_eq!(escalation_bbcode(&[]), "");
        let text = escalation_bbcode(&[step("A", "likely"), step("B", "unlikely")]);
        assert!(text.starts_with("[b]Qui peut entrer en guerre :[/b] A ([color=#8b1a1a]"));
        assert!(text.contains(" \u{2192} B ([color=#2a6a2a]"));
    }
}
