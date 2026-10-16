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
//! - [`icons`], [`tooltip_spec`], [`panel_stack`]: icon helper, structured tooltips, panel stack;
//!   [`layout_probe`] feeds the dump with frame/padding numbers.
//! - [`text_field`]: single-line input; [`text_field::TextInputFocus`] tells shortcuts to stay quiet.
//! - [`fr`]: French numbers, money (`₶`), plurals and dates.
//! - [`UiCamera`]: the 2D camera that draws the UI (default UI camera, offscreen-aware).
//! - Gallery: `CENT_ANS_KIT_GALLERY=1` + `MainMenu` shows every widget ([`gallery`]).
//! - Dump section `ui_kit`: widget counts by kind, open tooltips, modal queue.

pub mod contrast;
pub mod fa;
pub mod fr;
pub mod gallery;
pub mod icons;
pub mod layout;
pub mod layout_probe;
pub mod modal;
pub mod panel_stack;
pub mod richtext;
pub mod style;
pub mod text_field;
pub mod tooltip;
pub mod tooltip_spec;
pub mod widgets;

use std::collections::HashMap;

use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::sprite::{BorderRect, SliceScaleMode, TextureSlicer};
use bevy::text::{FontFeatureTag, FontFeatures, FontWeight};
use bevy::ui::{IsDefaultUiCamera, VisualBox};
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
    /// The texture was cropped to a compact 9-slice (see [`compact_image`]): painted borders keep
    /// their pixel size on small nodes instead of shrinking with `size / texture size`.
    pub compact: bool,
}

/// Side of the centre block kept by [`compact_image`].
const COMPACT_CENTER: u32 = 16;

/// Crops a 9-slice texture to `margins + 16 px + margins` (centre and edge blocks taken from the
/// middle of the source), so that nodes smaller than the original texture do not shrink the
/// painted border (Bevy scales corners by `node size / texture size` when the node is smaller;
/// a 34 px high note on a 120 px texture got a 3 px frame). `None` when the margins are not
/// positive on every side, the texture is not 8-bit RGBA, or its centre is smaller than 16 px.
pub fn compact_image(source: &Image, margins: [f32; 4]) -> Option<Image> {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    if margins.iter().any(|m| *m < 1.0) {
        return None;
    }
    let data = source.data.as_ref()?;
    let (src_w, src_h) = (source.width(), source.height());
    if data.len() != (src_w * src_h * 4) as usize {
        return None;
    }
    let [left, top, right, bottom] = margins.map(|m| m as u32);
    if src_w < left + right + COMPACT_CENTER || src_h < top + bottom + COMPACT_CENTER {
        return None;
    }
    let out_w = left + right + COMPACT_CENTER;
    let out_h = top + bottom + COMPACT_CENTER;
    let source_x = |x: u32| -> u32 {
        if x < left {
            x
        } else if x < left + COMPACT_CENTER {
            left + (src_w - left - right - COMPACT_CENTER) / 2 + (x - left)
        } else {
            src_w - (out_w - x)
        }
    };
    let source_y = |y: u32| -> u32 {
        if y < top {
            y
        } else if y < top + COMPACT_CENTER {
            top + (src_h - top - bottom - COMPACT_CENTER) / 2 + (y - top)
        } else {
            src_h - (out_h - y)
        }
    };
    let mut out = Vec::with_capacity((out_w * out_h * 4) as usize);
    for y in 0..out_h {
        let row = source_y(y) * src_w;
        for x in 0..out_w {
            let at = ((row + source_x(x)) * 4) as usize;
            out.extend_from_slice(&data[at..at + 4]);
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: out_w,
            height: out_h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        out,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = source.sampler.clone();
    Some(image)
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
    /// Icon ids → PNG (entity miniatures, ink icons, medallions), built at startup.
    pub icons: icons::IconLibrary,
    pub metrics: HashMap<String, SliceMetrics>,
    /// Multiplier of every font size (setting `interface/text_size`); applies to text built
    /// after it changes (open panels keep their size until rebuilt).
    pub text_scale: f32,
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
            icons: icons::IconLibrary::default(),
            metrics: HashMap::new(),
            text_scale: 1.0,
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
            font_size: (size * self.text_scale).into(),
            weight: FontWeight(if bold { WEIGHT_BOLD } else { WEIGHT_BODY }),
            font_features: FontFeatures::from([FontFeatureTag::LINING_FIGURES]),
            ..default()
        }
    }

    /// Title `TextFont` (IM Fell English).
    pub fn title_font(&self, size: f32) -> TextFont {
        TextFont {
            font: self.font_title.clone(),
            font_size: (size * self.text_scale).into(),
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
        let center = if metric.compact {
            SliceScaleMode::Stretch
        } else {
            tile
        };
        ImageNode {
            image: self.textures.get(name).cloned().unwrap_or_default(),
            image_mode: NodeImageMode::Sliced(TextureSlicer {
                border: BorderRect {
                    min_inset: Vec2::new(left, top),
                    max_inset: Vec2::new(right, bottom),
                },
                center_scale_mode: center,
                sides_scale_mode: tile,
                max_corner_scale: 1.0,
            }),
            // Bevy 0.20 draws images in the CONTENT box by default: a framed node would get its
            // painted frame shrunk by its own padding (text poking out of the frame).
            visual_box: VisualBox::PaddingBox,
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

    /// Padding that keeps content off the painted frame: the larger of the texture's painted
    /// border (`margins`) and its content margin (`content`), plus `gap` px of air, per side.
    pub fn inner_padding(&self, name: &str, gap: f32) -> UiRect {
        let metric = self.metric(name);
        let side = |index: usize| Val::Px(metric.content[index].max(metric.margins[index]) + gap);
        UiRect {
            left: side(0),
            top: side(1),
            right: side(2),
            bottom: side(3),
        }
    }

    /// Plain texture (no slicing) of kit texture `name`.
    pub fn image(&self, name: &str) -> ImageNode {
        ImageNode::new(self.textures.get(name).cloned().unwrap_or_default())
            .with_visual_box(VisualBox::PaddingBox)
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
                panel_stack::PanelStackPlugin,
                text_field::TextFieldPlugin,
                gallery::GalleryPlugin,
            ))
            .add_plugins(layout::plugin)
            .add_plugins(fa::plugin)
            .add_plugins(contrast::plugin)
            .add_plugins((super::fader::plugin, super::art_plates::plugin))
            .add_systems(Startup, (load_resources, spawn_ui_camera).chain())
            .add_systems(Update, apply_text_scale)
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
    mut images: ResMut<Assets<Image>>,
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

    theme.icons = icons::IconLibrary::from_assets(|relative| read_asset(relative).ok());

    match read_asset(&format!("{KIT_DIR}/kit.json")) {
        Ok(bytes) => match parse_kit_metrics(&bytes) {
            Ok(mut metrics) => {
                for (name, metric) in &mut metrics {
                    let relative = format!("{KIT_DIR}/{name}.png");
                    let compact = read_asset(&relative).ok().and_then(|bytes| {
                        let image = Image::from_buffer(
                            &bytes,
                            bevy::image::ImageType::Extension("png"),
                            bevy::image::CompressedImageFormats::NONE,
                            true,
                            bevy::image::ImageSampler::Default,
                            bevy::asset::RenderAssetUsages::default(),
                        )
                        .ok()?;
                        compact_image(&image, metric.margins)
                    });
                    let handle = match compact {
                        Some(image) => {
                            metric.size = [image.width() as f32, image.height() as f32];
                            metric.compact = true;
                            images.add(image)
                        }
                        None => assets.load(relative),
                    };
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
                compact: false,
            },
        );
    }
    Ok(result)
}

/// `interface/text_size` -> [`UiTheme::text_scale`] (clamped; no write when unchanged).
pub(super) fn apply_text_scale(settings: Option<Res<crate::settings::Settings>>, mut theme: ResMut<UiTheme>) {
    let Some(settings) = settings else { return };
    let wanted = settings.text_size.clamp(0.7, 1.6);
    if (theme.text_scale - wanted).abs() > 1e-4 {
        theme.text_scale = wanted;
    }
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
/// `with_children` or `children![]`. Padding = painted frame margin + 10 px.
pub fn panel(theme: &UiTheme) -> impl Bundle {
    (
        Node {
            padding: theme.inner_padding("panel", 10.0),
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
            "text_input": text_field::dump(world),
            "panel_stack": world.resource::<panel_stack::PanelStack>().dump(),
            "layout": layout_probe::dump(world),
            "icons": icons::dump(world),
            "access": contrast::dump(world),
            "textures": theme.textures.len(),
            "fonts_loaded": !matches!(theme.font, FontSource::Handle(_)),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_keeps_corners_and_shrinks_centre() {
        // 40x40 image whose pixel value encodes its x coordinate.
        let mut data = Vec::new();
        for _y in 0..40u32 {
            for x in 0..40u32 {
                data.extend_from_slice(&[x as u8, 0, 0, 255]);
            }
        }
        let source = Image::new(
            bevy::render::render_resource::Extent3d {
                width: 40,
                height: 40,
                depth_or_array_layers: 1,
            },
            bevy::render::render_resource::TextureDimension::D2,
            data,
            bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
            bevy::asset::RenderAssetUsages::default(),
        );
        let compact = compact_image(&source, [5.0, 5.0, 5.0, 5.0]).unwrap();
        assert_eq!((compact.width(), compact.height()), (26, 26));
        let pixels = compact.data.as_ref().unwrap();
        let red = |x: usize, y: usize| pixels[(y * 26 + x) * 4];
        assert_eq!(red(0, 0), 0); // left border
        assert_eq!(red(25, 0), 39); // right border
        assert_eq!(red(5, 5), 12); // centre block starts mid-source: 5 + (30 - 16) / 2
        // Margins of zero or a too small source are left alone.
        assert!(compact_image(&source, [0.0, 5.0, 5.0, 5.0]).is_none());
        assert!(compact_image(&source, [15.0, 5.0, 15.0, 5.0]).is_none());
    }

    #[test]
    fn inner_padding_exceeds_content_and_margin() {
        let mut theme = UiTheme::default();
        theme.metrics.insert(
            "panel".into(),
            SliceMetrics {
                size: [168.0, 168.0],
                margins: [20.0; 4],
                content: [18.0, 14.0, 18.0, 15.0],
                compact: false,
            },
        );
        let padding = theme.inner_padding("panel", 8.0);
        assert_eq!(padding.left, Val::Px(28.0));
        assert_eq!(padding.bottom, Val::Px(28.0));
    }
}
