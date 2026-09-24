# Lot C7b : rendu des colonies (arbres, routes, chemins, panneaux)

Spec : `docs/design/2026-09-24-echelle-colonies.md` § 6. Suit C5 (`docs/wip/c5-settlements-ui.md`)
et C6 (`docs/wip/c6-zoom-tiers.md`). Branche : `worktree-agent-aa38756fc56f579e1` (partie de `main` `ff21e60`).
Périmètre : rendu Godot (et pipeline géo si besoin) ; C7a touche `core/` en parallèle.

## État

- [x] 1. Arbres recalés sur le relief fin (`Vegetation`, signal `chunk_surface_changed`, `VegetationGroundJob`) ; test dans `settlements_render_test` (écart 0,00 contre 0,28 avant)
- [ ] 2. Routes principales lisibles au palier moyen (`RoadRenderer`)
- [ ] 3. Aperçu de chemin d'armée le long des routes réelles
- [ ] 4. Panneaux de colonie / province sans recouvrir la minicarte
- [ ] 5. Captures avant / après `docs/img/colonies/c7b-*.png`

## Décisions

- Arbres : semés directement sur la grille du maillage affiché (`TerrainBuilder.surface_grid`) et
  recalés dans une tâche `WorkerThreadPool` à chaque changement de niveau d'une tuile (fin, proche,
  lointain), pas seulement pour le relief fin. Tampons CPU gardés par tuile (64 o par instance).
  Tuile hors champ : recalée quand elle y revient. Coût mesuré ≈ 6 ms de fil par tuile.
- Les candidats d'arbres qui débordaient de leur tuile (dernière colonne de la grille, gigue des
  haies) sont écartés ou ramenés dans la tuile : leur pied était posé sur le maillage d'une autre
  tuile (écart 0,28 mesuré) et la bande de bord était deux fois plus dense.

## Prochaine étape

Tâche 2 (routes au palier moyen). Captures « avant » déjà prises (`c7b-avant-*.png`).
