# VH0 + VH4 — villes emblématiques 1:1 géoréférencées (ADR 0078)

Branche `feat/vh4-landmarks-1to1` (worktree d'agent, depuis `main` 7e1ac032). Orchestration SZ
(`docs/wip/sz-suites-zoom.md`), défaut S3 de ZG7c. Liens symboliques non versionnés
`data/map/pyramid`, `tools/geo/raw` → dépôt principal. Dylib :
`CARGO_TARGET_DIR=/Users/jean_hubert/dev/game_project/core/target cargo build -p godot-bridge` puis
copie dans `game/bin/libcent_ans.debug.dylib`. Aucun changement Rust. Format et moteur documentés
dans `docs/landmarks-v2.md` (à lire en premier pour VH5-VH8).

## Architecture
- Données : `data/landmarks_v2/<id>.json`, schéma `data/schemas/landmark_v2.schema.json`
  (EPSG:3035 : `origin_3035` + décalages [dE, dN] en m, axes de la grille).
- Outil : `cent-ans geo landmarks [--city rouen] [--refresh-osm]`
  (`tools/cent_ans_tools/geo/landmarks_v2.py`) : rues héritées tirées d'OSM (recette
  `osm_streets`, cache `tools/geo/raw/osm/<id>_highways.json`), fleuve recopié de la carte fine
  (`hydro_fine`, section `waters` origine `rivers_fine`).
- Godot : `LandmarkV2Library`, `LandmarkPlan`, `LandmarkMonuments`, `LandmarkCityLayer` ;
  `TownBuilder` étendu ; accroche `SettlementLayer` ; plancher caméra levé
  (`CampaignCamera.floor_zones`) ; fondu tramé de la maquette (`landmark.gdshader`, `fade`).

## État : terminé (à fusionner par l'orchestrateur)
- [x] ADR 0078 (commit ac568e3b) ; addendum ADR 0036
- [x] Schéma v2, outil `geo landmarks`, CLI, crédits OSM (`CREDITS.md`)
- [x] Rouen vers 1340 (`data/landmarks_v2/rouen.json`) : 706 rues OSM, enceinte 9 portes /
      65 tours, pont Mathilde habité, 26 monuments datés, 5 quartiers, 8 espaces libres
- [x] Plan, gabarits, couche, fondu, plancher caméra levé
- [x] Fusion de `main` (SZ2 : pyramide rebasculée, SZ4, SZ5) ; `geo landmarks` relancé (inchangé)
- [x] Captures `docs/img/vh4/` (après SZ2) ; mesure i/s (machine chargée ≈ 110) : Rouen 55-56 i/s,
      Amiens 46-60 i/s dans la même session
- [x] Tests après fusion de `main` : `vh4_landmarks_test`, `zg4_camera_test`, `zg6_towns_test`,
      `smoke` OK ; pytest complet 708 OK ; ruff propre sur les fichiers du lot
- [x] Docs `docs/landmarks-v2.md`, `docs/godot-map.md`

## Limites / suites
- Seine fine au mauvais bras à l'est du pont (île Lacroix) : bande de terre entre mur de rive et
  eau ; à corriger dans `hydro_fine` (SZ2 ou suite).
- Exagération du relief au palier vallée (S1/SZ1) : côtes de Rouen en murs.
- Arbres des jardins non dessinés ; couvertures (`roofs`) non rendues ; parcellaire généré.
- Plan en ≈ 2-4 s dans un fil de travail (GDScript) : à optimiser si Paris (3-4 fois plus grand)
  est trop lent (grille d'occupation, tests de quartier).
- 60 i/s à confirmer sur machine au repos.
- Au palier site, la Seine s'affiche en lit sableux sans nappe d'eau (état de la carte fine après
  SZ2, hors lot) ; une grève verte sépare le mur de rive du lit (couloir interdit = largeur fine).

## Prochaine étape
Fusion par l'orchestrateur ; puis VH5 (Paris), VH6 (Londres), VH7 (Orléans) sur ce format.
