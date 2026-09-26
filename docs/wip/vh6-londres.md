# VH6 — Londres vers 1340 à l'échelle 1:1 (format `landmark` v2, ADR 0078)

Branche `feat/vh6-london` (worktree d'agent, depuis `main` 89bc960a). Orchestration SZ
(`docs/wip/sz-suites-zoom.md`), chantier VH (`docs/wip/vh-villes-historiques.md`, lot VH6).
Référence du format : `docs/landmarks-v2.md` ; exemple : `data/landmarks_v2/rouen.json`.
Liens symboliques non versionnés `data/map/pyramid`, `tools/geo/raw` → dépôt principal ; dylib
copiée (pas de Rust). Caches OSM : `tools/geo/raw/osm/london_highways.json` (voies, pour l'outil),
`london_features.json` (églises, Tour, vestiges du mur : relevé des positions à la main).

## Objectif
`data/landmarks_v2/london.json` (maquette v1 `london`, colonie `set_londres`) : mur romain et
médiéval (portes, front de Tamise ouvert), Tour, Old St Paul's, London Bridge, Guildhall, Cheapside,
Blackfriars, Greyfriars, Southwark, Westminster (quartier séparé), Strand et ses hôtels, Temple ;
rues OSM (recette avec `exclude`) et rues disparues tracées à la main ; quais et pont à la hauteur
des rives basses ; ≥ 60 i/s ou pas de régression vs Rouen.

## État
- [x] Squelette (ce fichier)
- [x] Données géographiques : positions relevées sur OSM (ODbL) projetées en EPSG:3035
- [x] `london.json` : origine = ancre de la maquette (Temple), `extent_m` 2 800 ; mur ouvert
      (36 points, 7 portes + Moorgate datée 1415), 1 150 rues OSM + 6 rues à la main, Tamise fine,
      Fleet et douves de la Tour tracées, 3 quais, London Bridge, 64 monuments, 10 quartiers,
      10 espaces libres
- [x] Moteur : petits ajouts rétrocompatibles (ci-dessous)
- [x] Test `vh4_landmarks_test` étendu à Londres (OK) ; pytest landmarks v2 (17 OK)
- [ ] Captures `docs/img/vh6/`, mesure i/s, itérations
- [ ] Tests complets (zg4_camera, zg6_towns, smoke, pytest complet), doc `docs/landmarks-v2.md`

## Changements du moteur commun (rétrocompatibles, Rouen inchangé)
- Schéma : portes datées (`gates[].from_year/until_year`) ; pont : `chapel_side`,
  `drawbridge_at`, `pier_m`, `starling_m`.
- `LandmarkPlan._add_wall(…, year)` : portes filtrées par date (mur continu avant 1415 à Moorgate).
- `LandmarkPlan._add_bridge` : niveaux dans `_bridge_levels` (stockés dans `plan.bridge` : `a`,
  `c`, `deck_m`, `bank_from`, `bank_to`) ; travées sans maisons au pont-levis et à la chapelle ;
  chapelle de pont (`chapel_at`) = monument `church` à base fixe (crypte sur la pile, chapelle au
  niveau du tablier) ; `reground` recalcule les niveaux du pont et décale maisons, portes et
  chapelle du pont (avant : niveaux figés au premier plan).
- `TownBuilder` : piles d'épaisseur `pier_m`, avant-becs au ras de l'eau (`starling_m`), rampes
  d'accès inclinées du tablier jusqu'aux rives quand le tablier les domine de plus de 0,8 m.
- `LandmarkMonuments._belfry` : `top: "turrets"` (toit plat, tourelles d'angle : Tour Blanche).
- `LandmarkCityLayer._dated_signature` : tient compte des portes datées.

## Prochaine étape
Regarder les captures (`godot --path game --script res://tests/vh6_shots.gd -- --out=docs/img/vh6
--map-weather=clear`), corriger, mesurer i/s vs Rouen, puis tests complets et doc.
