# PB3a — Profil de build Rust (Apple Silicon)

Lot de PB3 (`docs/wip/pb3-performance.md`), vague 1. Worktree dédié
`agent-ac2766f2955685575`. Symlinks non versionnés créés : `data/map/pyramid`,
`tools/geo/raw` (absent du dépôt, non nécessaire à ce lot). `CARGO_TARGET_DIR`
partagé : `/Users/jean_hubert/dev/game_project/core/target`.

## Objectif
1. `[profile.release]` : `lto = "fat"`, `codegen-units = 1`,
   `debug = "line-tables-only"`. Pas de `panic = "abort"` (gdext attrape les
   panics). Pas de `target-cpu` imposé (portabilité M1-M4).
2. `[profile.dev.package.godot-bridge] opt-level = 2`.
3. `core/build.sh` : respecter `CARGO_TARGET_DIR` si défini.
4. Mesurer avant/après (turn_perf, banc bataille, pb1_turns.gd), médianes de 3.
5. ADR 0079.
6. fmt/clippy/test + smoke Godot avant rendu, merge main.

## État
- 26/09 : lecture conventions PB3, symlinks créés, squelette Cargo.toml/build.sh en cours.

## Prochaine étape
Implémenter le profil release + opt-level godot-bridge, mesurer avant/après.
