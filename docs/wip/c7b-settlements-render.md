# Lot C7b : rendu des colonies (arbres, routes, chemins, panneaux)

Spec : `docs/design/2026-09-24-echelle-colonies.md` § 6. Suit C5 (`docs/wip/c5-settlements-ui.md`)
et C6 (`docs/wip/c6-zoom-tiers.md`). Branche : `worktree-agent-aa38756fc56f579e1` (partie de `main` `ff21e60`).
Périmètre : rendu Godot (et pipeline géo si besoin) ; C7a touche `core/` en parallèle.

## État

- [ ] 1. Arbres recalés sur le relief fin (`Vegetation`, signal `chunk_surface_changed`)
- [ ] 2. Routes principales lisibles au palier moyen (`RoadRenderer`)
- [ ] 3. Aperçu de chemin d'armée le long des routes réelles
- [ ] 4. Panneaux de colonie / province sans recouvrir la minicarte
- [ ] 5. Captures avant / après `docs/img/colonies/c7b-*.png`

## Décisions

## Prochaine étape

Tâche 1.
