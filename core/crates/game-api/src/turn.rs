//! End-of-turn readiness and alerts.
//!
//! Replaces the state-reading half of `game/scripts/ui/alerts.gd` (`CampaignAlerts.collect`):
//! siege, enemy army near the border, debt, idle research, idle armies, free building slots,
//! pending diplomatic offers and chronicle decisions. Fog of war is a client concern: the
//! `enemy_army` alerts list every enemy army, the client filters the visible ones.

use data_model::{FactionId, GameData};
use serde::Serialize;
use sim_campaign::CampaignState;

use crate::army::is_idle;

/// Alert severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Danger,
}

/// One alert of the end-of-turn bell; `text` is French.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Alert {
    pub id: String,
    /// `siege`, `enemy_army`, `debt`, `research_idle`, `idle_army`, `free_slot`,
    /// `diplomacy_offer`, `chronicle_decision`.
    pub kind: String,
    pub severity: Severity,
    pub text: String,
    pub province: Option<String>,
    pub army: Option<String>,
    /// A blocking alert should stop the end of turn until handled.
    pub blocking: bool,
}

/// What the end-turn button needs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TurnReadiness {
    pub turn: u32,
    pub date_label: String,
    pub alerts: Vec<Alert>,
    /// Ids of the player's idle armies.
    pub idle_armies: Vec<String>,
    pub research_idle: bool,
    pub pending_decisions: usize,
    pub pending_offers: usize,
    /// True when a blocking alert is present.
    pub blocked: bool,
}

fn alert(id: String, kind: &str, severity: Severity, text: String) -> Alert {
    Alert {
        id,
        kind: kind.to_owned(),
        severity,
        text,
        province: None,
        army: None,
        blocking: false,
    }
}

/// Readiness and alerts of `faction` (normally the player's) for the current turn.
pub fn turn_readiness(
    data: &GameData,
    state: &CampaignState,
    faction: &FactionId,
) -> TurnReadiness {
    let mut alerts = Vec::new();
    let summary = state.faction_summary(faction);
    let enemies = summary
        .as_ref()
        .map(|s| s.at_war_with.clone())
        .unwrap_or_default();
    let owned: std::collections::BTreeSet<_> = state
        .provinces
        .keys()
        .filter(|p| {
            state
                .city_state(p)
                .is_some_and(|c| &c.owner == faction && &c.controller == faction)
        })
        .cloned()
        .collect();
    // Sieges against the faction's provinces.
    for province in &owned {
        let Some(siege) = state.city_state(province).and_then(|c| c.siege.as_ref()) else {
            continue;
        };
        if &siege.attacker == faction {
            continue;
        }
        let mut a = alert(
            format!("siege:{province}"),
            "siege",
            Severity::Danger,
            format!(
                "{} assiégée (vivres : {})",
                data.province_name(province),
                siege.supplies
            ),
        );
        a.province = Some(province.as_str().to_owned());
        alerts.push(a);
    }
    // Enemy armies in or next to an owned province.
    for (army_id, army) in state.armies() {
        if !enemies.contains(&army.faction) {
            continue;
        }
        let Some(location) = state.army_province(data, army) else {
            continue;
        };
        let threatened = owned.contains(&location)
            || data
                .provinces
                .get(&location)
                .is_some_and(|p| p.neighbors.iter().any(|n| owned.contains(n)));
        if !threatened {
            continue;
        }
        let strength: u32 = army.units.iter().map(|u| u.strength).sum();
        let mut a = alert(
            format!("army:{army_id}"),
            "enemy_army",
            Severity::Danger,
            format!(
                "Armée ennemie près de {} ({strength} hommes)",
                data.province_name(&location)
            ),
        );
        a.army = Some(army_id.to_string());
        a.province = Some(location.as_str().to_owned());
        alerts.push(a);
    }
    if let Some(summary) = &summary {
        if summary.treasury < 0 {
            alerts.push(alert(
                "debt".to_owned(),
                "debt",
                Severity::Danger,
                format!("Trésor endetté : {} livres", summary.treasury),
            ));
        }
    }
    let research_idle = state.research_info(data, faction).is_none()
        && state.research_points_per_turn(data, faction) > 0;
    if research_idle {
        alerts.push(alert(
            "research_idle".to_owned(),
            "research_idle",
            Severity::Warning,
            "Aucune recherche en cours".to_owned(),
        ));
    }
    let idle_armies: Vec<String> = state
        .armies()
        .iter()
        .filter(|(_, army)| &army.faction == faction && is_idle(army))
        .map(|(id, _)| id.to_string())
        .collect();
    for id in &idle_armies {
        let general = state
            .armies()
            .iter()
            .find(|(a, _)| a.to_string() == *id)
            .and_then(|(_, army)| army.general.as_ref())
            .map(|g| state.character_name(data, g))
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| "ost".to_owned());
        let mut a = alert(
            format!("idle:{id}"),
            "idle_army",
            Severity::Warning,
            format!("Armée sans ordre : {general}"),
        );
        a.army = Some(id.clone());
        alerts.push(a);
    }
    let overview = sim_campaign::holdings::holdings_overview(state, data, faction);
    let treasury = summary.as_ref().map_or(0, |s| s.treasury);
    let mut free_slots = 0usize;
    let mut first_province = None;
    for province in &overview.provinces {
        for place in &province.settlements {
            let cheapest = place
                .options_available
                .iter()
                .map(|o| i64::from(o.cost))
                .min();
            if place.idle && cheapest.is_some_and(|c| c <= treasury) {
                free_slots += 1;
                first_province.get_or_insert_with(|| province.province.as_str().to_owned());
            }
        }
    }
    if free_slots > 0 {
        let mut a = alert(
            "free_slot".to_owned(),
            "free_slot",
            Severity::Info,
            format!("{free_slots} colonie(s) sans chantier en cours"),
        );
        a.province = first_province;
        alerts.push(a);
    }
    let offers = state.valid_offers(data, faction).len();
    if offers > 0 {
        alerts.push(alert(
            "offers".to_owned(),
            "diplomacy_offer",
            Severity::Warning,
            format!("{offers} proposition(s) diplomatique(s) en attente"),
        ));
    }
    let decisions = state.decision_views(data, faction);
    for decision in &decisions {
        let mut a = alert(
            format!("chronicle:{}", decision.id),
            "chronicle_decision",
            Severity::Warning,
            format!("Chronique : {}", decision.title),
        );
        a.blocking = !matches!(decision.presentation, data_model::EventPresentation::Map);
        a.province = decision.province.as_ref().map(|p| p.as_str().to_owned());
        alerts.push(a);
    }
    let blocked = alerts.iter().any(|a| a.blocking);
    TurnReadiness {
        turn: state.turn(),
        date_label: state.date_label(),
        alerts,
        idle_armies,
        research_idle,
        pending_decisions: decisions.len(),
        pending_offers: offers,
        blocked,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readiness_at_start_and_with_debt() {
        let data = data_model::test_support::game_data();
        let mut state = sim_campaign::test_support::start_quiet(data, "fac_france", 1337);
        let france = FactionId::new("fac_france").unwrap();
        let ready = turn_readiness(data, &state, &france);
        assert_eq!(ready.turn, 0);
        assert!(ready.alerts.iter().all(|a| a.kind != "debt"));
        assert_eq!(
            ready.idle_armies.len(),
            ready
                .alerts
                .iter()
                .filter(|a| a.kind == "idle_army")
                .count()
        );
        assert!(!ready.idle_armies.is_empty(), "armies start without orders");
        assert!(serde_json::to_string(&ready).is_ok());
        state.factions.get_mut(&france).unwrap().treasury = -100;
        let debt = turn_readiness(data, &state, &france);
        assert!(debt
            .alerts
            .iter()
            .any(|a| a.kind == "debt" && a.severity == Severity::Danger));
    }
}
