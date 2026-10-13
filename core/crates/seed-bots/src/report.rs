//! Reports: `report.json` and a French `report.md`, findings deduplicated.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::Path;

use serde::Serialize;

use crate::oracles::{Finding, OracleKind, Severity};
use crate::runner::RunResult;

/// Message with every number replaced by `#`, so "tour 12, 3 unités" and "tour 40, 5 unités" match.
pub fn signature(message: &str) -> String {
    let mut out = String::with_capacity(message.len());
    let mut chars = message.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_ascii_digit() {
            while let Some(&next) = chars.peek() {
                let decimal = next == '.' || next == ',';
                if next.is_ascii_digit() {
                    chars.next();
                } else if decimal {
                    // Keep a trailing sentence dot: only swallow it when a digit follows.
                    let mut ahead = chars.clone();
                    ahead.next();
                    if ahead.peek().is_some_and(char::is_ascii_digit) {
                        chars.next();
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            out.push('#');
        } else {
            out.push(c);
        }
    }
    out
}

/// Groups findings by (oracle, message signature); keeps the first of each with its count,
/// in order of first appearance.
pub fn dedup(findings: &[Finding]) -> Vec<(Finding, u32)> {
    let mut index: HashMap<(OracleKind, String), usize> = HashMap::new();
    let mut groups: Vec<(Finding, u32)> = Vec::new();
    for finding in findings {
        let key = (finding.oracle, signature(&finding.message));
        if let Some(&position) = index.get(&key) {
            groups[position].1 += 1;
        } else {
            index.insert(key, groups.len());
            groups.push((finding.clone(), 1));
        }
    }
    groups
}

#[derive(Serialize)]
struct JsonGroup<'a> {
    count: u32,
    file: String,
    finding: &'a Finding,
}

#[derive(Serialize)]
struct JsonReport<'a> {
    games: u32,
    partial: bool,
    total_findings: usize,
    unique_findings: usize,
    groups: Vec<JsonGroup<'a>>,
}

fn oracle_label(oracle: OracleKind) -> &'static str {
    match oracle {
        OracleKind::Panic => "Panique",
        OracleKind::SlowTurn => "Tour lent",
        OracleKind::SaveRoundtrip => "Aller-retour de sauvegarde",
        OracleKind::Invariant => "Invariant de règles",
        OracleKind::RefusedLegalOrder => "Ordre légal refusé",
        OracleKind::BalanceDrift => "Dérive d'équilibre",
    }
}

fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Blocking => "bloquant",
        Severity::Warning => "avertissement",
    }
}

const ORACLES: [OracleKind; 6] = [
    OracleKind::Panic,
    OracleKind::SlowTurn,
    OracleKind::SaveRoundtrip,
    OracleKind::Invariant,
    OracleKind::RefusedLegalOrder,
    OracleKind::BalanceDrift,
];

/// Writes `report.json`, `report.md` and `findings/<n>.json` into `dir`.
///
/// `report.json` holds no wall time, so two runs of the same seeds give identical files.
pub fn write(result: &RunResult, dir: &Path) -> std::io::Result<()> {
    let findings_dir = dir.join("findings");
    std::fs::create_dir_all(&findings_dir)?;
    let groups = dedup(&result.findings);

    let mut json_groups = Vec::new();
    let mut md = String::new();
    let _ = writeln!(md, "# Rapport seed bots\n");
    let _ = writeln!(md, "- Parties jouées : {}", result.games);
    let _ = writeln!(md, "- Durée : {:.1} s", result.seconds);
    let _ = writeln!(
        md,
        "- Passe : {}",
        if result.partial {
            "partielle (plafond de durée atteint)"
        } else {
            "complète"
        }
    );
    let _ = writeln!(
        md,
        "- Constats : {} ({} distincts)\n",
        result.findings.len(),
        groups.len()
    );

    let _ = writeln!(md, "## Par oracle\n");
    let _ = writeln!(md, "| Oracle | Bloquants | Avertissements |");
    let _ = writeln!(md, "|---|---:|---:|");
    for oracle in ORACLES {
        let count = |severity| {
            result
                .findings
                .iter()
                .filter(|f| f.oracle == oracle && f.severity == severity)
                .count()
        };
        let _ = writeln!(
            md,
            "| {} | {} | {} |",
            oracle_label(oracle),
            count(Severity::Blocking),
            count(Severity::Warning)
        );
    }

    let _ = writeln!(md, "\n## Constats\n");
    if groups.is_empty() {
        let _ = writeln!(md, "Aucun constat.");
    }
    for (number, (finding, count)) in groups.iter().enumerate() {
        let file = format!("findings/{number}.json");
        std::fs::write(
            dir.join(&file),
            serde_json::to_string_pretty(finding).map_err(std::io::Error::other)?,
        )?;
        let _ = writeln!(
            md,
            "### {number}. {} ({})\n",
            oracle_label(finding.oracle),
            severity_label(finding.severity)
        );
        let _ = writeln!(md, "{}\n", finding.message);
        let _ = writeln!(md, "- Occurrences : {count}");
        let _ = writeln!(
            md,
            "- Premier : graine {}, bot {}, faction {}, tour {}",
            finding.seed, finding.bot, finding.faction, finding.turn
        );
        let _ = writeln!(
            md,
            "- Rejeu : `seed-bots replay {}`\n",
            dir.join(&file).display()
        );
        json_groups.push((file, finding, *count));
    }

    let report = JsonReport {
        games: result.games,
        partial: result.partial,
        total_findings: result.findings.len(),
        unique_findings: groups.len(),
        groups: json_groups
            .into_iter()
            .map(|(file, finding, count)| JsonGroup {
                count,
                file,
                finding,
            })
            .collect(),
    };
    std::fs::write(
        dir.join("report.json"),
        serde_json::to_string_pretty(&report).map_err(std::io::Error::other)?,
    )?;
    std::fs::write(dir.join("report.md"), md)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(oracle: OracleKind, message: &str, turn: u32) -> Finding {
        Finding {
            oracle,
            severity: Severity::Blocking,
            seed: 1,
            bot: "chaos".into(),
            faction: "fac_france".into(),
            turn,
            message: message.into(),
            order_log: vec![],
        }
    }

    #[test]
    fn signature_replaces_numbers() {
        assert_eq!(
            signature("tour 12, or -3.5 sur 100."),
            "tour #, or -# sur #."
        );
    }

    #[test]
    fn dedup_groups_by_oracle_and_signature() {
        let findings = vec![
            finding(OracleKind::Invariant, "or négatif : 12", 3),
            finding(OracleKind::Panic, "or négatif : 7", 4),
            finding(OracleKind::Invariant, "or négatif : 99", 9),
            finding(OracleKind::Invariant, "autre", 10),
        ];
        let groups = dedup(&findings);
        assert_eq!(groups.len(), 3);
        assert_eq!((groups[0].0.turn, groups[0].1), (3, 2));
        assert_eq!(groups[1].1, 1);
        assert_eq!(groups[2].0.message, "autre");
    }

    #[test]
    fn write_produces_files() {
        let dir = std::env::temp_dir().join(format!("seed-bots-report-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let result = RunResult {
            findings: vec![
                finding(OracleKind::Invariant, "or négatif : 12", 3),
                finding(OracleKind::Invariant, "or négatif : 13", 5),
            ],
            partial: true,
            games: 8,
            seconds: 1.5,
        };
        write(&result, &dir).unwrap();
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("report.json")).unwrap())
                .unwrap();
        assert_eq!(json["unique_findings"], 1);
        assert_eq!(json["groups"][0]["count"], 2);
        let md = std::fs::read_to_string(dir.join("report.md")).unwrap();
        assert!(md.contains("partielle") && md.contains("Occurrences : 2"));
        assert!(md.contains("seed-bots replay"));
        let saved: Finding =
            serde_json::from_str(&std::fs::read_to_string(dir.join("findings/0.json")).unwrap())
                .unwrap();
        assert_eq!(saved.turn, 3);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
