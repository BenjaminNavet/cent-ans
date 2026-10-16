//! Pre-battle dialog (`pre_battle_dialog.gd`): force balance bar and the core's verdict, both
//! armies (general, reinforcements, regiment cards), conditions, then « Combattre » /
//! « Résolution automatique » / « Retraite ». The player's side is always on the left.
//!
//! The brief itself (texts, chances, regiments) comes from `game_api::flow::battle_brief`.
//! Illustrated banner ([`banner`]: a loading plate of `ui::art_plates::ArtPlates`, cover-cropped,
//! darkened towards the gold rule, title and subtitle over it) and the arms of each side
//! ([`arms`], `heraldry/<faction>.png`); both are shared with the naval dialog.
//! The site mini-map, the forecast weather and the general medallions come from [`SitePreview`]:
//! a read-only `BattleSim` built from the same setup and seed as the battle that will be fought
//! (same site, same weather), drawn with the battle minimap's raster. Not ported: the hover
//! bubbles on the buttons.

use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;
use game_api::flow::{BattleBrief, BattleLaunch, BriefSide};
use sim_battle::BattleSim;

use crate::app::AppState;
use crate::battle::hud::minimap::{self, Raster};
use crate::battle::scene::setup::side_color;
use crate::paths;
use crate::ui::art_plates::ArtPlates;
use crate::ui::kit::style::{self, UiTier, UiType};
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::tooltip_spec::TooltipTexts;
use crate::ui::kit::widgets::{button_primary, button_small, caption, panel_sized, separator};
use crate::ui::kit::{UiTheme, fr, typed};
use data_model::GameData;
use sim_battle::SideId;

/// The open dialog and what it shows (read by the dump and the button handler).
#[derive(Component, Debug, Clone)]
pub struct PreBattleDialog {
    pub index: usize,
    pub title: String,
    pub verdict: String,
    pub share: f64,
    pub regiments: [usize; 2],
    pub can_withdraw: bool,
    /// What the site preview showed (`None` when the preview could not be built).
    pub preview: Option<PreviewSummary>,
}

/// The dump's view of a [`SitePreview`].
#[derive(Debug, Clone, PartialEq)]
pub struct PreviewSummary {
    pub weather: String,
    pub site_label: String,
    pub own_dots: usize,
    pub enemy_dots: usize,
    pub generals: [Option<String>; 2],
    /// Medallion picture of each side's general.
    pub medallions: [Option<String>; 2],
}

/// Names of the three buttons.
pub const FIGHT: &str = "flow:fight";
pub const AUTO: &str = "flow:auto";
pub const WITHDRAW: &str = "flow:withdraw";

fn tone_color(tone: &str) -> Color {
    match tone {
        "good" => style::GOOD,
        "fair" => Color::srgb(0.55, 0.40, 0.08),
        _ => style::RUBRIC,
    }
}

/// Size of the site map in the dialog (Godot `SITE_MAP_SIZE`).
const SITE_MAP_SIZE: Vec2 = Vec2::new(300.0, 186.0);
/// Diameter of a general's medallion (Godot 76).
const MEDALLION_SIZE: f32 = 76.0;

/// What a preview `BattleSim` of the battle to come tells (Godot `_preview`, `_fill_site_map`,
/// `_fill_conditions`): the field, where both armies start, the weather and the site, the
/// generals' characters. Read only; no rule lives here.
#[derive(Clone)]
pub struct SitePreview {
    pub weather_key: String,
    pub weather_label: String,
    pub site_label: String,
    pub field: Vec2,
    /// Static layer of the minimap, turned so the player's camp is at the bottom.
    pub base: Raster,
    pub flipped: bool,
    /// Start positions of the player's regiments then the enemy's (world x, z).
    pub own: Vec<Vec2>,
    pub enemy: Vec<Vec2>,
    /// Character id of the general of each brief slot (player first), when there is one.
    pub generals: [Option<String>; 2],
}

impl SitePreview {
    /// Previews `launch` on `site` (same construction as the battle); `player_side` is
    /// `attacker` or `defender`.
    pub fn build(
        launch: &BattleLaunch,
        site: Option<&sim_battle::HistoricalMap>,
        player_side: &str,
    ) -> Result<Self, String> {
        let root = paths::find_root()?;
        crate::battle::scene::setup::install_battle_rules(&paths::data_dir(&root));
        let sim = super::launch::start_of(launch, site).build()?;
        Ok(Self::of(&sim, player_side))
    }

    /// Reads an already built simulation.
    pub fn of(sim: &BattleSim, player_side: &str) -> Self {
        let player = if player_side == "defender" {
            SideId::Defender
        } else {
            SideId::Attacker
        };
        let foe = if player == SideId::Attacker {
            SideId::Defender
        } else {
            SideId::Attacker
        };
        let field = sim.field();
        let (mut own, mut enemy) = (Vec::new(), Vec::new());
        for unit in sim.units() {
            let at = Vec2::new(unit.x as f32, unit.z as f32);
            if unit.side == player {
                own.push(at);
            } else {
                enemy.push(at);
            }
        }
        let general = |side: SideId| {
            sim.setup()
                .side(side)
                .general
                .as_ref()
                .map(|g| g.character.clone())
        };
        let weather = sim.weather();
        SitePreview {
            weather_key: weather.key().to_owned(),
            weather_label: weather.label_fr().to_lowercase(),
            site_label: field.site_label_fr(),
            field: Vec2::new(field.width as f32, field.depth as f32),
            base: minimap::render_base(field, player == SideId::Attacker),
            flipped: player == SideId::Attacker,
            own,
            enemy,
            generals: [general(player), general(foe)],
        }
    }

    /// The picture: relief, obstacles and both armies' start positions in the side colours.
    pub fn raster(&self, colors: [Color; 2]) -> Raster {
        minimap::render_dynamic(
            &self.base,
            self.field,
            self.flipped,
            &self.own,
            &self.enemy,
            colors,
            false,
            None,
            None,
        )
    }
}

fn asset_exists(path: &str) -> bool {
    paths::find_root()
        .ok()
        .is_some_and(|root| paths::asset_root(&root).join(path).exists())
}

/// Picture of a general's medallion and whether it is his portrait (Godot `_general_row`): his
/// portrait, else the arms of his house, else those of his faction; `None` when nothing exists.
pub fn medallion_image(character: &str, faction: &str) -> Option<(String, bool)> {
    if !character.is_empty() {
        let portrait = format!("portraits/{character}.png");
        if asset_exists(&portrait) {
            return Some((portrait, true));
        }
        let house = paths::find_root().ok().and_then(|root| {
            let data_dir = paths::data_dir(&root);
            let text =
                std::fs::read_to_string(data_dir.join(format!("characters/{character}.json")))
                    .ok()?;
            let value: serde_json::Value = serde_json::from_str(&text).ok()?;
            crate::battle::units::banners::house_id(&data_dir, value.get("house")?.as_str()?)
        });
        if let Some(house) = house {
            let arms = format!("heraldry/houses/{house}.png");
            if asset_exists(&arms) {
                return Some((arms, false));
            }
        }
    }
    let arms = format!("heraldry/{faction}.png");
    (!faction.is_empty() && asset_exists(&arms)).then_some((arms, false))
}

impl SitePreview {
    /// Dump summary, with the medallion of each side (`factions` in slot order).
    pub fn summary(&self, factions: [&str; 2]) -> PreviewSummary {
        PreviewSummary {
            weather: self.weather_key.clone(),
            site_label: self.site_label.clone(),
            own_dots: self.own.len(),
            enemy_dots: self.enemy.len(),
            generals: self.generals.clone(),
            medallions: [0, 1].map(|slot| {
                medallion_image(self.generals[slot].as_deref().unwrap_or(""), factions[slot])
                    .map(|(path, _)| path)
            }),
        }
    }
}

fn medallion(
    row: &mut ChildSpawnerCommands,
    assets: &AssetServer,
    image: Option<(String, bool)>,
    color: Color,
) {
    row.spawn((
        Name::new("flow:medallion"),
        Node {
            width: Val::Px(MEDALLION_SIZE),
            height: Val::Px(MEDALLION_SIZE),
            flex_shrink: 0.0,
            border: UiRect::all(Val::Px(2.0)),
            overflow: Overflow::clip(),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border_radius: BorderRadius::MAX,
            ..default()
        },
        BackgroundColor(color.darker(0.3)),
        BorderColor::all(style::GOLD),
    ))
    .with_children(|disc| {
        if let Some((path, is_portrait)) = image {
            // Portraits fill the disc, arms are fitted inside it (Godot 1.3 × radius).
            let size = if is_portrait { 100.0 } else { 66.0 };
            disc.spawn((
                Node {
                    width: Val::Percent(size),
                    height: Val::Percent(size),
                    ..default()
                },
                ImageNode::new(assets.load(path)),
            ));
        }
    });
}

/// Banner height (Godot `BANNER_HEIGHT`; `BANNER_HEIGHT_SHORT` under 760 px of window).
pub const BANNER_HEIGHT: f32 = 132.0;
pub const BANNER_HEIGHT_SHORT: f32 = 84.0;
const BANNER_TITLE: Color = Color::srgb(0.99, 0.95, 0.84);
const BANNER_SUBTITLE: Color = Color::srgb(0.96, 0.88, 0.68);
const BANNER_FALLBACK: Color = Color::srgb(0.35, 0.22, 0.12);

/// What the dialogs need to draw their pictures.
pub struct DialogArt<'a> {
    pub assets: &'a AssetServer,
    pub plates: &'a ArtPlates,
    /// Window height (logical px), for the short banner.
    pub window_height: f32,
}

/// Picture of a banner: a loading plate of the context picked by `seed`, else the old miniature
/// (Godot `BANNER_FIELD` / `BANNER_SIEGE`: Crécy for a battle, Sluys for a siege or a sea fight).
pub fn banner_image(plates: &ArtPlates, context: &str, seed: u64) -> (String, String) {
    match plates.loading_screen(context, seed as usize) {
        Some(plate) => (plate.id.clone(), plate.image.clone()),
        None => {
            let id = if context == "battle" {
                "evt_crecy"
            } else {
                "evt_sluys"
            };
            (id.to_string(), format!("events/{id}.jpg"))
        }
    }
}

/// Illustrated banner: a plate of `context` (`battle`, `siege`, `naval`) picked by `seed`,
/// cover-cropped (centred), veiled towards the bottom, gold rule, title and subtitle at the bottom
/// left. Brown without a plate.
pub fn banner(
    parent: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    art: &DialogArt,
    context: &str,
    seed: u64,
    title: &str,
    subtitle: &str,
) {
    let (plate_id, plate_image) = banner_image(art.plates, context, seed);
    let height = if art.window_height >= 760.0 {
        BANNER_HEIGHT
    } else {
        BANNER_HEIGHT_SHORT
    };
    parent
        .spawn((
            Name::new("flow:banner"),
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(height),
                flex_shrink: 0.0,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                overflow: Overflow::clip(),
                border: UiRect::bottom(Val::Px(3.0)),
                ..default()
            },
            BorderColor::all(style::GOLD),
            BackgroundColor(BANNER_FALLBACK),
        ))
        .with_children(|banner| {
            {
                // Wider than tall: the full-width picture overflows and is centred (cover).
                banner.spawn((
                    Name::new(format!("flow:banner_art:{plate_id}")),
                    Node {
                        width: Val::Percent(100.0),
                        aspect_ratio: Some(16.0 / 9.0),
                        flex_shrink: 0.0,
                        ..default()
                    },
                    ImageNode::new(art.assets.load(plate_image)),
                ));
            }
            banner
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        right: Val::Px(0.0),
                        top: Val::Px(0.0),
                        bottom: Val::Px(0.0),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::FlexEnd,
                        padding: UiRect::new(
                            Val::Px(28.0),
                            Val::Px(28.0),
                            Val::Px(0.0),
                            Val::Px(14.0),
                        ),
                        ..default()
                    },
                    BackgroundGradient::from(LinearGradient::to_bottom(vec![
                        ColorStop::auto(Color::srgba(0.0, 0.0, 0.0, 0.05)),
                        ColorStop::auto(Color::srgba(0.08, 0.04, 0.02, 0.8)),
                    ])),
                ))
                .with_children(|texts| {
                    texts.spawn((
                        Text::new(title),
                        theme.title_font(UiType::Title.size()),
                        TextColor(BANNER_TITLE),
                        TextShadow {
                            offset: Vec2::splat(2.0),
                            color: Color::srgba(0.1, 0.05, 0.02, 0.9),
                        },
                    ));
                    if !subtitle.is_empty() {
                        texts.spawn((
                            Text::new(subtitle),
                            theme.text_font(false, true, UiType::Body.size()),
                            TextColor(BANNER_SUBTITLE),
                            TextShadow {
                                offset: Vec2::splat(1.5),
                                color: Color::srgba(0.1, 0.05, 0.02, 0.9),
                            },
                        ));
                    }
                });
        });
}

/// Arms of `faction` (`heraldry/<faction>.png`, 40 × 46, aspect kept); nothing when missing.
pub fn arms(parent: &mut ChildSpawnerCommands, assets: &AssetServer, faction: &str) {
    if faction.is_empty() {
        return;
    }
    let path = format!("heraldry/{faction}.png");
    let exists = paths::find_root()
        .ok()
        .is_some_and(|root| paths::asset_root(&root).join(&path).exists());
    if !exists {
        return;
    }
    parent.spawn((
        Name::new(format!("flow:arms:{faction}")),
        Node {
            width: Val::Px(40.0),
            height: Val::Px(46.0),
            flex_shrink: 0.0,
            ..default()
        },
        ImageNode::new(assets.load(path)).with_mode(bevy::ui::widget::NodeImageMode::Auto),
    ));
}

/// A side header: arms then name (player, left) or name then arms (opponent, right).
pub fn side_header(
    column: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    assets: &AssetServer,
    faction: &str,
    name: &str,
    slot: usize,
    color: Color,
) {
    column
        .spawn(Node {
            column_gap: Val::Px(10.0),
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|header| {
            if slot == 0 {
                arms(header, assets, faction);
            }
            header
                .spawn(typed(theme, name, UiType::Heading))
                .insert(TextColor(color.darker(0.1)));
            if slot != 0 {
                arms(header, assets, faction);
            }
        });
}

#[allow(clippy::too_many_arguments)]
pub fn spawn(
    commands: &mut Commands,
    theme: &UiTheme,
    art: &DialogArt,
    game: &GameData,
    brief: &BattleBrief,
    siege: bool,
    seed: u64,
    preview: Option<&SitePreview>,
    images: &mut Assets<Image>,
) {
    let dialog = PreBattleDialog {
        index: brief.index,
        title: brief.title.clone(),
        verdict: brief.verdict.clone(),
        share: brief.share,
        regiments: [brief.sides[0].regiments, brief.sides[1].regiments],
        can_withdraw: brief.can_withdraw,
        preview: preview.map(|p| p.summary([&brief.sides[0].faction, &brief.sides[1].faction])),
    };
    let colors: [Color; 2] = [0, 1].map(|slot| {
        let side = &brief.sides[slot];
        let id = if side.side == "attacker" {
            SideId::Attacker
        } else {
            SideId::Defender
        };
        let [r, g, b] = side_color(game, &side.faction, id);
        Color::srgb(r, g, b)
    });
    let colors = if colors[0] == colors[1] {
        [colors[0], colors[1].darker(0.2)]
    } else {
        colors
    };
    commands
        .spawn((
            Name::new("flow:pre_battle"),
            dialog,
            DespawnOnExit(AppState::Campaign),
            UiTier::Modal.index(),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.03, 0.02, 0.62)),
            Pickable::default(),
        ))
        .with_children(|root| {
            root.spawn(panel_sized(theme, 1080.0))
                .with_children(|panel| {
                    banner(
                        panel,
                        theme,
                        art,
                        if siege { "siege" } else { "battle" },
                        seed,
                        &brief.title,
                        &brief.subtitle,
                    );
                    // Verdict and balance bar.
                    panel
                        .spawn((
                            Name::new("flow:verdict"),
                            typed(theme, &brief.verdict, UiType::Heading),
                        ))
                        .insert(TextColor(tone_color(&brief.verdict_tone)));
                    panel
                        .spawn((
                            Name::new("flow:balance"),
                            Node {
                                width: Val::Px(620.0),
                                height: Val::Px(18.0),
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            BorderColor::all(style::INK),
                        ))
                        .with_children(|bar| {
                            bar.spawn((
                                Node {
                                    width: Val::Percent(
                                        (brief.share.clamp(0.0, 1.0) * 100.0) as f32,
                                    ),
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                                BackgroundColor(colors[0]),
                            ));
                            bar.spawn((
                                Node {
                                    flex_grow: 1.0,
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                                BackgroundColor(colors[1]),
                            ));
                        });
                    panel.spawn((
                        caption(theme, &brief.chance_text),
                        Node {
                            max_width: Val::Px(1000.0),
                            ..default()
                        },
                    ));
                    panel.spawn(separator(theme));
                    panel
                        .spawn(Node {
                            width: Val::Percent(100.0),
                            column_gap: Val::Px(24.0),
                            align_items: AlignItems::FlexStart,
                            ..default()
                        })
                        .with_children(|columns| {
                            for (slot, side) in brief.sides.iter().enumerate() {
                                side_column(
                                    columns,
                                    theme,
                                    art.assets,
                                    side,
                                    slot,
                                    colors[slot],
                                    preview.and_then(|p| p.generals[slot].as_deref()),
                                );
                            }
                        });
                    panel.spawn(separator(theme));
                    if let Some(preview) = preview {
                        let raster = preview.raster(colors);
                        let image = Image::new(
                            bevy::render::render_resource::Extent3d {
                                width: raster.width,
                                height: raster.height,
                                depth_or_array_layers: 1,
                            },
                            bevy::render::render_resource::TextureDimension::D2,
                            raster.data,
                            bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                            bevy::asset::RenderAssetUsages::default(),
                        );
                        panel.spawn((
                            Name::new("flow:site_map"),
                            Node {
                                width: Val::Px(SITE_MAP_SIZE.x),
                                height: Val::Px(SITE_MAP_SIZE.y),
                                flex_shrink: 0.0,
                                ..default()
                            },
                            ImageNode::new(images.add(image)),
                        ));
                    }
                    panel.spawn(caption(theme, &brief.conditions));
                    if let Some(preview) = preview {
                        let mut line = format!("Météo prévue\u{a0}: {}", preview.weather_label);
                        if !preview.site_label.is_empty() {
                            line += &format!("   ·   Site\u{a0}: {}", preview.site_label);
                        }
                        panel.spawn((Name::new("flow:weather"), caption(theme, &line)));
                    }
                    if !brief.modifiers.is_empty() {
                        panel.spawn((
                            Name::new("flow:modifiers"),
                            caption(theme, &brief.modifiers.join(" · ")),
                            Node {
                                max_width: Val::Px(1000.0),
                                ..default()
                            },
                        ));
                    }
                    panel
                        .spawn(Node {
                            width: Val::Percent(100.0),
                            column_gap: Val::Px(18.0),
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            margin: UiRect::top(Val::Px(8.0)),
                            ..default()
                        })
                        .with_children(|row| {
                            let mut withdraw =
                                row.spawn(button_small(theme, WITHDRAW, &brief.withdraw_label));
                            if !brief.can_withdraw {
                                withdraw.insert(bevy::ui::InteractionDisabled);
                            }
                            if !brief.auto_risk.is_empty() {
                                row.spawn((
                                    Name::new("flow:auto_risk"),
                                    caption(theme, &brief.auto_risk),
                                ))
                                .insert(TextColor(style::RUBRIC));
                            }
                            row.spawn(button_small(theme, AUTO, "Résolution automatique"));
                            row.spawn(button_primary(theme, FIGHT, &brief.fight_label));
                        });
                });
        });
}

/// Highest command level (`max_skill_level`, the stars row).
const MAX_COMMAND: u8 = 10;

/// Bubble of the command stars (`leader_command_level`: title of `tooltips.json`, then
/// "level / max").
fn command_tip(level: u8, max: u8) -> String {
    let title = TooltipTexts::load()
        .ok()
        .and_then(|texts| {
            texts
                .entry("plain", "leader_command_level")
                .map(|entry| entry.title.clone())
        })
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| "Commandement".to_owned());
    format!("[b]{title}[/b]\n{level} / {max}")
}

/// General of a side: medallion (portrait, else arms), name and command stars
/// (Godot `_general_row`); the player's medallion is on the left, the enemy's on the right.
fn general_row(
    column: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    assets: &AssetServer,
    side: &BriefSide,
    slot: usize,
    color: Color,
    character: Option<&str>,
) {
    let image = medallion_image(character.unwrap_or(""), &side.faction);
    column
        .spawn((
            Name::new("flow:general"),
            Node {
                column_gap: Val::Px(10.0),
                align_items: AlignItems::Center,
                flex_direction: if slot == 0 {
                    FlexDirection::Row
                } else {
                    FlexDirection::RowReverse
                },
                ..default()
            },
        ))
        .with_children(|row| {
            medallion(row, assets, image, color);
            row.spawn(Node {
                flex_direction: FlexDirection::Column,
                align_items: if slot == 0 {
                    AlignItems::FlexStart
                } else {
                    AlignItems::FlexEnd
                },
                ..default()
            })
            .with_children(|texts| match &side.general {
                Some(g) => {
                    texts.spawn(typed(theme, &g.name, UiType::Heading));
                    texts
                        .spawn(caption(
                            theme,
                            &format!(
                                "{}{}",
                                "★".repeat(usize::from(g.command.min(MAX_COMMAND))),
                                "☆".repeat(usize::from(MAX_COMMAND - g.command.min(MAX_COMMAND)))
                            ),
                        ))
                        .insert((
                            TextColor(style::GOLD),
                            Tooltip::new(&command_tip(g.command, MAX_COMMAND)),
                        ));
                }
                None => {
                    texts.spawn(typed(theme, "Sans chef", UiType::Heading));
                    texts
                        .spawn(caption(theme, "L’ost combat sans général : moral fragile."))
                        .insert(TextColor(style::RUBRIC));
                }
            });
        });
}

fn side_column(
    columns: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    assets: &AssetServer,
    side: &BriefSide,
    slot: usize,
    color: Color,
    general_character: Option<&str>,
) {
    columns
        .spawn((
            Name::new(format!("flow:side:{}", side.side)),
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                flex_basis: Val::Px(0.0),
                row_gap: Val::Px(4.0),
                align_items: if slot == 0 {
                    AlignItems::FlexStart
                } else {
                    AlignItems::FlexEnd
                },
                ..default()
            },
        ))
        .with_children(|column| {
            let you = if slot == 0 { "  (vous)" } else { "" };
            side_header(
                column,
                theme,
                assets,
                &side.faction,
                &format!("{}{you}", side.name),
                slot,
                color,
            );
            let role = {
                let mut chars = side.role.chars();
                chars
                    .next()
                    .map(|c| c.to_uppercase().collect::<String>() + chars.as_str())
                    .unwrap_or_default()
            };
            column.spawn(caption(
                theme,
                &format!(
                    "{role} · {} hommes en {} régiments",
                    fr::digits(i64::from(side.soldiers)),
                    side.regiments
                ),
            ));
            general_row(column, theme, assets, side, slot, color, general_character);
            let reinforcements = if side.reinforcements.is_empty() {
                "Aucun renfort à portée.".to_owned()
            } else {
                format!("Renforts : {}", side.reinforcements)
            };
            column.spawn(caption(theme, &reinforcements));
            // Regiment cards.
            column
                .spawn(Node {
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(4.0),
                    row_gap: Val::Px(4.0),
                    justify_content: if slot == 0 {
                        JustifyContent::FlexStart
                    } else {
                        JustifyContent::FlexEnd
                    },
                    ..default()
                })
                .with_children(|grid| {
                    for unit in &side.units {
                        grid.spawn((
                            Name::new("flow:regiment"),
                            Node {
                                width: Val::Px(76.0),
                                min_height: Val::Px(52.0),
                                padding: UiRect::all(Val::Px(3.0)),
                                flex_direction: FlexDirection::Column,
                                border: UiRect::all(Val::Px(if unit.is_general {
                                    2.0
                                } else {
                                    1.0
                                })),
                                ..default()
                            },
                            BackgroundColor(style::PARCHMENT_LIGHT),
                            BorderColor::all(if unit.is_general { style::GOLD } else { color }),
                            children![
                                (
                                    Text::new(unit.name.clone()),
                                    theme.text_font(false, false, 11.0),
                                    TextColor(style::INK),
                                ),
                                (
                                    Text::new(if unit.soldiers == unit.max_soldiers {
                                        format!("{}", unit.soldiers)
                                    } else {
                                        format!("{}/{}", unit.soldiers, unit.max_soldiers)
                                    }),
                                    theme.text_font(true, false, 12.0),
                                    TextColor(style::INK_SOFT),
                                ),
                            ],
                        ));
                    }
                });
            column.spawn((
                caption(theme, &side.composition),
                Node {
                    max_width: Val::Px(500.0),
                    ..default()
                },
            ));
        });
}

#[cfg(test)]
mod banner_tests {
    use super::*;

    #[test]
    fn banner_falls_back_to_event_miniature() {
        let plates = ArtPlates::default();
        assert_eq!(banner_image(&plates, "battle", 3).1, "events/evt_crecy.jpg");
        assert_eq!(banner_image(&plates, "siege", 3).1, "events/evt_sluys.jpg");
        assert_eq!(banner_image(&plates, "naval", 0).1, "events/evt_sluys.jpg");
    }
}

#[cfg(test)]
mod command_tip_tests {
    use super::*;

    #[test]
    fn stars_bubble_gives_level_over_max() {
        assert_eq!(command_tip(3, 10), "[b]Commandement[/b]\n3 / 10");
        assert_eq!(command_tip(0, MAX_COMMAND), "[b]Commandement[/b]\n0 / 10");
    }
}

#[cfg(test)]
mod preview_tests {
    use super::*;

    fn preview(player: &str) -> SitePreview {
        let root = paths::find_root().expect("repo root");
        let dir = paths::data_dir(&root);
        let (game, _) = GameData::load(&dir).expect("game data");
        crate::battle::scene::setup::install_battle_rules(&dir);
        let custom = crate::battle::scene::setup::default_custom_battle(&game);
        let sim = crate::battle::scene::setup::build_sim(&game, &dir, &custom, 7).unwrap();
        SitePreview::of(&sim, player)
    }

    #[test]
    fn preview_reads_weather_site_and_both_armies() {
        let p = preview("attacker");
        assert!(["clear", "rain", "fog", "snow"].contains(&p.weather_key.as_str()));
        assert!(!p.weather_label.is_empty());
        assert!(!p.own.is_empty() && !p.enemy.is_empty());
        assert!(p.flipped);
        let raster = p.raster([Color::srgb(0.2, 0.3, 0.8), Color::srgb(0.8, 0.2, 0.1)]);
        assert_eq!((raster.width, raster.height), (400, 248));
        // Same seed, same weather; the defender's view only turns the map.
        let q = preview("defender");
        assert_eq!(p.weather_key, q.weather_key);
        assert!(!q.flipped);
        assert_eq!(p.own.len(), q.enemy.len());
    }

    #[test]
    fn medallion_falls_back_to_arms() {
        assert!(medallion_image("no_such_character", "").is_none());
        let found = medallion_image("no_such_character", "fac_alania");
        assert_eq!(found, Some(("heraldry/fac_alania.png".to_owned(), false)));
    }
}
