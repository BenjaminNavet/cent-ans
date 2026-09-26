# EP13 — Rejeu d'après bataille

Branche `feat/ep13-replay` (worktree agent). ADR 0072 (numéro choisi pour laisser 0070-0071 à EP11/EP12).

## Approche
- Simulation déterministe : on enregistre le départ (`ReplayStart` : setup, graine, échelle, site
  historique) et les entrées horodatées par pas (`ReplayEntry`), puis on re-simule.
- Empreintes d'état (`state_digest`) au départ, toutes les 10 s et à la fin (+ issue) : la lecture
  signale la première divergence (règles changées) au lieu de diverger en silence.
- Sauts : copies en mémoire (`BattleSim: Clone`) toutes les 20 s pendant la lecture.
- Fichier JSON versionné (`REPLAY_FORMAT`), schéma `data/schemas/battle_replay.schema.json`,
  réglages `data/rules/battle_replay.json`.

## État
- [x] `core/crates/sim-battle/src/replay.rs` (enregistreur, lecteur, empreinte)
- [x] tests Rust `tests/ep13_replay.rs` (déterminisme, divergence, sauts, format, exemple) + mesures
  (re-simulation ≈ 2000× temps réel, copie 0,03 ms)
- [x] serde_json `float_roundtrip` (sinon un rejeu relu diffère d'un ulp)
- [x] pont : `battle_sim.rs` (enregistrement de chaque entrée, refus en rejeu), `battle_replay.rs`
  (save/list/load/start/seek/get_replay), `historical_battles.rs`
- [x] Godot : `BattleReplayBar`, mode rejeu de `battle_scene.gd`, bouton de l'écran de fin, menu
  « Rejeux » (`ReplaysMenu`, bouton du menu principal), test `tests/ep13_replay_test.gd` (OK)
- [x] ADR 0072, schémas, pytest `test_battle_replay_schema.py`

## Vérifications (26/09, après merge de main b04b88ce)
fmt, clippy --all-targets -D warnings, cargo test --workspace (118 binaires OK), build.sh,
pytest (674 OK), smoke Godot OK, `ep13_replay_test.gd` OK, `ep7_historical_test.gd` OK.

## Prochaine étape
Fusion dans main par l'orchestrateur. Points ouverts : voir ADR 0072 § Conséquences (corps non
redessinés après un saut en arrière, empreintes sans décor/étendards) ; relecture à l'œil de la barre
de rejeu (non vérifiée en fenêtre) ; `core/Cargo.toml` touché (`float_roundtrip`), à signaler à EP11/EP12.
