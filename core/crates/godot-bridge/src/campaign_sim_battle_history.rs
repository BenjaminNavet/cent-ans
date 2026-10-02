//! `CampaignSim` battle history (chantier TB, ADR 0157 « révision »), read
//! only: the recent land battles kept in the campaign state
//! (`sim_campaign::battle_history`), for the battlefield marks of the map.

use godot::prelude::*;

use crate::campaign_sim::CampaignSim;

#[godot_api(secondary)]
impl CampaignSim {
    /// Recent land battles, oldest first: `[{turn, age, province, position:
    /// Vector2 (map pixels), kind ("field", "assault", "sortie"), attacker,
    /// defender, winner (faction ids), attacker_strength, defender_strength,
    /// attacker_losses, defender_losses}]`. `age` is the number of turns
    /// since the battle (0 the turn it was fought). Bounded by
    /// `data/rules/battle_history.json`; saved with the campaign. Empty
    /// before a campaign starts and for a save older than the history.
    #[func]
    fn get_battle_history(&self) -> VarArray {
        VarArray::new()
    }
}
