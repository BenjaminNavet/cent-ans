//! `CampaignSim` map encounters API (lot CV3-3, spec
//! `docs/design/2026-09-27-campagne-vivante.md` § 2), in a secondary
//! `#[godot_api]` block.

use godot::prelude::*;

use crate::campaign_sim::CampaignSim;

#[godot_api(secondary)]
impl CampaignSim {
    /// Encounter sites the player sees.
    #[func]
    fn get_encounter_sites(&self) -> VarArray {
        VarArray::new()
    }

    /// Encounters of the player awaiting a choice.
    #[func]
    fn get_pending_encounters(&self) -> VarArray {
        VarArray::new()
    }
}
