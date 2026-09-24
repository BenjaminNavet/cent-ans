//! `CampaignSim` settlements API (lot C1, `docs/design/2026-09-24-echelle-colonies.md` § 4.6).

use godot::prelude::*;

use crate::campaign_sim::CampaignSim;

#[godot_api(secondary)]
impl CampaignSim {
    /// Every settlement, sorted by id: `[{id, province, kind, name, lonlat,
    /// controller, owner, fortification_level}]`. `kind` is `city`, `town`,
    /// `castle`, `abbey` or `village`; `name` is the French display name;
    /// `lonlat` is a `Vector2(lon, lat)` in WGS84 degrees. Empty before a
    /// campaign exists.
    #[func]
    fn settlements(&self) -> VarArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarArray::new();
        };
        data.settlements
            .iter()
            .filter_map(|(id, settlement)| {
                let live = state.settlement_state(id)?;
                let lonlat = Vector2::new(settlement.lonlat[0] as f32, settlement.lonlat[1] as f32);
                Some(
                    vdict! {
                        "id" => id.as_str(),
                        "province" => settlement.province.as_str(),
                        "kind" => settlement.kind.key(),
                        "name" => settlement.name.display.as_str(),
                        "lonlat" => lonlat,
                        "controller" => live.controller.as_str(),
                        "owner" => live.owner.as_str(),
                        "fortification_level" => i64::from(live.fortification_level),
                    }
                    .to_variant(),
                )
            })
            .collect()
    }
}
