//! What the result screen adds to the battle figures: the « suites » cards (hero, captives and
//! ransoms, experience, loot), the « Dans l’Histoire » card of a historical battle, and the
//! painted backdrop of the outcome. Port of battle_result_screen.gd `_fill_aftermath`,
//! `history_card_lines`, `_aftermath_card` and of `ArtPlates.ending`.
//!
//! The campaign side of the suites (captives, ransoms, experience of the general, veterans) is the
//! flow lot's [`game_api::flow::Aftermath`]: it exists only once the outcome has been applied to
//! the campaign, which flow does after [`crate::flow::LeaveBattle`] — so on this screen it is read
//! from [`ResultExtras`], which flow fills if it applies the outcome before the screen opens (until
//! then the cards say « Suites connues au retour en campagne », as Godot does outside a campaign).
//! Likewise the historical battle's data (`ResultExtras::history`) is set by whoever starts a
//! historical battle (the battle scene cannot build one yet): without it no history card.

use bevy::prelude::*;
use game_api::flow::Aftermath;
use serde_json::Value;
use sim_battle::{BattleOutcome, SideId};

use super::result::{RegimentRow, ResultView};
use crate::paths;
use crate::ui::kit::fr;
use crate::ui::kit::style;

/// One card of the row under the notable facts.
#[derive(Debug, Clone, PartialEq)]
pub struct Card {
    pub title: String,
    pub lines: Vec<String>,
    pub accent: Color,
}

/// A historical battle played (`BattleSim.get_historical`): its name, date, historical winner.
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryInfo {
    pub name: String,
    pub date_fr: String,
    pub historical_winner: SideId,
    /// Faction names `[attacker, defender]`.
    pub faction_names: [String; 2],
}

/// Inputs of the result screen that other lots fill.
#[derive(Resource, Default, Debug, Clone)]
pub struct ResultExtras {
    pub aftermath: Option<Aftermath>,
    pub history: Option<HistoryInfo>,
}

const RED: Color = Color::srgb(0.55, 0.1, 0.08);
const GREEN: Color = Color::srgb(0.16, 0.4, 0.16);
const GOLD: Color = Color::srgb(0.6, 0.45, 0.08);
const GOLD_DEEP: Color = Color::srgb(0.5, 0.37, 0.06);

fn texts() -> Value {
    paths::find_root()
        .ok()
        .and_then(|root| {
            std::fs::read_to_string(paths::data_dir(&root).join("ui/transitions.json")).ok()
        })
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or(Value::Null)
}

fn plural(count: u32, one: &str, many: &str) -> String {
    format!("{count} {}", if count > 1 { many } else { one })
}

/// Lines of the « Dans l’Histoire » card (`[]` without a historical battle): the verdict compares
/// the outcome with History.
pub fn history_lines(
    history: Option<&HistoryInfo>,
    winner: SideId,
    player: SideId,
    texts: &Value,
) -> Vec<String> {
    let Some(history) = history else {
        return Vec::new();
    };
    let block = &texts["history"];
    let key = format!(
        "{}_{}",
        if winner == history.historical_winner {
            "repeated"
        } else {
            "reversed"
        },
        if winner == player { "win" } else { "loss" }
    );
    let winner_name = &history.faction_names[match history.historical_winner {
        SideId::Attacker => 0,
        SideId::Defender => 1,
    }];
    let line = block["line"]
        .as_str()
        .unwrap_or("{date} — {name} : {winner} l’emporta.")
        .replace("{date}", &history.date_fr)
        .replace("{name}", &history.name)
        .replace("{winner}", winner_name);
    vec![block[&key].as_str().unwrap_or_default().to_owned(), line]
}

/// Title of the history card.
pub fn history_title(texts: &Value) -> String {
    texts["history"]["title"]
        .as_str()
        .unwrap_or("Dans l’Histoire")
        .to_owned()
}

fn hero_card(view: &ResultView) -> Card {
    let mut lines = Vec::new();
    match view.rows[0].iter().find(|r| r.hero) {
        Some(hero) => lines.push(format!(
            "{} : {} ennemis abattus, {} survivants.",
            hero.name, hero.kills, hero.soldiers
        )),
        None => lines.push("Aucun régiment ne s’est distingué.".to_owned()),
    }
    if let Some(foe) = view.rows[1].iter().find(|r: &&RegimentRow| r.hero) {
        lines.push(format!(
            "Le plus redoutable adversaire : {} ({} des nôtres).",
            foe.name, foe.kills
        ));
    }
    Card {
        title: "Héros de la bataille".into(),
        lines,
        accent: GOLD,
    }
}

/// The four cards of `_fill_aftermath`.
pub fn cards(
    view: &ResultView,
    outcome: &BattleOutcome,
    player: SideId,
    aftermath: Option<&Aftermath>,
) -> Vec<Card> {
    let enemy = match player {
        SideId::Attacker => SideId::Defender,
        SideId::Defender => SideId::Attacker,
    };
    let own = outcome.side(player);
    let foe = outcome.side(enemy);
    let in_campaign = aftermath.is_some();
    let mut out = vec![hero_card(view)];
    // Captives and ransoms.
    let mut captives = Vec::new();
    if let Some(aftermath) = aftermath {
        for captive in &aftermath.captives {
            let rank = if captive.rank.is_empty() {
                String::new()
            } else {
                format!(" ({})", captive.rank)
            };
            captives.push(format!(
                "{}{rank} — rançon {} ₶",
                captive.name,
                fr::digits(captive.ransom)
            ));
        }
        for captive in &aftermath.lost {
            captives.push(format!(
                "Des nôtres pris : {} — rançon exigée {} ₶",
                captive.name,
                fr::digits(captive.ransom)
            ));
        }
    }
    if captives.is_empty() {
        let winner = outcome.side(outcome.winner);
        captives.push(if winner.no_quarter {
            "Pas de quartier : aucun prisonnier.".to_owned()
        } else if foe.general_captured {
            "Le chef ennemi est pris : rançon à fixer à la cour.".to_owned()
        } else if in_campaign {
            "Aucun captif de rang.".to_owned()
        } else {
            "Suites connues au retour en campagne.".to_owned()
        });
    }
    let chased: u32 = foe.pursuit_losses.iter().sum();
    if chased > 0 || foe.captured > 0 {
        captives.push(format!(
            "Poursuite : {chased} fuyards abattus, {} prisonniers de troupe.",
            foe.captured
        ));
    }
    let own_chased: u32 = own.pursuit_losses.iter().sum();
    if own_chased > 0 || own.captured > 0 {
        captives.push(format!(
            "Nos fuyards rattrapés : {own_chased} tués, {} pris.",
            own.captured
        ));
    }
    out.push(Card {
        title: "Captifs et rançons".into(),
        lines: captives,
        accent: RED,
    });
    // Experience.
    let mut xp = Vec::new();
    if let Some(aftermath) = aftermath {
        if !aftermath.general_name.is_empty() {
            if aftermath.general_fate.is_empty() {
                let mut line = format!(
                    "{} : +{} d’expérience",
                    aftermath.general_name, aftermath.general_xp
                );
                if aftermath.skill_points > 0 {
                    line += &format!(", +{} point de compétence", aftermath.skill_points);
                }
                xp.push(line + ".");
            } else {
                xp.push(format!(
                    "{} : {}.",
                    aftermath.general_name, aftermath.general_fate
                ));
            }
        }
        xp.push(if aftermath.veterans == 0 {
            "Aucun régiment aguerri.".to_owned()
        } else {
            let n = aftermath.veterans;
            format!(
                "{} aguerri{} (chevron gagné).",
                plural(n, "régiment", "régiments"),
                if n > 1 { "s" } else { "" }
            )
        });
    } else {
        xp.push("Suites connues au retour en campagne.".to_owned());
    }
    out.push(Card {
        title: "Expérience".into(),
        lines: xp,
        accent: GREEN,
    });
    // Loot: expected ransoms, trophies, standards lost.
    let mut loot = Vec::new();
    let total = aftermath.map_or(0, |a| a.ransom_total);
    loot.push(if total > 0 {
        format!("{} ₶ de rançons à percevoir.", fr::digits(total))
    } else {
        "Aucun butin : pas de rançon à percevoir.".to_owned()
    });
    if !own.standards_taken.is_empty() {
        let labels: Vec<String> = own
            .standards_taken
            .iter()
            .map(|t| {
                if t.general {
                    format!("bannière du chef ({})", t.unit_name)
                } else {
                    t.unit_name.clone()
                }
            })
            .collect();
        loot.push(format!("Trophées : {}.", labels.join(", ")));
    }
    if own.standards_lost > 0 {
        loot.push(format!(
            "{} aux mains de l’ennemi.",
            if own.standards_lost == 1 {
                "Un de nos étendards".to_owned()
            } else {
                format!("{} de nos étendards", own.standards_lost)
            }
        ));
    }
    loot.push("Le pillage vient des chevauchées et des villes prises.".to_owned());
    out.push(Card {
        title: "Butin".into(),
        lines: loot,
        accent: GOLD_DEEP,
    });
    out
}

/// All the cards: the four, then the history card when the battle is a historical one.
pub fn all_cards(
    view: &ResultView,
    outcome: &BattleOutcome,
    player: SideId,
    extras: &ResultExtras,
) -> Vec<Card> {
    let mut out = cards(view, outcome, player, extras.aftermath.as_ref());
    let texts = texts();
    let lines = history_lines(extras.history.as_ref(), outcome.winner, player, &texts);
    if !lines.is_empty() {
        out.push(Card {
            title: history_title(&texts),
            lines,
            accent: style::GOLD,
        });
    }
    out
}

/// Asset path (under `game/assets`) of the painted backdrop of the outcome
/// (`illustrations.json`, `endings`: `battle_victory` / `battle_defeat`).
pub fn ending_image(won: bool) -> Option<String> {
    let root = paths::find_root().ok()?;
    let text =
        std::fs::read_to_string(paths::data_dir(&root).join("ui/illustrations.json")).ok()?;
    let data: Value = serde_json::from_str(&text).ok()?;
    ending_from(&data, won)
}

pub fn ending_from(data: &Value, won: bool) -> Option<String> {
    let wanted = if won {
        "battle_victory"
    } else {
        "battle_defeat"
    };
    data["endings"]
        .as_array()?
        .iter()
        .find(|e| e["outcome"] == wanted)
        .and_then(|e| e["image"].as_str())
        .map(|path| path.trim_start_matches("res://assets/").to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn history() -> HistoryInfo {
        HistoryInfo {
            name: "Azincourt".into(),
            date_fr: "25 octobre 1415".into(),
            historical_winner: SideId::Attacker,
            faction_names: ["Angleterre".into(), "France".into()],
        }
    }

    fn texts_fixture() -> Value {
        json!({"history": {"title": "Dans l’Histoire", "line": "{date} — {name} : {winner} l’emporta.",
            "reversed_win": "renversé", "repeated_win": "répétée", "reversed_loss": "plus favorable", "repeated_loss": "défaite répétée"}})
    }

    #[test]
    fn history_compares_with_the_historical_winner() {
        let texts = texts_fixture();
        // The attacker won in History; the player (defender) lost too: the defeat repeats.
        let lines = history_lines(Some(&history()), SideId::Attacker, SideId::Defender, &texts);
        assert_eq!(lines[0], "défaite répétée");
        assert_eq!(
            lines[1],
            "25 octobre 1415 — Azincourt : Angleterre l’emporta."
        );
        let lines = history_lines(Some(&history()), SideId::Defender, SideId::Defender, &texts);
        assert_eq!(lines[0], "renversé");
        assert!(history_lines(None, SideId::Attacker, SideId::Attacker, &texts).is_empty());
    }

    #[test]
    fn ending_images_lose_the_res_prefix() {
        let data = json!({"endings": [
            {"outcome": "battle_victory", "image": "res://assets/art/endings/v.jpg"},
            {"outcome": "battle_defeat", "image": "res://assets/art/endings/d.jpg"}]});
        assert_eq!(
            ending_from(&data, true).as_deref(),
            Some("art/endings/v.jpg")
        );
        assert_eq!(
            ending_from(&data, false).as_deref(),
            Some("art/endings/d.jpg")
        );
        assert_eq!(ending_from(&json!({}), true), None);
    }

    #[test]
    fn plural_forms() {
        assert_eq!(plural(1, "régiment", "régiments"), "1 régiment");
        assert_eq!(plural(3, "régiment", "régiments"), "3 régiments");
    }
}
