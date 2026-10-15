//! Result of a battle for the player (`outcome_notice.gd`, the auto-resolution summary and
//! the aftermath of `battle_aftermath.gd`).
//!
//! - the **outcome band** (« Victoire décisive · Normandie ») under the top bar for six seconds
//!   whenever the core records a new field-battle class for the player (auto-resolution, end of
//!   turn, AI attack); a battle played in 3D has already shown its class, so flow marks it seen;
//! - the **result modal**: summary of an auto-resolution or of a returned 3D battle, with the
//!   aftermath (general, veterans, captives, lost captives).
//!
//! The class labels come from the core (`game_api::flow::last_battle_outcome`).

use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;
use game_api::flow::{Aftermath, AutoResolved, OutcomeView, last_battle_outcome};

use super::FlowState;
use crate::app::AppState;
use crate::sim::Sim;
use crate::ui::kit::style::{self, UiTier, UiType};
use crate::ui::kit::widgets::panel_note;
use crate::ui::kit::{UiTheme, fr, typed};

/// Seconds the band stays.
pub const SHOW_SECONDS: f32 = 6.0;

/// The band on screen.
#[derive(Component, Debug, Clone)]
pub struct OutcomeBand {
    pub class: String,
    pub text: String,
    pub remaining: f32,
}

/// Colour of a class (victories gold-green, defeats rubric).
pub fn class_color(class: &str) -> Color {
    match class {
        "heroic" | "decisive" | "victory" => style::GOOD,
        "pyrrhic" | "honourable_defeat" => Color::srgb(0.55, 0.40, 0.08),
        _ => style::RUBRIC,
    }
}

/// Lines of the aftermath, in French.
pub fn aftermath_lines(after: &Aftermath) -> Vec<String> {
    let mut lines = Vec::new();
    if !after.general_name.is_empty() {
        let mut parts = Vec::new();
        if after.general_xp > 0 {
            parts.push(format!("+{} expérience", after.general_xp));
        }
        if after.skill_points > 0 {
            parts.push(format!("+{} point(s) de compétence", after.skill_points));
        }
        if !after.general_fate.is_empty() {
            parts.push(after.general_fate.clone());
        }
        if !parts.is_empty() {
            lines.push(format!(
                "Général {} : {}.",
                after.general_name,
                parts.join(", ")
            ));
        }
    }
    if after.veterans > 0 {
        lines.push(format!(
            "{} régiment{} aguerri{}.",
            after.veterans,
            if after.veterans > 1 { "s" } else { "" },
            if after.veterans > 1 { "s" } else { "" }
        ));
    }
    for captive in &after.captives {
        lines.push(format!(
            "Captif : {} ({}), rançon {}.",
            captive.name,
            captive.faction,
            fr::amount(captive.ransom)
        ));
    }
    for lost in &after.lost {
        lines.push(format!(
            "Prisonnier de {} : {}, rançon {}.",
            lost.faction,
            lost.name,
            fr::amount(lost.ransom)
        ));
    }
    lines
}

/// Title and BBCode body of the auto-resolution summary.
pub fn auto_summary(
    resolved: &AutoResolved,
    outcome: Option<&OutcomeView>,
    aftermath: &Aftermath,
) -> (String, String) {
    let Some(report) = &resolved.report else {
        let mut lines: Vec<String> = resolved.events.iter().map(|e| e.text_fr.clone()).collect();
        lines.extend(aftermath_lines(aftermath));
        return ("Résolution automatique".to_owned(), lines.join("\n"));
    };
    let title = format!("Résolution automatique — {}", report.province);
    let mut lines = Vec::new();
    if let Some(label) = outcome.and_then(|o| o.player_label.as_deref()) {
        lines.push(format!("[b]{label}[/b]"));
    }
    for (side, label) in [
        (&report.attacker, "Assaillant"),
        (&report.defender, "Défenseur"),
    ] {
        let mut flags = Vec::new();
        if side.general_killed {
            flags.push("général tué");
        }
        if side.general_captured {
            flags.push("général pris");
        }
        if side.routed {
            flags.push("en déroute");
        }
        lines.push(format!(
            "{label} — {}{} : {} hommes, {} pertes{}",
            side.faction_name,
            if side.player { " (vous)" } else { "" },
            fr::digits(i64::from(side.soldiers_before)),
            fr::digits(i64::from(side.losses)),
            if flags.is_empty() {
                String::new()
            } else {
                format!(" ({})", flags.join(", "))
            }
        ));
    }
    lines.push(format!(
        "{} l’emporte.",
        if report.attacker_won {
            &report.attacker.faction_name
        } else {
            &report.defender.faction_name
        }
    ));
    lines.extend(aftermath_lines(aftermath));
    (title, lines.join("\n"))
}

/// Shows the band when the core records a new class for the player; the first call only learns
/// the current one (a loaded game's result is not announced).
pub fn watch_outcome(
    mut commands: Commands,
    sim: Res<Sim>,
    theme: Res<UiTheme>,
    mut flow: ResMut<FlowState>,
    bands: Query<Entity, With<OutcomeBand>>,
) {
    let Some(state) = sim.state.as_ref() else {
        return;
    };
    let current = last_battle_outcome(state);
    if !flow.primed_outcome {
        flow.primed_outcome = true;
        flow.seen_outcome = current;
        return;
    }
    if current == flow.seen_outcome {
        return;
    }
    flow.seen_outcome = current.clone();
    let Some(view) = current else { return };
    let (Some(class), Some(label)) = (view.player_class.clone(), view.player_label.clone()) else {
        return;
    };
    let mut text = label;
    if let Some(data) = sim.data.as_ref() {
        let province = data_model::ProvinceId::new(view.province.as_str())
            .map(|id| data.province_name(&id))
            .unwrap_or_default();
        if !province.is_empty() {
            text = format!("{text} · {province}");
        }
    }
    for old in &bands {
        commands.entity(old).despawn();
    }
    flow.stats.bands_shown += 1;
    commands
        .spawn((
            Name::new("flow:outcome_band"),
            OutcomeBand {
                class: class.clone(),
                text: text.clone(),
                remaining: SHOW_SECONDS,
            },
            DespawnOnExit(AppState::Campaign),
            UiTier::Banner.index(),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(74.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|root| {
            root.spawn((panel_note(&theme), Pickable::IGNORE))
                .with_children(|note| {
                    note.spawn(typed(&theme, &text, UiType::Heading))
                        .insert(TextColor(class_color(&class)));
                });
        });
}

pub fn tick_band(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut bands: Query<(Entity, &mut OutcomeBand)>,
) {
    for (entity, mut band) in &mut bands {
        band.remaining -= time.delta_secs();
        if band.remaining <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}
