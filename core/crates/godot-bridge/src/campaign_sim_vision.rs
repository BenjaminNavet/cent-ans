//! `CampaignSim` line of sight (lot C1, fog of war), read only.

use data_model::FactionId;
use godot::prelude::*;

use crate::campaign_sim::CampaignSim;

#[godot_api(secondary)]
impl CampaignSim {
    /// Province ids `faction` sees this turn (`CampaignState::visible_provinces`,
    /// ranges from `data/rules/vision.json`); empty before a campaign starts.
    #[func]
    fn get_visible_provinces(&self, faction: GString) -> PackedStringArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return PackedStringArray::new();
        };
        let Ok(faction) = FactionId::new(faction.to_string()) else {
            return PackedStringArray::new();
        };
        state
            .visible_provinces(data, &faction)
            .iter()
            .map(|id| GString::from(id.as_str()))
            .collect()
    }
}
