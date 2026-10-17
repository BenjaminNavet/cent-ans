//! Colonies tab: build hammer and build companion (ports `game/scripts/ui/build_hammer.gd` and
//! `build_companion.gd`, Godot lot RG5).
//!
//! The hammer sits beside each colony the player holds: lit when the core says a building can be
//! started there (`BuildingsState::can_build`: free slot or upgrade, room in the queue, something
//! buildable; cost excluded), dimmed with the reason as tooltip otherwise. A click opens the
//! companion under the Colonies list: a « Bâtir à » header with one selector per colony (marked
//! `⚒` when buildable), a close button and the buildings view of the chosen colony, so one can
//! build without leaving the list. The verdicts come from the core; nothing is decided here.
//! Widget names: `prov:hammer:<settlement>`, `prov:csel:<settlement>` (selector),
//! `prov:companion_close`; the view inside has the Buildings tab's names (`prov:build:<id>`...).

use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use serde::Serialize;

use super::images::BuildingImages;
use super::labels::UiLabels;
use super::view::{Kids, Snapshot};
use crate::ui::kit::close_button;
use crate::ui::kit::style::UiType;
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::widgets::{button_glyph, caption, heading, list_item, separator};
use crate::ui::kit::{UiTheme, typed};

/// List group of the companion's colony selector.
pub const COMPANION_GROUP: &str = "provcomp";
/// Icon id of the hammer (`IconLibrary.get_icon("glyph_hammer")`).
pub const HAMMER_ICON: &str = "glyph_hammer";
const HAMMER_SIZE: f32 = 22.0;
const NO_REASON: &str = "Construction impossible ici.";

/// « Can build here » verdict of a colony the player holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Verdict {
    pub settlement: String,
    pub name: String,
    pub ok: bool,
    pub reason: String,
}

impl Verdict {
    /// Tooltip of the hammer (`BuildHammer.setup`).
    pub fn tooltip(&self) -> String {
        if self.ok {
            "Construction possible ici : cliquer pour bâtir.".to_owned()
        } else if self.reason.is_empty() {
            NO_REASON.to_owned()
        } else {
            self.reason.clone()
        }
    }

    /// Selector caption: `⚒ ` before a colony where one can build (`BuildCompanion.set_choices`).
    pub fn selector_caption(&self) -> String {
        if self.ok {
            format!("⚒ {}", self.name)
        } else {
            self.name.clone()
        }
    }
}

/// Spawns the hammer button of a colony; greyed and inert when one cannot build there (Godot's
/// dimmed `modulate` and ignored press).
pub fn spawn_hammer(line: &mut Kids, theme: &UiTheme, assets: &AssetServer, verdict: &Verdict) {
    let mut hammer = line.spawn((
        button_glyph(
            theme,
            assets,
            &format!("prov:hammer:{}", verdict.settlement),
            HAMMER_ICON,
            "⚒",
            HAMMER_SIZE,
        ),
        Tooltip::new(&verdict.tooltip()),
    ));
    if !verdict.ok {
        hammer.insert(InteractionDisabled);
    }
}

/// The companion section under the colony cards.
pub fn section(
    pane: &mut Kids,
    theme: &UiTheme,
    assets: &AssetServer,
    labels: &UiLabels,
    images: &BuildingImages,
    snap: &Snapshot,
) {
    pane.spawn(separator(theme));
    pane.spawn((
        Name::new("prov:companion"),
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            ..default()
        },
    ))
    .with_children(|companion| {
        companion
            .spawn(Node {
                column_gap: Val::Px(6.0),
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|head| {
                head.spawn((
                    Node {
                        flex_grow: 1.0,
                        ..default()
                    },
                    heading(theme, "Bâtir à"),
                ));
                head.spawn(close_button(theme, "prov:companion_close"));
            });
        companion
            .spawn(Node {
                flex_wrap: FlexWrap::Wrap,
                column_gap: Val::Px(4.0),
                ..default()
            })
            .with_children(|choices| {
                for verdict in &snap.verdicts {
                    let mut item = choices.spawn(list_item(
                        theme,
                        COMPANION_GROUP,
                        &format!("prov:csel:{}", verdict.settlement),
                        &verdict.selector_caption(),
                        verdict.settlement == snap.selected_settlement,
                    ));
                    if !verdict.ok {
                        item.insert(Tooltip::new(&verdict.tooltip()));
                    }
                }
            });
        if snap.verdicts.is_empty() {
            companion.spawn(caption(theme, "Aucune colonie à vous dans cette province."));
        } else {
            companion.spawn(typed(
                theme,
                &format!("Colonie : {}", selected_name(snap)),
                UiType::Body,
            ));
            super::view::buildings_view(companion, theme, assets, labels, images, snap);
        }
    });
}

fn selected_name(snap: &Snapshot) -> String {
    snap.verdicts
        .iter()
        .find(|v| v.settlement == snap.selected_settlement)
        .map_or_else(|| snap.selected_settlement.clone(), |v| v.name.clone())
}

/// The colonies of `places` held by `player`, with the core's verdict.
pub fn verdicts(
    data: &data_model::GameData,
    state: &sim_campaign::CampaignState,
    player: &str,
    places: &[game_api::province::SettlementView],
) -> Vec<Verdict> {
    places
        .iter()
        .filter(|place| place.controller == player)
        .filter_map(|place| {
            let id = data_model::SettlementId::new(&place.id).ok()?;
            let verdict = state.can_build(data, &id);
            Some(Verdict {
                settlement: place.id.clone(),
                name: place.name.clone(),
                ok: verdict.ok,
                reason: verdict.reason.unwrap_or_default(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn verdict(ok: bool, reason: &str) -> Verdict {
        Verdict {
            settlement: "s".into(),
            name: "Bourg".into(),
            ok,
            reason: reason.into(),
        }
    }

    #[test]
    fn tooltip_and_caption_follow_the_verdict() {
        assert!(
            verdict(true, "")
                .tooltip()
                .starts_with("Construction possible")
        );
        assert_eq!(verdict(false, "File pleine.").tooltip(), "File pleine.");
        assert_eq!(verdict(false, "").tooltip(), NO_REASON);
        assert_eq!(verdict(true, "").selector_caption(), "⚒ Bourg");
        assert_eq!(verdict(false, "x").selector_caption(), "Bourg");
    }
}
