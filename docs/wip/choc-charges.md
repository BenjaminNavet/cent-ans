# Choc des charges — lots 1-4 (ADR 0711)

Branche `feat/choc`, worktree `../gp-choc`. Mandat du joueur (20/10) : enchaîner les 4 lots.

## État : lots 1-4 codés, équilibre vert
1. Constantes → `data/rules/battle_impact.json` (`ImpactRules`).
2. Élan couru (`run_up`) × poids de la cible : poids actif ; élan neutre (`elan.min` 1, seuil 0).
3. Arc-boutement graduel : infanterie à l'arrêt ou en mêlée, face à la charge, par rangs ; actif
   (0.15 + 0.05/rang, max 0.6 ; cohésion ×0.25 ; émousse le bonus à cheval ×0.5).
4. Paliers recul/chute actifs ; immunité (0 s) et décroissance du bonus (0 s) neutres.

## Constat
Impacts de charge rejoués à chaque retour en mêlée (≈ 7 000 pour la cavalerie d'un camp sur 10 duels).
Le seuil `min_impact_run_up_m` 5 m corrige, mais l'équilibre actuel en dépend : avec le seuil, ou
`elan.min` ≤ 0.7, ou l'immunité, échouent b6, `ai_beats_passive`, `ai_battles_last_minutes`, duel
symétrique, Poitiers, Crécy.

## Suite (chantier séparé, à proposer au joueur)
Activer seuil 5 m, `elan.min` ~0.5, immunité ~1 s, décroissance ~13 s, puis recaler `battle_pace.melee_rate`,
l'IA de charge (reprendre du champ avant de recharger) et les tests d'équilibre.
