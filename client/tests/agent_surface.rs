//! Pure parts of the agent surface: CLI, script, key names and repository detection.

use cent_ans_client::agent::cli::{Cli, DEFAULT_FRAMES};
use cent_ans_client::agent::key_code;
use cent_ans_client::agent::script::{ScriptCommand, parse_script};
use cent_ans_client::app::AppState;
use cent_ans_client::paths;

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn cli_defaults() {
    let cli = Cli::parse(&[]).unwrap();
    assert_eq!(cli.faction, "fac_france");
    assert_eq!(cli.seed, 1337);
    assert_eq!(cli.frames, DEFAULT_FRAMES);
    assert!(cli.state.is_none() && !cli.wants_offscreen());
}

#[test]
fn cli_flags() {
    let cli = Cli::parse(&args(&[
        "--state",
        "campaign",
        "--faction",
        "fac_england",
        "--seed",
        "7",
        "--frames",
        "5",
        "--exit-after",
        "100",
        "--shot",
        "/tmp/a.png",
        "--dump-state",
        "/tmp/a.json",
    ]))
    .unwrap();
    assert_eq!(cli.state, Some(AppState::Campaign));
    assert_eq!(
        (cli.faction.as_str(), cli.seed, cli.frames),
        ("fac_england", 7, 5)
    );
    assert_eq!(cli.exit_after, Some(100));
    assert!(cli.wants_offscreen(), "--shot implies offscreen");
}

#[test]
fn cli_rejects_garbage() {
    assert!(Cli::parse(&args(&["--state", "Nowhere"])).is_err());
    assert!(Cli::parse(&args(&["--seed"])).is_err());
    assert!(Cli::parse(&args(&["--bogus"])).is_err());
}

#[test]
fn script_parses_every_command() {
    let text = "# comment\n\nstate Campaign\nwait 30\nkey Escape\nclick Nouvelle partie\ndump /tmp/d.json\nshot /tmp/s.png\nexit\n";
    let commands = parse_script(text).unwrap();
    assert_eq!(commands.len(), 7);
    assert_eq!(commands[0], ScriptCommand::State(AppState::Campaign));
    assert_eq!(commands[1], ScriptCommand::Wait(30));
    assert_eq!(commands[3], ScriptCommand::Click("Nouvelle partie".into()));
    assert_eq!(commands[6], ScriptCommand::Exit);
}

#[test]
fn script_errors_name_the_line() {
    let error = parse_script("wait 1\nfly away\n").unwrap_err();
    assert!(error.contains("line 2"), "{error}");
    assert!(parse_script("wait x").is_err());
    assert!(parse_script("click").is_err());
}

#[test]
fn key_names() {
    assert!(key_code("Escape").is_some());
    assert!(key_code("keya").is_some());
    assert!(key_code("Nope").is_none());
}

#[test]
fn root_is_found_from_the_client_folder() {
    let root = paths::find_root_from(std::path::Path::new(env!("CARGO_MANIFEST_DIR")))
        .expect("client/ lives inside the repository");
    assert!(root.join("data").is_dir() && root.join("game/assets").is_dir());
    assert!(paths::asset_root(&root).ends_with("game/assets"));
}

#[test]
fn cli_bench_flags() {
    let cli = Cli::parse(&args(&[
        "--bench-map",
        "--bench-seconds",
        "5",
        "--bench-pan-only",
        "--bench-out",
        "/tmp/b.json",
    ]))
    .unwrap();
    let bench = cli.bench.unwrap();
    assert_eq!(bench.seconds, 5.0);
    assert!(bench.pan_only && !bench.descent_only);
    assert_eq!(bench.out.unwrap().to_str(), Some("/tmp/b.json"));
    assert!(Cli::parse(&[]).unwrap().bench.is_none());
    assert!(Cli::parse(&args(&["--bench-seconds", "x"])).is_err());
}
