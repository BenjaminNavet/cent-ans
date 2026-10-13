//! `seed-bots`: run the seeded bots, or replay a finding.
//!
//! `seed-bots run [--quick] [--seeds a..b] [--turns n] [--bots a,b] [--max-minutes m] [--balance] [--out dir]`
//! `seed-bots replay <finding.json>`

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use seed_bots::oracles::{Finding, Severity};
use seed_bots::runner::{self, RunConfig};
use seed_bots::{bots, report};

const USAGE: &str = "seed-bots run [--quick] [--seeds a..b] [--turns n] [--bots a,b] [--max-minutes m] [--balance] [--out dir]\nseed-bots replay <finding.json>";

/// A parsed command line.
#[derive(Debug)]
enum Command {
    Run {
        config: RunConfig,
        out: Option<PathBuf>,
    },
    Replay(PathBuf),
}

fn parse_seeds(text: &str) -> Result<Vec<u64>, String> {
    let bad = || format!("graines invalides : {text}");
    match text.split_once("..") {
        Some((a, b)) => {
            let (a, b): (u64, u64) = (a.parse().map_err(|_| bad())?, b.parse().map_err(|_| bad())?);
            if a > b {
                return Err(bad());
            }
            Ok((a..=b).collect())
        }
        None => Ok(vec![text.parse().map_err(|_| bad())?]),
    }
}

fn parse_args(args: &[String], data_root: &Path) -> Result<Command, String> {
    match args.first().map(String::as_str) {
        Some("replay") => match args {
            [_, path] => Ok(Command::Replay(PathBuf::from(path))),
            _ => Err("replay attend un fichier de constat".into()),
        },
        Some("run") => {
            let (mut quick, mut balance) = (false, false);
            let (mut seeds, mut turns, mut bot_names, mut max_minutes, mut out) =
                (None, None, None, None, None);
            let mut iter = args[1..].iter();
            while let Some(flag) = iter.next() {
                let mut value = || {
                    iter.next()
                        .ok_or_else(|| format!("{flag} attend une valeur"))
                };
                match flag.as_str() {
                    "--quick" => quick = true,
                    "--balance" => balance = true,
                    "--seeds" => seeds = Some(parse_seeds(value()?)?),
                    "--turns" => {
                        turns = Some(value()?.parse::<u32>().map_err(|_| "--turns invalide")?)
                    }
                    "--bots" => {
                        bot_names =
                            Some(value()?.split(',').map(str::to_string).collect::<Vec<_>>())
                    }
                    "--max-minutes" => {
                        max_minutes = Some(
                            value()?
                                .parse::<u32>()
                                .map_err(|_| "--max-minutes invalide")?,
                        )
                    }
                    "--out" => out = Some(PathBuf::from(value()?)),
                    other => return Err(format!("option inconnue : {other}")),
                }
            }
            let bot_names: Vec<String> =
                bot_names.unwrap_or_else(|| bots::ALL.iter().map(|b| b.to_string()).collect());
            if let Some(unknown) = bot_names.iter().find(|b| !bots::ALL.contains(&b.as_str())) {
                return Err(format!("bot inconnu : {unknown}"));
            }
            let config = RunConfig {
                seeds: seeds.unwrap_or_else(|| if quick { vec![1] } else { (1..=4).collect() }),
                turns: turns.unwrap_or(if quick { 20 } else { 150 }),
                bots: bot_names,
                max_minutes: max_minutes.unwrap_or(if quick { 1 } else { 15 }),
                slow_turn_ms: 5000,
                balance: balance || !quick,
                data_root: data_root.to_path_buf(),
            };
            Ok(Command::Run { config, out })
        }
        _ => Err("commande attendue : run ou replay".into()),
    }
}

/// `AAAA-MM-JJ_HHMMSS` in UTC from a Unix timestamp (civil-from-days, Hinnant).
fn timestamp(unix_seconds: u64) -> String {
    let days = (unix_seconds / 86_400) as i64;
    let rest = unix_seconds % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let mp = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}_{:02}{:02}{:02}",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

fn print_findings(findings: &[Finding]) {
    for (finding, count) in report::dedup(findings) {
        println!(
            "[{:?}/{:?}] x{count} graine {} bot {} faction {} tour {} : {}",
            finding.oracle,
            finding.severity,
            finding.seed,
            finding.bot,
            finding.faction,
            finding.turn,
            finding.message
        );
    }
}

fn exit_code(findings: &[Finding]) -> ExitCode {
    if findings.iter().any(|f| f.severity == Severity::Blocking) {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn main() -> ExitCode {
    let data_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = match parse_args(&args, &data_root) {
        Ok(command) => command,
        Err(message) => {
            eprintln!("{message}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    match command {
        Command::Replay(path) => {
            let finding: Finding = match std::fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|text| serde_json::from_str(&text).map_err(|e| e.to_string()))
            {
                Ok(finding) => finding,
                Err(message) => {
                    eprintln!("{}: {message}", path.display());
                    return ExitCode::from(2);
                }
            };
            let findings = runner::replay(&finding, &data_root);
            print_findings(&findings);
            exit_code(&findings)
        }
        Command::Run { config, out } => {
            let result = runner::run(&config);
            let out = out.unwrap_or_else(|| {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs());
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../target/seed-bots")
                    .join(timestamp(now))
            });
            if let Err(error) = report::write(&result, &out) {
                eprintln!("{}: {error}", out.display());
                return ExitCode::from(2);
            }
            println!("Rapport : {}", out.join("report.md").display());
            print_findings(&result.findings);
            exit_code(&result.findings)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Command, String> {
        let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
        parse_args(&args, Path::new("data"))
    }

    fn config(args: &[&str]) -> RunConfig {
        match parse(args).unwrap() {
            Command::Run { config, .. } => config,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn quick_defaults() {
        let c = config(&["run", "--quick"]);
        assert_eq!(
            (c.seeds.clone(), c.turns, c.max_minutes, c.balance),
            (vec![1], 20, 1, false)
        );
        assert_eq!(c.bots.len(), 8);
    }

    #[test]
    fn long_defaults_and_overrides() {
        let c = config(&["run"]);
        assert_eq!(
            (c.seeds.len(), c.turns, c.max_minutes, c.balance),
            (4, 150, 15, true)
        );
        let c = config(&[
            "run",
            "--seeds",
            "3..5",
            "--turns",
            "20",
            "--bots",
            "chaos,turtle",
        ]);
        assert_eq!(
            (c.seeds, c.turns, c.bots),
            (vec![3, 4, 5], 20, vec!["chaos".into(), "turtle".into()])
        );
    }

    #[test]
    fn errors_and_replay() {
        assert!(parse(&["run", "--bots", "nope"]).is_err());
        assert!(parse(&["run", "--turns"]).is_err());
        assert!(parse(&["run", "--seeds", "5..2"]).is_err());
        assert!(parse(&["wat"]).is_err());
        assert!(
            matches!(parse(&["replay", "f.json"]), Ok(Command::Replay(p)) if p == std::path::Path::new("f.json"))
        );
    }

    #[test]
    fn timestamp_is_utc_civil() {
        assert_eq!(timestamp(0), "1970-01-01_000000");
        assert_eq!(timestamp(1_791_849_600 + 3_661), "2026-10-13_010101");
    }
}
