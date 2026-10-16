//! Applies [`FocusCamera`] (province, army, point, follow) and the campaign camera hotkeys.
//!
//! Ports `FlowController.focus_province / focus_army / focus_next_idle / toggle_follow_selected`
//! and `CampaignHotkeys` (`game/scripts/map/flow_controller.gd`, `campaign_hotkeys.gd`; keys of
//! `project.godot`): Tab / Shift+Tab next / previous idle army, Ctrl+Tab next province of the
//! player, Home capital, `.` centre on the selected army, `Y` follow it (a second press, or any
//! manual move, stops). The glide itself (`focus_glide_s`, eased) is the rig's `look_at_point`.

use bevy::prelude::*;
use sim_campaign::ArmyId;

use super::rig::{CampaignRig, Ground};
use super::{FocusCamera, drive_camera};
use crate::campaign::armies::{ArmyViews, SelectedArmy};
use crate::campaign::provinces::data::ProvinceMap;
use crate::campaign::terrain::relief_state::ReliefState;
use crate::campaign::terrain::surface::Surface;
use crate::sim::Sim;

/// Distance of a focus as a share of the larger map side (`extent * 0.09` in Godot).
pub const FOCUS_DISTANCE_SHARE: f32 = 0.09;

/// Army the camera follows (`None` = free).
#[derive(Resource, Default, Debug, Clone, PartialEq)]
pub struct FollowedArmy {
    pub id: Option<ArmyId>,
    /// Set once the rig carries the follow point (a later `rig.follow == None` means the player
    /// moved the camera by hand).
    armed: bool,
}

/// Last request applied, for the dump.
#[derive(Resource, Default, Debug, Clone, PartialEq)]
pub struct LastFocus(pub String);

/// Next / previous element of `ids` after `current`, cyclic (`CampaignHotkeys.next_in_cycle`).
pub fn next_in_cycle<T: PartialEq + Clone>(ids: &[T], current: Option<&T>, step: i32) -> Option<T> {
    if ids.is_empty() {
        return None;
    }
    let index = current.and_then(|c| ids.iter().position(|id| id == c));
    let next = match index {
        None if step >= 0 => 0,
        None => ids.len() - 1,
        Some(i) => {
            (i as i32 + if step >= 0 { 1 } else { -1 }).rem_euclid(ids.len() as i32) as usize
        }
    };
    ids.get(next).cloned()
}

#[allow(clippy::too_many_arguments)]
pub fn hotkeys(
    keys: Res<ButtonInput<KeyCode>>,
    keymap: Res<crate::settings::KeyMap>,
    views: Res<ArmyViews>,
    selected: Res<SelectedArmy>,
    followed: Res<FollowedArmy>,
    sim: Res<Sim>,
    map: Option<Res<ProvinceMap>>,
    mut last_province: Local<Option<String>>,
    mut requests: MessageWriter<FocusCamera>,
) {
    let next_idle = keymap.just_pressed("campaign_next_idle", &keys);
    let prev_idle = keymap.just_pressed("campaign_prev_idle", &keys);
    let next_settlement = keymap.just_pressed("campaign_next_settlement", &keys);
    if next_idle || prev_idle || next_settlement {
        if next_settlement {
            // Settlements are not exposed with a map position yet: cycle the provinces whose
            // city the player controls.
            let (Some(state), Some(_)) = (&sim.state, &map) else {
                return;
            };
            let mut ids: Vec<String> = state
                .controlled_provinces(state.player_faction())
                .map(|p| p.to_string())
                .collect();
            ids.sort();
            if let Some(next) = next_in_cycle(&ids, last_province.as_ref(), 1) {
                *last_province = Some(next.clone());
                requests.write(FocusCamera::Province(next));
            }
        } else {
            let idle: Vec<ArmyId> = views
                .0
                .iter()
                .filter(|v| v.player && v.status == "idle" && v.movement_left > 0)
                .map(|v| v.id.clone())
                .collect();
            let step = if prev_idle { -1 } else { 1 };
            if let Some(next) = next_in_cycle(&idle, selected.0.as_ref(), step) {
                requests.write(FocusCamera::Army(next.to_string()));
            }
        }
    }
    if keymap.just_pressed("campaign_capital", &keys)
        && let Some(state) = &sim.state
        && let Some(faction) = state.factions.get(state.player_faction())
    {
        requests.write(FocusCamera::Province(faction.capital.to_string()));
    }
    if keymap.just_pressed("army_center", &keys)
        && let Some(id) = &selected.0
    {
        requests.write(FocusCamera::Army(id.to_string()));
    }
    if keymap.just_pressed("army_follow", &keys) {
        if followed.id.is_some() {
            requests.write(FocusCamera::Follow(None));
        } else if let Some(id) = &selected.0 {
            requests.write(FocusCamera::Follow(Some(id.to_string())));
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn apply_focus(
    mut requests: MessageReader<FocusCamera>,
    views: Res<ArmyViews>,
    map: Option<Res<ProvinceMap>>,
    surface: Option<Res<Surface>>,
    relief: Option<Res<ReliefState>>,
    mut rig: ResMut<CampaignRig>,
    mut followed: ResMut<FollowedArmy>,
    mut selected: ResMut<SelectedArmy>,
    mut last: ResMut<LastFocus>,
) {
    let height = |x: f32, z: f32| match (&surface, &relief) {
        (Some(surface), Some(relief)) => surface.display_height_at(relief, x, z),
        _ => 0.0,
    };
    let ground = Ground {
        height: &height,
        pyramid: surface.as_ref().map(|s| s.pyramid.as_ref()),
    };
    let extent = map
        .as_ref()
        .map_or(7080.0, |m| m.width.max(m.height) as f32);
    let distance = extent * FOCUS_DISTANCE_SHARE;
    let army_point = |id: &str| {
        views
            .0
            .iter()
            .find(|v| v.id.as_str() == id)
            .map(|v| Vec2::new(v.point[0], v.point[1]))
    };
    for request in requests.read() {
        match request {
            FocusCamera::Province(id) | FocusCamera::FrameProvince(id) => {
                let Some(map) = &map else { continue };
                let Some(province) = map.province(map.index_of_id(id)) else {
                    warn!("FocusCamera: unknown province {id:?}");
                    continue;
                };
                let c = province.centroid;
                followed.id = None;
                rig.look_at_point(Vec3::new(c[0], 0.0, c[1]), distance, &ground);
                last.0 = format!("province:{id}");
            }
            FocusCamera::Army(id) => {
                let Some(point) = army_point(id) else {
                    warn!("FocusCamera: unknown army {id:?}");
                    continue;
                };
                followed.id = None;
                rig.look_at_point(Vec3::new(point.x, 0.0, point.y), distance, &ground);
                if let Some(view) = views.0.iter().find(|v| v.id.as_str() == id) {
                    selected.0 = Some(view.id.clone());
                }
                last.0 = format!("army:{id}");
            }
            FocusCamera::Point(p) => {
                followed.id = None;
                rig.look_at_point(Vec3::new(p.x, 0.0, p.y), 0.0, &ground);
                last.0 = format!("point:{},{}", p.x, p.y);
            }
            FocusCamera::Follow(Some(id)) => {
                let Some(view) = views.0.iter().find(|v| v.id.as_str() == id) else {
                    continue;
                };
                followed.id = Some(view.id.clone());
                followed.armed = false;
                last.0 = format!("follow:{id}");
            }
            FocusCamera::Follow(None) => {
                followed.id = None;
                rig.follow = None;
                last.0 = "follow:none".into();
            }
        }
    }
    // Continuous follow: the rig clears `follow` itself on a manual move.
    if let Some(id) = followed.id.clone() {
        if followed.armed && rig.follow.is_none() {
            followed.id = None;
        } else if let Some(point) = army_point(id.as_str()) {
            rig.follow = Some(point);
            followed.armed = true;
        } else {
            followed.id = None;
            rig.follow = None;
        }
    }
}

pub fn add(app: &mut App) {
    app.init_resource::<FollowedArmy>()
        .init_resource::<LastFocus>()
        .add_systems(
            Update,
            (
                hotkeys.run_if(crate::ui::kit::text_field::keyboard_free),
                apply_focus,
            )
                .chain()
                .in_set(super::CameraSet)
                .before(drive_camera)
                .run_if(in_state(crate::app::AppState::Campaign))
                .run_if(resource_exists::<CampaignRig>),
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_wraps_and_handles_missing_current() {
        let ids = ["a", "b", "c"];
        assert_eq!(next_in_cycle(&ids, Some(&"c"), 1), Some("a"));
        assert_eq!(next_in_cycle(&ids, Some(&"a"), -1), Some("c"));
        assert_eq!(next_in_cycle(&ids, None, 1), Some("a"));
        assert_eq!(next_in_cycle(&ids, Some(&"zz"), -1), Some("c"));
        assert_eq!(next_in_cycle::<&str>(&[], None, 1), None);
    }
}
