//! Season report (port of `season_report.gd`): sorts the events of the turn that just resolved
//! into the five rubrics and formats them as BBCode for a modal. No game calculation.

use sim_campaign::GameEvent;

use super::text::hex;
use crate::ui::kit::fr;

/// Rubrics in order of urgency: (id, title).
pub const SECTIONS: [(&str, &str); 5] = [
    ("lands", "Vos terres"),
    ("treasury", "Trésor"),
    ("armies", "Armées"),
    ("works", "Constructions et recherches"),
    ("world", "Le monde"),
];

const MAX_PER_GROUP: usize = 8;

const LANDS: &[&str] = &[
    "province_captured",
    "siege_started",
    "siege_lifted",
    "raid",
    "revolt",
    "plague",
    "famine",
    "death",
    "succession",
    "no_heir",
    "birth",
    "marriage",
    "regency",
    "table",
    "medicine",
    "chivalry",
    "agent",
    "excommunication",
    "heresy",
    "vassal_rebellion",
    "victory",
    "defeat",
    "campaign_ended",
    "edict",
    "mission",
    "crusade",
    "royal_act",
    "level_up",
    "captain",
    "loyalty",
];
const TREASURY: &[&str] = &["bankruptcy", "coinage", "ransom", "trade"];
const ARMIES: &[&str] = &[
    "battle",
    "army_destroyed",
    "general_captured",
    "recruited",
    "attrition",
];
const WORKS: &[&str] = &["building_completed", "technology_researched"];
/// Kinds reported for any faction.
const WORLD_KINDS: &[&str] = &[
    "war_declared",
    "peace_signed",
    "faction_destroyed",
    "schism",
    "chronicle",
    "victory",
    "defeat",
    "campaign_ended",
    "succession",
    "excommunication",
];
/// Kinds reported for a faction of interest (at war with or allied to the player).
const WORLD_NEWS_KINDS: &[&str] = &[
    "war_declared",
    "peace_signed",
    "alliance_formed",
    "alliance_broken",
    "vassalage",
    "vassal_rebellion",
    "faction_destroyed",
    "schism",
    "excommunication",
    "succession",
    "death",
    "province_captured",
    "battle",
    "revolt",
    "plague",
    "chronicle",
    "victory",
    "defeat",
    "campaign_ended",
    "general_captured",
];

/// Snake-case key of an event kind.
pub fn kind_key(event: &GameEvent) -> String {
    serde_json::to_value(event.kind)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// One report line.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub text: String,
    pub loss: bool,
    pub gain: bool,
}

/// Events by rubric (same order as [`SECTIONS`]).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Report {
    pub groups: [Vec<Entry>; 5],
}

impl Report {
    pub fn total(&self) -> usize {
        self.groups.iter().map(Vec::len).sum()
    }
}

/// Sorts `events` for `player`; `interest` = factions at war with or allied to the player.
pub fn build(events: &[GameEvent], player: &str, interest: &[String]) -> Report {
    let mut report = Report::default();
    for event in events {
        let kind = kind_key(event);
        let mine = event.faction.as_ref().is_some_and(|f| f.as_str() == player);
        let entry = |gain: bool| Entry {
            text: event.text_fr.clone(),
            loss: event.loss && mine,
            gain,
        };
        let group = if mine {
            if LANDS.contains(&kind.as_str()) {
                Some(0)
            } else if TREASURY.contains(&kind.as_str()) {
                Some(1)
            } else if ARMIES.contains(&kind.as_str()) {
                Some(2)
            } else if WORKS.contains(&kind.as_str()) {
                Some(3)
            } else {
                None
            }
        } else {
            let of_interest = event
                .faction
                .as_ref()
                .is_some_and(|f| interest.iter().any(|i| i == f.as_str()));
            let world = WORLD_KINDS.contains(&kind.as_str())
                || (event.public && WORLD_NEWS_KINDS.contains(&kind.as_str()))
                || (of_interest && WORLD_NEWS_KINDS.contains(&kind.as_str()));
            world.then_some(4)
        };
        if let Some(index) = group {
            let gain = mine && kind == "province_captured" && !event.loss;
            report.groups[index].push(entry(gain));
        }
    }
    // Losses first, then gains, then the rest (stable).
    for group in &mut report.groups {
        group.sort_by_key(|e| {
            if e.loss {
                0
            } else if e.gain {
                1
            } else {
                2
            }
        });
    }
    report
}

/// BBCode body of the report modal.
pub fn bbcode(report: &Report) -> String {
    if report.total() == 0 {
        return "Une saison sans événement notable.".to_owned();
    }
    let mut out = Vec::new();
    for ((_, title), group) in SECTIONS.iter().zip(&report.groups) {
        if group.is_empty() {
            continue;
        }
        out.push(format!("[b]{title}[/b]"));
        for entry in group.iter().take(MAX_PER_GROUP) {
            let color = if entry.loss {
                Some(fr::LOSS_COLOR)
            } else if entry.gain {
                Some(fr::GAIN_COLOR)
            } else {
                None
            };
            out.push(match color {
                Some(c) => format!("[color={}]• {}[/color]", hex(c), entry.text),
                None => format!("• {}", entry.text),
            });
        }
        if group.len() > MAX_PER_GROUP {
            out.push(format!(
                "[i]… et {} autre{}[/i]",
                group.len() - MAX_PER_GROUP,
                fr::plural_s((group.len() - MAX_PER_GROUP) as i64)
            ));
        }
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use data_model::FactionId;
    use sim_campaign::EventKind;

    fn ev(kind: EventKind, faction: &str, loss: bool) -> GameEvent {
        let mut e = GameEvent::new(kind, "texte");
        e.faction = Some(FactionId::new(faction).unwrap());
        e.loss = loss;
        e
    }

    #[test]
    fn sorts_into_rubrics_and_orders_losses_first() {
        let events = vec![
            ev(EventKind::ProvinceCaptured, "fac_france", false),
            ev(EventKind::Raid, "fac_france", true),
            ev(EventKind::Bankruptcy, "fac_france", true),
            ev(EventKind::Battle, "fac_england", false),
            ev(EventKind::WarDeclared, "fac_england", false),
        ];
        let report = build(&events, "fac_france", &[]);
        assert_eq!(report.groups[0].len(), 2);
        assert!(report.groups[0][0].loss);
        assert!(report.groups[0][1].gain);
        assert_eq!(report.groups[1].len(), 1);
        assert_eq!(
            report.groups[4].len(),
            1,
            "war news only; foreign battle dropped"
        );
        assert!(bbcode(&report).contains("Vos terres"));
    }

    #[test]
    fn empty_season() {
        assert_eq!(
            bbcode(&Report::default()),
            "Une saison sans événement notable."
        );
    }
}
