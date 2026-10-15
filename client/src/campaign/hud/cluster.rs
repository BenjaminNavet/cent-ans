//! Alert pills of the end-of-turn cluster (ports the grouping of game/scripts/ui/
//! end_turn_cluster.gd): alerts of the same kind are folded into one pill with a short label and
//! a counter; a click goes to the next alert of the group in turn (see `mod.rs`).

use game_api::Alert;

/// Display order of the kinds (most urgent first).
const KIND_ORDER: [&str; 11] = [
    "chronicle_decision",
    "enemy_army",
    "siege",
    "debt",
    "idle_army",
    "idle_character",
    "research_idle",
    "free_slot",
    "mission_due",
    "construction_done",
    "research_done",
];
/// Pills at most; the last one then gathers the rest under « Autres avis ».
pub const MAX_BADGES: usize = 7;
/// Kinds drawn in the danger colour.
pub const DANGER_KINDS: [&str; 4] = ["chronicle_decision", "enemy_army", "siege", "debt"];

/// Short label written on a pill.
pub fn short_label(kind: &str) -> &'static str {
    match kind {
        "other" => "Autres avis",
        "chronicle_decision" => "Décision",
        "enemy_army" => "Armée ennemie",
        "siege" => "Siège",
        "debt" => "Dette",
        "idle_army" => "Armée sans ordre",
        "free_slot" => "Chantier libre",
        "mission_due" => "Mission proche",
        "idle_character" => "Sans charge",
        "construction_done" => "Chantier fini",
        "research_done" => "Recherche finie",
        "research_idle" => "Aucune recherche",
        "ransom" => "Rançon",
        "table" => "Vivres",
        "medicine" => "Médecine",
        "herbarium" => "Herbier",
        "coinage" => "Monnaie",
        "chivalry" => "Chevalerie",
        "crusade" => "Croisade",
        "agent" => "Agents",
        "diplomacy_offer" => "Proposition",
        "trade" => "Commerce",
        "mission" => "Mission",
        "royal_act" => "Acte royal",
        "level_up" => "Montée en niveau",
        "loyalty" => "Loyauté",
        "captain" => "Capitaine",
        _ => "Avis",
    }
}

/// Longer description of a kind, first line of the pill tooltip.
pub fn long_label(kind: &str) -> &'static str {
    match kind {
        "chronicle_decision" => "Décision de chronique",
        "idle_army" => "Armée sans ordre de marche",
        "free_slot" => "Emplacement de construction libre",
        "mission_due" => "Mission proche de l’échéance",
        "idle_character" => "Personnage sans affectation",
        "construction_done" => "Construction achevée",
        "research_done" => "Recherche achevée",
        "research_idle" => "Aucune recherche en cours",
        "ransom" => "Captifs et rançons",
        "diplomacy_offer" => "Proposition diplomatique",
        "trade" => "Accords et routes commerciales",
        "mission" => "Mission obtenue, réussie ou échouée",
        other => short_label(other),
    }
}

/// Alerts of one kind.
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub kind: String,
    pub alerts: Vec<Alert>,
}

impl Group {
    pub fn danger(&self) -> bool {
        DANGER_KINDS.contains(&self.kind.as_str())
    }

    pub fn blocking(&self) -> bool {
        self.alerts.iter().any(|a| a.blocking)
    }

    pub fn tooltip(&self) -> String {
        let mut lines = vec![format!("[b]{}[/b]", long_label(&self.kind))];
        for alert in self.alerts.iter().take(6) {
            lines.push(format!("• {}", alert.text));
        }
        if self.alerts.len() > 6 {
            let more = self.alerts.len() - 6;
            lines.push(format!(
                "… et {more} autre{}",
                if more > 1 { "s" } else { "" }
            ));
        }
        lines.push("Clic : aller à la suivante".to_owned());
        lines.join("\n")
    }
}

/// Folds alerts by kind, most urgent first, at most [`MAX_BADGES`] groups.
pub fn group_alerts(alerts: &[Alert]) -> Vec<Group> {
    let mut by_kind: Vec<Group> = Vec::new();
    for alert in alerts {
        match by_kind.iter_mut().find(|g| g.kind == alert.kind) {
            Some(group) => group.alerts.push(alert.clone()),
            None => by_kind.push(Group {
                kind: alert.kind.clone(),
                alerts: vec![alert.clone()],
            }),
        }
    }
    let rank = |kind: &str| KIND_ORDER.iter().position(|k| *k == kind).unwrap_or(99);
    by_kind.sort_by(|a, b| {
        rank(&a.kind)
            .cmp(&rank(&b.kind))
            .then_with(|| a.kind.cmp(&b.kind))
    });
    if by_kind.len() > MAX_BADGES {
        let rest: Vec<Alert> = by_kind
            .split_off(MAX_BADGES - 1)
            .into_iter()
            .flat_map(|g| g.alerts)
            .collect();
        by_kind.push(Group {
            kind: "other".to_owned(),
            alerts: rest,
        });
    }
    by_kind
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_api::turn::Severity;

    fn alert(id: &str, kind: &str) -> Alert {
        Alert {
            id: id.to_owned(),
            kind: kind.to_owned(),
            severity: Severity::Info,
            text: format!("text {id}"),
            province: None,
            army: None,
            blocking: kind == "chronicle_decision",
        }
    }

    #[test]
    fn groups_by_kind_in_urgency_order() {
        let alerts = [
            alert("a", "research_idle"),
            alert("b", "siege"),
            alert("c", "siege"),
            alert("d", "chronicle_decision"),
        ];
        let groups = group_alerts(&alerts);
        let kinds: Vec<&str> = groups.iter().map(|g| g.kind.as_str()).collect();
        assert_eq!(kinds, ["chronicle_decision", "siege", "research_idle"]);
        assert_eq!(groups[1].alerts.len(), 2);
        assert!(groups[0].blocking() && groups[0].danger());
        assert!(!groups[2].danger());
    }

    #[test]
    fn extra_kinds_fold_into_other() {
        let kinds = [
            "chronicle_decision",
            "enemy_army",
            "siege",
            "debt",
            "idle_army",
            "idle_character",
            "research_idle",
            "free_slot",
            "trade",
        ];
        let alerts: Vec<Alert> = kinds.iter().map(|k| alert(k, k)).collect();
        let groups = group_alerts(&alerts);
        assert_eq!(groups.len(), MAX_BADGES);
        let last = groups.last().expect("groups");
        assert_eq!(last.kind, "other");
        assert_eq!(last.alerts.len(), 3);
        assert_eq!(short_label("other"), "Autres avis");
    }

    #[test]
    fn tooltip_lists_six_and_counts_the_rest() {
        let alerts: Vec<Alert> = (0..8).map(|i| alert(&i.to_string(), "siege")).collect();
        let tip = group_alerts(&alerts)[0].tooltip();
        assert!(tip.contains("… et 2 autres"));
    }
}
