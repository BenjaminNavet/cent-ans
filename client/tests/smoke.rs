//! Smoke tests of the whole client: `tests/smoke.sh` runs every scenario of `tests/smoke/` in the
//! background window of `run_bg.sh`. Slow (~4 min) and it opens a window, so it is `#[ignore]`d:
//! `cargo test --features dev --test smoke -- --ignored`.

use std::path::Path;
use std::process::Command;

#[test]
#[ignore = "opens a background window, about 4 minutes"]
fn client_smoke_scenarios_pass() {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/smoke.sh");
    let status = Command::new("/bin/zsh")
        .arg(script)
        .status()
        .expect("zsh runs smoke.sh");
    assert!(status.success(), "see out/smoke/logs/*.log");
}

#[test]
fn every_scenario_script_parses_and_exits() {
    use cent_ans_client::agent::script::{ScriptCommand, parse_script};
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/smoke");
    let mut count = 0;
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let text = std::fs::read_to_string(&path).unwrap();
        let commands = parse_script(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(
            commands.last(),
            Some(&ScriptCommand::Exit),
            "{} must end with `exit`",
            path.display()
        );
        count += 1;
    }
    assert!(count >= 9, "scenarios: {count}");
}
