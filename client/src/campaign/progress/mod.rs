//! Campaign progress: objectives and victory, end screen, army traditions, first-turns tutorial
//! (lot `bm/progress`, ADR 0357).
//!
//! Ports `game/scripts/map/victory_controller.gd` (objectives panel, end screen),
//! `traditions_controller.gd` and `tutorial_controller.gd` (+ `ui/tutorial_steps.gd`). Every
//! figure comes from `game_api::progress` (tested in `core/`); orders are `sim_campaign::Order`s
//! (`ChooseArmyTradition`) or the `set_victory_length` rule. The client holds only UI state.
//!
//! Messages the HUD / panels send (the `integ` lot wires the buttons):
//! - [`OpenObjectives`]: toggles the objectives panel (also key O, dev hook
//!   `CENT_ANS_OPEN_OBJECTIVES=1`);
//! - [`OpenTraditions`]: opens the traditions panel of an army (army strip button);
//! - [`StartTutorial`]: starts or resumes the guide (Menu, help F1, advisor).
//!
//! Dump sections: `progress_objectives`, `progress_end`, `progress_traditions`, `progress_tutorial`.

use bevy::prelude::*;

pub mod ending;
pub mod objectives;
pub mod traditions;
pub mod tutorial;

/// Toggles the objectives panel.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenObjectives;

/// Opens the traditions panel of army `.0` (an army of the player).
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct OpenTraditions(pub String);

/// Starts the tutorial, or resumes it where it was put aside.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartTutorial;

pub struct CampaignProgressPlugin;

impl Plugin for CampaignProgressPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<OpenObjectives>()
            .add_message::<OpenTraditions>()
            .add_message::<StartTutorial>()
            .add_plugins((
                objectives::plugin,
                ending::plugin,
                traditions::plugin,
                tutorial::plugin,
            ));
    }
}
