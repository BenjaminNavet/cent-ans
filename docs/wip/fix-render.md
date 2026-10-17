# fix-render (lot 1) — état

- [x] `--visible-window` (client/src/agent/cli.rs, run_bg.sh) : vraie fenêtre + swapchain, `shot` via Screenshot::primary_window.
- [x] Durcissement MSAA : `settings/quality.rs apply_msaa` synchronise aussi la caméra UI (test `ui_camera_follows_the_world_camera_msaa`).
- [ ] Repro en fenêtre réelle plein écran, avant/après.
- [ ] Ordre d'affichage étiquettes/menu pause (bloquant 2), UiScale (3).
