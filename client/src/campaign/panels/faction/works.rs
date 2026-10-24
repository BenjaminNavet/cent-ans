//! « Chantiers » tab (player only): realm-wide construction overview
//! (`docs/design/chantiers-royaume.md`). Pure logic here (filters, batch entries, texts); every
//! rule lives in `sim_campaign::realm_works`, which also plans and applies the batch.

use std::collections::BTreeMap;

use data_model::{BuildingId, SettlementId, SettlementKind};
use sim_campaign::realm_works::{BatchReport, RealmWorks, WorkRow};

/// Choices of the tab, kept across rebuilds.
#[derive(Debug, Clone, PartialEq)]
pub struct WorksUi {
    /// Only the colonies with a free slot (default).
    pub only_free: bool,
    /// Province filter (`None`: all).
    pub province: Option<String>,
    /// Settlement kind filter (`None`: all).
    pub kind: Option<SettlementKind>,
    /// Building of « Appliquer à tout ».
    pub global: Option<String>,
    /// Building chosen in a row menu, by settlement id.
    pub picks: BTreeMap<String, String>,
}

impl Default for WorksUi {
    fn default() -> Self {
        Self {
            only_free: true,
            province: None,
            kind: None,
            global: None,
            picks: BTreeMap::new(),
        }
    }
}

/// Next value of a menu that cycles through `all` then back to « none ».
pub fn cycle<T: PartialEq + Clone>(current: Option<&T>, all: &[T]) -> Option<T> {
    match current.and_then(|c| all.iter().position(|x| x == c)) {
        None => all.first().cloned(),
        Some(i) => all.get(i + 1).cloned(),
    }
}

/// `true` when the row passes the filters.
pub fn passes(row: &WorkRow, ui: &WorksUi) -> bool {
    (!ui.only_free || row.slots_free().is_none_or(|free| free > 0))
        && ui
            .province
            .as_ref()
            .is_none_or(|p| row.province.as_str() == p)
        && ui.kind.is_none_or(|k| row.kind == k)
}

/// Rows of the list, in the order of the core.
pub fn visible<'a>(works: &'a RealmWorks, ui: &WorksUi) -> Vec<&'a WorkRow> {
    works.rows.iter().filter(|row| passes(row, ui)).collect()
}

/// Provinces of the colonies, `(id, name)`, by name.
pub fn provinces(works: &RealmWorks) -> Vec<(String, String)> {
    let mut all: Vec<(String, String)> = works
        .rows
        .iter()
        .map(|r| (r.province.to_string(), r.province_name.clone()))
        .collect();
    all.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    all.dedup();
    all
}

/// Kinds present among the colonies.
pub fn kinds(works: &RealmWorks) -> Vec<SettlementKind> {
    let mut all: Vec<SettlementKind> = works.rows.iter().map(|r| r.kind).collect();
    all.sort();
    all.dedup();
    all
}

/// French label of a settlement kind.
pub fn kind_label(kind: SettlementKind) -> &'static str {
    match kind {
        SettlementKind::City => "Cité",
        SettlementKind::Town => "Ville",
        SettlementKind::Castle => "Château",
        SettlementKind::Abbey => "Abbaye",
        SettlementKind::Village => "Village",
    }
}

/// Ids of the buildings the row can start, in the order of the core.
pub fn available_ids(row: &WorkRow) -> Vec<String> {
    row.options
        .iter()
        .filter(|o| o.available)
        .map(|o| o.building.to_string())
        .collect()
}

/// Buildings that can be started in at least one visible colony, `(id, name)` by name.
pub fn global_choices(rows: &[&WorkRow]) -> Vec<(String, String)> {
    let mut all: Vec<(String, String)> = rows
        .iter()
        .flat_map(|r| r.options.iter())
        .filter(|o| o.available)
        .map(|o| (o.building.to_string(), o.name.clone()))
        .collect();
    all.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    all.dedup();
    all
}

/// Building the row will start: its own pick, else the global choice when available there.
pub fn chosen_for<'a>(row: &WorkRow, ui: &'a WorksUi) -> Option<&'a str> {
    let ok = |id: &&String| {
        row.options
            .iter()
            .any(|o| o.available && o.building.as_str() == id.as_str())
    };
    ui.picks
        .get(row.settlement.as_str())
        .filter(ok)
        .or_else(|| ui.global.as_ref().filter(ok))
        .map(String::as_str)
}

/// `(settlement, building)` entries of the batch, in the order of the list.
pub fn entries(rows: &[&WorkRow], ui: &WorksUi) -> Vec<(SettlementId, BuildingId)> {
    rows.iter()
        .filter_map(|row| {
            let building = BuildingId::new(chosen_for(row, ui)?.to_owned()).ok()?;
            Some((row.settlement.clone(), building))
        })
        .collect()
}

/// Header line: free slots, works, treasury.
pub fn header_text(works: &RealmWorks) -> String {
    format!(
        "Emplacements libres : {}  ·  Chantiers en cours : {}  ·  Trésorerie : {} écus",
        works.free_slots, works.works_in_progress, works.treasury
    )
}

/// `x/y` free slots of a row (`x/∞` when uncapped).
pub fn slots_text(row: &WorkRow) -> String {
    match row.slots_max {
        Some(max) => format!("{}/{}", row.slots_free().unwrap_or(0), max),
        None => "libres/∞".to_owned(),
    }
}

/// Tooltip of a row menu: every building with cost, duration or the reason it is unavailable.
pub fn options_tooltip(row: &WorkRow) -> String {
    let mut lines = Vec::new();
    for o in &row.options {
        if o.available {
            lines.push(format!("{} : {} écus, {} tours", o.name, o.cost, o.turns));
        } else {
            lines.push(format!(
                "{} : indisponible ({})",
                o.name,
                o.reason.as_deref().unwrap_or("raison inconnue")
            ));
        }
    }
    if lines.is_empty() {
        "Aucun bâtiment constructible ici.".to_owned()
    } else {
        lines.join("\n")
    }
}

/// Label of the menu button of a row.
pub fn pick_label(row: &WorkRow, ui: &WorksUi) -> String {
    match ui.picks.get(row.settlement.as_str()).and_then(|id| {
        row.options
            .iter()
            .find(|o| o.available && o.building.as_str() == id)
    }) {
        Some(o) => format!("{} ({} écus)", o.name, o.cost),
        None => "Choisir…".to_owned(),
    }
}

/// Recap of a planned batch, one line per entry, then the totals.
pub fn recap_lines(report: &BatchReport, rows: &[&WorkRow], treasury: i64) -> Vec<String> {
    let name_of = |id: &SettlementId| {
        rows.iter()
            .find(|r| &r.settlement == id)
            .map_or_else(|| id.to_string(), |r| r.name.clone())
    };
    let building_of = |row: &SettlementId, id: &BuildingId| {
        rows.iter()
            .find(|r| &r.settlement == row)
            .and_then(|r| r.options.iter().find(|o| &o.building == id))
            .map_or_else(|| id.to_string(), |o| o.name.clone())
    };
    let mut lines = Vec::new();
    for e in &report.entries {
        let place = name_of(&e.settlement);
        let building = building_of(&e.settlement, &e.building);
        lines.push(match &e.outcome {
            Ok(cost) => format!("{place} : {building}, {cost} écus"),
            Err(reason) => format!("{place} : {building} ignoré ({reason})"),
        });
    }
    let started = report.started();
    let skipped = report.entries.len() - started;
    lines.push(format!(
        "{started} chantier(s) pour {} écus (reste {}), {skipped} ignoré(s).",
        report.total_cost(),
        treasury - report.total_cost()
    ));
    lines
}

/// Text of the launch button.
pub fn launch_label(report: &BatchReport) -> String {
    format!(
        "Lancer ({} chantiers, {} écus)",
        report.started(),
        report.total_cost()
    )
}

/// Message after a launch: what was started and what was refused.
pub fn result_message(report: &BatchReport) -> (bool, String) {
    let started = report.started();
    let refused = report.entries.len() - started;
    let mut text = format!(
        "{started} chantier(s) lancé(s) pour {} écus.",
        report.total_cost()
    );
    if refused > 0 {
        text.push_str(&format!(" {refused} ignoré(s)."));
    }
    (started > 0, text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_campaign::buildings::BuildOption;
    use sim_campaign::realm_works::BatchEntry;

    fn option(id: &str, cost: u32, available: bool) -> BuildOption {
        BuildOption {
            building: BuildingId::new(id.to_owned()).unwrap(),
            name: id.to_owned(),
            cost,
            turns: 4,
            available,
            reason: (!available).then(|| "prérequis".to_owned()),
            import_cost: 0,
            imported: Default::default(),
        }
    }

    fn row(
        id: &str,
        kind: SettlementKind,
        used: usize,
        max: usize,
        opts: Vec<BuildOption>,
    ) -> WorkRow {
        WorkRow {
            settlement: SettlementId::new(id.to_owned()).unwrap(),
            name: id.to_owned(),
            province: data_model::ProvinceId::new("prov_a".to_owned()).unwrap(),
            province_name: "A".into(),
            kind,
            tier: 1,
            slots_used: used,
            slots_max: Some(max),
            queue_len: 0,
            queue_size: 2,
            options: opts,
        }
    }

    fn realm(rows: Vec<WorkRow>) -> RealmWorks {
        RealmWorks {
            treasury: 100,
            free_slots: 3,
            works_in_progress: 0,
            rows,
        }
    }

    #[test]
    fn cycle_walks_then_returns_to_none() {
        let all = vec!["a", "b"];
        assert_eq!(cycle(None, &all), Some("a"));
        assert_eq!(cycle(Some(&"a"), &all), Some("b"));
        assert_eq!(cycle(Some(&"b"), &all), None);
        assert_eq!(cycle(Some(&"z"), &all), Some("a"));
    }

    #[test]
    fn filters_free_province_and_kind() {
        let works = realm(vec![
            row("set_full", SettlementKind::Town, 3, 3, vec![]),
            row("set_free", SettlementKind::Castle, 1, 3, vec![]),
        ]);
        let mut ui = WorksUi::default();
        assert_eq!(visible(&works, &ui).len(), 1);
        ui.only_free = false;
        assert_eq!(visible(&works, &ui).len(), 2);
        ui.kind = Some(SettlementKind::Town);
        assert_eq!(visible(&works, &ui)[0].settlement.as_str(), "set_full");
        ui.kind = None;
        ui.province = Some("prov_other".into());
        assert!(visible(&works, &ui).is_empty());
    }

    #[test]
    fn entries_use_pick_then_global_when_available() {
        let works = realm(vec![
            row(
                "set_a",
                SettlementKind::Town,
                0,
                3,
                vec![option("bld_x", 10, true), option("bld_y", 20, true)],
            ),
            row(
                "set_b",
                SettlementKind::Town,
                0,
                3,
                vec![option("bld_x", 10, false)],
            ),
            row(
                "set_c",
                SettlementKind::Town,
                0,
                3,
                vec![option("bld_x", 10, true)],
            ),
        ]);
        let mut ui = WorksUi {
            global: Some("bld_x".into()),
            ..WorksUi::default()
        };
        ui.picks.insert("set_a".into(), "bld_y".into());
        let rows = visible(&works, &ui);
        let got: Vec<(String, String)> = entries(&rows, &ui)
            .into_iter()
            .map(|(s, b)| (s.to_string(), b.to_string()))
            .collect();
        assert_eq!(
            got,
            vec![
                ("set_a".to_owned(), "bld_y".to_owned()),
                ("set_c".to_owned(), "bld_x".to_owned())
            ]
        );
        assert_eq!(global_choices(&rows).len(), 2);
    }

    #[test]
    fn recap_names_refusals_and_totals() {
        let works = realm(vec![row(
            "set_a",
            SettlementKind::Town,
            0,
            3,
            vec![option("bld_x", 10, true)],
        )]);
        let rows: Vec<&WorkRow> = works.rows.iter().collect();
        let sid = |s: &str| SettlementId::new(s.to_owned()).unwrap();
        let bid = BuildingId::new("bld_x".to_owned()).unwrap();
        let report = BatchReport {
            entries: vec![
                BatchEntry {
                    settlement: sid("set_a"),
                    building: bid.clone(),
                    outcome: Ok(10),
                },
                BatchEntry {
                    settlement: sid("set_a"),
                    building: bid,
                    outcome: Err("fonds insuffisants".into()),
                },
            ],
        };
        let lines = recap_lines(&report, &rows, 100);
        assert!(lines[0].contains("10 écus"));
        assert!(lines[1].contains("ignoré (fonds insuffisants)"));
        assert!(lines[2].starts_with("1 chantier(s) pour 10 écus (reste 90), 1 ignoré"));
        assert_eq!(launch_label(&report), "Lancer (1 chantiers, 10 écus)");
    }
}
