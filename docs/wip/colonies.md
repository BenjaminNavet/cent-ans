# Orchestration : refonte « colonies » (échelle Total War)

Spec : `docs/design/2026-09-24-echelle-colonies.md`. ADR : `docs/decisions/0005-settlements-within-provinces.md`.
Le joueur a validé la spec et autorise toutes les décisions sans demander (2026-09-24).

## Lots et état

| Lot | État | Notes |
|---|---|---|
| Schéma `settlement` | fait | `data/schemas/settlement.schema.json`, `settlement_id` dans `common.schema.json` |
| C1 squelette cœur | en cours (vague 1) | types data-model, `SettlementState`, cité auto depuis `capital_city` |
| C2a colonies France | en cours (vague 1) | régions france_*, aquitaine, languedoc, provence_alpes (41 prov.) |
| C2b colonies Nord | en cours (vague 1) | angleterre_*, galles, ecosse, irlande, pays_bas, empire_*, scandinavie (56 prov.) |
| C2c colonies Sud | à faire (vague 2) | iberie_*, aragon, portugal, italie_* (35 prov.) |
| C3 pipeline géo | à faire (vague 2) | graphe des colonies, relief 8192² tuiles, hameaux, routes |
| C4 refonte cœur | à faire (vague 3) | |
| C5 pont + UI | à faire | |
| C6 rendu paliers | à faire | |
| C7 IA, équilibrage, docs | à faire | |

## Décisions prises en cours de route

- Découpage de la recherche par grande région (champ `region` des provinces) plutôt que par pays.

## Prochaine étape

Attendre la fin de la vague 1, fusionner, lancer la vague 2 (C2c, C3, puis C4).
