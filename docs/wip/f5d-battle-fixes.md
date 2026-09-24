# F5d — Simulation de bataille : correctifs

Branche : `worktree-agent-ae415cdff5eabc27d`. Périmètre : `core/crates/sim-battle`, ajouts dans
`core/crates/godot-bridge/src/battle_sim.rs`. Détail : `m7-battles.md` et `m8-sieges.md` § F5d.

## État
1. [x] Bataille de démo (France 1337, graine 1337) : contact à 75 s, personne dans l'eau.
2. [x] Escalade sans brèche (`siege 0 ""`) : 3/6 (tir des tours 25 → 5 carreaux).
3. [x] Renforts échelonnés au-delà de 20 régiments (`reserve: true` au pont).
4. [x] Sonde `ai` 4/10, `cargo test` vert, docs m7/m8/status ; `deploy_unit` : erreurs au nom du
   régiment ; main (F5c) fusionnée.

## Prochaine étape
- Godot (hors périmètre) : le message éphémère de F5c préfixe déjà le nom (« Chevaliers : … ») ;
  avec les nouveaux messages, afficher l'erreur seule. Masquer les régiments `reserve`.
