//! `CampaignSim` line of sight (lot C1 fog of war, per-cell radius since lot
//! M5a), read only.

use data_model::FactionId;
use godot::classes::image::Format;
use godot::classes::Image;
use godot::prelude::*;
use sim_campaign::vision::Vision;

use crate::campaign_sim::{CampaignSim, Ctx};

impl CampaignSim {
    fn faction_vision(&self, faction: &GString) -> Option<Vision> {
        let Some(Ctx { state, data }) = self.ctx() else {
            return None;
        };
        let faction = FactionId::new(faction.to_string()).ok()?;
        Some(state.vision(data, &faction))
    }
}

#[godot_api(secondary)]
impl CampaignSim {
    /// Province ids `faction` sees this turn (`CampaignState::visible_provinces`:
    /// enough of its land seen, a seen settlement, a friendly army or an
    /// agent); empty before a campaign starts.
    #[func]
    fn get_visible_provinces(&self, faction: GString) -> PackedStringArray {
        let Some(vision) = self.faction_vision(&faction) else {
            return PackedStringArray::new();
        };
        vision
            .provinces
            .iter()
            .map(|id| GString::from(id.as_str()))
            .collect()
    }

    /// `true` when map pixel `point` is seen by `faction` (lot M5a).
    #[func]
    fn is_point_visible(&self, faction: GString, point: Vector2) -> bool {
        let Some(data) = &self.data else {
            return false;
        };
        self.faction_vision(&faction)
            .is_some_and(|v| v.mask.sees_point(data, [point.x, point.y]))
    }

    /// Everything the map needs for the fog in one call (lot M5a):
    /// `{image: Image R8 (512², 255 = seen, soft edges, seen from 128),
    /// size: Vector2 (map pixels covered by the image), texel_px,
    /// provinces: PackedStringArray, armies: PackedStringArray,
    /// seen_share: float}`. Empty before a campaign starts.
    #[func]
    fn get_vision(&self, faction: GString) -> VarDictionary {
        let Some(Ctx { state, data }) = self.ctx() else {
            return VarDictionary::new();
        };
        let Some(vision) = self.faction_vision(&faction) else {
            return VarDictionary::new();
        };
        let mask = &vision.mask;
        let packed = PackedByteArray::from(mask.coverage.as_slice());
        let Some(image) = Image::create_from_data(
            mask.width as i32,
            mask.height as i32,
            false,
            Format::R8,
            &packed,
        ) else {
            return VarDictionary::new();
        };
        let provinces: PackedStringArray = vision
            .provinces
            .iter()
            .map(|id| GString::from(id.as_str()))
            .collect();
        let armies: PackedStringArray = state
            .armies
            .iter()
            .filter(|(_, army)| vision.sees_army(state, data, army))
            .map(|(id, _)| GString::from(id.as_str()))
            .collect();
        vdict! {
            "image" => &image,
            "size" => Vector2::new(
                mask.width as f32 * mask.texel_px,
                mask.height as f32 * mask.texel_px,
            ),
            "texel_px" => mask.texel_px,
            "provinces" => &provinces,
            "armies" => &armies,
            "seen_share" => mask.seen_share(),
        }
    }
}
