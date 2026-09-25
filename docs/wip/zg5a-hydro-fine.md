# ZG5a — hydrographie fine, ancrages et routes drapées (données)

Branche `worktree-agent-a2168a2690056db0a`, **rebasée sur `integration/zoom`** (3beab01c, ZG1+ZG3
fusionnés : code de la pyramide et manifeste rempli). Cache partagé par liens symboliques
(`tools/geo/raw`, `data/map/pyramid`, jamais commités). Coût : 0 $.

## Sources (vérifiées le 25/09/2026, HTTPS anonyme)
- France : BD TOPAGE® 2025 `TronconHydrographique_FXX` (SANDRE/IGN/OFB), Licence Ouverte 2.0 — 857 Mo zip, 3,0 Go gpkg, 3,04 M tronçons ; pas d'ordre de Strahler rempli (`NumeroOrdreTH` vide) → calculé par topologie.
- Grande-Bretagne : OS Open Rivers (GeoPackage, OGL v3), 193 k tronçons, Strahler calculé.
- Bénélux, Rhénanie, reste du cœur : EU-Hydro v1.3 via le service REST ArcGIS anonyme de l'AEE (`image.discomap.eea.europa.eu/.../EUHydro_RiverNetworkDatabase/MapServer`, couches par ordre de Strahler), politique de données ouvertes Copernicus.
- Hors cœur : Natural Earth (lignes de `rivers.geojson`).

## Modules
- `geo/valley_snap.py` : rééchantillonnage, recalage Viterbi sur le fond de vallée, PAVA (monotonie aval), drapé de route, Douglas-Peucker.
- `geo/fine_relief.py` : échantillonneur bilinéaire sur l'étage le plus fin présent (E7 → E0).
- `geo/fine_tiles.py` : format binaire CAFV par tuile E2, découpe aux bords de tuile.
- `geo/hydro_sources.py` : lecteurs TOPAGE / OS / EU-Hydro / NE, Strahler, traits (strokes), filtre des canaux.
- `geo/hydro_fine.py` : pipeline `geo hydro-fine` (en cours).
- Données : `data/map/historical_hydro_notes.json` (canaux modernes, zones divagantes/marais/estuaires).

## État
- [x] squelette, sources téléchargées (TOPAGE, OS, EU-Hydro 420 cellules)
- [x] pipeline `geo hydro-fine` (testé sur OS + EU-Hydro + NE : 25,7 k km GB, 29 k km EU-Hydro, 46 k km NE, 24 s)
- [x] `geo anchors-fine` (colonies, hameaux, ponts, routes drapées ; testé sans TOPAGE)
- [x] schémas + tests (`tools/tests/test_hydro_fine.py`), `docs/geo.md`, `CREDITS.md`
- [ ] préparation TOPAGE (`cache/links_topage.npz`, lecture de 3 Go lente sous charge) puis run complet
- [ ] aperçus `docs/img/zg5a/` (Rouen, Orléans, Bordeaux, Londres), commit de `rivers_fine.json` et `fine_anchors.json`

## Prochaine étape
Attendre `links_topage.npz` (tâche de fond, sinon relancer `hydro_fine.prepare_topage()`),
puis `cent-ans geo hydro-fine --workers 6` et `cent-ans geo anchors-fine --workers 6`,
aperçus via `hydro_preview.render_all`.
