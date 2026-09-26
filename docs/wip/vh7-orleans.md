# VH7 — Orléans vers 1340-1429 à l'échelle 1:1 (format v2, ADR 0078)

Branche `feat/vh7-orleans` (worktree d'agent, depuis `main` 89bc960a). Orchestration SZ
(`docs/wip/sz-suites-zoom.md`), chantier VH (`docs/wip/vh-villes-historiques.md`, lot VH7).
Référence : `docs/landmarks-v2.md`, exemple `data/landmarks_v2/rouen.json`. Liens symboliques
non versionnés `data/map/pyramid`, `tools/geo/raw` ; dylib copiée (aucun changement Rust).

## Plan
1. Squelette : ce fichier ; décision vue stratégique.
2. `data/landmarks_v2/orleans.json` : origine, enceintes datées, portes, pont des Tourelles,
   Châtelet, Sainte-Croix (chevet seul), faubourgs et églises hors les murs (`until_year` 1428),
   quartiers, places, recette OSM (+ exclusions), rues disparues à la main.
3. Moteur : Orléans n'a pas de maquette v1 (colonie ordinaire ZG6) → accrochage v2 sans
   `landmark` (petits changements rétrocompatibles, décrits ci-dessous).
4. Captures `docs/img/vh7/`, tests, i/s.

## État
- [x] squelette

## Prochaine étape
Relevé OSM des repères d'Orléans, écriture du fichier v2.
