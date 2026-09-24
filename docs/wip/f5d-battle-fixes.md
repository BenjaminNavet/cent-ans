# F5d — Simulation de bataille : correctifs

Branche : `worktree-agent-ae415cdff5eabc27d`. Périmètre : `core/crates/sim-battle`, ajouts dans
`core/crates/godot-bridge/src/battle_sim.rs`.

## État
1. [ ] Bataille de démo (France 1337, graine 1337) : contact avant 150 s, aucun régiment dans l'eau.
2. [ ] Rééquilibrage siège : escalade sans brèche ≈ 3/6.
3. [ ] Renforts échelonnés au-delà de 20 régiments (`reserve: true` au pont).
4. [ ] Sonde `ai` 4-5/10, `cargo test` vert, docs m7/m8/status.

## Prochaine étape
- Point 1 : fixture JSON de la bataille de démo, test de reproduction.
