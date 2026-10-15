//! Snapshot tests (insta) of the stable parts of the agent surface: the `--dump-state` JSON built
//! from the built-in sections, the parsed smoke scenarios and a parsed command line. Workflow:
//! `cargo insta review` (or `INSTA_UPDATE=always cargo test --test snapshots`), then commit the
//! accepted `tests/snapshots/*.snap` files. Volatile values (frame counters, timings, absolute
//! paths) are redacted with `insta::dynamic_redaction` / `[redactions]`.

use std::collections::BTreeMap;
use std::path::Path;

use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use cent_ans_client::agent::cli::Cli;
use cent_ans_client::agent::dump::{DumpAppExt, build_dump, dump_entities, dump_state};
use cent_ans_client::agent::script::parse_script;
use cent_ans_client::app::AppState;

/// Runs `body` with the absolute path of the checkout replaced by `[repo]` in the snapshot (the
/// worktrees live at different paths).
fn with_repo_path_redacted(body: impl FnOnce()) {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .display()
        .to_string();
    // Paths here hold no regex metacharacter but `.`.
    // Filters take `&str`: leaking a short string in a test is fine.
    let pattern: &'static str = Box::leak(repo.replace('.', "\\.").into_boxed_str());
    insta::with_settings!({filters => vec![(pattern, "[repo]")]}, { body() });
}

/// A frame counter that changes with every run: it must never reach a snapshot as a number.
fn dump_frame_counter(_world: &World) -> (String, serde_json::Value) {
    ("frame".to_owned(), serde_json::json!(123_456))
}

#[test]
fn agent_dump_builtin_sections() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin))
        .init_state::<AppState>()
        .register_dump(dump_state)
        .register_dump(dump_frame_counter)
        .register_dump(dump_entities);
    app.update();
    let mut dump = build_dump(app.world());
    // Entity counts depend on Bevy internals: keep only that it is a positive number.
    let entities = dump["entities"].as_u64().expect("entity count");
    assert!(entities > 0);
    dump["entities"] = serde_json::json!("<count>");
    insta::assert_json_snapshot!(dump, {".frame" => "[frame]"});
}

#[test]
fn smoke_scenarios_parse_to_stable_command_lists() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/smoke");
    let mut scenarios = BTreeMap::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        let commands = parse_script(&std::fs::read_to_string(&path).unwrap()).unwrap();
        // The commands, one short line each; `wait` lengths are kept (they are part of the script).
        let lines: Vec<String> = commands.iter().map(|c| format!("{c:?}")).collect();
        scenarios.insert(name, lines);
    }
    with_repo_path_redacted(|| insta::assert_yaml_snapshot!(scenarios));
}

#[test]
fn cli_of_a_typical_agent_run() {
    let args: Vec<String> = [
        "--state",
        "Campaign",
        "--faction",
        "fac_england",
        "--seed",
        "7",
        "--frames",
        "5",
        "--dump-state",
        "out/run.json",
        "--offscreen",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect();
    let cli = Cli::parse(&args).unwrap();
    with_repo_path_redacted(|| insta::assert_debug_snapshot!(cli));
}
