//! Tips that follow the cursor over the battlefield. Ports game/scripts/battle/battle_terrain_tip.gd
//! (the decor under the cursor: « Verger — couvert +x %, vitesse −y % ») and battle_queue_tip.gd
//! (« File d'ordres pleine » while Shift is held, « Attaque au pas » while Alt is held over an
//! enemy), driven like `BattleScene._update_hover_cursor`.
//!
//! Presentation only: the percentages come from the core
//! ([`sim_battle::Battlefield::decor_hover_at`]), the queue bound from `QueueRules`, the cursor
//! context from [`sim_battle::BattleSim::hover_context`]. Sign and arrow (▲/▼) back the colour.
//!
//! Deviation from Godot: the three tips are built once under the HUD root and shown by one
//! system; the terrain tip hides itself over interface pieces (Godot hid it only through the
//! hover cache), and with the cinematic shot.

use bevy::picking::hover::HoverMap;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use serde_json::{Value, json};
use sim_battle::{DecorHover, HoverKind, QueueRules};

use super::{BattleHudRoot, HoveredBattleUnits};
use crate::app::AppState;
use crate::battle::orders::BattleSelection;
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::BattleCamera;
use crate::ui::kit::UiTheme;
use crate::ui::kit::style::{self, UiType};

/// Offset of a tip from the cursor (`OFFSET` of the Godot scripts).
const OFFSET: Vec2 = Vec2::new(20.0, 18.0);
const QUEUE_RED: Color = Color::srgb(0.55, 0.12, 0.10);

/// Which tip a node is.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorTip {
    Terrain,
    QueueFull,
    Walk,
}

/// What each tip shows now (`None`: hidden); dump and tests read it.
#[derive(Resource, Default, Debug, Clone, PartialEq)]
pub struct CursorTipState {
    pub terrain: Option<String>,
    pub queue_full: Option<String>,
    pub walk: Option<String>,
}

/// « +12 % » / « −8 % » with the arrow (▲ better, ▼ worse), « 0 % » for nothing.
pub fn signed(percent: i64) -> String {
    match percent {
        0 => "0 %".into(),
        p if p > 0 => format!("▲ +{p} %"),
        p => format!("▼ −{} %", p.abs()),
    }
}

/// Text of the terrain tip (`BattleTerrainTip.text_for`).
pub fn terrain_text(decor: &DecorHover) -> String {
    let mut parts = Vec::new();
    if decor.cover_pct != 0 {
        parts.push(format!("couvert {}", signed(decor.cover_pct)));
    }
    if decor.foot_speed_pct != 0 || decor.horse_speed_pct != 0 {
        parts.push(if decor.foot_speed_pct == decor.horse_speed_pct {
            format!("vitesse {}", signed(decor.foot_speed_pct))
        } else {
            format!(
                "vitesse {} à pied, {} à cheval",
                signed(decor.foot_speed_pct),
                signed(decor.horse_speed_pct)
            )
        });
    }
    if decor.defense_pct != 0 {
        parts.push(format!("mêlée {}", signed(decor.defense_pct)));
    }
    if decor.breaks_charge {
        parts.push("brise les charges".into());
    }
    let mut chars = decor.label.chars();
    let title: String = chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default();
    if parts.is_empty() {
        title
    } else {
        format!("{title} — {}", parts.join(", "))
    }
}

/// `BattleInput.queue_full_text`.
pub fn queue_full_text(max: u32) -> String {
    format!("File d’ordres pleine : {max} ordres en attente au plus par régiment.")
}

/// `BattleInput.walk_tip_text`.
pub fn walk_tip_text() -> &'static str {
    "Attaque au pas : les régiments marchent au contact au lieu de charger (Alt + clic droit)."
}

/// `BattleInput.walk_tip_wanted`.
pub fn walk_tip_wanted(context: HoverKind, alt: bool, has_selection: bool) -> bool {
    alt && has_selection && context == HoverKind::Melee
}

pub struct CursorTipsPlugin;

impl Plugin for CursorTipsPlugin {
    fn build(&self, app: &mut App) {
        use crate::agent::dump::DumpAppExt;
        app.init_resource::<CursorTipState>()
            .add_systems(
                Update,
                (spawn_tips, update_tips)
                    .chain()
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>),
            )
            .add_systems(OnExit(AppState::Battle), |mut s: ResMut<CursorTipState>| {
                *s = CursorTipState::default();
            })
            .register_dump(|world: &World| ("battle_cursor_tips".into(), dump(world)));
    }
}

fn spawn_tips(
    mut commands: Commands,
    roots: Query<Entity, With<BattleHudRoot>>,
    tips: Query<(), With<CursorTip>>,
    theme: Res<UiTheme>,
) {
    let Ok(root) = roots.single() else { return };
    if !tips.is_empty() {
        return;
    }
    for (kind, color) in [
        (CursorTip::Terrain, style::INK),
        (CursorTip::QueueFull, QUEUE_RED),
        (CursorTip::Walk, style::INK),
    ] {
        let mut entity = commands.spawn((
            Name::new(format!("CursorTip{kind:?}")),
            kind,
            Node {
                position_type: PositionType::Absolute,
                display: Display::None,
                padding: UiRect::all(Val::Px(4.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            Pickable::IGNORE,
            ChildOf(root),
        ));
        if kind == CursorTip::Terrain {
            entity.insert((
                BackgroundColor(Color::srgba(0.93, 0.88, 0.76, 0.92)),
                BorderColor::all(Color::srgb(0.35, 0.24, 0.12)),
            ));
        }
        let id = entity.id();
        commands.spawn((
            Text::new(""),
            theme.text_font(false, false, UiType::Caption.size()),
            TextColor(color),
            Pickable::IGNORE,
            ChildOf(id),
        ));
    }
}

type UiNodes<'w, 's> = Query<
    'w,
    's,
    (
        Option<&'static BackgroundColor>,
        Option<&'static ImageNode>,
        Has<CursorTip>,
    ),
    With<Node>,
>;

fn over_ui(hover: Option<&HoverMap>, nodes: &UiNodes) -> bool {
    let Some(hover) = hover else { return false };
    hover.values().flat_map(|hits| hits.keys()).any(|entity| {
        nodes.get(*entity).is_ok_and(|(background, image, is_tip)| {
            !is_tip && (image.is_some() || background.is_some_and(|b| b.0.alpha() > 0.05))
        })
    })
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn update_tips(
    windows: Query<&Window, With<PrimaryWindow>>,
    keys: Res<ButtonInput<KeyCode>>,
    cameras: Query<(&Camera, &GlobalTransform), With<BattleCamera>>,
    selection: Res<BattleSelection>,
    hovered: Res<HoveredBattleUnits>,
    hover_map: Option<Res<HoverMap>>,
    nodes: UiNodes,
    cinematic: Option<Res<crate::battle::orders::cinematic::Cinematic>>,
    mut session: ResMut<BattleSession>,
    mut state: ResMut<CursorTipState>,
    mut tips: Query<(&CursorTip, &mut Node, &Children)>,
    mut texts: Query<&mut Text>,
) {
    let mut next = CursorTipState::default();
    let mut at = None;
    let cursor = windows.single().ok().and_then(Window::cursor_position);
    let hidden = cinematic.is_some_and(|c| c.active);
    if let Some(cursor) = cursor
        && !hidden
        && !session.replaying
        && !over_ui(hover_map.as_deref(), &nodes)
        && let Ok((camera, transform)) = cameras.single()
        && let Ok(ray) = camera.viewport_to_world(transform, cursor)
    {
        at = Some(cursor);
        let player = session.player;
        let sim = session.sim_mut();
        let point =
            crate::battle::orders::picking::ground_point(ray.origin, *ray.direction, sim.field());
        let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
        let alt = keys.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]);
        let max = QueueRules::bundled().max_queued_orders as usize;
        let full = shift
            && !selection.units.is_empty()
            && selection
                .units
                .iter()
                .filter_map(|id| sim.unit(*id))
                .any(|u| u.order_queue.len() >= max);
        if full {
            next.queue_full = Some(queue_full_text(max as u32));
        }
        if let Some(side) = player {
            let context = sim
                .hover_context(
                    f64::from(point.x),
                    f64::from(point.z),
                    &selection.units,
                    side,
                )
                .context;
            if walk_tip_wanted(context, alt, !selection.units.is_empty()) {
                next.walk = Some(walk_tip_text().into());
            }
        }
        if hovered.0.is_empty()
            && !full
            && let Some(decor) = sim
                .field()
                .decor_hover_at(f64::from(point.x), f64::from(point.z))
        {
            next.terrain = Some(terrain_text(&decor));
        }
    }
    for (kind, mut node, children) in &mut tips {
        let text = match kind {
            CursorTip::Terrain => &next.terrain,
            CursorTip::QueueFull => &next.queue_full,
            CursorTip::Walk => &next.walk,
        };
        let display = if text.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
        if let (Some(text), Some(at)) = (text, at) {
            node.left = Val::Px(at.x + OFFSET.x);
            node.top = Val::Px(at.y + OFFSET.y);
            for child in children.iter() {
                if let Ok(mut label) = texts.get_mut(child)
                    && label.0 != *text
                {
                    label.0.clone_from(text);
                }
            }
        }
    }
    if *state != next {
        *state = next;
    }
}

fn dump(world: &World) -> Value {
    let state = world.resource::<CursorTipState>();
    json!({
        "terrain": state.terrain,
        "queue_full": state.queue_full,
        "walk": state.walk,
        "sample_terrain": terrain_text(&DecorHover {
            kind: sim_battle::AreaKind::Orchard,
            label: "verger",
            cover_pct: 15,
            foot_speed_pct: -20,
            horse_speed_pct: -35,
            defense_pct: 0,
            breaks_charge: true,
        }),
        "sample_queue": queue_full_text(QueueRules::bundled().max_queued_orders),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decor(cover: i64, foot: i64, horse: i64, defense: i64, breaks: bool) -> DecorHover {
        DecorHover {
            kind: sim_battle::AreaKind::Orchard,
            label: "verger",
            cover_pct: cover,
            foot_speed_pct: foot,
            horse_speed_pct: horse,
            defense_pct: defense,
            breaks_charge: breaks,
        }
    }

    #[test]
    fn signs_carry_an_arrow() {
        assert_eq!(signed(0), "0 %");
        assert_eq!(signed(12), "▲ +12 %");
        assert_eq!(signed(-8), "▼ −8 %");
    }

    #[test]
    fn terrain_text_joins_the_parts() {
        assert_eq!(
            terrain_text(&decor(15, -20, -20, 0, false)),
            "Verger — couvert ▲ +15 %, vitesse ▼ −20 %"
        );
        assert_eq!(
            terrain_text(&decor(0, -20, -35, 5, true)),
            "Verger — vitesse ▼ −20 % à pied, ▼ −35 % à cheval, mêlée ▲ +5 %, brise les charges"
        );
        assert_eq!(terrain_text(&decor(0, 0, 0, 0, false)), "Verger");
    }

    #[test]
    fn walk_tip_needs_alt_selection_and_melee() {
        assert!(walk_tip_wanted(HoverKind::Melee, true, true));
        assert!(!walk_tip_wanted(HoverKind::Melee, false, true));
        assert!(!walk_tip_wanted(HoverKind::Move, true, true));
        assert!(!walk_tip_wanted(HoverKind::Melee, true, false));
    }

    #[test]
    fn queue_text_quotes_the_bound() {
        assert!(queue_full_text(8).contains("8 ordres"));
    }
}
