//! Illustrated loading card shown while a battle is built (`battle_loading_card.gd`).
//!
//! An illumination in a gold frame, the episode's title and a chronicler's quote
//! (`data/ui/illustrations.json`, `loading`, read once by `ui::art_plates::ArtPlates`), shown at least `min_seconds` (3 s) even when the
//! scene builds faster; a click or Space / Escape / Enter passes the rest of the wait. The card
//! lives above every state (it is not tied to `AppState`), so it covers the switch to the battle.
//! Not ported: the dimmed full-screen copy of the picture (Bevy UI cannot cover-fit an image)
//! and the 0.35 s fade (no per-node alpha).

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::ui::art_plates::ArtPlates;
use crate::ui::kit::style::{self, UiType};
use crate::ui::kit::widgets::{caption, panel_inset};
use crate::ui::kit::{UiTheme, typed};

/// Z index of the card (above the modal tier, like the Godot canvas layer 110).
const CARD_Z: i32 = 110;

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

/// Opens the card for `context` (`battle` or `siege`); `seed` picks the screen.
#[allow(clippy::too_many_arguments)]
pub fn open_card(
    commands: &mut Commands,
    theme: &UiTheme,
    assets: &AssetServer,
    plates: &ArtPlates,
    context: &str,
    seed: u64,
    window_height: f32,
    min_seconds: f32,
) -> Entity {
    let screen = plates
        .loading_screen(context, seed as usize)
        .cloned()
        .unwrap_or_default();
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

/// Marks the card shown while the campaign is entered (`loading_screen.gd`, context `campaign`).
#[derive(Component, Debug)]
pub struct CampaignCard;

/// Seconds the campaign card stays (`CENT_ANS_LOADING_SECONDS` overrides, 0 = no card; scripted
/// runs get none so that their clicks are not swallowed).
pub fn campaign_card_seconds(plates: &ArtPlates, scripted: bool) -> f32 {
    if let Some(seconds) = std::env::var("CENT_ANS_LOADING_SECONDS")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
    {
        return seconds;
    }
    if scripted { 0.0 } else { plates.min_seconds }
}

/// `OnEnter(Campaign)`: the illustrated plate of the `campaign` context (Paris, Avignon).
pub fn open_campaign_card(
    mut commands: Commands,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
    plates: Res<ArtPlates>,
    cli: Option<Res<crate::agent::cli::Cli>>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let scripted = cli.is_some_and(|cli| cli.script.is_some());
    let seconds = campaign_card_seconds(&plates, scripted);
    if seconds <= 0.0 || plates.loading_screens("campaign").is_empty() {
        return;
    }
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos() as u64);
    let card = open_card(
        &mut commands,
        &theme,
        &assets,
        &plates,
        "campaign",
        seed,
        window_height(&windows),
        seconds,
    );
    commands.entity(card).insert(CampaignCard);
}

/// Closes the campaign card once drawn and past its minimum time (or skipped).
pub fn close_campaign_card(
    mut commands: Commands,
    cards: Query<(Entity, &LoadingCard), With<CampaignCard>>,
) {
    for (entity, card) in &cards {
        if card.drawn() && card.may_close() {
            commands.entity(entity).despawn();
        }
    }
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
