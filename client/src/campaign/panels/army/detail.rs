//! Regiment sheet: builds the [`TooltipSpec`] of one regiment from `game_api::recruit::RegimentDetail`
//! (port of `RichTooltip.unit_spec`, in-army case). The hover bubble is the short version; a right
//! click (`unit_details_requested`, `HudController.show_unit_details`) pins the detailed one.
//! Alt chains the `ib:rule:<key>` links of both (ADR 0109). Presentation only: strengths and
//! weaknesses come from the core.

use bevy::prelude::*;
use game_api::recruit::{RegimentDetail, RegimentExtra};

use super::Ranks;
use crate::ui::kit::fr;
use crate::ui::kit::tooltip::TooltipRegistry;
use crate::ui::kit::tooltip_spec::{
    Effect, Footer, Headline, Requirement, StatRow, TooltipSpec, TooltipTexts, register_rule_texts,
};

/// `data/ui/tooltips.json`, loaded once for the army panels.
#[derive(Resource, Default)]
pub struct ArmyTexts(pub TooltipTexts);

/// Detailed bubble of the regiment, kept on its card for the right click.
#[derive(Component, Debug, Clone)]
pub struct RegimentCard {
    pub detailed: String,
}

/// Registers the rule bubbles (`ib:rule:*`) the chained tooltips open.
pub fn register_rules(texts: Res<ArmyTexts>, mut registry: ResMut<TooltipRegistry>) {
    register_rule_texts(&mut registry, &texts.0, |_| None);
}

fn label(texts: &TooltipTexts, block: &str, key: &str) -> String {
    texts
        .entry(block, key)
        .map(|e| {
            if e.label.is_empty() {
                e.title.clone()
            } else {
                e.label.clone()
            }
        })
        .filter(|l| !l.is_empty())
        .unwrap_or_else(|| key.to_owned())
}

fn number(value: f64) -> String {
    if (value - value.round()).abs() < 1e-9 {
        format!("{}", value as i64)
    } else {
        format!("{value:.1}").replace('.', ",")
    }
}

fn capitalised(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().collect::<String>() + chars.as_str()
    })
}

/// Spec of one regiment of an army.
pub fn regiment_spec(
    detail: &RegimentDetail,
    extra: Option<&RegimentExtra>,
    ranks: &Ranks,
    texts: &TooltipTexts,
) -> TooltipSpec {
    let category = capitalised(&label(texts, "categories", &detail.category));
    let mut subtitle = category;
    if detail.soldiers > 0 {
        subtitle.push_str(&format!(", {} hommes", detail.soldiers));
    }
    if detail.mercenary {
        subtitle.push_str(" · compagnie mercenaire");
    }
    let mut spec = TooltipSpec {
        id: detail.unit_type.clone(),
        kind: "unit".to_owned(),
        title: detail.name.clone(),
        subtitle,
        icon: "unit".to_owned(),
        ..TooltipSpec::default()
    };
    spec.headline.push(Headline {
        key: "strength".into(),
        icon: "gauge_strength".into(),
        label: "Effectif".into(),
        value: format!("{} / {}", detail.strength, detail.max_strength),
        ..Headline::default()
    });
    spec.headline.push(Headline {
        key: "morale".into(),
        icon: "gauge_morale".into(),
        label: "Moral".into(),
        value: detail.morale.to_string(),
        ..Headline::default()
    });
    let main = if detail
        .stats
        .iter()
        .find(|s| s.key == "ranged")
        .map_or(0.0, |s| s.value)
        > detail
            .stats
            .iter()
            .find(|s| s.key == "melee")
            .map_or(0.0, |s| s.value)
    {
        "ranged"
    } else {
        "melee"
    };
    if let Some(stat) = detail.stats.iter().find(|s| s.key == main) {
        spec.headline.push(Headline {
            key: "melee_or_ranged".into(),
            stat: main.into(),
            label: label(texts, "stats", main),
            value: number(stat.value),
            ..Headline::default()
        });
    }
    spec.effects.push(Effect {
        key: "army_experience".into(),
        text: ranks.summary(detail.experience),
        ..Effect::default()
    });
    spec.stats = detail
        .stats
        .iter()
        .map(|s| StatRow {
            key: s.key.clone(),
            label: label(texts, "stats", &s.key),
            value: number(s.value),
        })
        .collect();
    spec.traits.strengths = detail
        .strengths
        .iter()
        .map(|k| label(texts, "stats", k).to_lowercase())
        .collect();
    spec.traits.weaknesses = detail
        .weaknesses
        .iter()
        .map(|k| {
            if k == "ranged"
                && detail
                    .stats
                    .iter()
                    .all(|s| s.key != "ranged" || s.value <= 0.0)
            {
                "ne tire pas".to_owned()
            } else {
                label(texts, "stats", k).to_lowercase()
            }
        })
        .collect();
    if detail.rain_penalty {
        spec.traits
            .weaknesses
            .push(label(texts, "abilities", "rain_penalty"));
    }
    spec.traits.abilities = detail
        .abilities
        .iter()
        .map(|k| label(texts, "abilities", k))
        .collect();
    if let Some(tech) = &detail.required_technology {
        spec.requires.push(Requirement {
            text: tech.clone(),
            met: None,
        });
    }
    let upkeep = extra.map_or(detail.upkeep, |e| e.upkeep);
    spec.footer = Footer {
        cost: String::new(),
        upkeep: if upkeep > 0 {
            format!("{} / saison", fr::amount(i64::from(upkeep)))
        } else {
            "—".to_owned()
        },
        time: String::new(),
    };
    match (detail.available_from, detail.available_until) {
        (Some(from), Some(until)) => spec.detail.push(format!("Époque : {from} – {until}")),
        (Some(from), None) => spec.detail.push(format!("Époque : dès {from}")),
        (None, Some(until)) => spec.detail.push(format!("Époque : jusqu'en {until}")),
        (None, None) => {}
    }
    if !detail.source_class.is_empty() {
        spec.detail.push(format!(
            "Recrutés parmi : {}",
            label(texts, "classes", &detail.source_class).to_lowercase()
        ));
    }
    spec.flavour.clone_from(&detail.description);
    spec
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detail() -> RegimentDetail {
        RegimentDetail {
            unit_type: "unit_x".into(),
            name: "Arbalétriers".into(),
            category: "ranged".into(),
            soldiers: 60,
            mounted: false,
            mercenary: true,
            upkeep: 12,
            recruit_time_turns: Some(1),
            strength: 40,
            max_strength: 60,
            morale: 55,
            experience: 3,
            stats: vec![
                game_api::recruit::DetailStat {
                    key: "melee".into(),
                    value: 2.0,
                },
                game_api::recruit::DetailStat {
                    key: "ranged".into(),
                    value: 7.0,
                },
            ],
            strengths: vec!["ranged".into()],
            weaknesses: vec!["melee".into()],
            abilities: vec!["pavise".into()],
            rain_penalty: true,
            required_technology: None,
            source_class: "burghers".into(),
            available_from: Some(1337),
            available_until: None,
            description: "Tireurs.".into(),
        }
    }

    #[test]
    fn short_and_detailed_versions() {
        let texts = TooltipTexts::default();
        let spec = regiment_spec(&detail(), None, &Ranks::default(), &texts);
        assert_eq!(spec.headline.len(), 3);
        assert_eq!(spec.headline[2].stat, "ranged");
        let short = spec.to_bbcode(&texts, false);
        let long = spec.to_bbcode(&texts, true);
        assert!(short.contains("Effectif"));
        assert!(!short.contains("Époque"));
        assert!(long.contains("Époque : dès 1337"));
        assert!(long.len() > short.len());
        assert!(spec.subtitle.contains("mercenaire"));
    }
}
