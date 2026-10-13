//! Bevy side of the engine measurement prototype (P0): renders `scene.json`, writes metrics,
//! screenshots and a text dump of the world so agents can inspect it without captures.

mod scene;

use std::process::ExitCode;

/// Command-line options shared with the Godot client.
#[derive(Debug, Default)]
struct Options {
    scene: Option<String>,
    metrics: Option<String>,
    shot: Option<String>,
    shot_at_s: Option<f32>,
    dump_state: Option<String>,
    overrides: Vec<(String, String)>,
}

fn parse_options(args: &[String]) -> Result<Options, String> {
    let mut options = Options::default();
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        let mut value = || {
            index += 1;
            args.get(index).cloned().ok_or(format!("{flag} expects a value"))
        };
        match flag {
            "--scene" => options.scene = Some(value()?),
            "--metrics" => options.metrics = Some(value()?),
            "--shot" => options.shot = Some(value()?),
            "--at" => options.shot_at_s = Some(value()?.parse().map_err(|e| format!("--at: {e}"))?),
            "--dump-state" => options.dump_state = Some(value()?),
            "--set" => {
                let pair = value()?;
                let (key, val) = pair.split_once('=').ok_or("--set expects key=value")?;
                options.overrides.push((key.to_owned(), val.to_owned()));
            }
            other => return Err(format!("unknown option {other}")),
        }
        index += 1;
    }
    Ok(options)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse_options(&args) {
        Ok(options) => {
            println!("proto-bevy skeleton: {options:?}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_set_overrides() {
        let args: Vec<String> = ["--set", "vegetation.grass=0.5"].map(String::from).to_vec();
        let options = parse_options(&args).unwrap();
        assert_eq!(options.overrides, vec![("vegetation.grass".into(), "0.5".into())]);
    }
}
