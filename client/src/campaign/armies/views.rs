//! Army list shown to the player, read from the `Sim` resource.

use bevy::prelude::*;
use data_model::FactionId;
use sim_campaign::{ArmyId, ArmyPosition, Stance};

use crate::sim::Sim;

/// One army as the map shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct ArmyView {
    pub id: ArmyId,
    pub faction: FactionId,
    /// Faction primary colour (linear-sRGB-ish `#RRGGBB` bytes / 255).
    pub color: [f32; 3],
    /// Map pixel of the army.
    pub point: [f32; 2],
    pub men: u32,
    pub units: usize,
    pub player: bool,
    /// At war with the player (red cues).
    pub hostile: bool,
    /// `"moving"`, `"siege"` or `"idle"` (plate status, `ArmyPlate.STATUS_TEXT`).
    pub status: &'static str,
    pub in_settlement: bool,
    pub movement_left: u32,
}

#[derive(Resource, Default, Debug, Clone, PartialEq)]
pub struct ArmyViews(pub Vec<ArmyView>);

impl ArmyViews {
    pub fn get(&self, id: &ArmyId) -> Option<&ArmyView> {
        self.0.iter().find(|v| &v.id == id)
    }
}

/// `#RRGGBB` to 0-1 floats; grey on parse failure.
pub fn parse_hex_color(text: &str) -> [f32; 3] {
    let hex = text.trim_start_matches('#');
    if hex.len() >= 6 {
        let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
        if let (Some(r), Some(g), Some(b)) = (byte(0), byte(2), byte(4)) {
            return [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0];
        }
    }
    [0.5, 0.5, 0.5]
}

pub fn refresh_views(sim: Res<Sim>, mut views: ResMut<ArmyViews>) {
    if !sim.is_changed() {
        return;
    }
    let (Some(data), Some(state)) = (&sim.data, &sim.state) else {
        if !views.0.is_empty() {
            views.0.clear();
        }
        return;
    };
    let player = state.player_faction.clone();
    let seen = state.visible_armies(data, &player);
    let list: Vec<ArmyView> = state
        .armies
        .iter()
        .filter(|(id, _)| seen.contains(*id))
        .map(|(id, army)| ArmyView {
            id: id.clone(),
            faction: army.faction.clone(),
            color: data
                .factions
                .get(&army.faction)
                .map_or([0.5; 3], |f| parse_hex_color(&f.heraldry.primary_color)),
            point: state.army_point(data, army),
            men: army.units.iter().map(|u| u.strength).sum(),
            units: army.units.len(),
            player: army.faction == player,
            hostile: army.faction != player && state.is_at_war(&army.faction, &player),
            status: if army.stance == Stance::Siege {
                "siege"
            } else if !army.planned_path.is_empty() {
                "moving"
            } else {
                "idle"
            },
            in_settlement: matches!(army.position, ArmyPosition::Settlement(_)),
            movement_left: army.movement_left,
        })
        .collect();
    if list != views.0 {
        views.0 = list;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_colors() {
        assert_eq!(parse_hex_color("#FF0000"), [1.0, 0.0, 0.0]);
        assert_eq!(parse_hex_color("zz"), [0.5; 3]);
    }
}
