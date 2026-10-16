//! Campaign briefing (ports game/scripts/ui/campaign_briefing.gd): on turn 0 of a new game, a
//! window with the faction's objectives, the hostile neighbours (declared wars) and three first
//! tips. « À la guerre ! » closes it; « Ne plus afficher ce briefing » also turns the setting
//! `interface/campaign_briefing` off. Texts are in `data/ui/transitions.json`; objectives and
//! wars come from the core. `CENT_ANS_NO_BRIEFING=1` skips it (agent scripts).

use bevy::prelude::*;
use serde_json::{Value, json};

use super::{HudData, HudSet};
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::settings::{Settings, SettingsFile};
use crate::sim::Sim;
use crate::ui::kit::modal::{ModalQueue, ModalResult, ModalSpec, PRIORITY_DECISION};

const KEY: &str = "hud:briefing";

/// Texts of `transitions.json` `briefing`.
#[derive(Debug, Clone, PartialEq)]
pub struct Texts {
    pub title: String,
    pub intro: String,
    pub objectives_title: String,
    pub no_objectives: String,
    pub neighbours_title: String,
    pub no_neighbours: String,
    pub tips_title: String,
    pub tips: Vec<String>,
    pub never_again: String,
    pub start: String,
}

impl Texts {
    pub fn parse(text: &str) -> Option<Self> {
        let tree: Value = serde_json::from_str(text).ok()?;
        let b = tree.get("briefing")?;
        let s = |k: &str| b.get(k).and_then(Value::as_str).unwrap_or("").to_owned();
        Some(Self {
            title: s("title"),
            intro: s("intro"),
            objectives_title: s("objectives_title"),
            no_objectives: s("no_objectives"),
            neighbours_title: s("neighbours_title"),
            no_neighbours: s("no_neighbours"),
            tips_title: s("tips_title"),
            tips: b
                .get("tips")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|t| t.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default(),
            never_again: s("never_again"),
            start: s("start"),
        })
    }

    pub fn load() -> Option<Self> {
        let root = crate::paths::find_root().ok()?;
        Self::parse(&std::fs::read_to_string(root.join("data/ui/transitions.json")).ok()?)
    }
}

/// Body of the window (`build_model`).
pub fn body(
    texts: &Texts,
    faction: &str,
    date: &str,
    objectives: &[String],
    hostile: &[String],
) -> String {
    let mut parts = vec![format!(
        "[i]{}[/i]",
        texts
            .intro
            .replace("{faction}", faction)
            .replace("{date}", date)
    )];
    parts.push(format!("\n[b]{}[/b]", texts.objectives_title));
    if objectives.is_empty() {
        parts.push(texts.no_objectives.clone());
    } else {
        parts.extend(objectives.iter().map(|o| format!("• {o}")));
    }
    parts.push(format!("\n[b]{}[/b]", texts.neighbours_title));
    if hostile.is_empty() {
        parts.push(texts.no_neighbours.clone());
    } else {
        parts.extend(hostile.iter().map(|h| format!("• {h}")));
    }
    parts.push(format!("\n[b]{}[/b]", texts.tips_title));
    parts.extend(texts.tips.iter().take(3).map(|t| format!("• {t}")));
    parts.join("\n")
}

/// Shown on turn 0 of a new game when the setting allows it.
pub fn should_show(turn: u32, enabled: bool, skipped: bool) -> bool {
    turn == 0 && enabled && !skipped
}

#[derive(Resource, Default)]
pub struct Briefing {
    pending: bool,
    pub shown: u32,
    pub dismissed_for_good: u32,
    pub body: String,
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Briefing>()
        .register_dump(dump_briefing)
        .add_systems(OnEnter(AppState::Campaign), |mut b: ResMut<Briefing>| {
            b.pending = true;
        })
        .add_systems(Update, (show, answer).chain().in_set(HudSet::Ui));
}

fn show(
    mut briefing: ResMut<Briefing>,
    sim: Res<Sim>,
    hud: Res<HudData>,
    settings: Option<Res<Settings>>,
    mut queue: ResMut<ModalQueue>,
) {
    if !briefing.pending {
        return;
    }
    let (Some(data), Some(state), Some(top)) = (sim.data.as_ref(), sim.state.as_ref(), &hud.top)
    else {
        return;
    };
    briefing.pending = false;
    let skipped = std::env::var_os("CENT_ANS_NO_BRIEFING").is_some();
    let enabled = settings.is_none_or(|s| s.campaign_briefing);
    if !should_show(state.turn(), enabled, skipped) {
        return;
    }
    let Some(texts) = Texts::load() else { return };
    let objectives: Vec<String> = state
        .objectives(data, &sim.player)
        .iter()
        .map(|o| o.title.clone())
        .collect();
    let hostile: Vec<String> = top
        .at_war_with
        .iter()
        .filter_map(|id| data_model::FactionId::new(id.clone()).ok())
        .map(|id| data.faction_name(&id))
        .collect();
    let text = body(
        &texts,
        &top.faction_name,
        &top.date_label,
        &objectives,
        &hostile,
    );
    let mut spec = ModalSpec::confirm(&texts.title, &text);
    spec.yes_label = texts.start.clone();
    spec.no_label = texts.never_again.clone();
    spec.width = 620.0;
    spec.escape_passes = true;
    queue.request(KEY, PRIORITY_DECISION, spec);
    briefing.shown += 1;
    briefing.body = text;
}

/// « Ne plus afficher » (the cancel button) turns the setting off for good.
fn answer(
    mut results: MessageReader<ModalResult>,
    mut briefing: ResMut<Briefing>,
    settings: Option<ResMut<Settings>>,
    file: Option<ResMut<SettingsFile>>,
) {
    let (Some(mut settings), Some(mut file)) = (settings, file) else {
        results.clear();
        return;
    };
    for result in results.read() {
        if result.key == KEY && !result.confirmed {
            settings.campaign_briefing = false;
            file.save(&settings);
            briefing.dismissed_for_good += 1;
        }
    }
}

fn dump_briefing(world: &World) -> (String, Value) {
    let briefing = world.resource::<Briefing>();
    (
        "briefing".to_owned(),
        json!({
            "shown": briefing.shown,
            "dismissed_for_good": briefing.dismissed_for_good,
            "body": briefing.body,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts() -> Texts {
        Texts::parse(
            r#"{"briefing": {"title":"T","intro":"{faction} — {date}","objectives_title":"O",
            "no_objectives":"aucun","neighbours_title":"N","no_neighbours":"paix","tips_title":"C",
            "tips":["a","b","c","d"],"never_again":"non","start":"go"}}"#,
        )
        .expect("texts")
    }

    #[test]
    fn only_turn_zero_when_enabled() {
        assert!(should_show(0, true, false));
        assert!(!should_show(1, true, false));
        assert!(!should_show(0, false, false));
        assert!(!should_show(0, true, true));
    }

    #[test]
    fn body_has_the_four_blocks_and_three_tips() {
        let text = body(
            &texts(),
            "France",
            "Printemps 1337",
            &["Tenir Paris".into()],
            &[],
        );
        assert!(text.contains("France — Printemps 1337"));
        assert!(text.contains("• Tenir Paris"));
        assert!(text.contains("paix"));
        assert!(text.contains("• c") && !text.contains("• d"));
        let empty = body(&texts(), "F", "D", &[], &["Angleterre".into()]);
        assert!(empty.contains("aucun") && empty.contains("• Angleterre"));
    }
}
