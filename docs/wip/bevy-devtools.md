# Lot devtools (Bevy 0.20 dev tools)

## État
- Features Cargo : `bevy-devtools` (dans `dev`) = `bevy_dev_tools` + `render_dev_tools` + `bevy_debug_stepping` ; `hotpatch` séparée.
- Code : `client/src/agent/bevy_tools.rs` (dev uniquement), branché dans `DevToolsPlugin`.
- `free_camera` / `bevy_camera_controller` : non retenu (la caméra du jeu a ses propres contrôleurs ; un second contrôleur se battrait avec eux).

## Prochaine étape
- Compiler (`cargo clippy --features dev`), tester `hotpatch`, `cargo tree -e features` sans `dev`, README.
