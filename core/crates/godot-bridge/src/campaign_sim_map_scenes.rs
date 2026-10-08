//! `CampaignSim` living map (lot FK1, spec
//! `docs/design/2026-09-29-carte-vivante-folk.md` § 2.1.3), read only:
//! province scenes of the current turn (`sim_campaign::map_scenes`) and the
//! tuning of the figurines (`data/rules/map_scenes.json`).

use godot::prelude::*;
use sim_campaign::map_scenes;

use crate::campaign_sim::{CampaignSim, Ctx};

#[godot_api(secondary)]
impl CampaignSim {
    /// Scenes to stage this turn: `[{province, settlement, kind, since_turn,
    /// intensity}]`, `settlement` empty when the scene belongs to the whole
    /// province (the renderer falls back on the seat), `kind` in `plague`,
    /// `famine`, `revolt`, `devastation`, `siege`, `construction`, `fair`,
    /// `celebration`, `flood`, `muster`, `intensity` in [0, 1]. Purely
    /// visual and deterministic; read once per turn. Empty before a
    /// campaign starts.
    #[func]
    fn get_map_scenes(&self) -> VarArray {
        let Some(Ctx { state, data }) = self.ctx() else {
            return VarArray::new();
        };
        map_scenes::map_scenes(state, data)
            .iter()
            .map(|scene| {
                vdict! {
                    "province" => scene.province.as_str(),
                    "settlement" => scene.settlement.as_ref().map_or("", |s| s.as_str()),
                    "kind" => scene.kind.as_str(),
                    "since_turn" => i64::from(scene.since_turn),
                    "intensity" => f64::from(scene.intensity),
                }
                .to_variant()
            })
            .collect()
    }
}
