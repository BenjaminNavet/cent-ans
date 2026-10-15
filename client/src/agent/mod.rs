//! Agent control surface (ADR 0357 § 6): `--state`, `--dump-state`, `--shot`, `--script`,
//! `--exit-after`, offscreen rendering, and the Bevy Remote Protocol behind feature `remote`.
//!
//! Replaces game/scripts/agent_controller.gd and the debug hooks of game/scripts/debug/.
//! Scripts and CLI shot/dump share one driver, `drive`, an exclusive system run every frame.

pub mod bench;
pub mod cli;
pub mod devtools;
pub mod dump;
pub mod profile;
pub mod script;

use std::collections::VecDeque;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use serde_json::json;

use crate::app::AppState;
use crate::ui::kit::ButtonPressed;
use cli::Cli;
use dump::{DumpAppExt, build_dump, dump_entities, dump_state};
use script::{ScriptCommand, parse_script};

/// Size of the offscreen render target (logical pixels = window size).
pub const VIEW_SIZE: (u32, u32) = (1280, 800);
/// Frames a `click` waits for its target to exist before failing the run.
const CLICK_RETRY_FRAMES: u32 = 300;

/// Image the cameras render to under `--offscreen` / `--shot`. Cameras of every screen must use
/// `RenderTarget::Image(target.0.clone().into())` when this resource exists.
#[derive(Resource, Clone)]
pub struct OffscreenTarget(pub Handle<Image>);

#[derive(Resource)]
struct AgentRun {
    queue: VecDeque<ScriptCommand>,
    wait: u32,
    frame: u32,
    exit_after: Option<u32>,
    release: Vec<KeyCode>,
    click_retries: u32,
    pending_shots: Arc<AtomicUsize>,
}

pub struct AgentPlugin {
    pub cli: Cli,
}

impl Plugin for AgentPlugin {
    fn build(&self, app: &mut App) {
        let cli = &self.cli;
        install_backtrace_hook();
        let mut queue = VecDeque::new();
        if let Some(path) = &cli.script {
            match std::fs::read_to_string(path)
                .map_err(|e| format!("{}: {e}", path.display()))
                .and_then(|text| parse_script(&text))
            {
                Ok(commands) => queue.extend(commands),
                Err(message) => {
                    eprintln!("AGENT_FATAL: {message}");
                    std::process::exit(2);
                }
            }
        }
        if cli.dump_state.is_some() || cli.shot.is_some() {
            queue.push_back(ScriptCommand::Wait(cli.frames));
            if let Some(path) = &cli.dump_state {
                queue.push_back(ScriptCommand::Dump(path.clone()));
            }
            if let Some(path) = &cli.shot {
                queue.push_back(ScriptCommand::Shot(path.clone()));
            }
            queue.push_back(ScriptCommand::Exit);
        }
        if cli.wants_offscreen() {
            let mut images = app.world_mut().resource_mut::<Assets<Image>>();
            let target = images.add(Image::new_target_texture(
                VIEW_SIZE.0,
                VIEW_SIZE.1,
                TextureFormat::Rgba8UnormSrgb,
                None,
            ));
            app.insert_resource(OffscreenTarget(target));
        }
        app.insert_resource(AgentRun {
            queue,
            wait: 0,
            frame: 0,
            exit_after: cli.exit_after,
            release: Vec::new(),
            click_retries: 0,
            pending_shots: Arc::new(AtomicUsize::new(0)),
        })
        .register_dump(dump_state)
        .register_dump(dump_entities)
        .register_dump(dump_frame)
        .add_systems(Last, drive);
        bench::add(app, cli);
        // With `dev`, BrpExtrasPlugin (devtools) installs the same transport with a chosen port.
        #[cfg(all(feature = "remote", not(feature = "dev")))]
        app.add_plugins((
            bevy::remote::RemotePlugin::default(),
            bevy::remote::http::RemoteHttpPlugin::default(),
        ));
        #[cfg(feature = "dev")]
        app.add_plugins(devtools::DevToolsPlugin);
    }
}

/// `CENT_ANS_BACKTRACE=1` (forwarded by `run_bg.sh`, which cannot forward `RUST_BACKTRACE`): a
/// panic prints a full backtrace after its message, so a command applied to a despawned entity
/// can be traced to its bundle type.
fn install_backtrace_hook() {
    if std::env::var_os("CENT_ANS_BACKTRACE").is_none() {
        return;
    }
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        previous(info);
        eprintln!(
            "AGENT_BACKTRACE\n{}",
            std::backtrace::Backtrace::force_capture()
        );
    }));
}

fn dump_frame(world: &World) -> (String, serde_json::Value) {
    let frame = world.get_resource::<AgentRun>().map_or(0, |run| run.frame);
    ("frame".to_owned(), json!(frame))
}

fn fatal(world: &mut World, message: &str) {
    error!("AGENT_FATAL: {message}");
    world.write_message(AppExit::error());
    world.resource_mut::<AgentRun>().queue.clear();
}

/// Sends a keyboard event as if it came from the window: the input pass of the next frame turns
/// it into `pressed` + `just_pressed` (pressing `ButtonInput` directly in `Last` would see its
/// `just_pressed` cleared before any `Update` system ran).
fn send_key(world: &mut World, key_code: KeyCode, state: ButtonState) {
    let mut windows = world.query_filtered::<Entity, With<bevy::window::PrimaryWindow>>();
    let window = windows.iter(world).next().unwrap_or(Entity::PLACEHOLDER);
    world.write_message(KeyboardInput {
        key_code,
        logical_key: Key::Unidentified(NativeKey::Unidentified),
        state,
        text: None,
        repeat: false,
        window,
    });
}

/// Runs the next script commands of this frame.
fn drive(world: &mut World) {
    let frame = {
        let mut run = world.resource_mut::<AgentRun>();
        run.frame += 1;
        if let Some(limit) = run.exit_after
            && run.frame >= limit
        {
            run.exit_after = None;
            // Marker read by `tests/smoke.sh`: the safety net fired, the script did not finish.
            info!("AGENT_EXIT exit-after frame {}", run.frame);
            world.write_message(AppExit::Success);
            return;
        }
        run.frame
    };
    let release = std::mem::take(&mut world.resource_mut::<AgentRun>().release);
    for key in release {
        send_key(world, key, ButtonState::Released);
    }
    {
        let mut run = world.resource_mut::<AgentRun>();
        if run.wait > 0 {
            run.wait -= 1;
            return;
        }
    }
    loop {
        let Some(command) = world.resource_mut::<AgentRun>().queue.pop_front() else {
            return;
        };
        match command {
            ScriptCommand::Wait(frames) => {
                world.resource_mut::<AgentRun>().wait = frames;
                return;
            }
            ScriptCommand::State(state) => {
                world.resource_mut::<NextState<AppState>>().set(state);
                // One frame for the transition, one for the screen's OnEnter commands.
                world.resource_mut::<AgentRun>().wait = 2;
                return;
            }
            ScriptCommand::Key(name) => {
                let Some(key) = key_code(&name) else {
                    fatal(world, &format!("unknown key {name}"));
                    return;
                };
                send_key(world, key, ButtonState::Pressed);
                world.resource_mut::<AgentRun>().release.push(key);
                return;
            }
            ScriptCommand::Click(name) => {
                let mut query = world.query::<(Entity, &Name)>();
                let target = query
                    .iter(world)
                    .find(|(_, entity_name)| entity_name.as_str() == name)
                    .map(|(entity, _)| entity);
                match target {
                    Some(entity) => {
                        world.resource_mut::<AgentRun>().click_retries = 0;
                        world.write_message(ButtonPressed { entity, name });
                        // Let the consumer react before the next command.
                        world.resource_mut::<AgentRun>().wait = 2;
                    }
                    None => {
                        let retries = {
                            let mut run = world.resource_mut::<AgentRun>();
                            run.click_retries += 1;
                            run.queue.push_front(ScriptCommand::Click(name.clone()));
                            run.click_retries
                        };
                        if retries > CLICK_RETRY_FRAMES {
                            fatal(world, &format!("click: no entity named `{name}`"));
                        }
                    }
                }
                return;
            }
            ScriptCommand::Dump(path) => {
                let text = serde_json::to_string_pretty(&build_dump(world)).unwrap_or_default();
                if let Err(message) = write_file(&path, text.as_bytes()) {
                    fatal(world, &message);
                    return;
                }
                info!("dump written to {} (frame {frame})", path.display());
            }
            ScriptCommand::Shot(path) => {
                request_shot(world, path);
                world.resource_mut::<AgentRun>().wait = 2;
                return;
            }
            ScriptCommand::Exit => {
                if world
                    .resource::<AgentRun>()
                    .pending_shots
                    .load(Ordering::SeqCst)
                    > 0
                {
                    world
                        .resource_mut::<AgentRun>()
                        .queue
                        .push_front(ScriptCommand::Exit);
                } else {
                    info!("AGENT_EXIT script frame {frame}");
                    world.write_message(AppExit::Success);
                }
                return;
            }
        }
    }
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

fn request_shot(world: &mut World, path: std::path::PathBuf) {
    let pending = world.resource::<AgentRun>().pending_shots.clone();
    pending.fetch_add(1, Ordering::SeqCst);
    let shot = match world.get_resource::<OffscreenTarget>() {
        Some(target) => Screenshot::image(target.0.clone()),
        None => Screenshot::primary_window(),
    };
    world
        .spawn(shot)
        .observe(move |captured: On<ScreenshotCaptured>| {
            match captured.image.clone().try_into_dynamic() {
                Ok(image) => {
                    if let Some(parent) = path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    match image.to_rgb8().save(&path) {
                        Ok(()) => info!("shot written to {}", path.display()),
                        Err(error) => error!("{}: {error}", path.display()),
                    }
                }
                Err(error) => error!("screenshot conversion: {error}"),
            }
            pending.fetch_sub(1, Ordering::SeqCst);
        });
}

/// Key codes a script can press, by `Debug` name.
pub fn key_code(name: &str) -> Option<KeyCode> {
    use KeyCode::*;
    const KEYS: &[KeyCode] = &[
        Escape,
        Enter,
        Space,
        Tab,
        Backspace,
        Delete,
        ArrowUp,
        ArrowDown,
        ArrowLeft,
        ArrowRight,
        Home,
        End,
        PageUp,
        PageDown,
        F1,
        F2,
        F3,
        F4,
        F5,
        F6,
        F7,
        F8,
        F9,
        F10,
        F11,
        F12,
        KeyA,
        KeyB,
        KeyC,
        KeyD,
        KeyE,
        KeyF,
        KeyG,
        KeyH,
        KeyI,
        KeyJ,
        KeyK,
        KeyL,
        KeyM,
        KeyN,
        KeyO,
        KeyP,
        KeyQ,
        KeyR,
        KeyS,
        KeyT,
        KeyU,
        KeyV,
        KeyW,
        KeyX,
        KeyY,
        KeyZ,
        Digit0,
        Digit1,
        Digit2,
        Digit3,
        Digit4,
        Digit5,
        Digit6,
        Digit7,
        Digit8,
        Digit9,
        ShiftLeft,
        ControlLeft,
        AltLeft,
    ];
    KEYS.iter()
        .copied()
        .find(|key| format!("{key:?}").eq_ignore_ascii_case(name))
}
