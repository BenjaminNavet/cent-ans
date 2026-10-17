//! Front end: start menu, faction selection, save / load dialog, in-campaign pause menu.
//!
//! Ports game/scenes/start_menu.tscn and game/scripts/ui/{start_menu, faction_select,
//! front_end_data, pause_menu, save_load_dialog, save_slots, battles_menu, credits_screen,
//! intro_cards, menu_background}.gd.
//!
//! - `main_menu`: title column (Nouvelle partie, Continuer, Charger une partie, Prologue, Codex,
//!   Batailles, Réglages, Crédits, Quitter) over the illustrated menu map (`MenuBackground`).
//! - `faction_select`: [`AppState::FactionSelect`], cards, full list, pitch, difficulty, start.
//! - `saves`: save directory ([`saves::SaveDir`]), save / load dialog, `game_api::saves` calls.
//! - `battles` (+ `custom_battle`, `historical`, `demos`, `presets`): the screens of « Batailles »;
//!   they build a `PendingBattle` and enter `AppState::Battle`.
//! - `pause`: Escape in the campaign (Reprendre, Sauvegarder, Charger, Réglages, Menu principal,
//!   Quitter), resource [`pause::GamePaused`] and virtual time paused while it is open.
//!
//! Messages (all defined here, documented in `client/README.md` § Menu):
//! - [`OpenSettings`]: written by « Réglages » (main menu and pause); the settings lot listens.
//! - [`saves::OpenSaveDialog`]: opens the dialog (written by the menu buttons).
//! - [`saves::GameLoaded`]: a save has just replaced `Sim::state`.

pub mod battle_pause;
pub mod battle_prologue_invite;
pub mod battles;
pub mod custom_battle;
pub mod demos;
pub mod faction_map;
pub mod faction_select;
pub mod front_end;
pub mod historical;
pub mod main_menu;
pub mod pause;
pub mod presets;
pub mod save_extras;
pub mod saves;
pub mod unit_tips;

use bevy::prelude::*;
use serde_json::{Value, json};

use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::paths;
use crate::ui::kit::style::{self, UiTier, UiType};
use crate::ui::kit::{UiTheme, panel_sized, typed};

/// « Réglages » was chosen: the settings screen opens itself on this message (defined there).
pub use crate::settings::OpenSettings;

/// Loaded `data/ui/front_end.json` (empty when unreadable, with a warning).
#[derive(Resource, Default)]
pub struct MenuData {
    pub front: front_end::FrontEnd,
}

/// A full-screen dimmed layer holding a dialog. `layer` orders the Escape key (highest first).
#[derive(Component)]
pub struct MenuOverlay {
    pub layer: i32,
}

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<OverlayClosed>()
            .init_resource::<MenuData>()
            .register_dump(dump_menu)
            .add_plugins((
                main_menu::MainMenuPlugin,
                faction_select::FactionSelectPlugin,
                saves::SavesPlugin,
                save_extras::SaveExtrasPlugin,
                pause::PausePlugin,
                battle_pause::BattlePausePlugin,
                battles::BattlesPlugin,
                battle_prologue_invite::PrologueInvitePlugin,
                crate::battle::tutorial::BattleTutorialPlugin,
            ))
            .add_systems(Startup, load_front_end)
            .init_resource::<KeyEdges>()
            .add_systems(PreUpdate, track_keys.after(bevy::input::InputSystems))
            .add_systems(Update, (cover_fit, escape_overlays, menu_button_visuals));
    }
}

fn load_front_end(mut data: ResMut<MenuData>) {
    let loaded = paths::find_root().and_then(|root| {
        front_end::FrontEnd::load(&paths::data_dir(&root), &paths::asset_root(&root))
    });
    match loaded {
        Ok(front) => data.front = front,
        Err(message) => warn!("front_end.json: {message}"),
    }
}

// ---- shared widgets -----------------------------------------------------------------------

/// Crops an `ImageNode` like Godot's `STRETCH_KEEP_ASPECT_COVERED`: the part of the image that
/// has the node's aspect, placed by `anchor` (0..1 on each axis, (0.5, 0.5) = centred).
#[derive(Component)]
pub struct CoverFit {
    pub anchor: Vec2,
    last: Option<(UVec2, UVec2)>,
}

impl CoverFit {
    pub fn new(anchor: Vec2) -> Self {
        Self { anchor, last: None }
    }
}

fn cover_fit(
    mut nodes: Query<(&mut CoverFit, &mut ImageNode, &ComputedNode)>,
    images: Res<Assets<Image>>,
) {
    for (mut fit, mut image, computed) in &mut nodes {
        let Some(source) = images.get(&image.image) else {
            continue;
        };
        let size = computed.size();
        if size.x < 1.0 || size.y < 1.0 {
            continue;
        }
        let key = (source.size(), size.round().as_uvec2());
        if fit.last == Some(key) {
            continue;
        }
        fit.last = Some(key);
        let source_size = source.size().as_vec2();
        let (width, height) = if size.x / size.y > source_size.x / source_size.y {
            (source_size.x, source_size.x * size.y / size.x)
        } else {
            (source_size.y * size.x / size.y, source_size.y)
        };
        let min = (source_size - Vec2::new(width, height)) * fit.anchor;
        image.rect = Some(Rect::from_corners(min, min + Vec2::new(width, height)));
    }
}

/// Plain (frameless) menu button: title font, vellum text turning gold under the pointer.
#[derive(Component)]
pub struct MenuButton {
    pub size: f32,
}

pub fn menu_button(theme: &UiTheme, name: &str, caption: &str, size: f32) -> impl Bundle {
    (
        Name::new(name.to_owned()),
        MenuButton { size },
        bevy::ui_widgets::Button,
        bevy::picking::hover::Hovered::default(),
        Node {
            min_width: Val::Px(360.0),
            padding: UiRect::axes(Val::Px(22.0), Val::Px(4.0)),
            border: UiRect::left(Val::Px(3.0)),
            ..default()
        },
        BorderColor::all(Color::NONE),
        BackgroundColor(Color::NONE),
        children![(
            Text::new(caption),
            theme.title_font(size),
            TextColor(style::FE_VELLUM),
        )],
    )
}

#[allow(clippy::type_complexity)]
pub fn menu_button_visuals(
    mut buttons: Query<
        (
            &bevy::picking::hover::Hovered,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
        ),
        With<MenuButton>,
    >,
    mut inks: Query<&mut TextColor>,
) {
    for (hovered, mut fill, mut border, children) in &mut buttons {
        let (fill_target, border_target, ink) = if hovered.get() {
            (
                style::FE_GOLD_DARK.with_alpha(0.18),
                style::FE_GOLD,
                style::FE_GOLD,
            )
        } else {
            (Color::NONE, Color::NONE, style::FE_VELLUM)
        };
        if fill.0 != fill_target {
            fill.0 = fill_target;
            *border = BorderColor::all(border_target);
        }
        for child in children {
            if let Ok(mut color) = inks.get_mut(*child)
                && color.0 != ink
            {
                color.0 = ink;
            }
        }
    }
}

/// The illustrated menu map, covering the screen from the top right (`MenuBackground`), under a
/// dark veil, plus the left shading that keeps the column readable.
pub fn spawn_backdrop(commands: &mut Commands, assets: &AssetServer, root: Entity) {
    spawn_backdrop_image(commands, assets, root, "ui/menu_map.jpg");
}

/// [`spawn_backdrop`] without the title cartouche painted in `menu_map.jpg` (its zone is filled
/// with map): screens that are not the title menu must not show « Cent Ans » behind their panels.
pub fn spawn_backdrop_plain(commands: &mut Commands, assets: &AssetServer, root: Entity) {
    spawn_backdrop_image(commands, assets, root, "ui/menu_map_plain.jpg");
}

fn spawn_backdrop_image(commands: &mut Commands, assets: &AssetServer, root: Entity, path: &str) {
    commands.entity(root).with_children(|parent| {
        parent.spawn((
            Name::new("Backdrop"),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            ImageNode::new(assets.load(path.to_owned())),
            CoverFit::new(Vec2::new(1.0, 0.0)),
            ZIndex(-3),
        ));
        parent.spawn((
            Name::new("Veil"),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.25)),
            ZIndex(-2),
        ));
        parent.spawn((
            Name::new("LeftShade"),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(55.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundGradient::from(LinearGradient::to_right(vec![
                ColorStop::new(Color::srgba(0.02, 0.015, 0.01, 0.82), Val::Percent(0.0)),
                ColorStop::new(Color::srgba(0.02, 0.015, 0.01, 0.45), Val::Percent(55.0)),
                ColorStop::new(Color::srgba(0.02, 0.015, 0.01, 0.0), Val::Percent(100.0)),
            ])),
            ZIndex(-1),
        ));
    });
}

/// Full-screen dimmed layer (`MenuOverlay`) with a titled parchment panel of `width` px in the
/// middle. Returns `(overlay, panel)`; add the content under `panel`.
pub fn spawn_overlay(
    commands: &mut Commands,
    theme: &UiTheme,
    name: &str,
    title: &str,
    width: f32,
    layer: i32,
) -> (Entity, Entity) {
    let overlay = commands
        .spawn((
            Name::new(name.to_owned()),
            MenuOverlay { layer },
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
            GlobalZIndex(UiTier::Modal.z() + layer),
        ))
        .id();
    let panel = commands
        .spawn((panel_sized(theme, width), ChildOf(overlay)))
        .id();
    commands.spawn((
        Name::new(format!("{name} title")),
        typed(theme, title, UiType::Title),
        Node {
            align_self: AlignSelf::Center,
            ..default()
        },
        ChildOf(panel),
    ));
    (overlay, panel)
}

/// Escape closes the topmost overlay that is not a pause menu (the pause menu handles its own).
fn escape_overlays(
    mut commands: Commands,
    edges: Res<KeyEdges>,
    overlays: Query<(Entity, &MenuOverlay), Without<pause::PauseRoot>>,
    state: Res<State<AppState>>,
    mut closed: MessageWriter<OverlayClosed>,
    typing: Res<crate::ui::kit::text_field::TextInputFocus>,
) {
    // A focused text field takes the Escape (it only drops the focus).
    if !edges.escape || typing.is_active() || *state.get() == AppState::Boot {
        return;
    }
    if let Some((entity, _)) = overlays.iter().max_by_key(|(_, overlay)| overlay.layer) {
        commands.entity(entity).despawn();
        closed.write(OverlayClosed);
    }
}

/// Rising edges of the keys the menus read. Computed from `pressed` (not `just_pressed`) because
/// the agent's `key` command presses in `Last`, and `ButtonInput` clears `just_pressed` in the
/// next `PreUpdate`, before any `Update` system could see it.
#[derive(Resource, Default, Debug)]
pub struct KeyEdges {
    pub escape: bool,
    pub enter: bool,
    was_escape: bool,
    was_enter: bool,
}

/// The cancel edge follows the key map (`campaign_pause`, Escape by default); Enter stays the
/// fixed confirm key of the dialogs (Godot's `ui_accept`, not an action of the map).
fn track_keys(
    keys: Res<ButtonInput<KeyCode>>,
    keymap: Option<Res<crate::settings::KeyMap>>,
    mut edges: ResMut<KeyEdges>,
) {
    let escape = keymap.as_ref().map_or_else(
        || keys.pressed(KeyCode::Escape),
        |map| map.pressed("campaign_pause", &keys),
    );
    let enter = keys.pressed(KeyCode::Enter);
    edges.escape = escape && !edges.was_escape;
    edges.enter = enter && !edges.was_enter;
    edges.was_escape = escape;
    edges.was_enter = enter;
}

/// An overlay was closed by Escape.
#[derive(Message, Debug, Clone, Copy)]
pub struct OverlayClosed;

fn dump_menu(world: &World) -> (String, Value) {
    let mut overlays: Vec<String> = Vec::new();
    let mut query = world.try_query::<(&Name, &MenuOverlay)>().unwrap();
    for (name, overlay) in query.iter(world) {
        overlays.push(format!("{}@{}", name, overlay.layer));
    }
    overlays.sort();
    let front = &world.resource::<MenuData>().front;
    let value = json!({
        "overlays": overlays,
        "factions_pitched": front.factions.len(),
        "faction_choice": faction_select::dump(world),
        "faction_map": faction_map::dump(world),
        "battle_prologue_invite": battle_prologue_invite::dump(world),
        "saves": saves::dump(world),
        "paused": world.resource::<pause::GamePaused>().0,
    });
    ("menu".to_owned(), value)
}
