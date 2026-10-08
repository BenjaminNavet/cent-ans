# DN ME6+ME7+ME9 — décor ponctuel hors villes

Branche `dn/me6-decor`. Plan : `docs/wip/dn/carte-extra.md` §4 (ME6 humain hors villes, ME7 industrie et ruines, ME9 pèlerins/foires/steppe).

## Conception
- `data/map/map_landmarks_extra.json` (schéma `data/schemas/map_landmarks_extra.schema.json`) : `render` (portées, tenue à l'écran), `types` (id -> modèle du catalogue DN `env_*`, repli procédural), `rules` (modes road / gate / crossing / coast / province / route), `sites` (positions historiques réelles, lonlat + px cuits).
- `game/scripts/map/decor_planner.gd` : planification pure (données -> sites), déterministe.
- `game/scripts/map/decor_layer.gd` : rendu, un MultiMesh par type, voisinage de la caméra, taille tenue à l'écran (formules d'`OutbuildingLayer`).
- Branchement d'un glb du catalogue : rien à faire, `type.model` = id du catalogue, lu dans `data/art/dn_manifest.json` (fichiers `_lod{0,1,2}`), sinon repli procédural.

## État
- [ ] squelette
