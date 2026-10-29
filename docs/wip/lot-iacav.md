# Lot IA-CAV (boucle juge, iter 2) : charge groupée de la cavalerie

## État (PAUSE demandée par le joueur)
- Base : main 99d672914. Harnais `core/crates/sim-battle/tests/combat/matchups.rs` (`survey`, `trace` avec `DETAIL=1`).
- Avant (8 graines) : 10K/10L 0/8 (333 contre 50), 8K/8A 8/8 (4 contre 231), armée française mixte 0/8 (460 contre 52).
- Cause relevée au trace : l'archer défenseur plante ses pieux, donc aucune charge « isolée » n'est lancée ; les
  chevaliers avancent au pas sous les flèches (posture défensive de l'assaillant plus faible, vitesse x0,45).
- Implémenté : `ai/mass.rs` (alignement hors de portée, départ ensemble, flanc à 75 degrés quand pieux), branché
  en 1c dans `ai/horse.rs::plan_horse` ; règles `data/rules/battle_mass_charge.json` + schéma + `mass_charge.rs`.
- Après (8 graines, dernière mesure) : armée française mixte 3/8 (307 contre 85, Crécy reste anglais 5/8),
  10K/10L 0/8 (327 contre 66, pertes archers 50 -> 66), 8K/8A inchangé 8/8 (4 contre 231).
- Non fait : bandes `outcome_bands`, fixture `auto_resolve_scenarios.json`, ADR `docs/decisions/XXXX-iacav-*.md`,
  réduction du temps avant premier contact, 8K/8A, ajustement des tests à graine, fmt/clippy/test complets, seed bots.
- Le trace 10K/10L montrait, avant le dernier correctif (gate de puissance levé sous les flèches), des chevaliers
  qui marchent au pas sous les flèches de t=55 à t=85 : à re-tracer pour vérifier que `ride` s'exécute.

## Prochaine étape
- Re-tracer `TA=knights TB=longbowmen DETAIL=1` avec le binaire à jour (build : scratchpad `build.sh`, via heavy.sh).
- Comprendre pourquoi les chevaliers restent en `Marching` lent (autre commande postérieure qui écrase ? `react`, modes).
- Puis bandes, ADR, tests, seed bots.
