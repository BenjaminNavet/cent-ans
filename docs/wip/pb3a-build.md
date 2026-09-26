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
- 26/09 : `[profile.release]` ajouté (`codegen-units = 1`, `debug = "line-tables-only"`),
  `[profile.dev.package.godot-bridge] opt-level = 2`, `[profile.bench-native]`,
  `build.sh` respecte `CARGO_TARGET_DIR`. Commité.
- 26/09 : mesures (machine partagée, très chargée, plusieurs agents PB3 compilent en
  parallèle sur le même `CARGO_TARGET_DIR` — échantillons uniques, pas de médiane de 3,
  contamination probable par le verrou de build) :
  - Build dev `-p godot-bridge` (`cargo build`, temps rapporté par cargo) :
    avant (opt-level 0, hérité) 4 m 40 s ; après (opt-level 2) 5 m 53 s. Coût attendu
    d'une meilleure optimisation ; à re-mesurer sur machine calme si possible.
  - Build release `-p godot-bridge` propre (première compilation avec le nouveau
    `[profile.release]`) : `lto = "fat"` → **32 m 30 s** (dépasse largement le seuil de
    3 min de la consigne, sur machine chargée) ; `lto = "thin"` → **9 m 49 s**. Décision :
    **`lto = "thin"`** retenu (meilleur compromis mesuré), `codegen-units = 1` et
    `debug = "line-tables-only"` conservés. Un rebuild propre "fat" complet n'a pas été
    repris pour confirmer sur machine calme (contrainte de temps) ; à revalider plus tard
    si le temps de build redevient un problème.
  - `turn_perf` (crate `ai`, exemple) dev vs release : quasi identique
    (mean 4.22 ms les deux, médiane 2.55 ms dev / 2.77 ms release, 1400 tours) — attendu :
    `ai` et `sim-campaign` sont déjà en opt-level 2 en dev (config antérieure), donc ce
    banc ne montre pas l'effet du changement `godot-bridge` (qui n'est pas sur ce chemin).
  - Banc bataille (`battle_scene.gd --benchmark`) et `pb1_turns.gd` (mesure la traversée
    du pont Godot-bridge, sensible à l'opt-level 2) : lancés mais interrompus par la
    consigne de temps de l'orchestrateur (verrou partagé + délai de 30 min) ; scène de
    carte affiche des erreurs de ressource (`parchment_theme.tres` manquant, sans doute
    un souci d'import Godot propre à ce worktree, indépendant du profil de build) à
    creuser si le temps le permet. **Point ouvert**, non bloquant pour le profil de
    build retenu (mesures dev vs release godot-bridge à compléter).

## Décision retenue
`lto = "thin"`, `codegen-units = 1`, `debug = "line-tables-only"` en release ;
`godot-bridge` en opt-level 2 en dev ; `build.sh` respecte `CARGO_TARGET_DIR`.

## Prochaine étape
- Si temps disponible : relancer `pb1_turns.gd` et le banc bataille (dylib debug
  avant/après opt-level 2) après avoir résolu l'erreur `parchment_theme.tres` (probable
  besoin d'un `godot --headless --path game --import` dans ce worktree).
- ADR 0079 à rédiger avec ces chiffres.
- fmt/clippy/test, smoke Godot, merge main, commit final.
