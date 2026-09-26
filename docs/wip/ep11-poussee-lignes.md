# EP11 — Poussée continue des lignes en mêlée

Branche : worktree agent-a5b7ba925e656a5c7. ADR : 0073 (0070 pris par CT1, 0071 laissé à EP12,
0072 = EP13).

## État
- Règles `data/rules/battle_push.json` + schéma `battle_push_rules.schema.json` + pytest
  `tools/tests/test_battle_push_schema.py`.
- `sim-battle/src/push.rs` : règles, `PushShape` (champ `Unit::push`), poussée (`drive`) et tenue
  (`resistance` : carré, pieux), vitesse de recul, facteur de mêlée (compression, enroulement),
  déformation des figurines (bombement, rangs serrés, files qui pivotent).
- `sim-battle/src/sim/push.rs` : `resolve_push` (appelé juste avant `resolve_melee` dans `step`),
  blocage (bord, mur, maison, eau profonde, ami), suivi du vainqueur (pas s'il est aux pieux ou
  engagé par un second ennemi), compression, moral (recul, compression), forme du front.
- Branchements dans `sim.rs` : `mod push`, champ `push_rules`, appel, facteur dans `melee_damage` ;
  `sim/siege_assault.rs::soldier_poses` déforme les figurines.
- Tests `sim-battle/tests/ep11_push.rs` (lourd contre léger, lignes égales, déterminisme,
  enroulement, compression contre un ami et contre le bord) + sondes `probe`, `probe_historical`,
  `probe_step_cost`.
- Réglage : grille sur EP7 (graines 1-30) et ep9b (1-30), retenu pieux ×10, fatigue/300, bande
  morte 0,12, 0,45 m/s. b6 recalculé (graine 3 repasse aux Anglais).
- main fusionné (EP13, CT1) : tests sim-battle verts.
- Crochet temporaire `EP11_RULES_PROBE` dans `PushRules::bundled` (sondes) : À RETIRER avant la fin.

## Prochaine étape
- Script de capture Godot `game/tests/ep11_push_shot.gd`, captures `docs/img/ep11/`.
- Bench `tools/bench_ep1.sh`, ADR 0073, vérifications finales, retrait du crochet.
