//! Illustrated loading card shown while a battle is built (`battle_loading_card.gd`).
//!
//! An illumination in a gold frame, the episode's title and a chronicler's quote
//! (`data/ui/illustrations.json`, `loading`), shown at least `min_seconds` (3 s) even when the
//! scene builds faster; a click or Space / Escape / Enter passes the rest of the wait. The card
//! lives above every state (it is not tied to `AppState`), so it covers the switch to the battle.
//! Not ported: the dimmed full-screen copy of the picture (Bevy UI cannot cover-fit an image)
//! and the 0.35 s fade (no per-node alpha).

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use serde_json::Value;

use crate::paths;
use crate::ui::kit::style::{self, UiType};
use crate::ui::kit::widgets::{caption, panel_inset};
use crate::ui::kit::{UiTheme, typed};

/// Seconds below which a card is never closed (`min_seconds` of the data file).
const DEFAULT_MIN_SECONDS: f32 = 3.0;
/// Z index of the card (above the modal tier, like the Godot canvas layer 110).
const CARD_Z: i32 = 110;

#[derive(Debug, Clone, Default)]
struct Quote {
    text: String,
    author: String,
    source: String,
    date: String,
}

#[derive(Debug, Clone, Default)]
struct Screen {
    id: String,
    contexts: Vec<String>,
    /// Path relative to `game/assets`.
    image: String,
    title: String,
    quote: Option<Quote>,
}

/// Loading screens read from `data/ui/illustrations.json`.
#[derive(Resource, Default)]
pub struct LoadingScreens {
    screens: Vec<Screen>,
    pub min_seconds: f32,
}

impl LoadingScreens {
    /// Parses the `loading` section of the illustrations file.
    pub fn parse(json: &Value) -> Self {
        let loading = &json["loading"];
        let text = |v: &Value| v.as_str().unwrap_or_default().to_owned();
        let screens = loading["screens"]
            .as_array()
            .map(|list| {
                list.iter()
                    .map(|s| Screen {
                        id: text(&s["id"]),
                        contexts: s["contexts"]
                            .as_array()
                            .map(|c| c.iter().map(&text).collect())
                            .unwrap_or_default(),
                        image: text(&s["image"])
                            .trim_start_matches("res://assets/")
                            .to_owned(),
                        title: text(&s["title"]),
                        quote: s.get("quote").filter(|q| q.is_object()).map(|q| Quote {
                            text: text(&q["text"]),
                            author: text(&q["author"]),
                            source: text(&q["source"]),
                            date: text(&q["date"]),
                        }),
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self {
            screens,
            min_seconds: loading["min_seconds"]
                .as_f64()
                .map_or(DEFAULT_MIN_SECONDS, |v| v as f32),
        }
    }

    /// Screens of a context (`battle`, `siege`, `naval`, `campaign`).
    fn of_context(&self, context: &str) -> Vec<&Screen> {
        self.screens
            .iter()
            .filter(|s| s.contexts.iter().any(|c| c == context))
            .collect()
    }

    pub fn count(&self, context: &str) -> usize {
        self.of_context(context).len()
    }

    fn pick(&self, context: &str, seed: u64) -> Option<&Screen> {
        let list = self.of_context(context);
        (!list.is_empty()).then(|| list[(seed % list.len() as u64) as usize])
    }
}

/// The open card.
#[derive(Component, Debug, Clone)]
pub struct LoadingCard {
    pub context: String,
    pub screen_id: String,
    pub elapsed: f32,
    /// Frames rendered since it was spawned (the card is "drawn" after two).
    pub frames: u32,
    pub skipped: bool,
    pub min_seconds: f32,
}

impl LoadingCard {
    pub fn drawn(&self) -> bool {
        self.frames >= 2
    }

    /// The minimum time has passed, or the player skipped the wait.
    pub fn may_close(&self) -> bool {
        self.skipped || self.elapsed >= self.min_seconds
    }
}

pub fn load_screens(mut commands: Commands) {
    let loaded = paths::find_root().and_then(|root| {
        let path = paths::data_dir(&root).join("ui/illustrations.json");
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        serde_json::from_str::<Value>(&text).map_err(|e| e.to_string())
    });
    match loaded {
        Ok(json) => commands.insert_resource(LoadingScreens::parse(&json)),
        Err(message) => {
            warn!("loading screens: {message}");
            commands.insert_resource(LoadingScreens {
                min_seconds: DEFAULT_MIN_SECONDS,
                ..default()
            });
        }
    }
}

/// Opens the card for `context` (`battle` or `siege`); `seed` picks the screen.
#[allow(clippy::too_many_arguments)]
pub fn open_card(
    commands: &mut Commands,
    theme: &UiTheme,
    assets: &AssetServer,
    screens: &LoadingScreens,
    context: &str,
    seed: u64,
    window_height: f32,
    min_seconds: f32,
) -> Entity {
    let screen = screens.pick(context, seed).cloned().unwrap_or_default();
    let kicker = match context {
        "siege" => "SIÈGE",
        "naval" => "BATAILLE SUR MER",
        "campaign" => "CAMPAGNE",
        _ => "BATAILLE RANGÉE",
    };
    let art_height = (window_height * 0.58).clamp(180.0, 619.0);
    let art_width = art_height * 16.0 / 9.0;
    let card = LoadingCard {
        context: context.to_owned(),
        screen_id: screen.id.clone(),
        elapsed: 0.0,
        frames: 0,
        skipped: false,
        min_seconds,
    };
    commands
        .spawn((
            Name::new("flow:loading_card"),
            card,
            GlobalZIndex(CARD_Z),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(14.0),
                padding: UiRect::axes(Val::Px(56.0), Val::Px(32.0)),
                ..default()
            },
            BackgroundColor(style::FE_NIGHT),
            Pickable::default(),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new(kicker),
                theme.title_font(UiType::Caption.size()),
                TextColor(style::FE_GOLD),
            ));
            if !screen.title.is_empty() {
                root.spawn(typed(theme, &screen.title, UiType::Title))
                    .insert(TextColor(Color::srgb(0.97, 0.92, 0.80)));
            }
            root.spawn(panel_inset(theme))
                .insert(Node {
                    width: Val::Px(art_width),
                    height: Val::Px(art_height),
                    padding: UiRect::all(Val::Px(8.0)),
                    ..default()
                })
                .with_children(|frame| {
                    if !screen.image.is_empty() {
                        frame.spawn((
                            Name::new("flow:loading_art"),
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            ImageNode::new(assets.load(screen.image.clone())),
                        ));
                    }
                });
            if let Some(quote) = &screen.quote {
                root.spawn((
                    Text::new(format!("« {} »", quote.text)),
                    theme.text_font(false, true, UiType::Heading.size()),
                    TextColor(Color::srgb(0.95, 0.90, 0.78)),
                    TextLayout::justify(Justify::Center),
                    Node {
                        max_width: Val::Px(art_width.min(1000.0)),
                        ..default()
                    },
                ));
                root.spawn((
                    Text::new(format!(
                        "— {}, {} ({})",
                        quote.author, quote.source, quote.date
                    )),
                    theme.text_font(false, false, UiType::Body.size()),
                    TextColor(style::FE_GOLD),
                    TextLayout::justify(Justify::Center),
                ));
            }
            root.spawn(caption(
                theme,
                "Les armées prennent position… (clic pour passer)",
            ))
            .insert(TextColor(Color::srgb(0.80, 0.74, 0.62)));
        })
        .id()
}

/// Counts time and frames, and notes a skip (click, Space, Escape, Enter).
pub fn tick_cards(
    time: Res<Time<Real>>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut cards: Query<&mut LoadingCard>,
) {
    let skip = mouse.just_pressed(MouseButton::Left)
        || keys.just_pressed(KeyCode::Space)
        || keys.just_pressed(KeyCode::Escape)
        || keys.just_pressed(KeyCode::Enter);
    for mut card in &mut cards {
        card.elapsed += time.delta_secs();
        card.frames += 1;
        card.skipped |= skip;
    }
}

/// Height of the primary window (logical px), 900 without one.
pub fn window_height(windows: &Query<&Window, With<PrimaryWindow>>) -> f32 {
    windows.single().map_or(900.0, |w| w.height())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_real_file_and_picks_by_context() {
        let root = paths::find_root().expect("repo root");
        let text = std::fs::read_to_string(paths::data_dir(&root).join("ui/illustrations.json"))
            .expect("illustrations.json");
        let screens = LoadingScreens::parse(&serde_json::from_str(&text).unwrap());
        assert!(screens.count("battle") > 0 && screens.count("siege") > 0);
        let screen = screens.pick("battle", 7).expect("a battle screen");
        assert!(screen.image.starts_with("art/loading/"), "{}", screen.image);
        assert!(!screen.title.is_empty());
        assert!((screens.min_seconds - 3.0).abs() < 1e-3);
        assert!(screens.pick("nope", 1).is_none());
    }

    #[test]
    fn card_closes_after_minimum_or_skip() {
        let mut card = LoadingCard {
            context: "battle".into(),
            screen_id: String::new(),
            elapsed: 1.0,
            frames: 1,
            skipped: false,
            min_seconds: 3.0,
        };
        assert!(!card.may_close() && !card.drawn());
        card.frames = 2;
        assert!(card.drawn());
        card.skipped = true;
        assert!(card.may_close());
    }
}
