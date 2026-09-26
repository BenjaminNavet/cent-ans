# EP13 — Rejeu d'après bataille

Branche `feat/ep13-replay` (worktree agent). ADR 0072 (numéro choisi pour éviter EP11/EP12).

## Approche
- Simulation déterministe : on enregistre le départ (`ReplayStart` : setup, graine, échelle, site
  historique) et les entrées horodatées par pas (`ReplayEntry`), puis on re-simule.
- Empreintes d'état (`state_digest`) au départ, toutes les 10 s et à la fin (+ issue) : la lecture
  signale la première divergence (règles changées) au lieu de diverger en silence.
- Sauts : copies en mémoire (`BattleSim: Clone`) toutes les 20 s pendant la lecture.
- Fichier JSON versionné (`REPLAY_FORMAT`), schéma `data/schemas/battle_replay.schema.json`,
  réglages `data/rules/battle_replay.json`.

## État
- [x] `core/crates/sim-battle/src/replay.rs` (enregistreur, lecteur, empreinte) — compile
- [ ] tests Rust `tests/ep13_replay.rs` (déterminisme, divergence, saut) + mesures
- [ ] pont : enregistrement dans `battle_sim.rs`, `battle_replay.rs` (save/list/load/seek)
- [ ] Godot : barre de rejeu, bouton écran de fin, menu « Rejeux », test UI
- [ ] ADR 0072, schéma du fichier, pytest schéma

## Prochaine étape
Tests Rust de déterminisme.
