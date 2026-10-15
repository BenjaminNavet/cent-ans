//! Parchment UI kit (ADR 0357): fonts, theme, and bundle helpers every screen builds with.
//!
//! Ports game/scripts/ui/{hud_style,front_end_style,ui_type,rich_*,tooltip_*,panel_section,
//! confirm_*,list_menu,icon_chip,money,fr_text,modal_queue,ui_motion,lettrine,
//! illuminated_title}.gd and `scenes/ui/parchment_theme.tres`.
//!
//! - [`UiTheme`]: colours, fonts (EB Garamond body with Noto symbol fallbacks, IM Fell titles) and
//!   the 9-slice textures of `assets/ui/illumination/` with the margins of its `kit.json`.
//! - [`panel`], [`title`], [`label`], [`button`] (compatible API) and [`widgets`]: panels,
//!   sections, separators, buttons (4 states, primary, icon), toggles, checkboxes, tabs, sliders,
//!   scroll lists, [`richtext`] blocks, [`tooltip`] chains, [`modal`] queue and toasts.
//! - [`ButtonPressed`]: message sent when a kit button is pressed, by the mouse or by the agent
//!   script (`click <Name>`). Screens read it; they never poll `Interaction` themselves. Toggles,
//!   checkboxes, tabs and sliders also send [`ToggleChanged`], [`TabSelected`], [`SliderChanged`].
//! - [`fr`]: French numbers, money (`₶`), plurals and dates.
//! - [`UiCamera`]: the 2D camera that draws the UI (default UI camera, offscreen-aware).
//! - Gallery: `CENT_ANS_KIT_GALLERY=1` + `MainMenu` shows every widget ([`gallery`]).
//! - Dump section `ui_kit`: widget counts by kind, open tooltips, modal queue.

pub mod fr;
pub mod gallery;
pub mod modal;
pub mod richtext;
pub mod style;
pub mod tooltip;
pub mod widgets;

use std::collections::HashMap;

use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::sprite::{BorderRect, SliceScaleMode, TextureSlicer};
use bevy::text::{FontFeatureTag, FontFeatures, FontWeight};
use bevy::ui::IsDefaultUiCamera;
use bevy::ui_widgets::Activate;
use serde_json::Value;

use crate::agent::OffscreenTarget;
use crate::agent::dump::DumpAppExt;
use crate::paths;

pub use richtext::{RichLink, RichStyle, rich_text};
pub use style::{UiTier, UiType};
pub use widgets::*;

/// Default UI font, relative to `game/assets`.
pub const FONT_PATH: &str = "third_party/fonts/eb_garamond/EBGaramond-VariableFont_wght.ttf";
const FONT_ITALIC_PATH: &str =
    "third_party/fonts/eb_garamond/EBGaramond-Italic-VariableFont_wght.ttf";
const TITLE_FONT_PATH: &str = "third_party/fonts/im_fell_english/IMFeENrm28P.ttf";
const TITLE_ITALIC_PATH: &str = "third_party/fonts/im_fell_english/IMFeENit28P.ttf";
/// Fallback faces for `₶`, `▲`, `✓`, `⚠`… (`parchment_theme.tres`: currency, symbols, symbols 2).
const FALLBACK_FONT_PATHS: [&str; 3] = [
    "third_party/fonts/noto/NotoSerif-Currency.ttf",
    "third_party/fonts/noto/NotoSansSymbols-Subset.ttf",
    "third_party/fonts/noto/NotoSansSymbols2-Subset.ttf",
];
/// Directory of the illumination kit relative to `game/assets`.
const KIT_DIR: &str = "ui/illumination";
const WEIGHT_BODY: u16 = 470;
const WEIGHT_BOLD: u16 = 650;

/// 9-slice metrics of one kit texture (`kit.json`): painted border and content margins,
/// `[left, top, right, bottom]` in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SliceMetrics {
    pub size: [f32; 2],
    pub margins: [f32; 4],
    pub content: [f32; 4],
}

#[derive(Resource, Clone)]
pub struct UiTheme {
    pub background: Color,
    pub border: Color,
    pub text: Color,
    /// Button fill, and its fill under the pointer (flat fallback when textures are missing).
    pub button: Color,
    pub button_hover: Color,
    /// Body font: EB Garamond then the symbol fallbacks.
    pub font: FontSource,
    pub font_italic: FontSource,
    /// IM Fell English (titles) and its italic.
    pub font_title: FontSource,
    pub font_title_italic: FontSource,
    /// Kit textures by `kit.json` key (`panel`, `button_normal`, `tab_selected`…).
    pub textures: HashMap<String, Handle<Image>>,
    pub metrics: HashMap<String, SliceMetrics>,
}

impl Default for UiTheme {
    fn default() -> Self {
        Self {
            background: style::PARCHMENT,
            border: style::INK_SOFT,
            text: style::INK,
            button: style::INK_SOFT,
            button_hover: Color::srgb(0.54, 0.38, 0.23),
            font: FontSource::default(),
            font_italic: FontSource::default(),
            font_title: FontSource::default(),
            font_title_italic: FontSource::default(),
            textures: HashMap::new(),
            metrics: HashMap::new(),
        }
    }
}

impl UiTheme {
    /// Body `TextFont` (Garamond, lining figures); `bold` = weight 650, else 470.
    pub fn text_font(&self, bold: bool, italic: bool, size: f32) -> TextFont {
        TextFont {
            font: if italic {
                self.font_italic.clone()
            } else {
                self.font.clone()
            },
            font_size: size.into(),
            weight: FontWeight(if bold { WEIGHT_BOLD } else { WEIGHT_BODY }),
            font_features: FontFeatures::from([FontFeatureTag::LINING_FIGURES]),
            ..default()
        }
    }

    /// Title `TextFont` (IM Fell English).
    pub fn title_font(&self, size: f32) -> TextFont {
        TextFont {
            font: self.font_title.clone(),
            font_size: size.into(),
            ..default()
        }
    }

    /// Metrics of kit texture `name` (zero when `kit.json` is missing).
    pub fn metric(&self, name: &str) -> SliceMetrics {
        self.metrics.get(name).copied().unwrap_or_default()
    }

    /// 9-slice `ImageNode` of kit texture `name` (tiled centre and sides, like `TILE_FIT`).
    pub fn sliced(&self, name: &str) -> ImageNode {
        let metric = self.metric(name);
        let [left, top, right, bottom] = metric.margins;
        let tile = SliceScaleMode::Tile { stretch_value: 1.0 };
        ImageNode {
            image: self.textures.get(name).cloned().unwrap_or_default(),
            image_mode: NodeImageMode::Sliced(TextureSlicer {
                border: BorderRect {
                    min_inset: Vec2::new(left, top),
                    max_inset: Vec2::new(right, bottom),
                },
                center_scale_mode: tile,
                sides_scale_mode: tile,
                max_corner_scale: 1.0,
            }),
            ..default()
        }
    }

    /// Content margins of kit texture `name` as node padding.
    pub fn padding(&self, name: &str) -> UiRect {
        let [left, top, right, bottom] = self.metric(name).content;
        UiRect {
            left: Val::Px(left),
            top: Val::Px(top),
            right: Val::Px(right),
            bottom: Val::Px(bottom),
        }
    }

    /// Plain texture (no slicing) of kit texture `name`.
    pub fn image(&self, name: &str) -> ImageNode {
        ImageNode::new(self.textures.get(name).cloned().unwrap_or_default())
    }
}

/// A kit button was pressed (mouse or agent).
#[derive(Message, Debug, Clone)]
pub struct ButtonPressed {
    pub entity: Entity,
    /// The button's `Name`.
    pub name: String,
}

#[derive(Component)]
pub struct UiCamera;

pub struct UiKitPlugin;

impl Plugin for UiKitPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiTheme>()
            .add_message::<ButtonPressed>()
            .add_plugins((
                widgets::WidgetsPlugin,
                tooltip::TooltipPlugin,
                modal::ModalPlugin,
                gallery::GalleryPlugin,
            ))
            .add_systems(Startup, (load_resources, spawn_ui_camera).chain())
            .add_observer(on_activate)
            .register_dump(dump_ui_kit);
    }
}

fn read_asset(relative: &str) -> Result<Vec<u8>, String> {
    let root = paths::find_root()?;
    let path = paths::asset_root(&root).join(relative);
    std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))
}

/// Loads the fonts from the repository (the built-in font is ASCII-only) and the kit textures
/// and metrics (`kit.json`).
fn load_resources(
    mut theme: ResMut<UiTheme>,
    mut fonts: ResMut<Assets<Font>>,
    assets: Res<AssetServer>,
) {
    let mut load_font = |relative: &str| -> Option<FontSource> {
        match read_asset(relative) {
            Ok(bytes) => Some(FontSource::Handle(fonts.add(Font::from_bytes(bytes)))),
            Err(message) => {
                warn!("{message}; font skipped");
                None
            }
        }
    };
    let fallbacks: Vec<FontSource> = FALLBACK_FONT_PATHS
        .iter()
        .filter_map(|path| load_font(path))
        .collect();
    let with_fallbacks = |primary: Option<FontSource>| -> FontSource {
        match primary {
            Some(primary) => {
                let mut list = vec![primary];
                list.extend(fallbacks.iter().cloned());
                FontSource::List(list)
            }
            None => FontSource::default(),
        }
    };
    theme.font = with_fallbacks(load_font(FONT_PATH));
    theme.font_italic = with_fallbacks(load_font(FONT_ITALIC_PATH));
    theme.font_title = with_fallbacks(load_font(TITLE_FONT_PATH));
    theme.font_title_italic = with_fallbacks(load_font(TITLE_ITALIC_PATH));

    match read_asset(&format!("{KIT_DIR}/kit.json")) {
        Ok(bytes) => match parse_kit_metrics(&bytes) {
            Ok(metrics) => {
                for name in metrics.keys() {
                    let handle = assets.load(format!("{KIT_DIR}/{name}.png"));
                    theme.textures.insert(name.clone(), handle);
                }
                theme.metrics = metrics;
            }
            Err(message) => warn!("kit.json: {message}"),
        },
        Err(message) => warn!("{message}; flat UI"),
    }
}

/// Parses `kit.json` (`{name: {size, margins, content}}`).
pub fn parse_kit_metrics(bytes: &[u8]) -> Result<HashMap<String, SliceMetrics>, String> {
    let value: Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let object = value.as_object().ok_or("not an object")?;
    let numbers = |entry: &Value, key: &str| -> Vec<f32> {
        entry
            .get(key)
            .and_then(Value::as_array)
            .map(|array| {
                array
                    .iter()
                    .filter_map(Value::as_f64)
                    .map(|v| v as f32)
                    .collect()
            })
            .unwrap_or_default()
    };
    let mut result = HashMap::new();
    for (name, entry) in object {
        let size = numbers(entry, "size");
        let margins = numbers(entry, "margins");
        let content = numbers(entry, "content");
        if size.len() != 2 || margins.len() != 4 || content.len() != 4 {
            return Err(format!("entry `{name}` is malformed"));
        }
        result.insert(
            name.clone(),
            SliceMetrics {
                size: [size[0], size[1]],
                margins: [margins[0], margins[1], margins[2], margins[3]],
                content: [content[0], content[1], content[2], content[3]],
            },
        );
    }
    Ok(result)
}

fn spawn_ui_camera(mut commands: Commands, target: Option<Res<OffscreenTarget>>) {
    let mut camera = commands.spawn((
        Name::new("UiCamera"),
        UiCamera,
        Camera2d,
        IsDefaultUiCamera,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.13, 0.10, 0.07)),
            ..default()
        },
    ));
    if let Some(target) = target {
        camera.insert(RenderTarget::Image(target.0.clone().into()));
    }
}

/// A kit button was activated by the mouse (or keyboard): forwards it as [`ButtonPressed`].
fn on_activate(
    activate: On<Activate>,
    names: Query<&Name>,
    mut pressed: MessageWriter<ButtonPressed>,
) {
    if let Ok(name) = names.get(activate.entity) {
        pressed.write(ButtonPressed {
            entity: activate.entity,
            name: name.to_string(),
        });
    }
}

fn text(theme: &UiTheme, content: &str, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(content),
        theme.text_font(false, false, size),
        TextColor(color),
    )
}

/// Parchment panel (`panel.png` frame): centred column container. Add children with
/// `with_children` or `children![]`. Padding = painted frame content margin + 10 px.
pub fn panel(theme: &UiTheme) -> impl Bundle {
    let content = theme.metric("panel").content;
    (
        Node {
            padding: UiRect {
                left: Val::Px(content[0] + 10.0),
                top: Val::Px(content[1] + 10.0),
                right: Val::Px(content[2] + 10.0),
                bottom: Val::Px(content[3] + 10.0),
            },
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(16.0),
            ..default()
        },
        theme.sliced("panel"),
        BackgroundColor(Color::NONE),
        BorderColor::all(Color::NONE),
    )
}

/// Large heading text (IM Fell, 48 px: screen titles of the front end).
pub fn title(theme: &UiTheme, content: &str) -> impl Bundle {
    (
        Text::new(content),
        theme.title_font(48.0),
        TextColor(style::INK_TITLE),
    )
}

/// Body text.
pub fn label(theme: &UiTheme, content: &str) -> impl Bundle {
    text(theme, content, 20.0, theme.text)
}

/// Text of a given type variation (`Title`, `Heading`, `Body`, `Caption`).
pub fn typed(theme: &UiTheme, content: &str, variation: UiType) -> impl Bundle {
    let font = match variation {
        UiType::Title | UiType::Heading => theme.title_font(variation.size()),
        _ => theme.text_font(false, false, variation.size()),
    };
    (Text::new(content), font, TextColor(variation.color()))
}

/// Dump section `ui_kit`.
fn dump_ui_kit(world: &World) -> (String, serde_json::Value) {
    let theme = world.resource::<UiTheme>();
    (
        "ui_kit".to_owned(),
        serde_json::json!({
            "widgets": widgets::count_widgets(world),
            "tooltips": tooltip::dump(world),
            "modal": modal::dump(world),
            "textures": theme.textures.len(),
            "fonts_loaded": !matches!(theme.font, FontSource::Handle(_)),
        }),
    )
}
