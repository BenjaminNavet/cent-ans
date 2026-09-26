# VH6 — Londres vers 1340 à l'échelle 1:1 (format `landmark` v2, ADR 0078)

Branche `feat/vh6-london` (worktree d'agent, depuis `main` 89bc960a). Orchestration SZ
(`docs/wip/sz-suites-zoom.md`), chantier VH (`docs/wip/vh-villes-historiques.md`, lot VH6).
Référence du format : `docs/landmarks-v2.md` ; exemple : `data/landmarks_v2/rouen.json`.
Liens symboliques non versionnés `data/map/pyramid`, `tools/geo/raw` → dépôt principal ; dylib
copiée (pas de Rust).

## Objectif
`data/landmarks_v2/london.json` (maquette v1 `london`, colonie `set_londres`) : mur romain et
médiéval (portes, front de Tamise ouvert), Tour, Old St Paul's, London Bridge, Guildhall, Cheapside,
Blackfriars, Greyfriars, Southwark, Westminster (quartier séparé), Strand et ses hôtels, Temple ;
rues OSM (recette avec `exclude`) et rues disparues tracées à la main ; quais et pont à la hauteur
des rives basses ; ≥ 60 i/s ou pas de régression vs Rouen.

## État
- [ ] Squelette (ce fichier)
- [ ] Données géographiques (OSM : positions des monuments, rues)
- [ ] `london.json` écrit à la main + `cent-ans geo landmarks --city london`
- [ ] Moteur : petits ajouts rétrocompatibles si nécessaire (décrits ci-dessous)
- [ ] Tests : pytest, vh4_landmarks_test (Londres), zg4_camera, zg6_towns, smoke
- [ ] Captures `docs/img/vh6/`, mesure i/s

## Changements du moteur commun
(aucun pour l'instant)

## Prochaine étape
Extraire les positions OSM (monuments, rues), écrire `london.json`.
