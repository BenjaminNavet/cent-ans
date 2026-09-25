# EP9 — Batailles décisives

Lot : les batailles de campagne doivent se décider d'elles-mêmes (recette Q3, point 12).
Branche : `worktree-agent-a6207325088470391`. ADR 0056.

## État : terminé (prêt à fusionner)

- Règles `data/rules/battle_decision.json` (+ schéma, pytest) ; `sim-battle/src/decision.rs`,
  `src/sim/decision.rs` : armée brisée (part en état de combattre < 0,40, 0,50 sans général,
  pendant 30 s), bataille refusée (300 s sans engagement), accalmie après mêlée (150 s),
  `BattleOutcome::end`.
- IA : `DEFENDER_QUIET` (le défenseur n'abandonne pas sa position tant que personne ne combat),
  `ATTACKER_DUEL_LIMIT` 180 s.
- Campagne : chronique « Bataille refusée » (`battle_request.rs`), test `sim-campaign/tests/ep9_refused.rs`.
- UI : écran de résultat (nuance « Bataille refusée », mention de fin) ; smoke : bataille sans ordre
  terminée avant 720 s simulées (326 s, refusée).
- Tests : `sim-battle/tests/ep9_decisive.rs` (7 actifs, sondes `survey`, `survey_crecy`,
  `survey_reference`, `trace` ignorées) ; b6 digests, ai.rs (4-12 min), ep1_scale (seuil 12 → 10,
  fin avant 720 s) mis à jour.
- Vérifié en jeu : `q3_playtest.gd` (branche de la recette Q3) modifié sans ordres → refusée à 326 s,
  écran de résultat atteint.

## Points ouverts

- Passage de rivière profonde par l'IA attaquante (EP3) : noyades, quelques batailles à 13-22 min.
- Bataille rangée : l'attaquant IA perd souvent contre un défenseur immobile (tir à volonté).
- ep1_scale : le seuil de 20 régiments en mêlée ne peut pas revenir (l'armée se brise avant).
