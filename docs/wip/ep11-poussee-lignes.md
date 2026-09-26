# EP11 — Poussée continue des lignes en mêlée

Branche : worktree agent-a5b7ba925e656a5c7. ADR : 0071 (attribuée par le coordinateur : 0070 EP12,
0072 EP13, 0073 CT1, 0074 FR1).

## État
- Règles `data/rules/battle_push.json` + schéma `battle_push_rules.schema.json` + pytest
  `tools/tests/test_battle_push_schema.py`.
- `sim-battle/src/push.rs` : règles, `PushShape` (champ `Unit::push`), poussée (`drive`) et tenue
  (`resistance` : carré, pieux), vitesse de recul, facteur de mêlée (compression, enroulement),
  déformation des figurines (bombement, rangs serrés, files qui pivotent).
- `sim-battle/src/sim/push.rs` : `resolve_push` (appelé juste avant `resolve_melee` dans `step`),
  blocage (bord, mur, maison, eau profonde, ami — écart par axes séparateurs), suivi du vainqueur
  (pas s'il est aux pieux ou engagé par un second ennemi), compression, moral (recul, compression),
  forme du front (bombement de l'un = creux de l'autre).
- Branchements dans `sim.rs` : `mod push`, champ `push_rules`, appel, facteur dans `melee_damage` ;
  `sim/siege_assault.rs::soldier_poses` déforme les figurines. Pont : `get_units` expose
  `push_speed`, `compression`, `ground_lost`.
- Tests `sim-battle/tests/ep11_push.rs` + sondes `probe`, `probe_historical`, `probe_step_cost`,
  `probe_passive`.
- Réglage : recherche aléatoire sur EP7 (graines 1-30), ep9b (1-30) et `ai` ; retenu vitesse
  0,45 m/s, bande morte 0,15, pieux ×10, fatigue/300, moral de recul 0,25/s, compression 0,1/s
  (+20 % de pertes, −25 % de coups, 0,25 moral/s), enroulement +15 %. b6 recalculé.
- Godot : `game/tests/ep11_push_shot.gd` (scènes line, wrap, backed), captures `docs/img/ep11/`.
- ATTENTION : les binaires de test changent de hash après la fusion de main (fonctionnalité
  `float_roundtrip`) ; toujours reprendre les chemins donnés par `cargo test --no-run`.
- Crochet temporaire `EP11_RULES_PROBE` dans `PushRules::bundled` (sondes) : À RETIRER avant la fin.

## Prochaine étape
- Mesures finales (EP7, ep9b, ep9, coût du pas), bench Godot, ADR 0071, retrait du crochet,
  vérifications finales (fmt, clippy, test workspace, build.sh, pytest, smoke).
