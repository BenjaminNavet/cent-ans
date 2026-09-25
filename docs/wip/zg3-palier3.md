# ZG3 — relief palier 3 (E5-E7) sur les zones de détail

Lot ZG3 de l'ADR 0036. Branche `worktree-agent-a17b7688304a10059`.
Commande : `uv run --project tools cent-ans geo detail-dem [--zones id,...] [--force]`.

## État
- [x] Services vérifiés (25/09) : IGN WMS-R `data.geopf.fr/wms-r` (couche
  `ELEVATION.ELEVATIONGRIDCOVERAGE.HIGHRES`, GeoTIFF float32, 5010 px max, sans clé) ;
  EA WCS 2.0.1 `environment.data.gov.uk/spatialdata/lidar-composite-digital-terrain-model-dtm-1m/wcs`
  (`scalefactor` accepté) ; AHN PDOK WCS `dtm_05m` (`scalesize` accepté, lent) ; Flandre
  `geo.api.vlaanderen.be/DHMV/wcs` (multipart GML + TIFF, pas de mise à l'échelle, ≤ 2000² px
  par requête ; `DHMVI_DTM_5m` + `DHMVII_DTM_1m`) ; Wallonie : WMS rendu seulement, pas de WCS
  → repli GLO-30 ; Overpass `overpass-api.de` OK.
- [ ] Squelette (API, CLI, modules) — en cours
- [ ] Zones (`data/map/detail_zones.json`)
- [ ] Récupérateurs
- [ ] Effacement des anachronismes
- [ ] Cuisson E5-E7 + manifeste
- [ ] Aperçus `docs/img/zg3/`
- [ ] Docs (`docs/geo.md`, `CREDITS.md`)

## Prochaine étape
Implémenter `detail_dem.py` (grille, emprises, grappes), `detail_sources.py`, `anachronisms.py`.

## Zones faites / tailles
(à remplir)
