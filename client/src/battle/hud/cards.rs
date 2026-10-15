//! Bottom band of the battle HUD: one card per regiment of the player, arranged in the three
//! « batailles » (vanguard, main body, rear). Port of the card part of battle_hud.gd
//! (`_build_bottom`, `update_cards`, `_make_card`) and unit_card.gd.
//!
//! A card shows the painted illustration of the unit type (else side colour + class icon), the
//! number of soldiers, thin bars (strength, morale, ammunition), a state veil (routed, gone,
//! reserve), the general's star, state badges (rout, wavering, under fire, charge, modes, tired); name, formation and figures are in the tooltip. A click sends
//! [`SelectBattleUnits`] (Shift/Ctrl adds); a double click centres the battle camera on the
//! regiment. The highlight reads [`BattleSelection`].

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::Button;
use serde_json::{Value, json};
use sim_battle::UnitState;

use super::tips::unit_bbcode;
use super::view::{BattleRow, UnitView, state_text};
use super::{BuildCtx, HudData, HudTexts};
use crate::app::AppState;
use crate::battle::orders::{BattleSelection, SelectBattleUnits};
use crate::battle::scene::camera::BattleCameraState;
use crate::paths;
use crate::ui::kit::icons::icon;
use crate::ui::kit::style;
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::tooltip_spec::TooltipTexts;
use crate::ui::kit::widgets::{caption, panel_page};
use crate::ui::kit::{ButtonPressed, UiTheme};

pub const CARD_WIDTH: f32 = 64.0;
pub const CARD_HEIGHT: f32 = 94.0;
const BAR_HEIGHT: f32 = 4.0;
/// Right margin of the band: room for the speed controls and the minimap (8 + speed column + 6
/// + minimap + 6).
pub const BAND_RIGHT_MARGIN: f32 =
    8.0 + super::speed::SPEED_PANEL_WIDTH + 6.0 + super::minimap::MAP_SIZE.x + 6.0;
const DOUBLE_CLICK_S: f32 = 0.35;
const BORDER_IDLE: Color = Color::srgb(0.42, 0.29, 0.16);
const BORDER_SELECTED: Color = Color::srgb(0.95, 0.75, 0.15);
const CARD_BG: Color = Color::srgb(0.2, 0.15, 0.1);
const COUNT_INK: Color = Color::srgb(1.0, 0.97, 0.88);

/// Root of a regiment card.
#[derive(Component)]
pub struct UnitCard(pub u32);

#[derive(Component)]
struct CardCount(u32);

#[derive(Component)]
struct CardState(u32);

/// One state badge slot (round pill at the right of the card, from the top), filled by
/// [`update_cards`] from [`super::view::state_badges`].
#[derive(Component)]
struct CardBadge {
    id: u32,
    index: usize,
}

/// Ink icon of the badge (`battle_state_<kind>`, `battle_mode_<mode>`).
#[derive(Component)]
struct CardBadgeIcon {
    id: u32,
    index: usize,
}

/// Fallback glyph of the badge when its ink icon is missing.
#[derive(Component)]
struct CardBadgeGlyph {
    id: u32,
    index: usize,
}

/// Size (px) of a badge pill and the step between two of them.
const BADGE_SIZE: f32 = 16.0;
const BADGE_STEP: f32 = 18.0;

/// Icon id of a badge kind.
pub fn badge_icon_id(kind: &str) -> String {
    match kind.strip_prefix("mode_") {
        Some(mode) => format!("battle_mode_{mode}"),
        None => format!("battle_state_{kind}"),
    }
}

#[derive(Component)]
struct CardVeil(u32);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BarKind {
    Strength,
    Morale,
    Ammo,
}

#[derive(Component)]
struct CardBar {
    id: u32,
    kind: BarKind,
}

/// What the cards did, for the dump: requests sent, last click for the double-click test.
#[derive(Resource, Default, Debug)]
pub struct CardLog {
    pub sent: Vec<SelectBattleUnits>,
    pub centred: u32,
    last_click: Option<(u32, f32)>,
}

pub struct CardsPlugin;

impl Plugin for CardsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CardLog>()
            .add_systems(OnEnter(AppState::Battle), clear_log)
            .add_systems(
                Update,
                (card_clicks, update_cards, update_badges)
                    .chain()
                    .run_if(in_state(AppState::Battle)),
            );
    }
}

fn clear_log(mut log: ResMut<CardLog>) {
    *log = CardLog::default();
}

/// Bar colour by ratio (green → red), as `gauge_color`.
pub fn bar_color(kind: BarKind, ratio: f32) -> Color {
    match kind {
        BarKind::Strength => Color::srgb(0.86, 0.80, 0.58),
        BarKind::Morale => style::gauge_color(ratio),
        BarKind::Ammo => Color::srgb(0.55, 0.75, 0.45),
    }
}

fn illustration_exists(unit_type: &str) -> bool {
    paths::find_root()
        .map(|root| {
            paths::asset_root(&root)
                .join(format!("illustrations/{unit_type}.jpg"))
                .exists()
        })
        .unwrap_or(false)
}

/// Spawns the bottom band under `root`.
pub fn spawn_band(commands: &mut Commands, root: Entity, ctx: &BuildCtx, texts: &TooltipTexts) {
    let wrapper = commands
        .spawn((
            Name::new("BottomBand"),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(super::orders_bar::BAND_LEFT_MARGIN),
                right: Val::Px(BAND_RIGHT_MARGIN),
                bottom: Val::Px(super::geometry::BAND_BOTTOM),
                ..default()
            },
            Pickable::IGNORE,
            ChildOf(root),
        ))
        .id();
    let panel = commands
        .spawn((
            panel_page(ctx.theme),
            ChildOf(wrapper),
            Name::new("BandPanel"),
        ))
        .id();
    commands
        .entity(panel)
        .entry::<Node>()
        .and_modify(|mut node| {
            // The strip scrolls: the panel never grows past the band's margins (it used to run
            // under the minimap and the speed buttons).
            node.min_width = Val::Px(0.0);
            node.max_width = Val::Percent(100.0);
        });
    let strip = commands
        .spawn((
            Name::new("CardStrip"),
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(12.0),
                overflow: Overflow::scroll_x(),
                ..default()
            },
            ChildOf(panel),
        ))
        .id();
    for row in BattleRow::ORDER {
        let members: Vec<&UnitView> = ctx.data.own.iter().filter(|u| u.row == row).collect();
        if members.is_empty() {
            continue;
        }
        let column = commands
            .spawn((
                Name::new(format!("Battle_{}", row.key())),
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(1.0),
                    ..default()
                },
                ChildOf(strip),
            ))
            .id();
        commands.spawn((caption(ctx.theme, row.label()), ChildOf(column)));
        let cards = commands
            .spawn((
                Name::new("Cards"),
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(3.0),
                    ..default()
                },
                ChildOf(column),
            ))
            .id();
        for unit in members {
            // The card and, under it, its ability buttons.
            let slot = commands
                .spawn((
                    Name::new(format!("cardslot:{}", unit.id)),
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(1.0),
                        ..default()
                    },
                    ChildOf(cards),
                ))
                .id();
            spawn_card(commands, slot, ctx, unit, texts);
            super::abilities::spawn_row(commands, slot, ctx.theme, unit.id);
        }
    }
}

fn spawn_card(
    commands: &mut Commands,
    parent: Entity,
    ctx: &BuildCtx,
    unit: &UnitView,
    texts: &TooltipTexts,
) {
    let id = unit.id;
    let side_color = ctx.data.colors[0];
    let name = ctx.data.name_of(id);
    let display = if unit.is_general {
        format!("{name} ★")
    } else {
        name
    };
    let card = commands
        .spawn((
            Name::new(format!("unitcard:{id}")),
            UnitCard(id),
            Button,
            Hovered::default(),
            Tooltip::new(&unit_bbcode(unit, &display, texts)),
            Node {
                width: Val::Px(CARD_WIDTH),
                height: Val::Px(CARD_HEIGHT),
                border: UiRect::all(Val::Px(3.0)),
                overflow: Overflow::clip(),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(CARD_BG),
            BorderColor::all(BORDER_IDLE),
            ChildOf(parent),
        ))
        .id();
    // Illustration, or side colour + class icon.
    if illustration_exists(&unit.unit_type) {
        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            ImageNode::new(
                ctx.assets
                    .load(format!("illustrations/{}.jpg", unit.unit_type)),
            )
            .with_mode(NodeImageMode::Auto),
            Pickable::IGNORE,
            ChildOf(card),
        ));
    } else {
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    padding: UiRect::top(Val::Px(18.0)),
                    ..default()
                },
                BackgroundColor(side_color.darker(0.35)),
                Pickable::IGNORE,
                ChildOf(card),
            ))
            .with_children(|center| {
                center.spawn((
                    icon(ctx.theme, ctx.assets, &icon_id(ctx.theme, unit), 38.0),
                    Pickable::IGNORE,
                ));
            });
    }
    // Side band.
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Px(4.0),
            ..default()
        },
        BackgroundColor(side_color),
        Pickable::IGNORE,
        ChildOf(card),
    ));
    // Class icon when an illustration is shown.
    if illustration_exists(&unit.unit_type) {
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(1.0),
                    top: Val::Px(5.0),
                    ..default()
                },
                Pickable::IGNORE,
                ChildOf(card),
            ))
            .with_children(|corner| {
                corner.spawn((
                    icon(ctx.theme, ctx.assets, &icon_id(ctx.theme, unit), 20.0),
                    Pickable::IGNORE,
                ));
            });
    }
    if unit.is_general {
        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(2.0),
                top: Val::Px(4.0),
                ..default()
            },
            Text::new("★"),
            ctx.theme.text_font(true, false, 15.0),
            TextColor(style::GOLD_PALE),
            Pickable::IGNORE,
            ChildOf(card),
        ));
    }
    // State pill.
    commands.spawn((
        CardState(id),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(2.0),
            top: Val::Px(22.0),
            ..default()
        },
        Text::new(""),
        ctx.theme.text_font(false, false, 11.0),
        TextColor(style::PARCHMENT_LIGHT),
        Pickable::IGNORE,
        ChildOf(card),
    ));
    // State badges (rout, wavering, under fire, charge, modes, tired...).
    for index in 0..super::view::MAX_BADGES {
        let badge = commands
            .spawn((
                CardBadge { id, index },
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(2.0),
                    top: Val::Px(22.0 + index as f32 * BADGE_STEP),
                    width: Val::Px(BADGE_SIZE),
                    height: Val::Px(BADGE_SIZE),
                    display: Display::None,
                    border: UiRect::all(Val::Px(1.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border_radius: BorderRadius::MAX,
                    ..default()
                },
                BackgroundColor(style::PARCHMENT_LIGHT),
                BorderColor::all(style::INK),
                Pickable::IGNORE,
                ChildOf(card),
            ))
            .id();
        commands.spawn((
            CardBadgeIcon { id, index },
            Node {
                width: Val::Px(BADGE_SIZE - 5.0),
                height: Val::Px(BADGE_SIZE - 5.0),
                position_type: PositionType::Absolute,
                ..default()
            },
            ImageNode::default().with_color(Color::NONE),
            Pickable::IGNORE,
            ChildOf(badge),
        ));
        commands.spawn((
            CardBadgeGlyph { id, index },
            Text::new(""),
            ctx.theme.text_font(true, false, 10.0),
            TextColor(style::INK),
            Pickable::IGNORE,
            ChildOf(badge),
        ));
    }
    // Shade, count and bars.
    let shade = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                bottom: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(2.0)),
                row_gap: Val::Px(1.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            Pickable::IGNORE,
            ChildOf(card),
        ))
        .id();
    commands.spawn((
        CardCount(id),
        Text::new(""),
        ctx.theme.text_font(true, false, 19.0),
        TextColor(COUNT_INK),
        Pickable::IGNORE,
        ChildOf(shade),
    ));
    let mut kinds = vec![BarKind::Strength, BarKind::Morale];
    if unit.can_shoot {
        kinds.push(BarKind::Ammo);
    }
    for kind in kinds {
        let track = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(BAR_HEIGHT),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.15, 0.1, 0.06, 0.9)),
                Pickable::IGNORE,
                ChildOf(shade),
            ))
            .id();
        commands.spawn((
            CardBar { id, kind },
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(Color::WHITE),
            Pickable::IGNORE,
            ChildOf(track),
        ));
    }
    // Veil over everything (gone, routed, reserve).
    commands.spawn((
        CardVeil(id),
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::NONE),
        Pickable::IGNORE,
        ChildOf(card),
    ));
}

/// Icon id of a unit: its type (painted miniature), else its category medallion.
fn icon_id(theme: &UiTheme, unit: &UnitView) -> String {
    if theme.icons.get(&unit.unit_type).is_some() {
        unit.unit_type.clone()
    } else {
        format!("unit_category_{}", unit.category.key())
    }
}

/// Veil colour of a card: grey when gone, red in rout, pale in reserve.
pub fn veil_color(unit: &UnitView, blink: bool) -> Color {
    if !unit.present && !unit.reserve || unit.left_field {
        Color::srgba(0.25, 0.22, 0.2, 0.65)
    } else if unit.state == UnitState::Routing {
        Color::srgba(0.8, 0.1, 0.05, if blink { 0.3 } else { 0.15 })
    } else if unit.reserve {
        Color::srgba(0.9, 0.85, 0.7, 0.35)
    } else {
        Color::NONE
    }
}

/// Ids to highlight: the selected regiments that have a card.
pub fn highlighted(selection: &BattleSelection, data: &HudData) -> Vec<u32> {
    selection
        .units
        .iter()
        .copied()
        .filter(|id| data.own.iter().any(|u| u.id == *id))
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn card_clicks(
    mut pressed: MessageReader<ButtonPressed>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time<Real>>,
    data: Res<HudData>,
    mut log: ResMut<CardLog>,
    mut select: MessageWriter<SelectBattleUnits>,
    camera: Option<ResMut<BattleCameraState>>,
) {
    let mut camera = camera;
    for press in pressed.read() {
        let Some(id) = press
            .name
            .strip_prefix("unitcard:")
            .and_then(|text| text.parse::<u32>().ok())
        else {
            continue;
        };
        // Shift/Ctrl (not remappable actions: mouse-click modifiers, as in unit_card.gd) add to the
        // selection instead of replacing it.
        let additive = keys.any_pressed([
            KeyCode::ShiftLeft,
            KeyCode::ShiftRight,
            KeyCode::ControlLeft,
            KeyCode::ControlRight,
        ]);
        let request = SelectBattleUnits {
            units: vec![id],
            additive,
        };
        log.sent.push(request.clone());
        select.write(request);
        let now = time.elapsed_secs();
        let double = log
            .last_click
            .is_some_and(|(last, at)| last == id && now - at <= DOUBLE_CLICK_S);
        log.last_click = Some((id, now));
        if double && let (Some(camera), Some(unit)) = (camera.as_deref_mut(), data.unit(id)) {
            camera.target.x = unit.x as f32;
            camera.target.z = unit.z as f32;
            log.centred += 1;
        }
    }
}

fn blink_now(time: &Time<Real>) -> bool {
    (time.elapsed_secs() % 0.6) < 0.3
}

/// The badge slots, as one system parameter.
#[derive(bevy::ecs::system::SystemParam)]
#[allow(clippy::type_complexity)]
struct BadgeQueries<'w, 's> {
    pills: Query<
        'w,
        's,
        (
            &'static CardBadge,
            &'static mut Node,
            &'static mut BackgroundColor,
        ),
        (Without<CardBadgeIcon>, Without<CardBadgeGlyph>),
    >,
    icons: Query<'w, 's, (&'static CardBadgeIcon, &'static mut ImageNode), Without<CardBadgeGlyph>>,
    glyphs: Query<'w, 's, (&'static CardBadgeGlyph, &'static mut Text), Without<CardState>>,
}

fn update_badges(
    data: Res<HudData>,
    time: Res<Time<Real>>,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
    mut slots: BadgeQueries,
) {
    let blink = blink_now(&time);
    let badges_of = |id: u32| {
        data.own
            .iter()
            .find(|u| u.id == id)
            .map(super::view::state_badges)
            .unwrap_or_default()
    };
    for (pill, mut node, mut background) in &mut slots.pills {
        let badges = badges_of(pill.id);
        let (display, colour) = match badges.get(pill.index) {
            Some(kind) => {
                let [r, g, b] = super::view::badge_color(kind, blink);
                (Display::Flex, Color::srgb(r, g, b))
            }
            None => (Display::None, Color::NONE),
        };
        if node.display != display {
            node.display = display;
        }
        if background.0 != colour {
            background.0 = colour;
        }
    }
    for (slot, mut image) in &mut slots.icons {
        let badges = badges_of(slot.id);
        let entry = badges
            .get(slot.index)
            .and_then(|kind| theme.icons.get(&badge_icon_id(kind)));
        match entry {
            Some(entry) => {
                let handle = assets.load(entry.path.clone());
                if image.image != handle {
                    image.image = handle;
                }
                image.color = Color::WHITE;
            }
            None => image.color = Color::NONE,
        }
    }
    for (slot, mut text) in &mut slots.glyphs {
        let badges = badges_of(slot.id);
        let shown = match badges.get(slot.index) {
            Some(kind) if theme.icons.get(&badge_icon_id(kind)).is_none() => {
                super::view::badge_glyph(kind)
            }
            _ => "",
        };
        if text.0 != shown {
            text.0 = shown.to_owned();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn update_cards(
    data: Res<HudData>,
    selection: Res<BattleSelection>,
    time: Res<Time<Real>>,
    texts: Res<HudTexts>,
    mut roots: Query<(&UnitCard, &mut BorderColor, &mut Tooltip)>,
    mut counts: Query<(&CardCount, &mut Text), Without<CardState>>,
    mut states: Query<(&CardState, &mut Text), Without<CardCount>>,
    mut bars: Query<(&CardBar, &mut Node, &mut BackgroundColor), Without<CardVeil>>,
    mut veils: Query<(&CardVeil, &mut BackgroundColor), Without<CardBar>>,
) {
    let blink = (time.elapsed_secs() % 0.6) < 0.3;
    let chosen = highlighted(&selection, &data);
    let refresh_text = data.is_changed();
    for (card, mut border, mut tooltip) in &mut roots {
        *border = BorderColor::all(if chosen.contains(&card.0) {
            BORDER_SELECTED
        } else {
            BORDER_IDLE
        });
        if refresh_text && let Some(unit) = data.own.iter().find(|u| u.id == card.0) {
            let name = data.name_of(card.0);
            let display = if unit.is_general {
                format!("{name} ★")
            } else {
                name
            };
            let bbcode = unit_bbcode(unit, &display, &texts.texts);
            if tooltip.bbcode != bbcode {
                tooltip.bbcode = bbcode;
            }
        }
    }
    for (count, mut text) in &mut counts {
        if let Some(unit) = data.own.iter().find(|u| u.id == count.0) {
            let shown = unit.soldiers.to_string();
            if text.0 != shown {
                text.0 = shown;
            }
        }
    }
    for (state, mut text) in &mut states {
        if let Some(unit) = data.own.iter().find(|u| u.id == state.0) {
            let shown = match unit.state {
                _ if !unit.present || unit.left_field || unit.reserve => state_text(unit),
                _ => "",
            };
            if text.0 != shown {
                text.0 = shown.to_owned();
            }
        }
    }
    for (bar, mut node, mut color) in &mut bars {
        if let Some(unit) = data.own.iter().find(|u| u.id == bar.id) {
            let ratio = match bar.kind {
                BarKind::Strength => unit.soldiers as f32 / unit.initial.max(1) as f32,
                BarKind::Morale => unit.morale / 100.0,
                BarKind::Ammo => unit.ammo as f32 / unit.max_ammo.max(1) as f32,
            }
            .clamp(0.0, 1.0);
            node.width = Val::Percent(ratio * 100.0);
            *color = BackgroundColor(bar_color(bar.kind, ratio));
        }
    }
    for (veil, mut color) in &mut veils {
        if let Some(unit) = data.own.iter().find(|u| u.id == veil.0) {
            *color = BackgroundColor(veil_color(unit, blink));
        }
    }
}

pub fn dump(world: &World) -> Value {
    let data = world.resource::<HudData>();
    let selection = world.resource::<BattleSelection>();
    let log = world.resource::<CardLog>();
    let per_row = |row: BattleRow| data.own.iter().filter(|u| u.row == row).count();
    json!({
        "count": data.own.len(),
        "vanguard": per_row(BattleRow::Vanguard),
        "main": per_row(BattleRow::Main),
        "rear": per_row(BattleRow::Rear),
        "highlighted": highlighted(selection, data),
        "selection": selection.units,
        "requests_sent": log.sent.iter().map(|r| json!({"units": r.units, "additive": r.additive})).collect::<Vec<_>>(),
        "camera_centred": log.centred,
        "first": data.own.first().map(|u| json!({
            "id": u.id, "name": data.name_of(u.id), "soldiers": u.soldiers, "state": state_text(u), "badges": super::view::state_badges(u),
        })),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::hud::view::UnitView;
    use data_model::UnitCategory;
    use sim_battle::UnitFate;

    fn view(id: u32) -> UnitView {
        UnitView {
            id,
            unit_type: "unit_x".into(),
            name: "X".into(),
            category: UnitCategory::Infantry,
            soldiers: 10,
            initial: 10,
            morale: 50.0,
            fatigue: 0.0,
            ammo: 0,
            max_ammo: 0,
            can_shoot: false,
            is_general: false,
            present: true,
            left_field: false,
            reserve: false,
            state: UnitState::Idle,
            formation: "Ligne".into(),
            experience: 0,
            kills: 0,
            fate: UnitFate::Held,
            melee: 1,
            ranged: 0,
            armor: 1,
            x: 0.0,
            z: 0.0,
            row: BattleRow::Main,
            status: Default::default(),
            formation_key: "line".into(),
            reform_progress: None,
        }
    }

    #[test]
    fn highlight_keeps_only_own_cards() {
        let data = HudData {
            own: vec![view(1), view(2)],
            ..default()
        };
        let selection = BattleSelection { units: vec![2, 9] };
        assert_eq!(highlighted(&selection, &data), vec![2]);
    }

    #[test]
    fn veil_by_state() {
        let mut unit = view(1);
        assert_eq!(veil_color(&unit, true), Color::NONE);
        unit.state = UnitState::Routing;
        assert_ne!(veil_color(&unit, true), Color::NONE);
        unit.reserve = true;
        unit.state = UnitState::Idle;
        assert_ne!(veil_color(&unit, false), Color::NONE);
        unit.left_field = true;
        assert_eq!(
            veil_color(&unit, false),
            Color::srgba(0.25, 0.22, 0.2, 0.65)
        );
    }
}
