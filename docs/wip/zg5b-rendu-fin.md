# ZG5b — rendu de l'hydrographie fine, des routes drapées, des ancrages et du parcellaire de près

Branche `worktree-agent-a1d8f6f50c1e29cce` (depuis `main` 5a33ec1f ; `main` refusionné après ZG4,
PF1, PB1, puis epic + bulles3 le 2026-09-25 ; tests zg2, zg5b, smoke verts après fusion). Cache partagé par liens symboliques non versionnés `data/map/pyramid`, `tools/geo/raw`.
ADR 0036, contrat `docs/geo.md` § « Hydrographie fine ». Doc : `docs/godot-map.md` § « Hydrographie
fine, routes drapées, ancrages et parcellaire de près (lot ZG5b) ».

## Architecture (choix)
- `cafv_tile.gd` : lecture CAFV v1 (en-tête de **28 octets** ; `docs/geo.md` corrigé) ; **rang** =
  max(ordre, ordre équivalent à la largeur) car ZG5a sous-estime l'ordre des grands fleuves.
- `fine_geo_store.gd` : index + ancrages, tuiles dans des fils, LRU verrouillé (`fetch_threadsafe`).
- **Lit creusé = abaissement des pages ≥ E3** (`fine_bed_carver.gd`, crochet `page_filter` de
  `relief_quadtree.gd`, tâche de fil par page, E/S comprises). Ancien lit `river_bed.png` non creusé.
- `fine_ribbon_job.gd` (fil) + `fine_geo_layer.gd` (`Rivers/FineGeo`) : rubans fleuves + routes par
  tuile E2, hauteurs en mètres × `campaign_vertical_scale`, fondu `fine_zone`, ponts-portes fins,
  qualité PF1, `FrameBudget` PB1.
- `river_fine.gdshader`, `road_fine.gdshader`, `fine_parcels.gdshaderinc` (3 crochets d'une ligne dans
  `terrain.gdshader` : `fp_splat`, `fp_tile_level`, `fp_parcels`).
- Ancrages : `SettlementLayer.apply_fine_anchors`, `RiverCrossings.set_fine_anchors/set_fine_mode`
  (ponts à l'échelle réelle, tablier à `z_deck`).

## État
- [x] lecture, store, lit, rubans, fondu, ancrages, parcellaire, splat, détail, qualité, test
- [x] fusions de `main` (ZG4, PF1, PB1)
- [x] doc `docs/godot-map.md`
- [x] captures `docs/img/zg5b/` et mesures dans la doc ; smoke OK, tests zg2/zg5b OK

## État : terminé (à fusionner)

## Points ouverts (pour la fusion)
- Fichiers partagés avec ZG4 : `campaign_map.gd` (2 lignes : `attach_roads`, `flush_fine` ; ponts
  visibles au palier site si rendu fin), `settlement_layer.gd` (ancrages), `relief_quadtree.gd`
  (crochet `page_filter`), `river_crossings.gd`.
- Villes emblématiques (zones personnalisées : Paris, Londres, Rouen, Bordeaux…) : rien de fin dedans
  (comme V4) ; London Bridge / ponts de Rouen relèvent de VH4.
- Coût : ≈ 10 % d'i/s sur le banc complet (machine chargée), surtout la concurrence des fils
  (≈ 200 ms de maillage par tuile, 8-30 ms de creusement par page).
