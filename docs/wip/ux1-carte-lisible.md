# UX1 — Carte lisible (audit A3 : C8, C14, U14)

Branche : `worktree-agent-a90f2668279e9e81c`. Plan d'ensemble : `docs/wip/ux-prise-en-main.md`.

## Contenu
1. Plaques d'effectif d'armée décalées hors des noms de ville et des autres plaques
   (`LabelPlacer`, grille spatiale, hystérésis ; recalcul quand la caméra bouge).
2. Légende de la carte : bouton « ? Légende » dans la rangée de la minicarte, panneau parchemin
   (`MapLegend`), textes dans `data/ui/map_legend.json` (schéma `map_legend.schema.json`),
   sections selon le mode de carte (politique, mécontentement, diplomatie, religion).
3. Boutons de la minicarte au thème parchemin.

## État
- [x] `label_placer.gd` (logique pure), données + schéma + test Python
- [ ] branchement dans `army_markers.gd` (obstacles : `SettlementLayer` / `CityMarkers.screen_label_rects`)
- [ ] `map_legend.gd` + `legend_sample.gd`, bouton dans `campaign_minimap.gd`
- [ ] thème parchemin minicarte
- [ ] stage `legend` dans `campaign_map.gd`, test `game/tests/ux1_test.gd`, captures `docs/audit/captures/ux1/`

## Prochaine étape
Brancher le placement dans `ArmyMarkers._update_plates`.
