//! Developer tools tried in lot devtools2 (ADR 0364). Everything here exists only with the cargo
//! feature `dev`; release builds contain none of it.
//!
//! - **BRP extras** (`bevy_brp_extras`): screenshot, key / text / mouse input and clean shutdown
//!   over the Bevy Remote Protocol. Port from `CENT_ANS_BRP_PORT` (default 15702) so parallel
//!   agents do not collide.
//! - **Schedule shuffling**: `CENT_ANS_SHUFFLE_SEED=<n>` randomises the order of the systems of
//!   every schedule (single-threaded executor, so a seed reproduces its order). See
//!   `client/tests/shuffle.sh`.
//! - **Debug console** (`bevy_console` on `bevy_egui`): toggled with the key of
//!   `CENT_ANS_CONSOLE_KEY` (a `KeyCode` name, default `F8`) or the physical key under Escape
//!   (`Backquote`, `²` on AZERTY). `F12` is not usable: it is the `map_screenshot` action of the
//!   `KeyMap`. Commands: `dump-state [file]`, `goto <State>`, `wireframe`, `shadows`.

use bevy::app::PluginGroupBuilder;
#[cfg(feature = "dev")]
use bevy::prelude::*;

/// Wraps the default plugin group so the renderer asks for the GPU features the wireframe needs.
/// Identity in release builds (called from `main`).
#[cfg(feature = "dev")]
pub fn tune_default_plugins(group: PluginGroupBuilder) -> PluginGroupBuilder {
    use bevy::render::RenderPlugin;
    use bevy::render::settings::{WgpuFeatures, WgpuSettings};
    group.set(RenderPlugin {
        render_creation: WgpuSettings {
            // Used only by the `wireframe` console command; both are checked by `WireframePlugin`.
            features: WgpuFeatures::POLYGON_MODE_LINE | WgpuFeatures::IMMEDIATES,
            ..default()
        }
        .into(),
        ..default()
    })
}

/// Release builds: nothing changes.
#[cfg(not(feature = "dev"))]
pub fn tune_default_plugins(group: PluginGroupBuilder) -> PluginGroupBuilder {
    group
}

/// Dev-only plugin: BRP extras, schedule shuffling, console.
#[cfg(feature = "dev")]
pub struct DevToolsPlugin;

#[cfg(feature = "dev")]
impl Plugin for DevToolsPlugin {
    fn build(&self, app: &mut App) {
        let port = std::env::var("CENT_ANS_BRP_PORT")
            .ok()
            .and_then(|text| text.parse::<u16>().ok());
        match port {
            Some(port) => app.add_plugins(bevy_brp_extras::BrpExtrasPlugin::with_port(port)),
            None => app.add_plugins(bevy_brp_extras::BrpExtrasPlugin::default()),
        };
        register_dump_method(app);
        if std::env::var_os("CENT_ANS_PROBE_EMPTY_MESH").is_some() {
            app.add_systems(Update, probe_empty_meshes);
        }
        app.add_plugins(console::DevConsolePlugin);
        if let Some(seed) = std::env::var("CENT_ANS_SHUFFLE_SEED")
            .ok()
            .and_then(|text| text.parse::<u64>().ok())
        {
            info!("AGENT_SHUFFLE seed {seed}");
            app.add_systems(Startup, move |mut schedules: ResMut<Schedules>| {
                shuffle_all(&mut schedules, seed);
            });
            // Startup itself is already built when it runs: shuffle before the first update too.
            shuffle_all(&mut app.world_mut().resource_mut::<Schedules>(), seed);
        }
    }
}

/// `CENT_ANS_PROBE_EMPTY_MESH=1`: warns (once per entity) about every entity drawing a mesh with
/// no vertices, with its name, its parents' names and its component types, so the code that
/// spawns it can be found (Bevy's own warning only gives an opaque asset id).
#[cfg(feature = "dev")]
fn probe_empty_meshes(world: &mut World, mut seen: Local<std::collections::HashSet<Entity>>) {
    // Checked the frame a mesh handle appears: later the render world has taken the data.
    let meshes = world.resource::<Assets<Mesh>>();
    let empty: Vec<Entity> = world
        .try_query_filtered::<(Entity, &Mesh3d), Changed<Mesh3d>>()
        .map(|mut q| {
            q.iter(world)
                .filter(|(_, m)| {
                    meshes.get(&m.0).is_some_and(|mesh| {
                        mesh.try_attributes()
                            .is_ok_and(|a| a.map(|(_, v)| v.len()).min().unwrap_or(0) == 0)
                    })
                })
                .map(|(e, _)| e)
                .collect()
        })
        .unwrap_or_default();
    for entity in empty {
        if !seen.insert(entity) {
            continue;
        }
        let mut chain = Vec::new();
        let mut current = Some(entity);
        while let Some(e) = current {
            chain.push(
                world
                    .get::<Name>(e)
                    .map_or_else(|| format!("{e}"), |n| n.to_string()),
            );
            current = world.get::<ChildOf>(e).map(|c| c.parent());
        }
        let components: Vec<String> = world
            .inspect_entity(entity)
            .map(|infos| infos.map(|(_, i)| i.name().to_string()).collect())
            .unwrap_or_default();
        warn!(
            "empty mesh on {entity}: chain {chain:?}, components {}",
            components.join(", ")
        );
    }
}

/// BRP method `cent_ans/dump`: the same JSON as `--dump-state`, read from the running client.
#[cfg(feature = "dev")]
fn register_dump_method(app: &mut App) {
    use bevy::remote::{BrpResult, RemoteMethodSystemId, RemoteMethods};
    fn handler(In(_): In<Option<serde_json::Value>>, world: &mut World) -> BrpResult {
        Ok(super::dump::build_dump(world))
    }
    let id = app.world_mut().register_system(handler);
    app.world_mut()
        .resource_mut::<RemoteMethods>()
        .insert("cent_ans/dump", RemoteMethodSystemId::Instant(id));
}

/// Randomises the system order (constraints kept) of every schedule built from now on, with a
/// single-threaded executor so that one seed gives one order.
#[cfg(feature = "dev")]
pub fn shuffle_all(schedules: &mut Schedules, seed: u64) {
    use bevy::ecs::schedule::SingleThreadedExecutor;
    for (_, schedule) in schedules.iter_mut() {
        let mut settings = schedule.get_build_settings();
        settings.shuffle_seed = Some(seed);
        // `CENT_ANS_AMBIGUITIES=1`: also log every pair of conflicting systems with no order.
        if std::env::var_os("CENT_ANS_AMBIGUITIES").is_some() {
            settings.ambiguity_detection = bevy::ecs::schedule::LogLevel::Warn;
        }
        schedule.set_build_settings(settings);
        schedule.set_executor(SingleThreadedExecutor::new());
    }
}

#[cfg(feature = "dev")]
mod console {
    use bevy::pbr::wireframe::{WireframeConfig, WireframePlugin};
    use bevy::prelude::*;
    use bevy_console::{
        AddConsoleCommand, ConsoleCommand, ConsoleCommandEntered, ConsoleConfiguration,
        ConsolePlugin, PrintConsoleLine, reply,
    };
    use clap::Parser;

    use crate::agent::dump::build_dump;
    use crate::agent::key_code;
    use crate::app::AppState;

    pub struct DevConsolePlugin;

    impl Plugin for DevConsolePlugin {
        fn build(&self, app: &mut App) {
            let mut keys = vec![KeyCode::Backquote];
            keys.push(
                std::env::var("CENT_ANS_CONSOLE_KEY")
                    .ok()
                    .and_then(|name| key_code(&name))
                    .unwrap_or(KeyCode::F8),
            );
            app.add_plugins((ConsolePlugin, WireframePlugin::default()))
                .insert_resource(ConsoleConfiguration {
                    keys,
                    // Typing in the console must not fire game shortcuts, nor clicks go through.
                    block_keyboard: true,
                    block_mouse: true,
                    title_name: "Console de développement".to_owned(),
                    ..default()
                })
                .add_console_command::<DumpStateCommand, _>(dump_state_command)
                .add_console_command::<GotoCommand, _>(goto_command)
                .add_console_command::<WireframeCommand, _>(wireframe_command)
                .add_console_command::<ShadowsCommand, _>(shadows_command);
            // Proof without a window: `CENT_ANS_CONSOLE_DEMO="wireframe;goto codex"` enters these
            // lines 90 frames after start (egui cannot render offscreen) and logs the replies.
            if let Ok(lines) = std::env::var("CENT_ANS_CONSOLE_DEMO") {
                app.insert_resource(ConsoleDemo {
                    lines: lines.split(';').map(|l| l.trim().to_owned()).collect(),
                    frame: 0,
                })
                .add_systems(Update, (run_demo, log_replies));
            }
        }
    }

    #[derive(Resource)]
    struct ConsoleDemo {
        lines: Vec<String>,
        frame: u32,
    }

    fn run_demo(mut demo: ResMut<ConsoleDemo>, mut out: MessageWriter<ConsoleCommandEntered>) {
        demo.frame += 1;
        // One line every 30 frames from frame 90.
        if demo.frame >= 90 && (demo.frame - 90).is_multiple_of(30) {
            let index = ((demo.frame - 90) / 30) as usize;
            if let Some(line) = demo.lines.get(index) {
                let mut words = line.split_whitespace().map(str::to_owned);
                if let Some(command_name) = words.next() {
                    out.write(ConsoleCommandEntered {
                        command_name,
                        args: words.collect(),
                    });
                }
            }
        }
    }

    fn log_replies(mut replies: MessageReader<PrintConsoleLine>) {
        for reply in replies.read() {
            info!("CONSOLE {}", reply.line);
        }
    }

    /// Prints the dump sections (names and size), or writes the whole JSON to a file
    #[derive(Parser, ConsoleCommand)]
    #[command(name = "dump-state")]
    struct DumpStateCommand {
        /// Output file (relative to the working directory); without it, section names are listed
        file: Option<String>,
    }

    fn dump_state_command(mut cmd: ConsoleCommand<DumpStateCommand>, world: &World) {
        let Some(Ok(DumpStateCommand { file })) = cmd.take() else {
            return;
        };
        let dump = build_dump(world);
        match file {
            Some(path) => match serde_json::to_string_pretty(&dump)
                .map_err(|e| e.to_string())
                .and_then(|text| std::fs::write(&path, text).map_err(|e| e.to_string()))
            {
                Ok(()) => reply!(cmd, "dump written to {path}"),
                Err(message) => reply!(cmd, "dump failed: {message}"),
            },
            None => {
                if let Some(sections) = dump.as_object() {
                    for (name, value) in sections {
                        reply!(cmd, "{name}: {} bytes", value.to_string().len());
                    }
                }
            }
        }
        cmd.ok();
    }

    /// Switches to an application state (MainMenu, FactionSelect, Campaign, Battle, Codex)
    #[derive(Parser, ConsoleCommand)]
    #[command(name = "goto")]
    struct GotoCommand {
        /// State name, case-insensitive
        state: String,
    }

    fn goto_command(mut cmd: ConsoleCommand<GotoCommand>, mut next: ResMut<NextState<AppState>>) {
        let Some(Ok(GotoCommand { state })) = cmd.take() else {
            return;
        };
        match AppState::from_name(&state) {
            Some(target) => {
                next.set(target);
                reply!(cmd, "going to {target:?}");
                cmd.ok();
            }
            None => {
                reply!(cmd, "unknown state {state}");
                cmd.failed();
            }
        }
    }

    /// Toggles the wireframe of every mesh
    #[derive(Parser, ConsoleCommand)]
    #[command(name = "wireframe")]
    struct WireframeCommand;

    fn wireframe_command(
        mut cmd: ConsoleCommand<WireframeCommand>,
        mut config: ResMut<WireframeConfig>,
    ) {
        if cmd.take().is_some() {
            config.global = !config.global;
            reply!(
                cmd,
                "wireframe {}",
                if config.global { "on" } else { "off" }
            );
            cmd.ok();
        }
    }

    /// Toggles the shadow maps of every directional light
    #[derive(Parser, ConsoleCommand)]
    #[command(name = "shadows")]
    struct ShadowsCommand;

    fn shadows_command(
        mut cmd: ConsoleCommand<ShadowsCommand>,
        mut lights: Query<&mut DirectionalLight>,
    ) {
        if cmd.take().is_some() {
            let enable = lights.iter().any(|light| !light.shadow_maps_enabled);
            for mut light in &mut lights {
                light.shadow_maps_enabled = enable;
            }
            reply!(
                cmd,
                "shadows {} ({} lights)",
                if enable { "on" } else { "off" },
                lights.iter().count()
            );
            cmd.ok();
        }
    }
}
