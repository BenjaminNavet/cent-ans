//! HUD buttons of the progress lot: « Objectifs » (messages `OpenObjectives`) and « Traditions »
//! of the selected army of the player (`OpenTraditions`, « Traditions (n) » while ranks wait).
//!
//! Ports the Objectives entry of `hud_controller.gd` and `TraditionsController.on_army_shown`
//! (button shown for an army of the player, text with the number of ranks to spend).

use bevy::prelude::*;
use serde_json::{Value, json};
use sim_campaign::ArmyId;

use super::{HudSet, HudWidget, PanelButtons};
use crate::agent::dump::DumpAppExt;
use crate::campaign::armies::SelectedArmy;
use crate::campaign::progress::{OpenObjectives, OpenTraditions};
use crate::sim::Sim;
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::widgets::button_small;
use crate::ui::kit::{ButtonPressed, UiTheme};

/// Button of the traditions of the selected army.
#[derive(Component)]
struct TraditionsButton;

#[derive(Component)]
struct ObjectivesButton;

/// What the traditions button shows now: `(army id, ranks to spend)`.
#[derive(Resource, Default, Debug, Clone, PartialEq)]
pub struct TraditionsShown {
    pub shown: Option<(String, u8)>,
    pub opened: u32,
    pub objectives_opened: u32,
}

/// Caption of the traditions button.
pub fn traditions_caption(pending: u8) -> String {
    if pending > 0 {
        format!("Traditions ({pending})")
    } else {
        "Traditions".to_owned()
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<TraditionsShown>()
        .register_dump(dump)
        .add_systems(
            Update,
            (sync_buttons, handle_clicks).chain().in_set(HudSet::Ui),
        );
}

#[allow(clippy::too_many_arguments)]
fn sync_buttons(
    mut commands: Commands,
    sim: Res<Sim>,
    selected: Res<SelectedArmy>,
    theme: Res<UiTheme>,
    column: Query<Entity, With<PanelButtons>>,
    objectives: Query<Entity, With<ObjectivesButton>>,
    traditions: Query<(Entity, &Children), With<TraditionsButton>>,
    mut texts: Query<&mut Text>,
    mut state: ResMut<TraditionsShown>,
) {
    let Ok(column) = column.single() else { return };
    if objectives.is_empty() {
        commands.spawn((
            button_small(&theme, "hud:open_objectives", "Objectifs"),
            ObjectivesButton,
            HudWidget,
            ChildOf(column),
            Tooltip::new("Objectifs historiques, échéance et score (O)."),
        ));
    }
    // Traditions: only for an army of the player the core knows traditions for.
    let wanted = match (&sim.data, &sim.state, &selected.0) {
        (Some(data), Some(game), Some(id))
            if game.army(id).is_some_and(|a| a.faction == sim.player) =>
        {
            game_api::progress::traditions_view(data, game, id)
                .map(|v| (id.as_str().to_owned(), v.pending))
        }
        _ => None,
    };
    if state.shown != wanted {
        state.shown = wanted.clone();
    }
    match (traditions.single().ok(), wanted) {
        (None, Some((_, pending))) => {
            commands.spawn((
                button_small(&theme, "hud:open_traditions", &traditions_caption(pending)),
                TraditionsButton,
                HudWidget,
                ChildOf(column),
                Tooltip::new("Rang, expérience et traditions de l’armée choisie."),
            ));
        }
        (Some((entity, children)), None) => {
            let _ = children;
            commands.entity(entity).despawn();
        }
        (Some((_, children)), Some((_, pending))) => {
            let caption = traditions_caption(pending);
            for child in children.iter() {
                if let Ok(mut text) = texts.get_mut(child)
                    && text.0 != caption
                {
                    text.0 = caption.clone();
                }
            }
        }
        (None, None) => {}
    }
}

fn handle_clicks(
    mut pressed: MessageReader<ButtonPressed>,
    selected: Res<SelectedArmy>,
    mut objectives: MessageWriter<OpenObjectives>,
    mut traditions: MessageWriter<OpenTraditions>,
    mut state: ResMut<TraditionsShown>,
) {
    for press in pressed.read() {
        match press.name.as_str() {
            "hud:open_objectives" => {
                objectives.write(OpenObjectives);
                state.objectives_opened += 1;
            }
            "hud:open_traditions" => {
                if let Some(id) = selected.0.as_ref().map(ArmyId::as_str) {
                    traditions.write(OpenTraditions(id.to_owned()));
                    state.opened += 1;
                }
            }
            _ => {}
        }
    }
}

fn dump(world: &World) -> (String, Value) {
    let state = world.resource::<TraditionsShown>();
    (
        "hud_buttons".to_owned(),
        json!({
            "traditions": state.shown.as_ref().map(|(id, n)| json!({"army": id, "pending": n})),
            "traditions_opened": state.opened,
            "objectives_opened": state.objectives_opened,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caption_counts_pending_ranks() {
        assert_eq!(traditions_caption(0), "Traditions");
        assert_eq!(traditions_caption(2), "Traditions (2)");
    }
}
