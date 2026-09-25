# ZG4b — correctifs de la vue rapprochée relevés en recette (Q3)

Worktree d'agent (depuis `main`, refusionné après ZG6 7f38c532 puis MF1 / d80007a9). Liens symboliques
non versionnés `data/map/pyramid`, `tools/geo/raw`. Dylib : `CARGO_TARGET_DIR=/Users/jean_hubert/dev/game_project/core/target`.
Rendu seulement. Captures : `godot --path game res://scenes/start_menu.tscn -- --autostart=fac_england
--hide-armies --screenshot=… --focus=2018,1486.6,<d>` (Londres ; joueur anglais, sinon le brouillard de
guerre voile Londres). Doc : `docs/godot-map.md` § « Correctifs de recette (lot ZG4b) », addendum ADR 0036.

## État : terminé (à fusionner)
- [x] plancher déclaratif `landmark_min_distance` = 2,6 (`close_camera.tres`), levé par VH4
- [x] brouillard matinal météo atténué de près (`weather_mist_near`), cause principale du sol beige
- [x] parcellaire ZG5b sur les terres basses relevées à 0,5 m (`hc < 0.25`)
- [x] ponts-portes fins à l'échelle réelle (`FineGeoLayer._build_gates`)
- [x] bascule des ponts étalée (`RiverCrossings.pump_reshape`, `FrameBudget`), maillages gardés par mode
- [x] tests zg2, zg4 (plancher), zg5b (bascule étalée), smoke : OK après fusion de main
- [x] captures `docs/img/zg4b/`, doc, ADR

## Hors lot (signalé)
- Ruban rouge et gris de Londres = `TradeRouteLayer` (C5) : `refresh()` force `_wanted_visible = true`
  (couche visible sans le mode Commerce ; toujours présent après MF1).
- `PathPreview` (aperçu de chemin, orange ~1 km) à l'échelle de la carte aux paliers vallée / site.
- Pic de bascule non remesuré sur le banc (`--bench-descent-only`) ; attendu ≤ budget 6 ms par image.
