//! Colonies of the open province on the map (port of `province_mates_overlay.gd`, CO-B): while
//! the province panel shows a colony (Buildings or Colonies tab) and the province holds several,
//! every colony gets a parchment pastille (ring: rubric for the city, gold for the selected
//! one, the kind's icon inside) tied to the city by a dotted ink line laid on the relief, and a
//! cartouche "Province de X — n colonies" tops the map. Rendering only, no rule.
//!
//! The pastilles are UI nodes re-projected every frame from the colony's map position; the
//! dotted lines are gizmos on the displayed ground. Dump section `province_mates`.

use crate::ui::kit::viewport_to_ui;
use bevy::picking::Pickable;
use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;
use bevy::ui::UiScale;
use serde_json::json;

use super::ProvincePanel;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::camera::CampaignCamera;
use crate::campaign::markers::places::Places;
use crate::campaign::terrain::surface::GroundHeight;
use crate::sim::Sim;
use crate::ui::kit::icons::icon;
use crate::ui::kit::style::{self, UiTier, UiType};
use crate::ui::kit::{UiTheme, typed};

/// Pastille diameter in logical pixels (Godot: 56 px texture at a fixed screen size).
const PASTILLE: f32 = 34.0;
/// Step and length of the dotted line, in map metres (`DOT_STEP`, `DOT_LEN`).
const DOT_STEP: f32 = 3.0;
const DOT_LEN: f32 = 1.2;
/// Lift of the line above the relief.
const LINE_LIFT: f32 = 0.4;
/// Lift of the pastille above the ground (`LIFT`).
const PASTILLE_LIFT: f32 = 6.0;

pub fn plugin(app: &mut App) {
    app.init_resource::<Mates>()
        .add_systems(
            Update,
            (rebuild, place_pastilles, draw_lines)
                .chain()
                .run_if(in_state(AppState::Campaign)),
        )
        .register_dump(dump);
}

/// One colony of the shown province.
#[derive(Debug, Clone, PartialEq)]
pub struct Mate {
    pub id: String,
    pub kind: String,
    pub px: Vec2,
    pub city: bool,
    pub selected: bool,
}

/// What the overlay shows now.
#[derive(Resource, Default)]
pub struct Mates {
    province: String,
    pub rows: Vec<Mate>,
    pub title: String,
    key: String,
    root: Option<Entity>,
}

#[derive(Component)]
struct Pastille(usize);

/// Ablative of a place name after "Province": « de Paris », « d'Orléans », « du Mans ».
pub fn de(name: &str) -> String {
    let lower = name.to_lowercase();
    if let Some(rest) = name.strip_prefix("Le ") {
        return format!("du {rest}");
    }
    if let Some(rest) = name.strip_prefix("Les ") {
        return format!("des {rest}");
    }
    if let Some(rest) = name.strip_prefix("La ") {
        return format!("de la {rest}");
    }
    let first = lower.chars().next().unwrap_or('x');
    if "aeiouyhàâäéèêëîïôöùûüœ".contains(first) {
        format!("d'{name}")
    } else {
        format!("de {name}")
    }
}

/// Cartouche text (`cartouche_text`).
pub fn cartouche(province_name: &str, count: usize) -> String {
    format!(
        "Province {} — {count} colonie{}",
        de(province_name),
        if count > 1 { "s" } else { "" }
    )
}

/// Colonies of `province`, the city first; empty when it holds fewer than two.
pub fn mates_of(places: &Places, province: &str, selected: Option<&str>) -> Vec<Mate> {
    let mut rows: Vec<Mate> = places
        .places
        .iter()
        .filter(|p| p.province == province)
        .map(|p| Mate {
            id: p.id.clone(),
            kind: p.kind.clone(),
            px: p.px,
            city: p.kind == "city",
            selected: selected == Some(p.id.as_str()),
        })
        .collect();
    if rows.len() < 2 {
        return Vec::new();
    }
    rows.sort_by_key(|m| !m.city);
    // No colony chosen yet: the panel shows the city.
    if selected.is_none() {
        rows[0].selected = true;
    }
    rows
}

fn kind_icon(kind: &str) -> &'static str {
    match kind {
        "castle" => "bld_castle",
        "abbey" => "bld_abbey",
        "village" | "hamlet" => "delapouite-village",
        _ => "bld_town_hall",
    }
}

fn rebuild(
    mut commands: Commands,
    mut mates: ResMut<Mates>,
    panel: Res<ProvincePanel>,
    places: Res<Places>,
    sim: Res<Sim>,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
) {
    let wanted = panel
        .mates_view()
        .map(|(province, selected)| (province.to_owned(), selected.map(str::to_owned)));
    let rows = wanted
        .as_ref()
        .map(|(province, selected)| mates_of(&places, province, selected.as_deref()))
        .unwrap_or_default();
    let key = format!(
        "{}|{}",
        wanted.as_ref().map_or("", |(p, _)| p.as_str()),
        rows.iter()
            .map(|m| format!("{}{}", m.id, u8::from(m.selected)))
            .collect::<Vec<_>>()
            .join(",")
    );
    if key == mates.key {
        return;
    }
    mates.key = key;
    if let Some(root) = mates.root.take() {
        commands.entity(root).despawn();
    }
    mates.rows = rows;
    mates.province.clear();
    mates.title.clear();
    if mates.rows.is_empty() {
        return;
    }
    let province = wanted.map(|(p, _)| p).unwrap_or_default();
    let name = sim
        .data
        .as_ref()
        .and_then(|d| {
            d.provinces
                .get(&data_model::ProvinceId::new(&province).ok()?)
                .map(|p| p.name.display.clone())
        })
        .unwrap_or_else(|| province.clone());
    mates.title = cartouche(&name, mates.rows.len());
    mates.province = province;
    let rows = mates.rows.clone();
    let title = mates.title.clone();
    let root = commands
        .spawn((
            Name::new("ProvinceMates"),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexStart,
                padding: UiRect::top(Val::Px(56.0)),
                ..default()
            },
            Pickable::IGNORE,
            UiTier::Hud.index(),
            DespawnOnExit(AppState::Campaign),
        ))
        .with_children(|root| {
            root.spawn((
                Name::new("mates:cartouche"),
                Node {
                    padding: UiRect::axes(Val::Px(14.0), Val::Px(6.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(style::PARCHMENT_LIGHT),
                BorderColor::all(style::INK_SOFT),
                Pickable::IGNORE,
                children![typed(&theme, &title, UiType::Body)],
            ));
            for (index, mate) in rows.iter().enumerate() {
                let ring = if mate.selected {
                    style::GOLD
                } else if mate.city {
                    style::RUBRIC
                } else {
                    style::INK_SOFT
                };
                let size = PASTILLE
                    * if mate.selected {
                        1.45
                    } else if mate.city {
                        1.2
                    } else {
                        1.0
                    };
                root.spawn((
                    Name::new(format!("mates:pastille:{}", mate.id)),
                    Pastille(index),
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Px(size),
                        height: Val::Px(size),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(if mate.selected || mate.city {
                            3.0
                        } else {
                            2.0
                        })),
                        display: Display::None,
                        border_radius: BorderRadius::MAX,
                        ..default()
                    },
                    BackgroundColor(style::PARCHMENT_LIGHT),
                    BorderColor::all(ring),
                    Pickable::IGNORE,
                    children![icon(&theme, &assets, kind_icon(&mate.kind), size * 0.55)],
                ));
            }
        })
        .id();
    mates.root = Some(root);
}

fn ground_at(ground: &GroundHeight, px: Vec2) -> f32 {
    ground.at(px.x, px.y).unwrap_or(0.0)
}

fn place_pastilles(
    mates: Res<Mates>,
    ground: GroundHeight,
    camera: Query<(&Camera, &GlobalTransform), With<CampaignCamera>>,
    mut pastilles: Query<(&Pastille, &mut Node)>,
    ui_scale: Res<UiScale>,
) {
    let Ok((camera, transform)) = camera.single() else {
        return;
    };
    for (pastille, mut node) in &mut pastilles {
        let Some(mate) = mates.rows.get(pastille.0) else {
            continue;
        };
        let world = Vec3::new(
            mate.px.x,
            ground_at(&ground, mate.px) + PASTILLE_LIFT,
            mate.px.y,
        );
        match camera.world_to_viewport(transform, world) {
            Ok(at) => {
                let at = viewport_to_ui(at, &ui_scale);
                let half = match node.width {
                    Val::Px(w) => w * 0.5,
                    _ => 0.0,
                };
                node.left = Val::Px(at.x - half);
                node.top = Val::Px(at.y - half);
                node.display = Display::Flex;
            }
            Err(_) => node.display = Display::None,
        }
    }
}

/// Dotted ink line from the city to each other colony, hugging the relief.
fn draw_lines(mates: Res<Mates>, ground: GroundHeight, mut gizmos: Gizmos) {
    let Some(hub) = mates.rows.iter().find(|m| m.city).or(mates.rows.first()) else {
        return;
    };
    for mate in mates.rows.iter().filter(|m| m.id != hub.id) {
        for (a, b) in dotted(hub.px, mate.px) {
            let lift = |p: Vec2| Vec3::new(p.x, ground_at(&ground, p) + LINE_LIFT, p.y);
            gizmos.line(lift(a), lift(b), style::INK);
        }
    }
}

/// Dash segments (`_dotted`) between two map points.
pub fn dotted(from: Vec2, to: Vec2) -> Vec<(Vec2, Vec2)> {
    let length = from.distance(to);
    let steps = ((length / DOT_STEP) as usize).max(1);
    (0..steps)
        .map(|i| {
            let a = i as f32 / steps as f32;
            let b = (a + DOT_LEN / length.max(0.001)).min(1.0);
            (from.lerp(to, a), from.lerp(to, b))
        })
        .collect()
}

fn dump(world: &World) -> (String, serde_json::Value) {
    let mates = world.resource::<Mates>();
    (
        "province_mates".to_owned(),
        json!({
            "province": mates.province,
            "title": mates.title,
            "count": mates.rows.len(),
            "selected": mates.rows.iter().find(|m| m.selected).map(|m| m.id.as_str()),
            "cities": mates.rows.iter().filter(|m| m.city).count(),
            "ids": mates.rows.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn possessive_forms() {
        assert_eq!(cartouche("Paris", 3), "Province de Paris — 3 colonies");
        assert_eq!(cartouche("Orléans", 1), "Province d'Orléans — 1 colonie");
        assert_eq!(de("Le Mans"), "du Mans");
        assert_eq!(de("Les Landes"), "des Landes");
    }

    #[test]
    fn dotted_line_covers_the_distance() {
        let dashes = dotted(Vec2::ZERO, Vec2::new(30.0, 0.0));
        assert_eq!(dashes.len(), 10);
        assert!((dashes[0].1.x - 1.2).abs() < 1e-4);
        assert_eq!(dotted(Vec2::ZERO, Vec2::new(1.0, 0.0)).len(), 1);
    }
}
