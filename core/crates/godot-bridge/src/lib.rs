//! GDExtension entry point: exposes the Rust simulation to Godot.
//!
//! Only plain values cross the boundary (integers, strings, packed arrays);
//! Godot never holds a pointer into the simulation.

use godot::classes::RefCounted;
use godot::prelude::*;
use sim_campaign::CampaignState;

struct CentAnsExtension;

#[gdextension]
unsafe impl ExtensionLibrary for CentAnsExtension {}

/// Godot-facing handle on a campaign simulation.
#[derive(GodotClass)]
#[class(base = RefCounted)]
pub struct CampaignSim {
    state: Option<CampaignState>,
    base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for CampaignSim {
    fn init(base: Base<RefCounted>) -> Self {
        CampaignSim { state: None, base }
    }
}

#[godot_api]
impl CampaignSim {
    /// Starts a fresh campaign in spring 1337 with the given RNG seed.
    #[func]
    fn new_campaign(&mut self, seed: i64) {
        self.state = Some(CampaignState::new(seed as u64));
    }

    /// Advances the campaign by one season. Does nothing before `new_campaign`.
    #[func]
    fn end_turn(&mut self) {
        match self.state.as_mut() {
            Some(state) => state.end_turn(),
            None => godot_warn!("CampaignSim.end_turn called before new_campaign"),
        }
    }

    /// Zero-based turn counter (-1 before `new_campaign`).
    #[func]
    fn get_turn(&self) -> i64 {
        self.state
            .as_ref()
            .map_or(-1, |state| i64::from(state.turn))
    }

    /// Current campaign year (0 before `new_campaign`).
    #[func]
    fn get_year(&self) -> i64 {
        self.state.as_ref().map_or(0, |state| i64::from(state.year))
    }

    /// French date label, e.g. `"Printemps 1337"` (empty before `new_campaign`).
    #[func]
    fn get_date_label(&self) -> GString {
        self.state
            .as_ref()
            .map_or_else(GString::new, |state| GString::from(&state.date_label()))
    }
}
