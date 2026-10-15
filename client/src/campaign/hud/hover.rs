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
use crate::campaign::provinces::data::ProvinceMap;
use crate::campaign::provinces::modes::{MapMode, MapModeState, ProvinceView};
use crate::campaign::provinces::overlay::OverlayState;
use crate::campaign::provinces::picker::HoveredProvince;
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

/// Text added to the name of the hovered province in the current map mode ("" in political).
pub fn mode_text(mode: MapMode, view: &ProvinceView, index: u16) -> String {
    let lens = view.lens.get(usize::from(index)).copied().flatten();
    match mode {
        MapMode::Political | MapMode::Terrain => String::new(),
        MapMode::Diplomacy => stance_text(view.stance_of(view.controller_of(index))).to_owned(),
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

pub fn plugin(app: &mut App) {
    app.init_resource::<Hover>()
        .register_dump(dump_hover)
        .add_systems(OnEnter(AppState::Campaign), spawn_note)
        .add_systems(Update, update_hover.in_set(HudSet::Ui));
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
    let mode = modes.map_or(MapMode::Political, |m| m.mode);
    let lord = {
        let id = overlay.view.controller_of(hovered.index);
        match (sim.data.as_ref(), data_model::FactionId::new(id)) {
            (Some(data), Ok(faction)) if !id.is_empty() => data.faction_name(&faction),
            _ => String::new(),
        }
    };
    let extra = mode_text(mode, &overlay.view, hovered.index);
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
        json!({ "shown": hover.shown, "visible": hover.shown_for, "text": hover.text }),
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
    fn political_adds_nothing() {
        assert_eq!(mode_text(MapMode::Political, &view(), 1), "");
    }

    #[test]
    fn modes_read_their_figure() {
        let view = view();
        assert_eq!(mode_text(MapMode::Diplomacy, &view, 1), "en guerre");
        assert_eq!(mode_text(MapMode::Unrest, &view, 1), "mécontentement 18 %");
        assert!(mode_text(MapMode::Wealth, &view, 1).contains("livres par saison"));
        assert!(mode_text(MapMode::Population, &view, 1).ends_with("habitants"));
        assert_eq!(mode_text(MapMode::Unrest, &view, 0), "");
    }
}
