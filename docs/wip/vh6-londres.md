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
- [x] Allées nommées de la Cité (`osm_streets.alleys`, +176 venelles) : 1 326 rues OSM, ≈ 7 100 maisons
- [x] Captures `docs/img/vh6/` (stratégique, transition, vallée, site, Tour, pont, pont bas,
      St Paul's, Westminster, toits) ; script `game/tests/vh6_shots.gd`
- [x] Mesure (machine chargée par d'autres agents) : 1er passage Londres 59,9 i/s à d = 1,6 et
      0,6 (pont idem), Rouen 59,9 / 52,1 dans la même session ; 2e passage (plus chargé) Londres
      36,7-59,9, Rouen 48-58. Plan ≈ 2,6-5 s dans le fil de travail (Rouen ≈ 2,9 s)
- [x] Tests : vh4_landmarks_test (Rouen + Londres) OK, zg4_camera OK, zg6_towns OK, smoke OK,
      pytest complet 731 OK ; ruff propre ; doc `docs/landmarks-v2.md` (section Londres)

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

- Outil `geo landmarks` : option de recette `alleys` (voir ci-dessus), testée.

## Limites / suites
- Tamise invisible aux paliers vallée et site (sol vert à la place de l'eau) : absence de nappe
  d'eau de la carte fine, traitée par SZ2b ; à revoir après sa fusion (quais, avant-becs et
  tablier ont été calés sur les hauteurs du relief : eau ≈ 1,9 m, rives 2,6-3,8 m, tablier
  = eau + 5,5 m, rampes jusqu'aux rives).
- Bosses sombres dans la Cité : relief GLO-30 (modèle de surface : immeubles modernes) sous la
  ville ; correction dans le relief, pas dans la ville.
- Tissu moins dense que le Londres réel : parcelles le long des rues OSM seulement, îlots
  modernes plus grands que les îlots médiévaux (cœurs d'îlots vides).
- Nombreux gabarits hypothétiques (hôtels du Strand, Guildhall d'avant 1411, prieuré de la
  Trinité, Austin Friars, palais de Westminster) : marqués `certainty`.
- Pas d'autre enceinte à Southwark ; Walbrook non dessiné (couvert en grande partie au XIVe s.,
  à vérifier).

## Faits à faire relire
- Tracé du mur entre Newgate et Aldersgate et position d'Aldersgate ; point de départ à la Tour.
- Position et axe d'Old St Paul's (−4° grille, centre 15 m à l'est de la cathédrale de Wren),
  hauteur de la tour de croisée (87 m) + flèche (62 m).
- London Bridge : extrémités (à l'est de St Magnus), hauteur du tablier (5,5 m au-dessus de
  l'eau), fractions du pont-levis (0,66) et de la chapelle (0,48).
- Dates : nef romane de Westminster jusqu'en 1375, Moorgate 1415, tour du Joyau 1366.

## Prochaine étape
Fusion par l'orchestrateur ; revoir les captures quand SZ2b (eau au palier site) est dans `main`.
