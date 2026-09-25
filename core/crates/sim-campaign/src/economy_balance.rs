//! Net balance of a faction's purse (UI audit A3, E1/E4): the single source
//! of the « solde » shown by the top bar and the faction panel, so that the
//! interface never sums the budget lines itself.

use data_model::FactionId;

use crate::economy::FactionEconomy;
use crate::state::CampaignState;

impl FactionEconomy {
    /// Projected net balance for the coming season: receipts (seigniorage
    /// included) minus armies, buildings, court and administration
    /// (recoinage included) and the Table. This is exactly what
    /// `resolve_economy` will add to the treasury if nothing changes.
    pub fn net_income(&self) -> i64 {
        self.projected_income
            - self.army_upkeep
            - self.building_upkeep
            - self.administration_upkeep
            - self.table_upkeep
    }
}

impl CampaignState {
    /// Net balance actually booked by the last resolved turn (receipts minus
    /// every upkeep, the Table included); `None` for an unknown faction.
    pub fn faction_net_last_turn(&self, id: &FactionId) -> Option<i64> {
        let faction = self.factions.get(id)?;
        Some(faction.income_last_turn - faction.upkeep_last_turn)
    }
}
