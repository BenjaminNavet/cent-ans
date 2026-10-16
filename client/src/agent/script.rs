//! Agent script: one command per line, `#` starts a comment. Pure parsing, tested in `client/tests/`.
//!
//! ```text
//! state <Name>        change screen (Boot, MainMenu, FactionSelect, Campaign, Battle, Codex)
//! wait <frames>       let the app run
//! key <KeyCode>       press and release a key (Escape, Enter, Space, KeyA, Digit1, ArrowUp, F1...)
//! type <text>        type the text (rest of the line, one character per frame) into the focused text field
//! click <Name>        press the UI node whose `Name` is <Name> (rest of the line, spaces allowed)
//! click_if <Name>     like `click`, but a missing node is skipped (after 10 frames of grace), never fatal
//! dump <file>         write the --dump-state JSON now
//! shot <file>         save a screenshot
//! exit                wait for pending screenshots, then quit
//! ```

use std::path::PathBuf;

use crate::agent::cli::absolute;
use crate::app::AppState;

#[derive(Debug, Clone, PartialEq)]
pub enum ScriptCommand {
    State(AppState),
    Wait(u32),
    Key(String),
    Click(String),
    /// `click` that gives up quietly when the node does not exist.
    ClickIf(String),
    Type(String),
    Dump(PathBuf),
    Shot(PathBuf),
    Exit,
}

/// Parses a whole script; the error names the line.
pub fn parse_script(text: &str) -> Result<Vec<ScriptCommand>, String> {
    let mut commands = Vec::new();
    for (number, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let command = parse_line(line).map_err(|e| format!("script line {}: {e}", number + 1))?;
        commands.push(command);
    }
    Ok(commands)
}

fn parse_line(line: &str) -> Result<ScriptCommand, String> {
    let (verb, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
    let rest = rest.trim();
    let need = |what: &str| {
        if rest.is_empty() {
            Err(format!("`{verb}` expects {what}"))
        } else {
            Ok(rest)
        }
    };
    match verb {
        "state" => AppState::from_name(need("a state")?)
            .map(ScriptCommand::State)
            .ok_or(format!("unknown state {rest}")),
        "wait" => need("a frame count")?
            .parse()
            .map(ScriptCommand::Wait)
            .map_err(|e| format!("wait: {e}")),
        "key" => Ok(ScriptCommand::Key(need("a key code")?.to_owned())),
        "click" => Ok(ScriptCommand::Click(need("an entity name")?.to_owned())),
        "click_if" | "try_click" => Ok(ScriptCommand::ClickIf(need("an entity name")?.to_owned())),
        "type" => Ok(ScriptCommand::Type(need("a text")?.to_owned())),
        "dump" => Ok(ScriptCommand::Dump(absolute(need("a file")?))),
        "shot" => Ok(ScriptCommand::Shot(absolute(need("a file")?))),
        "exit" => Ok(ScriptCommand::Exit),
        other => Err(format!("unknown command {other}")),
    }
}
