# 0144 — Ville détaillée plus haut, toits lointains de la couleur des blocs

Date : 2026-09-30. Statut : acceptée. Amende l'ADR 0138.

## Contexte
Retour du joueur sur Paris (vue rapprochée) : la ville détaillée 1:1 ne s'affichait que sous
`max_rig_distance` = 16 et ses blocs sous `block_range` = 14 (unités, 1 u ≈ 719 m). À une distance
intermédiaire, on voyait donc la partie proche en blocs à tuiles rouges et le reste en maillage
lointain brun foncé (teinte « masse de toits » moyennée sur tuile, chaume, ardoise, plomb). Le
joueur trouve ce contraste gênant et veut la ville détaillée plus haut.

## Décision
- `max_rig_distance` 16 → 45, `block_range` 14 → 42, `stream_max` 18 → 48
  (`resources/town_render.tres` et valeurs par défaut de `TownRenderProfile`). L'enfoncement du
  lointain suit (`block_range` × qualité × `sink_factor`). Les facteurs de qualité sont inchangés
  (bas : blocs jusqu'à 25).
- Toits du maillage lointain : moyenne de la couche `RoofTile` × teinte des blocs
  (`TownBuilder.BLOCK_ROOF_TINT`), avec la variation par cellule de la masse de toits ; le passage
  bloc → lointain garde la même couleur (`town_far.gdshader`, `block_roof_layer`).

## Conséquences
- Plus de villes chargées en 1:1 aux hauteurs moyennes (rayon de chargement jusqu'à 48 u) : coût
  mesuré dans `docs/wip/vt.md` (banc d = 30 et 40).
- Les villes vues de très loin prennent la teinte de tuile au lieu du mélange de couches.
