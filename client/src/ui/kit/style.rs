//! Palette, typography scale and z tiers of the parchment UI.
//!
//! Colours: game/scripts/ui/hud_style.gd and front_end_style.gd (themselves taken from
//! `parchment_theme.tres`). Scale: game/scripts/ui/ui_type.gd (bible DA § 12.2, 900 px reference).

use bevy::prelude::*;

pub const PARCHMENT: Color = Color::srgb(0.93, 0.87, 0.72);
pub const PARCHMENT_LIGHT: Color = Color::srgb(0.97, 0.93, 0.82);
pub const PARCHMENT_DARK: Color = Color::srgb(0.80, 0.70, 0.50);
pub const INK: Color = Color::srgb(0.22, 0.14, 0.07);
pub const INK_SOFT: Color = Color::srgb(0.42, 0.29, 0.16);
pub const INK_FADED: Color = Color::srgb(0.50, 0.44, 0.36);
/// Heading ink (titles, headings).
pub const INK_TITLE: Color = Color::srgb(0.20, 0.13, 0.07);
pub const RUBRIC: Color = Color::srgb(0.62, 0.13, 0.08);
pub const GOLD: Color = Color::srgb(0.72, 0.56, 0.24);
pub const GOLD_PALE: Color = Color::srgb(0.84, 0.72, 0.42);
pub const WAX: Color = Color::srgb(0.55, 0.11, 0.08);
pub const WAX_DARK: Color = Color::srgb(0.36, 0.06, 0.04);
pub const WAX_LIGHT: Color = Color::srgb(0.70, 0.22, 0.14);
pub const WAX_GREEN: Color = Color::srgb(0.24, 0.34, 0.18);
pub const SHADOW: Color = Color::srgba(0.0, 0.0, 0.0, 0.30);
/// Gauges: good (grey-green ink), fair (ochre), poor (red).
pub const GOOD: Color = Color::srgb(0.33, 0.42, 0.20);
pub const FAIR: Color = Color::srgb(0.72, 0.52, 0.16);
pub const POOR: Color = Color::srgb(0.62, 0.13, 0.08);

// Front-end (menus) colours.
pub const FE_GOLD: Color = Color::srgb(0.93, 0.78, 0.42);
pub const FE_GOLD_DARK: Color = Color::srgb(0.62, 0.46, 0.18);
pub const FE_AZURE: Color = Color::srgb(0.10, 0.17, 0.42);
pub const FE_GULES: Color = Color::srgb(0.58, 0.10, 0.08);
pub const FE_VELLUM: Color = Color::srgb(0.95, 0.90, 0.78);
/// Dark warm leather behind the Codex (lighter than `FE_NIGHT`, which is almost black).
pub const FE_LEATHER: Color = Color::srgb(0.20, 0.14, 0.09);
pub const FE_NIGHT: Color = Color::srgb(0.05, 0.04, 0.035);

// Button text per state (parchment_theme.tres).
pub const BUTTON_HOVER_INK: Color = Color::srgb(0.15, 0.09, 0.04);
pub const BUTTON_PRESSED_INK: Color = Color::srgb(0.12, 0.10, 0.22);
pub const PRIMARY_INK: Color = Color::srgb(0.97, 0.93, 0.80);
pub const PRIMARY_HOVER_INK: Color = Color::srgb(1.0, 0.97, 0.88);
pub const PRIMARY_PRESSED_INK: Color = Color::srgb(0.90, 0.84, 0.66);
pub const PRIMARY_DISABLED_INK: Color = Color::srgba(0.82, 0.82, 0.86, 0.8);
pub const TAB_SELECTED_INK: Color = Color::srgb(0.15, 0.09, 0.04);
pub const TAB_UNSELECTED_INK: Color = Color::srgb(0.40, 0.32, 0.22);
/// Tooltip muted text (`RichTooltip.MUTED`).
pub const MUTED: Color = Color::srgb(0.42, 0.353, 0.251);
/// Error text on parchment (`PanelSection.ERROR_COLOR`).
pub const ERROR: Color = Color::srgb(0.55, 0.20, 0.15);

/// Colour of a 0..1 gauge (strength, morale, supply).
pub fn gauge_color(ratio: f32) -> Color {
    if ratio >= 0.66 {
        GOOD
    } else if ratio >= 0.33 {
        FAIR
    } else {
        POOR
    }
}

/// The four type variations of the theme (base sizes at the 900 px reference height).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UiType {
    /// Screen or modal title (26 px).
    Title,
    /// Panel or section heading, city name (20 px).
    Heading,
    /// Running text (17 px).
    Body,
    /// Captions, secondary figures (15 px, floor at 720p).
    Caption,
}

impl UiType {
    pub const fn size(self) -> f32 {
        match self {
            Self::Title => 26.0,
            Self::Heading => 20.0,
            Self::Body => 17.0,
            Self::Caption => 15.0,
        }
    }

    /// Default ink of the variation.
    pub const fn color(self) -> Color {
        match self {
            Self::Title | Self::Heading => INK_TITLE,
            Self::Body => INK,
            Self::Caption => TAB_UNSELECTED_INK,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Title => "Title",
            Self::Heading => "Heading",
            Self::Body => "Body",
            Self::Caption => "Caption",
        }
    }
}

/// Stacking tiers (`PanelStack.Tier`): HUD < toasts < panels < banners < modals < tutorial, then
/// tooltips above everything. Used through `GlobalZIndex`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UiTier {
    Hud,
    /// Notices and advisor subtitles: above the HUD, under every panel and window, so they never
    /// cover a title, a button or a decision (they wait behind it).
    Toast,
    Panel,
    Banner,
    Modal,
    Tutorial,
    Tooltip,
}

impl UiTier {
    pub const fn z(self) -> i32 {
        match self {
            Self::Hud => 0,
            Self::Toast => 5,
            Self::Panel => 10,
            Self::Banner => 20,
            Self::Modal => 30,
            Self::Tutorial => 40,
            Self::Tooltip => 100,
        }
    }

    pub const fn index(self) -> GlobalZIndex {
        GlobalZIndex(self.z())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_is_monotonic() {
        assert!(UiType::Title.size() > UiType::Heading.size());
        assert!(UiType::Heading.size() > UiType::Body.size());
        assert!(UiType::Body.size() > UiType::Caption.size());
        assert!(UiType::Caption.size() >= 15.0);
    }

    #[test]
    fn gauge_thresholds() {
        assert_eq!(gauge_color(0.9), GOOD);
        assert_eq!(gauge_color(0.5), FAIR);
        assert_eq!(gauge_color(0.1), POOR);
    }

    #[test]
    fn tiers_stack_in_order() {
        assert!(UiTier::Modal.z() > UiTier::Panel.z());
        assert!(UiTier::Tooltip.z() > UiTier::Tutorial.z());
        // Notices never cover a panel or a window.
        assert!(UiTier::Toast.z() > UiTier::Hud.z());
        assert!(UiTier::Toast.z() < UiTier::Panel.z());
    }
}
