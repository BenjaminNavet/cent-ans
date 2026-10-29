# Lot IA-CAV (boucle juge, iter 2) : charge groupée de la cavalerie

## État
- Base : main 99d672914. Harnais `core/crates/sim-battle/tests/combat/matchups.rs` (`survey`, `trace` avec `DETAIL=1`).
- Avant (8 graines) : 10K/10L 0/8 (333 contre 50), 8K/8A 8/8 (4 contre 231), armée française mixte 0/8 (460 contre 52).
- Cause relevée au trace : l'archer défenseur plante ses pieux, donc aucune charge « isolée » n'est lancée ; les
  chevaliers avancent au pas sous les flèches (posture défensive de l'assaillant plus faible, 0,45 de vitesse).
- Implémenté : `ai/mass.rs` (alignement hors de portée, départ ensemble, flanc quand pieux), règles dans
  `data/rules/battle_mass_charge.json` + schéma.

## Prochaine étape
- Mesurer, ajuster, bandes `matchups::outcome_bands`, fixture d'auto-résolution, ADR, tests à graine.
