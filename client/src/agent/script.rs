//! Agent script: one command per line, `#` starts a comment. Pure parsing, tested in `client/tests/`.
//!
//! ```text
//! state <Name>        change screen (Boot, MainMenu, FactionSelect, Campaign, Battle, Codex)
//! wait <frames>       let the app run
//! key <KeyCode>       press and release a key (Escape, Enter, Space, KeyA, Digit1, ArrowUp, F1...)
//! type <text>        type the text (rest of the line, one character per frame) into the focused text field
//! click <Name>        press the UI node whose `Name` is <Name> (rest of the line, spaces allowed)
//! click_if <Name>     like `click`, but a missing node is skipped (after 10 frames of grace), never fatal
//! mouse <x> <y>      move the pointer to the window position (logical pixels, origin top-left)
//! mouse_down <left|right|middle>   press a mouse button (stays down until `mouse_up`)
//! mouse_up <left|right|middle>     release it
//! mouse_click <x> <y> [button]     move, press, release (left by default)
//! mouse_drag <button> <x1> <y1> <x2> <y2> [steps]   move, press, move in `steps` (8) frames, release
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
    /// Moves the pointer (logical window pixels).
    MouseMove(f32, f32),
    /// Presses (`true`) or releases a mouse button.
    MouseButton(MouseButtonName, bool),
    Dump(PathBuf),
    Shot(PathBuf),
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButtonName {
    Left,
    Right,
    Middle,
}

fn button_name(text: &str) -> Result<MouseButtonName, String> {
    match text {
        "left" => Ok(MouseButtonName::Left),
        "right" => Ok(MouseButtonName::Right),
        "middle" => Ok(MouseButtonName::Middle),
        other => Err(format!("unknown mouse button {other}")),
    }
}

fn numbers(parts: &[&str], from: usize, count: usize) -> Result<Vec<f32>, String> {
    parts
        .get(from..from + count)
        .ok_or_else(|| format!("expects {count} numbers"))?
        .iter()
        .map(|p| p.parse::<f32>().map_err(|e| format!("{p}: {e}")))
        .collect()
}

/// `mouse*` verbs, expanded into the primitive commands (with a frame between each so the game
/// sees the move, the press and the release on separate frames like a real pointer).
fn parse_mouse(verb: &str, rest: &str) -> Result<Vec<ScriptCommand>, String> {
    use ScriptCommand::{MouseButton as Button, MouseMove, Wait};
    let parts: Vec<&str> = rest.split_whitespace().collect();
    match verb {
        "mouse" => {
            let n = numbers(&parts, 0, 2)?;
            Ok(vec![MouseMove(n[0], n[1]), Wait(2)])
        }
        "mouse_down" | "mouse_up" => {
            let button = button_name(parts.first().copied().unwrap_or("left"))?;
            Ok(vec![Button(button, verb == "mouse_down"), Wait(2)])
        }
        "mouse_click" => {
            let n = numbers(&parts, 0, 2)?;
            let button = button_name(parts.get(2).copied().unwrap_or("left"))?;
            Ok(vec![
                MouseMove(n[0], n[1]),
                Wait(2),
                Button(button, true),
                Wait(2),
                Button(button, false),
                Wait(2),
            ])
        }
        "mouse_drag" => {
            let button = button_name(parts.first().copied().ok_or("expects a button")?)?;
            let n = numbers(&parts, 1, 4)?;
            let steps: u32 = parts
                .get(5)
                .map_or(Ok(8), |s| s.parse().map_err(|e| format!("{s}: {e}")))?;
            let mut out = vec![
                MouseMove(n[0], n[1]),
                Wait(2),
                Button(button, true),
                Wait(2),
            ];
            for i in 1..=steps.max(1) {
                let t = i as f32 / steps.max(1) as f32;
                out.push(MouseMove(
                    n[0] + (n[2] - n[0]) * t,
                    n[1] + (n[3] - n[1]) * t,
                ));
                out.push(Wait(1));
            }
            out.extend([Wait(2), Button(button, false), Wait(2)]);
            Ok(out)
        }
        _ => Err(format!("unknown command {verb}")),
    }
}

/// Parses a whole script; the error names the line.
pub fn parse_script(text: &str) -> Result<Vec<ScriptCommand>, String> {
    let mut commands = Vec::new();
    for (number, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let wrap = |e: String| format!("script line {}: {e}", number + 1);
        if line.starts_with("mouse") {
            let (verb, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
            commands.extend(parse_mouse(verb, rest.trim()).map_err(wrap)?);
        } else {
            commands.push(parse_line(line).map_err(wrap)?);
        }
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

#[cfg(test)]
mod mouse_tests {
    use super::*;

    #[test]
    fn a_drag_is_expanded_into_moves_presses_and_waits() {
        let commands = parse_script("mouse_drag right 10 20 110 20 4").unwrap();
        assert_eq!(commands[0], ScriptCommand::MouseMove(10.0, 20.0));
        assert!(commands.contains(&ScriptCommand::MouseButton(MouseButtonName::Right, true)));
        assert!(commands.contains(&ScriptCommand::MouseMove(110.0, 20.0)));
        assert_eq!(
            commands[commands.len() - 2],
            ScriptCommand::MouseButton(MouseButtonName::Right, false)
        );
    }

    #[test]
    fn bad_mouse_lines_name_the_line() {
        let error = parse_script("wait 1\nmouse 5").unwrap_err();
        assert!(error.contains("line 2"), "{error}");
        assert!(parse_script("mouse_down fourth").is_err());
    }
}
