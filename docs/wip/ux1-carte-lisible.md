# UX1 — Carte lisible (audit A3 : C8, C14, U14)

Branche : `worktree-agent-a90f2668279e9e81c`. Plan d'ensemble : `docs/wip/ux-prise-en-main.md`.

## Contenu
1. Plaques d'effectif d'armée décalées hors des noms de ville et des autres plaques
   (`game/scripts/map/label_placer.gd` : 10 positions candidates, grille spatiale de 64 px,
   hystérésis de 6 px ; recalcul quand la caméra bouge, sinon toutes les 0,25 s).
   Obstacles : `SettlementLayer.screen_label_rects` et `CityMarkers.screen_label_rects`
   (rectangle mesuré avec la police du `Label3D`, `LabelPlacer.label3d_screen_rect`), branchés
   par `campaign_map.gd` sur `ArmyMarkers.label_obstacles`.
2. Légende de la carte : bouton « Légende » (icône Codex) dans la rangée de la minicarte,
   panneau parchemin `MapLegend` à gauche de la minicarte, échantillons `LegendSample`
   (formes du shader des colonies redessinées, vraie plaque `ArmyMarkers.build_plate`, vrai
   jeton d'agent, étendard de faction, couleurs de relation). Textes : `data/ui/map_legend.json`
   (schéma `data/schemas/map_legend.schema.json`, test `tools/tests/test_map_legend_schema.py`).
   Sections par mode de carte : politique, mécontentement (M), diplomatie (N), religion (R).
3. Minicarte au thème parchemin (`parchment_theme.tres`) : Politique, Relief, Commerce, Légende.

## État
- [x] placement des plaques + obstacles
- [x] légende (données, schéma, panneau, bouton, suivi du mode de carte)
- [x] thème parchemin minicarte
- [x] stages `legend` / `legend_armies`, test `game/tests/ux1_test.gd` (« ux1 OK »)
- [x] captures `docs/audit/captures/ux1/` (avant = captures UI3 ; après = carte 1280, légende 1600)
- [ ] smoke, `git merge main`

## Prochaine étape
Smoke, fusion de main, rapport.

## Limites connues
- Les jetons d'agents (couche 2D à part) ne sont pas des obstacles pour les plaques.
- Les noms de ville qui se chevauchent entre eux restent gérés par le masquage existant
  (declutter) ; UX1 ne déplace que les plaques d'armée.
- Captures non headless : une fenêtre Godot qui prend le focus peut recevoir des touches
  tapées ailleurs (une capture a montré la carte religieuse et le Codex ouverts) ; refaire la capture.
