//! Army list shown to the player, read from the `Sim` resource.

use bevy::prelude::*;
use data_model::{FactionId, UnitCategory};
use sim_campaign::{ArmyId, ArmyPosition, Stance};

use crate::campaign::weather::FogOfWar;
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
    /// Settlement the army stands in (garrison stand-off, ADR 0160).
    pub settlement: Option<String>,
    /// One category per regiment (figures follow the composition).
    pub categories: Vec<UnitCategory>,
    pub movement_left: u32,
    /// Commander (character id) and whether he is the ruler of the faction (royal banner).
    pub general: Option<String>,
    pub royal: bool,
    /// One entry per regiment: figure family, variant, strength (skinned troop).
    pub roster: Vec<super::figures::RosterEntry>,
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

/// Armies hidden by the fog of war at the last refresh (dump).
#[derive(Resource, Default, Debug)]
pub struct FogHidden(pub usize);

pub fn refresh_views(
    sim: Res<Sim>,
    fog: Res<FogOfWar>,
    mut hidden: ResMut<FogHidden>,
    mut views: ResMut<ArmyViews>,
) {
    if !(sim.is_changed() || fog.is_changed()) {
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
        .filter(|(id, army)| army.faction == player || fog.is_army_visible(id.as_str()))
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
            settlement: army.settlement().map(|s| s.as_str().to_owned()),
            categories: army
                .units
                .iter()
                .map(|u| {
                    data.unit_types
                        .get(&u.unit_type)
                        .map_or(UnitCategory::Infantry, |t| t.category)
                })
                .collect(),
            movement_left: army.movement_left,
            general: army.general.as_ref().map(|g| g.as_str().to_owned()),
            royal: army.general.as_ref().is_some_and(|g| {
                data.factions
                    .get(&army.faction)
                    .and_then(|f| f.ruler.as_ref())
                    .is_some_and(|r| r == g)
            }),
            roster: army
                .units
                .iter()
                .filter_map(|u| {
                    let unit_type =
                        crate::battle::scene::setup::unit_type(data, u.unit_type.as_str())?;
                    let kind = match (unit_type.category, unit_type.mounted) {
                        (UnitCategory::Siege, _) => return None,
                        (_, true) | (UnitCategory::Cavalry, _) => "cavalry",
                        (UnitCategory::Ranged, _) => "archer",
                        _ => "infantry",
                    };
                    Some(super::figures::RosterEntry {
                        kind,
                        variant: crate::battle::units::library::variant_of_figure(
                            unit_type.figure.as_deref(),
                        ),
                        strength: u.strength,
                        unit_type: u.unit_type.as_str().to_owned(),
                    })
                })
                .collect(),
        })
        .collect();
    let visible_to_sim = seen.len();
    hidden.0 = visible_to_sim.saturating_sub(list.len());
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
