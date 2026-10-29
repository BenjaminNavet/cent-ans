# Lot COMBAT (boucle juge, iter 1)

## Etat
- Harnais de matchups IA contre IA en bataille 3D : `core/crates/sim-battle/tests/combat/matchups.rs`
  (`survey` et `trace`, ignores ; `FILTER=`, `SEEDS=`).
- La fixture `auto_resolve_scenarios.json` est perimee par rapport a la sim 3D actuelle (ex. 4 chevaliers contre
  20 milices : 6/6 dans la fixture, ~2/6 avant correction).
- Leviers retenus : exposant d'armure des volees (`ranged_armor_exponent`), panique des montures, munitions arc long,
  archers montes (portee/tir), cohesion/brace du choc, moral par perte, vitesse d'approche.

## Prochaine etape
- Figer les bandes de test (`matchups::bands`), ADR, tests a graine, clippy, seed bots rapides.
