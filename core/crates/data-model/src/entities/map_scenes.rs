//! Living map tuning (lot FK1), mirroring
//! `data/schemas/map_scenes_rules.schema.json` (`data/rules/map_scenes.json`).
//! Spec `docs/design/2026-09-29-carte-vivante-folk.md` §§ 2.1.1, 2.1.4.
//!
//! Purely visual: the scene durations feed `sim-campaign::map_scenes`, the
//! densities and caps are read by the renderer (`game/scripts/map/life_folk`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::entities::event::SceneKind;

/// Turns a scene lasts when a kind is missing from `durations`.
pub const DEFAULT_SCENE_TURNS: u32 = 2;

/// Contents of `data/rules/map_scenes.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapSceneRules {
    /// Turns a scene born of a chronicle event stays on the map, by kind.
    pub durations: BTreeMap<SceneKind, u32>,
    /// Devastation (0-100) from which a province shows a `devastation` scene.
    pub devastation_threshold: u8,
    /// Most figurines alive at once on the close view (halved when the
    /// frame budget is exceeded).
    pub pool_cap: u32,
    /// Merchant carts per unit of trade route value.
    pub carts_per_trade_value: f64,
    /// Peasants in the fields per thousand inhabitants of the province.
    pub peasants_per_thousand: f64,
    /// Radius (world units) around the camera centre where figurines live.
    pub activity_radius: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl MapSceneRules {
    /// Turns a scene of `kind` born of an event lasts (at least one).
    pub fn duration(&self, kind: SceneKind) -> u32 {
        self.durations
            .get(&kind)
            .copied()
            .unwrap_or(DEFAULT_SCENE_TURNS)
            .max(1)
    }

    /// Longest duration of any kind (history the chronicle keeps).
    pub fn max_duration(&self) -> u32 {
        SceneKind::ALL
            .iter()
            .map(|kind| self.duration(*kind))
            .max()
            .unwrap_or(DEFAULT_SCENE_TURNS)
    }
}

impl Default for MapSceneRules {
    /// Fallback when `data/rules/map_scenes.json` is absent; kept equal to
    /// that file.
    fn default() -> Self {
        let durations = [
            (SceneKind::Plague, 4),
            (SceneKind::Famine, 3),
            (SceneKind::Revolt, 2),
            (SceneKind::Devastation, 3),
            (SceneKind::Siege, 1),
            (SceneKind::Construction, 2),
            (SceneKind::Fair, 1),
            (SceneKind::Celebration, 2),
            (SceneKind::Flood, 2),
            (SceneKind::Muster, 1),
        ]
        .into_iter()
        .collect();
        MapSceneRules {
            durations,
            devastation_threshold: 25,
            pool_cap: 600,
            carts_per_trade_value: 0.05,
            peasants_per_thousand: 0.5,
            activity_radius: 60.0,
            description: None,
        }
    }
}
