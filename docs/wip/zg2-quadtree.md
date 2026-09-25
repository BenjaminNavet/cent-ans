# ZG2 — moteur de relief streamé en quadtree (ADR 0036)

Branche `zg2-quadtree` (worktree agent). Liens symboliques non versionnés : `data/map/pyramid`,
`tools/geo/raw` → dépôt principal.

## Architecture retenue (résumé, détail dans `docs/godot-map.md`)
- `ReliefPyramid` : manifeste (RLE → ensembles de tuiles par étage), E0 = `data/map/height/`.
- `ReliefQuadtree` : quadtree CDLOD sur toute la carte (racine 4096 unités, profondeur n,
  étage de tuile L = n − 4), sélection par distance équivalente à l'erreur à l'écran
  (espacement des sommets projeté ≤ `max_vertex_px`), morphing géomorphe + jupes, patch fixe
  partagé (64 quads, demi-patch 32 pour les quadrants), pages `Texture2DArray` R16 + mipmaps,
  paramètres de nœud en `instance uniform`, décodage PNG dans `WorkerThreadPool` (Rust
  `GameDataStore`, repli `Png16`), LRU, fondu des pages.
- Shader : `relief_quadtree.gdshaderinc` + 3 crochets d'une ligne dans `terrain.gdshader`.

## État
- [x] ReliefPyramid
- [x] ReliefQuadtree (sélection, pages, décodage, LRU)
- [x] Shader include + crochets (`qt_vertex`, `qt_relief`, `qt_debug_color`)
- [x] Intégration TerrainBuilder (surface_height_at, chunk_surface_changed, grilles instantanées)
- [x] Pyramide synthétique (`game/tests/fixtures/zg2/make_pyramid.py`) + test headless
  `game/tests/zg2_quadtree_test.gd` (OK)
- [x] Banc `--bench-map` (`game/scripts/dev/map_bench.gd`), options `--pyramid-dir=`,
  `--no-pyramid`, `--qt-debug=1|2`, `--camera-min=`
- [x] Décodeur Rust asynchrone `ReliefDecoder` (fils natifs, `request`/`poll`)
- [x] Docs godot-map.md (section « Relief streamé »)
- [x] Captures `docs/img/zg2/` (Chartreuse, Rouen, puy de Dôme avant/après)
- [x] Mesures du banc (relatives, charge 200+), délai de repos 700 ms des recalages par pages
- [x] Addendum ADR 0036, test de fumée OK, test ZG2 OK

## Décisions en cours de route
- godot-rust est mono-fil (panique si `GameDataStore` est appelé hors fil principal) : décodage
  Rust sur le fil principal, ≤ 2 tuiles et ≤ 4 ms par image (≈ 3 ms par tuile) ; repli GDScript
  `Png16` dans `WorkerThreadPool` (≈ 100 ms à 1 s par tuile : inutilisable pour du streaming).
- Recalages : les pages qui arrivent ne signalent que les morceaux proches (niveau ≥ 1), au plus
  2 morceaux toutes les 250 ms.

## Prochaine étape
Lot terminé (en attente de fusion). Pyramide réelle d'essai :
`python3 game/tests/fixtures/zg2/manifest_from_cache.py <dossier>/relief_pyramid.json` (lien
`<dossier>/pyramid` → `data/map/pyramid`), puis `--pyramid-dir=<dossier>`.
Suites : banc sur machine au repos (cible 60 i/s), recalages des maquettes (`landmark_model`,
50-300 ms synchrones) à rendre incrémentaux, caméra rapprochée (ZG4).

## Mesures
Voir `docs/godot-map.md` (fin de la section « Relief streamé ») et l'addendum de l'ADR 0036.
