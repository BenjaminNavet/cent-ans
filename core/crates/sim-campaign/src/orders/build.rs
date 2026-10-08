//! Construction orders: build, cancel the work in progress, cancel a queued one.

use data_model::{BuildingId, FactionId, GameData, SettlementId};

use super::OrderError;
use crate::buildings::CANCEL_REFUND_PERCENT;
use crate::state::{CampaignState, Construction};

impl CampaignState {
    pub(super) fn order_build(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        settlement: &SettlementId,
        building: &BuildingId,
    ) -> Result<(), OrderError> {
        self.own_settlement(faction, settlement)?;
        let Some(definition) = data.buildings.get(building) else {
            return Err(OrderError::UnknownBuilding(building.clone()));
        };
        if !definition.allowed_in(self.settlement_kind(settlement)) {
            return Err(OrderError::BuildUnavailable(
                "impossible dans ce type de colonie".to_owned(),
            ));
        }
        let option = self
            .buildable(data, settlement)
            .into_iter()
            .find(|o| &o.building == building)
            .ok_or_else(|| OrderError::UnknownBuilding(building.clone()))?;
        if !option.available {
            return Err(OrderError::BuildUnavailable(
                option.reason.unwrap_or_default(),
            ));
        }
        // B7c: the resources come from the faction's producing provinces
        // (reserved for the construction), the rest is imported and paid.
        let supply = self.free_supply(data, faction);
        let draw = crate::buildings::resource_draw(data, &supply, &definition.cost.resources);
        self.factions
            .get_mut(faction)
            .expect("checked above")
            .treasury -= i64::from(option.cost);
        self.settlements
            .get_mut(settlement)
            .expect("checked above")
            .enqueue_build(Construction {
                building: building.clone(),
                turns_left: option.turns,
                paid: option.cost,
                drawn: draw.drawn,
            });
        Ok(())
    }

    pub(super) fn order_cancel_build(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        settlement: &SettlementId,
    ) -> Result<(), OrderError> {
        self.own_settlement(faction, settlement)?;
        let settlement_state = self.settlements.get_mut(settlement).expect("checked above");
        let Some(construction) = settlement_state.construction.take() else {
            return Err(OrderError::NoConstruction);
        };
        // B7c: half of what was paid (imports included); pre-B7c saves
        // did not record it and fall back on the money cost.
        let paid = if construction.paid > 0 {
            construction.paid
        } else {
            data.buildings
                .get(&construction.building)
                .map_or(0, |b| b.cost.money)
        };
        settlement_state.promote_queued_build();
        let refund = paid * CANCEL_REFUND_PERCENT / 100;
        self.factions
            .get_mut(faction)
            .expect("checked above")
            .treasury += i64::from(refund);
        Ok(())
    }

    pub(super) fn order_cancel_queued_build(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        settlement: &SettlementId,
        index: usize,
    ) -> Result<(), OrderError> {
        self.own_settlement(faction, settlement)?;
        let settlement_state = self.settlements.get_mut(settlement).expect("checked above");
        if index >= settlement_state.build_queue.len() {
            return Err(OrderError::NoConstruction);
        }
        let work = settlement_state.build_queue.remove(index);
        let paid = if work.paid > 0 {
            work.paid
        } else {
            data.buildings
                .get(&work.building)
                .map_or(0, |b| b.cost.money)
        };
        self.factions
            .get_mut(faction)
            .expect("checked above")
            .treasury += i64::from(paid * CANCEL_REFUND_PERCENT / 100);
        Ok(())
    }
}
