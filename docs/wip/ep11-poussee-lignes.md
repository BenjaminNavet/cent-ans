# EP11 — Poussée continue des lignes en mêlée

Branche : worktree agent-a5b7ba925e656a5c7. ADR : `docs/decisions/0071-poussee-continue-des-lignes.md`.

## État : terminé, en attente de fusion par l'orchestrateur
- Règles `data/rules/battle_push.json` + schéma `battle_push_rules.schema.json` + pytest
  `tools/tests/test_battle_push_schema.py`.
- `sim-battle/src/push.rs` : règles, `PushShape` (champ `Unit::push`), poussée (`drive`) et tenue
  (`resistance` : carré, pieux), vitesse de recul, facteur de mêlée (compression, enroulement),
  déformation des figurines (`deform_figures` : bombement, rangs serrés, files qui pivotent).
- `sim-battle/src/sim/push.rs` : `resolve_push` (appelé juste avant `resolve_melee` dans `step`),
  blocage (bord, mur, maison, eau profonde, ami — écart par axes séparateurs), suivi du vainqueur
  (pas s'il est aux pieux ou engagé par un second ennemi), compression, moral, forme du front.
- `sim.rs` : `mod push`, champ `push_rules`, appel, facteur dans `melee_damage` ;
  `sim/siege_assault.rs::soldier_poses` déforme les figurines ; pont : `get_units` expose
  `push_speed`, `compression`, `ground_lost`.
- Tests `sim-battle/tests/ep11_push.rs` (7) + sondes `probe`, `probe_historical`,
  `probe_step_cost`, `probe_passive`. b6 recalculé.
- Godot : `game/tests/ep11_push_shot.gd` (scènes line, wrap, edge ; test sans affichage),
  captures `docs/img/ep11/*.jpg`.
- Vérifié après fusion de main (EP12, FR1) : fmt, clippy, cargo test --workspace (828), build.sh,
  pytest (689), smoke Godot (28 OK).

## Points ouverts
- Fourchettes de test serrées : Azincourt 18/20 (≤ 19), ep9b 4/10 (≥ 3) ; la mêlée est chaotique,
  toute règle de mêlée future devra revérifier EP7, ep9b et `ai`.
- Un ami au repos décalé derrière le régiment poussé est repoussé par la séparation F5a au lieu de
  bloquer le recul (voir ADR 0071, Conséquences).
- Pas de poussée en siège.
