# ZG5b — rendu de l'hydrographie fine, des routes drapées, des ancrages et du parcellaire de près

Branche `worktree-agent-a1d8f6f50c1e29cce` (depuis `main` 5a33ec1f, ZG0-ZG5a). Cache partagé par
liens symboliques non versionnés `data/map/pyramid`, `tools/geo/raw`. ADR 0036, contrat
`docs/geo.md` § « Hydrographie fine ».

## Architecture (choix)
- `cafv_tile.gd` : lecture CAFV v1 (en-tête de **28 octets**, pas 32 comme l'écrit `docs/geo.md`).
- `fine_geo_store.gd` : index `rivers_fine.json` + `fine_anchors.json` (routes, ancrages), tuiles
  chargées dans `WorkerThreadPool`, LRU 64 tuiles par couche, `load_sync` pour le lit.
- **Lit creusé = abaissement des pages** (`fine_bed_carver.gd`) : crochet `page_filter` dans
  `relief_quadtree.gd` ; chaque page ≥ E2 est creusée dans un fil avant téléversement (profil
  parabolique sous `z`, berges fondues, sillon ≥ 0,75 px de page). Un seul état pour le vertex
  shader, les normales, le morphing et `surface_height_at`.
- `fine_ribbon_job.gd` (fil) + `fine_geo_layer.gd` (enfant `Rivers/FineGeo`) : tuiles E2 autour du
  point visé, rubans fleuves + routes par tuile, hauteurs en **mètres** (× `height_scale` dans le
  shader), remaillage 600 ms après les pages, disque `fine_zone` pour le fondu avec l'ancien rendu.
- Shaders : `river_fine.gdshader` (eau, bancs, vasières à marée, roselières, galets),
  `road_fine.gdshader` (terre, principale, pavé, chaussée) ; `fine_zone_mask` dans
  `river_bed.gdshaderinc` (anciens rubans et berges effacés dans le disque), `road.gdshader`.
- Ancrages : `SettlementLayer.apply_fine_anchors` (maquettes, hameaux, étiquettes de près),
  `RiverCrossings.set_fine_anchors` / `set_fine_mode` (ponts sur le fleuve fin au palier près),
  ponts-portes recalculés sur le fleuve fin (`FineGeoLayer._build_gates`).

## État
- [x] squelette + lecture CAFV, store, lit creusé, rubans, fondu, ancrages (non testés)
- [ ] fusion de `main` (ZG4 dans main : a3389a92) — demandée par l'orchestrateur
- [ ] test headless `tests/zg5b_fine_geo_test.gd`
- [ ] parcellaire (`fine_parcels.gdshaderinc`) + casse des carrés de la splat + détail HF
- [ ] fondu du lit `river_bed.png` aux paliers vallée/site
- [ ] banc, captures `docs/img/zg5b/`, `docs/godot-map.md`

## Prochaine étape
Fusionner `main`, brancher `vertical_scale_changed` de ZG4, lancer le jeu sur Rouen.
