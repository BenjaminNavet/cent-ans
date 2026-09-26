# VH5 — Paris vers 1340 à l'échelle 1:1 (format landmark v2, ADR 0078)

Branche `feat/vh5-paris` (worktree d'agent, depuis `main` 89bc960a). Orchestration SZ
(`docs/wip/sz-suites-zoom.md`), chantier VH (`docs/wip/vh-villes-historiques.md`). Référence :
`docs/landmarks-v2.md`. Liens symboliques non versionnés `data/map/pyramid`, `tools/geo/raw` →
dépôt principal ; dylib copiée de `game/bin/` du dépôt principal. Aucun changement Rust.

## Découverte clé : ALPAGE est téléchargeable (ODbL)
- « Paris en 1380 » (P. Rouet) : voies, îlots, usages du sol, hydrographie ; « Vasserot v1 »
  (A.-L. Bethe) : parcelles 1810-1836. GeoPackage EPSG:2154, cache `tools/geo/raw/alpage/`.
- Module `tools/cent_ans_tools/geo/alpage.py` : lecture GPKG, rues de 1380 (`origin: "alpage"`),
  parcelles Vasserot filtrées (`parcels` = [dE, dN, angle, façade, profondeur]).
- `paris.json` écrit une fois par un script d'autorat (scratchpad, non versionné) à partir des
  géométries ALPAGE (murs, portes, emprises des monuments, îlots → quartiers, Seine de 1380) +
  faits datés ; ensuite maintenu à la main, `cent-ans geo landmarks --city paris` régénère rues
  et parcelles.

## Moteur commun (petits changements rétrocompatibles, Rouen identique : 5 838 maisons)
- `landmark_plan.gd` : index en grille des quartiers (`Districts.build_index`) et des eaux
  (`_water_index`) ; eaux en polygone (`polygon` + `holes`) ; `_imported_parcels` (section
  `parcels`) avant les lanières générées ; minutages par étape (`stats.marks_usec`).
- Schéma v2 : `streets.origin` et `waters.origin` += `alpage` ; `waters.polygon/holes` ;
  `bridges.certainty` ; recette `alpage` ; `parcels` = tableaux de 5 nombres.
- Outil : `fine_rivers: []` retire le fleuve fin (Paris : Seine fine trop large et décalée sur
  la Cité, remplacée par le lit de 1380 d'ALPAGE).

## État
- [x] Squelette, ALPAGE vérifié et importé
- [x] `paris.json` : 4 enceintes (PA ×2, Charles V levée 1356-1364 et maçonnée 1365+), 32 portes,
      119 monuments datés, 37 quartiers, 33 espaces libres, 6 ponts, 5 quais, Seine de 1380
- [x] Rues ALPAGE 1380 (1 227), 4 772 parcelles Vasserot gardées
- [x] Test `vh4_landmarks_test` étendu (Paris) : OK ; plan Paris ≈ 2-5 s (machine chargée)
- [ ] Captures `docs/img/vh5/`, i/s
- [ ] Docs (`docs/landmarks-v2.md`, crédits), pytest, zg4/zg6/smoke

## Prochaine étape
Captures et mesure i/s (`vh4_shots.gd --city=paris`), puis docs et tests.
