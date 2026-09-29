//! `CampaignSim` living map (lot FK1, spec
//! `docs/design/2026-09-29-carte-vivante-folk.md` § 2.1.3), read only:
//! province scenes of the current turn (`sim_campaign::map_scenes`) and the
//! tuning of the figurines (`data/rules/map_scenes.json`).

use godot::prelude::*;
use sim_campaign::map_scenes;

use crate::campaign_sim::CampaignSim;

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
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
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

    /// Tuning of the living map (`data/rules/map_scenes.json`, every key of
    /// `MapSceneRules` but `description`, with `durations: {kind: turns}`);
    /// empty before a campaign starts. The renderer reads the same file
    /// directly (`FolkPool.load_settings`, needed before a campaign exists).
    #[func]
    fn get_map_scene_rules(&self) -> VarDictionary {
        let Some(data) = &self.data else {
            return VarDictionary::new();
        };
        let rules = &data.map_scene_rules;
        let mut durations = VarDictionary::new();
        for kind in map_scenes::SceneKind::ALL {
            durations.set(kind.as_str(), i64::from(rules.duration(kind)));
        }
        vdict! {
            "devastation_threshold" => i64::from(rules.devastation_threshold),
            "pool_cap" => i64::from(rules.pool_cap),
            "activity_radius" => rules.activity_radius,
            "figure_height" => rules.figure_height,
            "road_folk_per_unit" => rules.road_folk_per_unit,
            "field_work_probability" => rules.field_work_probability,
            "herd_probability" => rules.herd_probability,
            "woodcutter_probability" => rules.woodcutter_probability,
            "pilgrim_probability" => rules.pilgrim_probability,
            "carts_per_trade_value" => rules.carts_per_trade_value,
            "guard_value" => rules.guard_value,
            "scene_figures_min" => i64::from(rules.scene_figures_min),
            "scene_figures_max" => i64::from(rules.scene_figures_max),
            "durations" => &durations,
        }
    }
}
