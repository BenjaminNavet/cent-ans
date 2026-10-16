//! Hover text of the map (ports game/scripts/ui/object_hover.gd + decor_hover.gd timing and the
//! `hover_text` of game/scripts/map/map_mode_controller.gd): once the pointer rests on a province
//! for `interface/decor_hover_delay` seconds (0 = off) a small note beside it gives the name, the
//! lord and the figure of the current map mode (stance, unrest, income, inhabitants).
//!
//! Armies and settlements have their own bubbles (armies / markers lots).

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use serde_json::{Value, json};

use super::{HudSet, HudWidget};
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::camera::zoom::ZoomView;
use crate::campaign::provinces::data::ProvinceMap;
use crate::campaign::provinces::modes::{MapMode, MapModeState, ProvinceView};
use crate::campaign::provinces::overlay::OverlayState;
use crate::campaign::provinces::picker::HoveredProvince;
use crate::campaign::provinces::picker::pick_ray;
use crate::campaign::terrain::surface::GroundHeight;
use crate::campaign::water::lanes::{SeaLaneLayer, lane_tooltip, nearest_lane};
use crate::settings::Settings;
use crate::sim::Sim;
use crate::ui::kit::richtext::RichStyle;
use crate::ui::kit::style::UiTier;
use crate::ui::kit::widgets::panel_note;
use crate::ui::kit::{UiTheme, fr, rich_text};

/// Pointer movement (logical px) that restarts the wait.
const STILL_PX: f32 = 4.0;

/// French word of a stance key (`_relation_hover`).
pub fn stance_text(stance: &str) -> &'static str {
    match stance {
        "self" => "vos terres",
        "war" => "en guerre",
        "truce" => "trêve",
        "peace" | "neutral" => "neutre",
        "alliance" | "ally" => "allié",
        "vassal" => "vassal",
        "suzerain" => "suzerain",
        "agreement" => "en paix, avec un accord",
        "tension" => "tensions (hostilité, embargo ou intrusion)",
        _ => "",
    }
}

/// Diplomacy hover seen from a chosen faction (`_focus_hover_text`, DZ).
pub fn focus_stance_text(stance: &str, faction_name: &str) -> String {
    let template = match stance {
        "self" => "terres de %s",
        "war" => "en guerre contre %s",
        "ally" | "alliance" => "allié de %s",
        "agreement" => "en paix avec %s, avec un accord",
        "neutral" | "peace" | "truce" => "neutre envers %s",
        "tension" => "tensions avec %s",
        "vassal" | "suzerain" => "vassal ou suzerain de %s",
        _ => "",
    };
    template.replace("%s", faction_name)
}

/// Text added to the name of the hovered province in the current map mode ("" in political).
pub fn mode_text(
    mode: MapMode,
    view: &ProvinceView,
    index: u16,
    focus_name: Option<&str>,
) -> String {
    let lens = view.lens.get(usize::from(index)).copied().flatten();
    match mode {
        MapMode::Political | MapMode::Terrain => String::new(),
        MapMode::Diplomacy => {
            let stance = view.stance_of(view.controller_of(index));
            match focus_name {
                Some(name) => focus_stance_text(stance, name),
                None => stance_text(stance).to_owned(),
            }
        }
        MapMode::Religion => religion_text(view, index),
        MapMode::Loyalty => {
            view.cell(index)
                .map_or_else(String::new, |cell| match cell.vassal_loyalty {
                    None => "pas un vassal".to_owned(),
                    Some(loyalty) => {
                        format!("loyauté {loyalty} envers {}", view.name_of(&cell.suzerain))
                    }
                })
        }
        MapMode::Supply => view.cell(index).map_or_else(String::new, |cell| {
            let change = cell.supply_change;
            if change >= 0 {
                format!("ravitaillement +{change} par saison")
            } else {
                format!("attrition : ravitaillement {change} par saison")
            }
        }),
        MapMode::Claims => view.cell(index).map_or_else(String::new, |cell| {
            match cell.claim.as_str() {
                "ours" => "vous la revendiquez",
                "against_us" => "revendiquée contre vous",
                "contested" => "revendiquée par vous et par un autre",
                _ => "",
            }
            .to_owned()
        }),
        MapMode::Feudal => feudal_text(view, index),
        MapMode::Unrest => lens.map_or_else(String::new, |(_, _, unrest)| {
            format!("mécontentement {} %", unrest.round() as i64)
        }),
        MapMode::Wealth => lens.map_or_else(String::new, |(income, _, _)| {
            format!(
                "{} livres par saison (impôt de base)",
                fr::digits(income.round() as i64)
            )
        }),
        MapMode::Population => lens.map_or_else(String::new, |(_, population, _)| {
            format!("{} habitants", fr::digits(population.round() as i64))
        }),
    }
}

/// Religion mode: faith, heresy share, conversion under way (`_religion_hover`, ADR 0326).
fn religion_text(view: &ProvinceView, index: u16) -> String {
    let Some(faith) = view.cell(index).and_then(|c| c.religion.as_ref()) else {
        return String::new();
    };
    let mut text = faith.religion_name.clone();
    if faith.heresy > 0 {
        text.push_str(&format!(", hérésie {} %", faith.heresy));
    }
    if faith.conversion_progress > 0 {
        text.push_str(&format!(
            ", conversion vers {} : {} %",
            faith.conversion_to_name, faith.conversion_progress
        ));
    }
    text
}

/// Feudal mode: « tenu par Bourgogne, sous France ; double allégeance (fief de Y) ».
fn feudal_text(view: &ProvinceView, index: u16) -> String {
    let Some(cell) = view.cell(index).and_then(|c| c.feudal.as_ref()) else {
        return String::new();
    };
    let mut text = if cell.holder == cell.sovereign || cell.holder.is_empty() {
        format!("royaume de {}", view.name_of(&cell.sovereign))
    } else {
        format!(
            "tenu par {}, sous {}",
            view.name_of(&cell.holder),
            view.name_of(&cell.sovereign)
        )
    };
    if !cell.second_lord.is_empty() {
        text.push_str(&format!(
            " ; double allégeance (fief de {})",
            view.name_of(&cell.second_lord)
        ));
    }
    text
}

#[derive(Resource, Default)]
pub struct Hover {
    cursor: Vec2,
    still: f32,
    shown_for: Option<u16>,
    pub shown: u32,
    pub text: String,
}

#[derive(Component)]
struct HoverNote;

/// Tooltip of the sea lane under the pointer (`_update_sea_lane_hover`): shown at once, without
/// the province delay; leaving the lane gives the province hover back.
#[derive(Resource, Default, Debug)]
pub struct LaneHover {
    pub text: Option<String>,
    /// The note currently shows a lane.
    shown: bool,
}

/// Lane picking radius (map units) at a camera distance (Godot `distance * 0.008`, 2..30).
pub fn lane_threshold(camera_distance: f32) -> f32 {
    (camera_distance * 0.008).clamp(2.0, 30.0)
}

#[allow(clippy::too_many_arguments)]
fn pick_lane_hover(
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    map: Option<Res<ProvinceMap>>,
    ground: GroundHeight,
    layer: Option<Res<SeaLaneLayer>>,
    layers: Option<Res<super::layers::MapLayers>>,
    drawn: Option<Res<super::layers::DrawnRoutes>>,
    zoom: Option<Res<ZoomView>>,
    widgets: Query<&bevy::picking::hover::Hovered, With<HudWidget>>,
    mut out: ResMut<LaneHover>,
) {
    out.text = None;
    let Some(map) = map else { return };
    let trade_on = layers.is_some_and(|l| l.trade);
    let lanes = layer.as_deref().filter(|l| !l.views.is_empty());
    if widgets.iter().any(|h| h.get()) || (lanes.is_none() && !trade_on) {
        return;
    }
    let Some(cursor) = windows.single().ok().and_then(Window::cursor_position) else {
        return;
    };
    let Some((camera, transform)) = cameras.iter().find(|(camera, _)| camera.is_active) else {
        return;
    };
    let Ok(ray) = camera.viewport_to_world(transform, cursor) else {
        return;
    };
    let size = Vec2::new(map.width as f32, map.height as f32);
    let height = |x: f32, z: f32| ground.at(x, z).unwrap_or(0.0);
    let Some(hit) = pick_ray(ray.origin, *ray.direction, size, &height) else {
        return;
    };
    let distance = zoom.map_or(0.0, |z| z.distance);
    let point = Vec2::new(hit.x, hit.z);
    // A trade route under the pointer wins over the sea lane beneath it (campaign_map.gd).
    if let (true, Some(drawn)) = (trade_on, drawn.as_deref()) {
        let reach = (distance * 0.01).clamp(3.0, 40.0);
        if let Some(route) = super::layers::nearest_route(&drawn.0, point, reach) {
            out.text = Some(route.tooltip.clone());
            return;
        }
    }
    if let Some(layer) = lanes {
        out.text = nearest_lane(layer, point, lane_threshold(distance)).map(lane_tooltip);
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Hover>()
        .init_resource::<LaneHover>()
        .register_dump(dump_hover)
        .add_systems(OnEnter(AppState::Campaign), spawn_note)
        .add_systems(
            Update,
            (pick_lane_hover, update_hover).chain().in_set(HudSet::Ui),
        );
}

fn spawn_note(mut commands: Commands, theme: Res<UiTheme>) {
    commands
        .spawn((
            Name::new("hud:hover_note"),
            HoverNote,
            DespawnOnExit(AppState::Campaign),
            UiTier::Tooltip.index(),
            panel_note(&theme),
            Pickable::IGNORE,
        ))
        .insert(Node {
            position_type: PositionType::Absolute,
            max_width: Val::Px(320.0),
            display: Display::None,
            padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
            ..default()
        });
}

#[allow(clippy::too_many_arguments)]
fn update_hover(
    mut commands: Commands,
    mut hover: ResMut<Hover>,
    time: Res<Time>,
    window: Query<&Window, With<PrimaryWindow>>,
    hovered: Res<HoveredProvince>,
    overlay: Option<Res<OverlayState>>,
    map: Option<Res<ProvinceMap>>,
    modes: Option<Res<MapModeState>>,
    settings: Option<Res<Settings>>,
    sim: Res<Sim>,
    mut lane: ResMut<LaneHover>,
    widgets: Query<&bevy::picking::hover::Hovered, With<HudWidget>>,
    mut note: Query<(Entity, &mut Node), With<HoverNote>>,
    assets: Res<AssetServer>,
    theme: Res<UiTheme>,
) {
    let Ok((note_entity, mut node)) = note.single_mut() else {
        return;
    };
    let delay = settings.map_or(1.5, |s| s.decor_hover_delay);
    let cursor = window.single().ok().and_then(Window::cursor_position);
    let over_widget = widgets.iter().any(|h| h.get());
    let (Some(cursor), Some(overlay), Some(map)) = (cursor, overlay, map) else {
        node.display = Display::None;
        hover.shown_for = None;
        return;
    };
    if let Some(text) = lane.text.clone() {
        commands.entity(note_entity).despawn_children();
        commands.entity(note_entity).with_children(|inner| {
            inner.spawn(rich_text(
                &theme,
                &assets,
                &text,
                &RichStyle::caption(&theme),
            ));
        });
        node.left = Val::Px(cursor.x + 18.0);
        node.top = Val::Px(cursor.y + 18.0);
        node.display = Display::Flex;
        hover.shown_for = None;
        hover.cursor = cursor;
        hover.still = 0.0;
        if !lane.shown {
            hover.shown += 1;
        }
        lane.shown = true;
        hover.text = text;
        return;
    }
    if lane.shown {
        lane.shown = false;
        node.display = Display::None;
    }
    if cursor.distance(hover.cursor) > STILL_PX || hovered.index != hover.shown_for.unwrap_or(0) {
        hover.cursor = cursor;
        hover.still = 0.0;
        if hover.shown_for.is_some() {
            hover.shown_for = None;
            node.display = Display::None;
        }
    } else {
        hover.still += time.delta_secs();
    }
    if delay <= 0.0 || over_widget || hovered.index == 0 || hover.shown_for.is_some() {
        if over_widget || hovered.index == 0 {
            node.display = Display::None;
            hover.shown_for = None;
        }
        return;
    }
    if hover.still < delay {
        return;
    }
    let Some(province) = map.province(hovered.index) else {
        return;
    };
    let mode = modes.as_ref().map_or(MapMode::Political, |m| m.mode);
    let lord = {
        let id = overlay.view.controller_of(hovered.index);
        match (sim.data.as_ref(), data_model::FactionId::new(id)) {
            (Some(data), Ok(faction)) if !id.is_empty() => data.faction_name(&faction),
            _ => String::new(),
        }
    };
    let focus_name = modes
        .and_then(|m| m.focus_faction.clone())
        .and_then(|id| data_model::FactionId::new(id).ok())
        .zip(sim.data.as_ref())
        .map(|(id, data)| {
            crate::campaign::parchment::realms::short_name(&data.faction_name(&id)).to_owned()
        });
    let extra = mode_text(mode, &overlay.view, hovered.index, focus_name.as_deref());
    let mut bbcode = format!("[b]{}[/b]", province.name);
    if !lord.is_empty() {
        bbcode.push_str(&format!("\n{lord}"));
    }
    if !extra.is_empty() {
        bbcode.push_str(&format!("\n{extra}"));
    }
    commands.entity(note_entity).despawn_children();
    commands.entity(note_entity).with_children(|inner| {
        inner.spawn(rich_text(
            &theme,
            &assets,
            &bbcode,
            &RichStyle::caption(&theme),
        ));
    });
    node.left = Val::Px(cursor.x + 18.0);
    node.top = Val::Px(cursor.y + 18.0);
    node.display = Display::Flex;
    hover.shown_for = Some(hovered.index);
    hover.shown += 1;
    hover.text = bbcode;
}

fn dump_hover(world: &World) -> (String, Value) {
    let hover = world.resource::<Hover>();
    (
        "hover_text".to_owned(),
        json!({ "shown": hover.shown, "visible": hover.shown_for, "text": hover.text,
            "lane": world.get_resource::<LaneHover>().and_then(|l| l.text.clone()) }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view() -> ProvinceView {
        let mut view = ProvinceView {
            owners: vec![String::new(), "fac_england".into()],
            controllers: vec![String::new(), String::new()],
            viewer: "fac_france".into(),
            lens: vec![None, Some((1234.4, 56789.0, 17.6))],
            ..Default::default()
        };
        view.stances.insert("fac_england".into(), "war");
        view
    }

    #[test]
    fn lane_threshold_follows_the_zoom() {
        assert_eq!(lane_threshold(0.0), 2.0);
        assert_eq!(lane_threshold(1000.0), 8.0);
        assert_eq!(lane_threshold(1e6), 30.0);
    }

    #[test]
    fn political_adds_nothing() {
        assert_eq!(mode_text(MapMode::Political, &view(), 1, None), "");
    }

    #[test]
    fn modes_read_their_figure() {
        let view = view();
        assert_eq!(mode_text(MapMode::Diplomacy, &view, 1, None), "en guerre");
        assert_eq!(
            mode_text(MapMode::Unrest, &view, 1, None),
            "mécontentement 18 %"
        );
        assert!(mode_text(MapMode::Wealth, &view, 1, None).contains("livres par saison"));
        assert!(mode_text(MapMode::Population, &view, 1, None).ends_with("habitants"));
        assert_eq!(mode_text(MapMode::Unrest, &view, 0, None), "");
    }

    #[test]
    fn chosen_faction_view_names_the_faction() {
        assert_eq!(
            focus_stance_text("self", "Bourgogne"),
            "terres de Bourgogne"
        );
        assert_eq!(
            focus_stance_text("war", "Bourgogne"),
            "en guerre contre Bourgogne"
        );
        assert_eq!(focus_stance_text("?", "Bourgogne"), "");
    }
}
