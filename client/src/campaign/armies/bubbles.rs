//! Rich hover bubbles of the map: the army or the colony under the pointer (the Godot
//! `army_bubble_text` / `settlement_bubble_text` of `campaign_map.gd`, text in `hover_text.rs`).
//! The result goes to [`RichHover`], which `feedback::update_hint` shows beside the pointer.
//! An army lost from sight (ghost memory) is not ported: `hover_text::army_text` already takes
//! `seen_ago`, but the client keeps no memory of enemy armies yet.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use data_model::{GameData, SettlementId};
use sim_campaign::{ArmyId, CampaignState, MoveTarget};

use super::HoveredArmy;
use super::hover_text::{
    ArmyBubble, SettlementBubble, SiegeLine, army_text, composition, settlement_text,
};
use super::markers::camera_distance;
use super::pointer::PointerOverUi;
use super::projection::ray_ground;
use super::sea::{Glides, settlement_at};
use crate::campaign::markers::places::Places;
use crate::campaign::terrain::surface::GroundHeight;
use crate::campaign::weather::FogOfWar;
use crate::sim::Sim;

/// Bubble text (BBCode) of the object under the pointer.
#[derive(Resource, Default, Debug, Clone, PartialEq)]
pub struct RichHover {
    pub text: Option<String>,
    /// `army:<id>` or `settlement:<id>` the text was built for.
    pub target: Option<String>,
    /// Texts built since the campaign opened (dump).
    pub builds: u32,
}

/// The bubble of army `id`, as the player may see it.
pub fn army_bubble(
    data: &GameData,
    state: &CampaignState,
    id: &ArmyId,
    at_sea: bool,
) -> Option<ArmyBubble> {
    let army = state.army(id)?;
    let is_player = &army.faction == state.player_faction();
    let general_name = army
        .general
        .as_ref()
        .map_or_else(String::new, |g| state.character_name(data, g));
    let liege_name = sim_campaign::feudal::liege_of(state, data, &army.faction)
        .map(|liege| data.faction_name(&liege));
    let destination_name = match &army.destination {
        Some(MoveTarget::Settlement(s)) => Some(data.settlement_name(s)),
        _ => None,
    };
    let units: Vec<_> = army
        .units
        .iter()
        .map(|u| {
            (
                data.unit_types
                    .get(&u.unit_type)
                    .map_or(data_model::UnitCategory::Infantry, |t| t.category),
                u.strength,
            )
        })
        .collect();
    Some(ArmyBubble {
        general_name,
        faction_name: data.faction_name(&army.faction),
        liege_name,
        men: army.units.iter().map(|u| u.strength).sum(),
        composition: composition(&units),
        stance: army.stance.key().to_owned(),
        is_player,
        embarked: at_sea,
        moving: !army.planned_path.is_empty() || army.destination.is_some(),
        destination_name,
        supply: army.supply,
        seen_ago: None,
    })
}

/// The bubble of colony `id`: garrison and siege only when the province is in sight or ours.
pub fn settlement_bubble(
    data: &GameData,
    state: &CampaignState,
    fog: &FogOfWar,
    id: &SettlementId,
) -> Option<SettlementBubble> {
    let live = state.settlement_state(id)?;
    let player = state.player_faction();
    let visible = &live.owner == player
        || &live.controller == player
        || fog.is_province_visible(live.province.as_str());
    let mut bubble = SettlementBubble {
        name: data.settlement_name(id),
        owner_name: data.faction_name(&live.owner),
        controller_name: data.faction_name(&live.controller),
        visible,
        is_player: &live.controller == player,
        ..Default::default()
    };
    if visible {
        bubble.buildings = live
            .buildings
            .iter()
            .map(|b| b.as_str().to_owned())
            .collect();
        bubble.fortification = state.fortification_level(data, id);
        bubble.siege = live.siege.as_ref().map(|siege| SiegeLine {
            attacker_name: data.faction_name(&siege.attacker),
            turns_elapsed: siege.turns_elapsed,
            supplies: siege.supplies,
            breach: siege.breach,
        });
        bubble.garrison_units = live.garrison.len();
        bubble.garrison_men = live.garrison.iter().map(|u| u.strength).sum();
    }
    Some(bubble)
}

/// Finds what the pointer rests on and builds its bubble when the target or the simulation
/// changed.
#[allow(clippy::too_many_arguments)]
pub fn update_rich_hover(
    sim: Res<Sim>,
    fog: Res<FogOfWar>,
    hovered: Res<HoveredArmy>,
    glides: Res<Glides>,
    places: Res<Places>,
    pointer: Res<PointerOverUi>,
    window: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    ground: GroundHeight,
    mut rich: ResMut<RichHover>,
) {
    let changed = sim.is_changed() || fog.is_changed();
    let target: Option<String> = (|| {
        if pointer.over {
            return None;
        }
        if let Some(army) = &hovered.0 {
            return Some(format!("army:{}", army.as_str()));
        }
        let cursor = window.iter().next()?.cursor_position()?;
        let (camera, transform) = camera.iter().next()?;
        let ray = camera.viewport_to_world(transform, cursor).ok()?;
        let px = ray_ground(ray, &ground)?;
        let distance = camera_distance(transform, Vec3::new(px.x, 0.0, px.y));
        settlement_at(&places, px, distance).map(|(id, _)| format!("settlement:{}", id.as_str()))
    })();
    if target == rich.target && !changed {
        return;
    }
    let text = target.as_deref().and_then(|target| {
        let (data, state) = (sim.data.as_ref()?, sim.state.as_ref()?);
        if let Some(id) = target.strip_prefix("army:") {
            let id = ArmyId::parse(id)?;
            let at_sea = glides.active.contains_key(&id);
            army_bubble(data, state, &id, at_sea).map(|b| army_text(&b))
        } else {
            let id = SettlementId::new(target.strip_prefix("settlement:")?).ok()?;
            settlement_bubble(data, state, &fog, &id).map(|b| settlement_text(&b))
        }
    });
    if rich.target != target || rich.text != text {
        rich.builds += 1;
        rich.target = target;
        rich.text = text;
    }
}
