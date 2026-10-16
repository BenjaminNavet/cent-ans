//! Dressing of the campaign loading plate with what `loading_screen.gd` adds to the picture:
//! the plate's picture again as a full-screen backdrop (cover-cropped, darkened to
//! `Color(0.24, 0.2, 0.17)`), the faction's shield with its name and the campaign date at the
//! top left, and a game tip at the bottom. Kept apart from `loading.rs`: it only decorates the
//! card that module spawns (`CampaignCard`), as a first child (so behind) and two absolute
//! overlays. Tips come from `data/ui/front_end.json` (`loading.tips`), picked by the card's seed
//! (its plate id).

use bevy::prelude::*;
use serde_json::Value;

use super::loading::{CampaignCard, LoadingCard};
use crate::agent::dump::DumpAppExt;
use crate::menu::CoverFit;
use crate::sim::Sim;
use crate::ui::art_plates::ArtPlates;
use crate::ui::kit::style::{self, UiType};
use crate::ui::kit::{UiTheme, typed};

/// Tint of the backdrop picture (`Color(0.24, 0.2, 0.17)` of `loading_screen.gd`).
const BACKDROP_TINT: Color = Color::srgb(0.24, 0.20, 0.17);

/// Marks the pieces added to the card.
#[derive(Component)]
pub struct VeilPart(pub &'static str);

/// What the dressing last showed (dump).
#[derive(Resource, Default, Debug)]
pub struct VeilReport {
    pub faction: String,
    pub subtitle: String,
    pub tip: String,
    pub backdrop: String,
    pub shield: bool,
}

pub fn plugin(app: &mut App) {
    app.init_resource::<VeilReport>()
        .register_dump(dump_veil)
        .add_systems(Update, dress_card);
}

/// Tips of `data/ui/front_end.json` (`loading.tips`), empty when unreadable.
pub fn load_tips() -> Vec<String> {
    let Ok(root) = crate::paths::find_root() else {
        return Vec::new();
    };
    std::fs::read_to_string(crate::paths::data_dir(&root).join("ui/front_end.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .map(|json| parse_tips(&json))
        .unwrap_or_default()
}

pub fn parse_tips(json: &Value) -> Vec<String> {
    json.pointer("/loading/tips")
        .and_then(Value::as_array)
        .map(|tips| {
            tips.iter()
                .filter_map(|t| t.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// Deterministic pick from the plate id (the card's own random choice).
pub fn pick<'a>(tips: &'a [String], key: &str) -> Option<&'a String> {
    if tips.is_empty() {
        return None;
    }
    let hash = key.bytes().fold(7usize, |h, b| {
        h.wrapping_mul(31).wrapping_add(usize::from(b))
    });
    tips.get(hash % tips.len())
}

#[allow(clippy::too_many_arguments)]
fn dress_card(
    mut commands: Commands,
    cards: Query<(Entity, &LoadingCard), Added<CampaignCard>>,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
    plates: Res<ArtPlates>,
    sim: Res<Sim>,
    mut report: ResMut<VeilReport>,
) {
    for (entity, card) in &cards {
        let image = plates
            .loading_screens("campaign")
            .into_iter()
            .find(|p| p.id == card.screen_id)
            .map(|p| p.image.clone())
            .unwrap_or_default();
        let faction = sim.player.as_str().to_owned();
        let name = sim
            .data
            .as_ref()
            .map_or_else(|| "Cent Ans".to_owned(), |d| d.faction_name(&sim.player));
        let subtitle = sim.state.as_ref().map_or_else(String::new, |state| {
            let date = state.date_label();
            if state.turn == 0 {
                format!("Nouvelle campagne — {date}")
            } else {
                format!("Campagne — {date}")
            }
        });
        let tip = pick(&load_tips(), &card.screen_id)
            .cloned()
            .unwrap_or_default();
        let shield_path = format!("heraldry/{faction}.png");
        let has_shield = crate::paths::find_root()
            .ok()
            .is_some_and(|root| crate::paths::asset_root(&root).join(&shield_path).is_file());
        *report = VeilReport {
            faction: faction.clone(),
            subtitle: subtitle.clone(),
            tip: tip.clone(),
            backdrop: image.clone(),
            shield: has_shield,
        };

        // Backdrop: first child, so every other part of the card is drawn over it.
        let backdrop = commands
            .spawn((
                VeilPart("backdrop"),
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(style::FE_NIGHT),
                Pickable::IGNORE,
            ))
            .with_children(|back| {
                if !image.is_empty() {
                    back.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        ImageNode::new(assets.load(image.clone())).with_color(BACKDROP_TINT),
                        CoverFit::new(Vec2::splat(0.5)),
                        Pickable::IGNORE,
                    ));
                }
            })
            .id();
        commands.entity(entity).insert_children(0, &[backdrop]);

        // Header: shield, faction name and date.
        let header = commands
            .spawn((
                VeilPart("header"),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(64.0),
                    top: Val::Px(40.0),
                    column_gap: Val::Px(16.0),
                    align_items: AlignItems::Center,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|head| {
                if has_shield {
                    head.spawn((
                        Node {
                            width: Val::Px(56.0),
                            height: Val::Px(64.0),
                            ..default()
                        },
                        ImageNode::new(assets.load(shield_path.clone())),
                    ));
                }
                head.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    ..default()
                })
                .with_children(|titles| {
                    titles
                        .spawn(typed(&theme, &name, UiType::Title))
                        .insert(TextColor(Color::srgb(0.97, 0.92, 0.80)));
                    titles.spawn((
                        Text::new(subtitle.clone()),
                        theme.title_font(UiType::Heading.size()),
                        TextColor(style::FE_GOLD),
                    ));
                });
            })
            .id();

        // Footer: the tip.
        let mut footer = commands.spawn((
            VeilPart("tip"),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(64.0),
                right: Val::Px(64.0),
                bottom: Val::Px(36.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(3.0),
                ..default()
            },
            Pickable::IGNORE,
        ));
        if !tip.is_empty() {
            footer.with_children(|foot| {
                foot.spawn((
                    Text::new("Conseil"),
                    theme.title_font(UiType::Heading.size()),
                    TextColor(style::FE_GOLD),
                ));
                foot.spawn((
                    Text::new(tip.clone()),
                    theme.text_font(false, false, UiType::Heading.size()),
                    TextColor(Color::srgb(0.88, 0.83, 0.72)),
                ));
            });
        }
        let footer = footer.id();
        commands.entity(entity).add_children(&[header, footer]);
    }
}

fn dump_veil(world: &World) -> (String, Value) {
    let report = world.resource::<VeilReport>();
    let mut parts = world.try_query::<&VeilPart>();
    let live: Vec<&str> = parts
        .as_mut()
        .map(|q| q.iter(world).map(|p| p.0).collect())
        .unwrap_or_default();
    (
        "loading_veil".to_owned(),
        serde_json::json!({
            "faction": report.faction,
            "subtitle": report.subtitle,
            "tip": report.tip,
            "backdrop": report.backdrop,
            "shield": report.shield,
            "live_parts": live,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tips_come_from_front_end_data() {
        let json = serde_json::json!({"loading": {"tips": ["a", "b"]}});
        assert_eq!(parse_tips(&json), vec!["a", "b"]);
        assert!(parse_tips(&serde_json::json!({})).is_empty());
    }

    #[test]
    fn real_data_has_tips_and_a_stable_pick() {
        let tips = load_tips();
        assert!(!tips.is_empty());
        assert_eq!(pick(&tips, "plate_x"), pick(&tips, "plate_x"));
        assert_eq!(pick(&[], "k"), None);
    }
}
