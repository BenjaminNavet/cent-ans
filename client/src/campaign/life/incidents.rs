//! Incidents on the map (FK5, `life_folk/incident_markers.gd`): every pending decision of the
//! player with the `map` presentation gets a wax seal above its province, with the turns left;
//! a click opens the decision of the chronicle on this one and slides the camera to the province.
//! The scene of the incident (FK4, `scenes.rs`) is clickable too: an invisible disc over its
//! footprint does the same gesture. `CENT_ANS_FOLK_OFF=incidents` turns the seals off (the
//! chronicle then reopens these decisions as dialogs, like `--folk-off=incidents`).
//!
//! Differences with Godot: the seal is a plain wax disc with the number of turns (the wax-seal
//! painting and the `hud_chronicle_decision` pictogram of `HudStyle` belong to the UI kit), shown
//! at every zoom without the `Signes` layer filter (`MapReadability`).

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;
use bevy::ui_widgets::Button;
use sim_campaign::chronicle::DecisionView;

use super::scenes::StagedScenes;
use super::stream::LifeView;
use crate::app::AppState;
use crate::campaign::camera::FocusCamera;
use crate::campaign::hud::decision::{OpenDecision, pending};
use crate::campaign::provinces::data::ProvinceMap;
use crate::campaign::terrain::surface::GroundHeight;
use crate::sim::Sim;
use crate::ui::kit::ButtonPressed;
use crate::ui::kit::tooltip::Tooltip;

pub const MARKER_SIZE: f32 = 34.0;
/// Lift above the centre of the province (screen px = `MARKER_SIZE * LIFT`).
pub const LIFT: f32 = 0.9;
/// Screen radius (px) of the click disc of a scene: bounds that keep it reachable from afar and
/// do not cover the map close up.
pub const SCENE_HIT_MIN: f32 = 22.0;
pub const SCENE_HIT_MAX: f32 = 130.0;

const WAX: Color = Color::srgb(0.56, 0.1, 0.08);
const WAX_LIGHT: Color = Color::srgb(0.68, 0.16, 0.12);
const PARCHMENT: Color = Color::srgb(0.95, 0.9, 0.78);
const RUBRIC: Color = Color::srgb(0.62, 0.13, 0.08);
const GOLD: Color = Color::srgb(0.72, 0.58, 0.2);

#[derive(Component)]
pub struct IncidentSeal {
    pub id: u32,
    pub world: Vec3,
    pub turns_left: u32,
}

#[derive(Component)]
pub struct IncidentSceneHit {
    pub id: u32,
    pub world: Vec3,
    pub footprint: f32,
}

#[derive(Component)]
pub struct SealBadge(pub u32);

#[derive(Resource, Default)]
pub struct IncidentStats {
    pub seals: usize,
    pub scene_hits: usize,
    pub visible_seals: usize,
    pub clicks: usize,
}

/// True when the seals are turned off by `CENT_ANS_FOLK_OFF=incidents`.
pub fn seals_off() -> bool {
    std::env::var("CENT_ANS_FOLK_OFF").is_ok_and(|v| v.split(',').any(|s| s.trim() == "incidents"))
}

/// Decision laid on the map: `map` presentation and a known province.
pub fn is_map_decision(d: &DecisionView) -> bool {
    matches!(d.presentation, data_model::EventPresentation::Map) && d.province.is_some()
}

/// Bubble of an incident: title, province, deadline, gesture.
pub fn seal_tooltip(d: &DecisionView) -> String {
    let mut lines = vec![format!("[b]{}[/b]", d.title)];
    if !d.province_name.is_empty() {
        lines.push(d.province_name.clone());
    }
    if d.expires_in <= 1 {
        lines.push(
            "[color=#9e2114]À décider ce tour-ci, sinon le conseil tranche.[/color]".to_string(),
        );
    } else {
        lines.push(format!(
            "Encore {} tours pour décider, sinon le conseil tranche.",
            d.expires_in
        ));
    }
    lines.push("[i]Clic : ouvrir la décision.[/i]".to_string());
    lines.join("\n")
}

/// Screen radius of the click disc of a scene from the projected footprint.
pub fn hit_radius(projected: f32) -> f32 {
    projected.clamp(SCENE_HIT_MIN, SCENE_HIT_MAX)
}

fn spawn_seal(commands: &mut Commands, d: &DecisionView, world: Vec3) {
    let turns = d.expires_in.max(1);
    commands
        .spawn((
            Name::new(format!("Incident_{}", d.id)),
            IncidentSeal {
                id: d.id,
                world,
                turns_left: turns,
            },
            Button,
            Hovered::default(),
            Tooltip::new(&seal_tooltip(d)),
            DespawnOnExit(AppState::Campaign),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(MARKER_SIZE),
                height: Val::Px(MARKER_SIZE),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(2.0)),
                border_radius: BorderRadius::MAX,
                ..default()
            },
            BackgroundColor(WAX),
            BorderColor::all(Color::srgb(0.35, 0.05, 0.04)),
            GlobalZIndex(-1),
            Visibility::Hidden,
        ))
        .with_children(|seal| {
            seal.spawn((
                SealBadge(d.id),
                Text::new(turns.to_string()),
                TextFont {
                    font_size: bevy::text::FontSize::Px(15.0),
                    ..default()
                },
                TextColor(if turns <= 1 { PARCHMENT } else { GOLD }),
            ));
        });
}

fn spawn_scene_hit(commands: &mut Commands, d: &DecisionView, world: Vec3, footprint: f32) {
    commands.spawn((
        Name::new(format!("IncidentScene_{}", d.id)),
        IncidentSceneHit {
            id: d.id,
            world,
            footprint,
        },
        Button,
        Hovered::default(),
        Tooltip::new(&seal_tooltip(d)),
        DespawnOnExit(AppState::Campaign),
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(SCENE_HIT_MIN * 2.0),
            height: Val::Px(SCENE_HIT_MIN * 2.0),
            border_radius: BorderRadius::MAX,
            ..default()
        },
        BackgroundColor(Color::NONE),
        GlobalZIndex(-2),
        Visibility::Hidden,
    ));
}

/// One seal per pending map decision (refreshed twice a second: decisions change at the end of a
/// turn or when one is taken).
pub fn refresh_incidents(
    mut commands: Commands,
    sim: Res<Sim>,
    map: Option<Res<ProvinceMap>>,
    scenes: Res<StagedScenes>,
    ground: GroundHeight,
    mut seals: Query<(Entity, &mut IncidentSeal)>,
    hits: Query<(Entity, &IncidentSceneHit)>,
    mut timer: Local<f32>,
    time: Res<Time>,
    mut stats: ResMut<IncidentStats>,
) {
    *timer -= time.delta_secs();
    if *timer > 0.0 {
        return;
    }
    *timer = 0.5;
    let Some(map) = map else { return };
    let decisions: Vec<DecisionView> = if seals_off() {
        Vec::new()
    } else {
        pending(&sim).into_iter().filter(is_map_decision).collect()
    };
    let mut seen = std::collections::HashSet::new();
    let mut seen_hits = std::collections::HashSet::new();
    for d in &decisions {
        let Some(province) = d.province.as_ref() else {
            continue;
        };
        let Some(feature) = map.province(map.index_of_id(province.as_str())) else {
            continue;
        };
        let c = feature.centroid;
        let world = Vec3::new(c[0], ground.at(c[0], c[1]).unwrap_or(0.0), c[1]);
        seen.insert(d.id);
        if let Some((_, mut seal)) = seals.iter_mut().find(|(_, s)| s.id == d.id) {
            seal.world = world;
            seal.turns_left = d.expires_in.max(1);
        } else {
            spawn_seal(&mut commands, d, world);
        }
        if let Some(scene) = scenes
            .scenes
            .iter()
            .find(|s| s.province == province.as_str())
        {
            let at = Vec3::new(
                scene.center.x,
                ground.at(scene.center.x, scene.center.y).unwrap_or(0.0),
                scene.center.y,
            );
            seen_hits.insert(d.id);
            if !hits.iter().any(|(_, h)| h.id == d.id) {
                spawn_scene_hit(&mut commands, d, at, scene.footprint);
            }
        }
    }
    for (entity, seal) in &seals {
        if !seen.contains(&seal.id) {
            commands.entity(entity).despawn();
        }
    }
    for (entity, hit) in &hits {
        if !seen_hits.contains(&hit.id) {
            commands.entity(entity).despawn();
        }
    }
    stats.seals = seen.len();
    stats.scene_hits = seen_hits.len();
}

/// Keeps the badge text at the turns left.
pub fn refresh_badges(
    seals: Query<&IncidentSeal, Changed<IncidentSeal>>,
    mut badges: Query<(&SealBadge, &mut Text, &mut TextColor)>,
) {
    for seal in &seals {
        for (badge, mut text, mut color) in &mut badges {
            if badge.0 == seal.id {
                *text = Text::new(seal.turns_left.to_string());
                color.0 = if seal.turns_left <= 1 {
                    PARCHMENT
                } else {
                    GOLD
                };
            }
        }
    }
}

/// Places the seals above their province and the click discs over their scene.
pub fn place_incidents(
    camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut seals: Query<
        (&IncidentSeal, &mut Node, &mut Visibility, &mut BorderColor),
        Without<IncidentSceneHit>,
    >,
    mut hits: Query<(&IncidentSceneHit, &mut Node, &mut Visibility), Without<IncidentSeal>>,
    mut stats: ResMut<IncidentStats>,
) {
    let Some((camera, transform)) = camera.iter().next() else {
        return;
    };
    let mut visible_ids = std::collections::HashSet::new();
    let mut visible = 0;
    for (seal, mut node, mut visibility, mut border) in &mut seals {
        match camera.world_to_viewport(transform, seal.world) {
            Ok(screen) => {
                node.left = Val::Px(screen.x - MARKER_SIZE * 0.5);
                node.top = Val::Px(screen.y - MARKER_SIZE * 0.5 - MARKER_SIZE * LIFT);
                *visibility = Visibility::Inherited;
                *border = BorderColor::all(if seal.turns_left <= 1 {
                    RUBRIC
                } else {
                    Color::srgb(0.35, 0.05, 0.04)
                });
                visible_ids.insert(seal.id);
                visible += 1;
            }
            Err(_) => *visibility = Visibility::Hidden,
        }
    }
    stats.visible_seals = visible;
    for (hit, mut node, mut visibility) in &mut hits {
        let projected = camera.world_to_viewport(transform, hit.world).ok().zip(
            camera
                .world_to_viewport(transform, hit.world + Vec3::X * hit.footprint)
                .ok(),
        );
        match projected {
            Some((center, edge)) if visible_ids.contains(&hit.id) => {
                let r = hit_radius(center.distance(edge));
                node.width = Val::Px(r * 2.0);
                node.height = Val::Px(r * 2.0);
                node.left = Val::Px(center.x - r);
                node.top = Val::Px(center.y - r);
                *visibility = Visibility::Inherited;
            }
            _ => *visibility = Visibility::Hidden,
        }
    }
}

/// Id of the decision of a seal or scene disc from the name of the pressed button.
pub fn decision_of_button(name: &str) -> Option<u32> {
    name.strip_prefix("IncidentScene_")
        .or_else(|| name.strip_prefix("Incident_"))
        .and_then(|n| n.parse().ok())
}

/// Click on a seal or on the disc of a scene (kit `ButtonPressed` by the entity name): opens the
/// decision and slides to the province.
pub fn click_incidents(
    mut pressed: MessageReader<ButtonPressed>,
    seals: Query<&IncidentSeal>,
    hits: Query<&IncidentSceneHit>,
    mut open: MessageWriter<OpenDecision>,
    mut focus: MessageWriter<FocusCamera>,
    mut stats: ResMut<IncidentStats>,
) {
    for press in pressed.read() {
        let Some(id) = decision_of_button(&press.name) else {
            continue;
        };
        let world = seals
            .iter()
            .find(|s| s.id == id)
            .map(|s| s.world)
            .or_else(|| hits.iter().find(|h| h.id == id).map(|h| h.world));
        let Some(world) = world else { continue };
        stats.clicks += 1;
        focus.write(FocusCamera::Point(Vec2::new(world.x, world.z)));
        open.write(OpenDecision::Id(id));
    }
}

/// Hover colour of the seal.
pub fn hover_seals(
    mut seals: Query<(&Hovered, &mut BackgroundColor), (With<IncidentSeal>, Changed<Hovered>)>,
) {
    for (hovered, mut bg) in &mut seals {
        bg.0 = if hovered.get() { WAX_LIGHT } else { WAX };
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<IncidentStats>().add_systems(
        Update,
        (
            refresh_incidents,
            refresh_badges,
            place_incidents,
            hover_seals,
            click_incidents.run_if(crate::campaign::hud::flow::orders_open),
        )
            .chain()
            .run_if(in_state(AppState::Campaign))
            .run_if(resource_exists::<LifeView>),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use data_model::{EventId, EventPresentation, ProvinceId};

    fn decision(
        presentation: EventPresentation,
        province: Option<&str>,
        expires_in: u32,
    ) -> DecisionView {
        DecisionView {
            id: 7,
            event: EventId::new("evt_test").expect("id"),
            title: "Une émeute".into(),
            text: String::new(),
            historical: false,
            options: Vec::new(),
            expires_in,
            province: province.map(|p| ProvinceId::new(p).expect("id")),
            province_name: "Normandie".into(),
            presentation,
        }
    }

    #[test]
    fn only_map_decisions_with_a_province_are_incidents() {
        assert!(is_map_decision(&decision(
            EventPresentation::Map,
            Some("prov_x"),
            2
        )));
        assert!(!is_map_decision(&decision(EventPresentation::Map, None, 2)));
        assert!(!is_map_decision(&decision(
            EventPresentation::Dialog,
            Some("prov_x"),
            2
        )));
    }

    #[test]
    fn tooltip_warns_on_the_last_turn() {
        let last = seal_tooltip(&decision(EventPresentation::Map, Some("prov_x"), 1));
        assert!(last.contains("À décider ce tour-ci"));
        let more = seal_tooltip(&decision(EventPresentation::Map, Some("prov_x"), 3));
        assert!(more.contains("Encore 3 tours"));
        assert!(more.contains("Normandie") && more.contains("Une émeute"));
    }

    #[test]
    fn button_names_give_the_decision() {
        assert_eq!(decision_of_button("Incident_12"), Some(12));
        assert_eq!(decision_of_button("IncidentScene_3"), Some(3));
        assert_eq!(decision_of_button("Incident_x"), None);
        assert_eq!(decision_of_button("end_turn"), None);
    }

    #[test]
    fn hit_radius_is_bounded() {
        assert_eq!(hit_radius(3.0), SCENE_HIT_MIN);
        assert_eq!(hit_radius(5000.0), SCENE_HIT_MAX);
        assert_eq!(hit_radius(60.0), 60.0);
    }
}
