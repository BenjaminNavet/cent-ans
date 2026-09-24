# Orchestration : refonte « colonies » (échelle Total War)

Spec : `docs/design/2026-09-24-echelle-colonies.md`. ADR : `docs/decisions/0005-settlements-within-provinces.md`.
Le joueur a validé la spec et autorise toutes les décisions sans demander (2026-09-24).

## Lots et état

| Lot | État | Notes |
|---|---|---|
| Schéma `settlement` | fait | `data/schemas/settlement.schema.json`, `settlement_id` dans `common.schema.json` |
| C1 squelette cœur | fait (fusionné `a4de853`) | types data-model, `SettlementState`, cité auto, `rules.json`, getter `settlements()` |
| C2a colonies France | fait | 41 prov., 196 colonies, 8 enclaves |
| C2b colonies Nord | fait | 56 prov., 227 colonies ; Stirling anglais, Dunbar écossais |
| C2c colonies Sud | en cours (vague 2) | iberie_*, aragon, portugal, italie_* (35 prov.) |
| C3 pipeline géo | en cours (vague 2) | graphe des colonies, relief 8192² tuiles, hameaux, routes |
| C4 refonte cœur | en cours (vague 2) | graphe de repli si `settlement_graph.json` absent |
| C5 pont + UI | à faire | |
| C6 rendu paliers | à faire | |
| C7 IA, équilibrage, docs | à faire | |

## Décisions prises en cours de route

- Découpage de la recherche par grande région (champ `region` des provinces) plutôt que par pays.
- Avignon (cité du Comtat) laissée à la Papauté (`owner: null`) : juridiquement angevine jusqu'en 1348, mais siège de fait du pape ; sinon la règle « contrôleur de province = contrôleur de la cité » retirait sa capitale à la Papauté.
- Test des colonies : bornes vérifiées en pixels de la carte carrée (Vienne à 16,37° E est sur la carte), pas en degrés.
- `set_marmoutier` (Alsace) / `set_marmoutier_tours` (Touraine) : homonymes.
- Le smoke Godot échoue déjà avant la refonte (`projected_income should increase after construction`, `smoke.gd:358`, fichier modifié par une autre session) : non imputable aux colonies.
- C4 lancé en parallèle de C3 : si `settlement_graph.json` manque, le cœur construit un graphe de repli (colonies d'une province reliées entre elles, cités reliées aux cités voisines, ports aux ports `sea_neighbors`).

## Prochaine étape

Attendre la vague 2 (C2c, C3, C4), fusionner dans un worktree séparé puis ff-only ; ensuite vague 3 : C5 (pont + UI), C6 (rendu).
