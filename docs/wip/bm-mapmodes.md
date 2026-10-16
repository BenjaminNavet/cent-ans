# bm-mapmodes : les cinq modes de carte manquants

Branche `bm/mapmodes`. Ports de `map_mode_controller.gd` et `feudal_map_lens.gd`.

## Fait
- `game-api::lens` : `lens_view(data, state, viewer, LensWanted)` donne par province revenu, population,
  mécontentement, loyauté du vassal, suzerain, ravitaillement, revendication, cellule religion (foi du
  contrôleur, hérésie, conversion) et cellule féodale (souverain, tenant, second seigneur). Seule la
  lecture, aucune règle ; `supply_loss_winter` vient de `economy_rules`.
- `provinces/modes.rs` : `MapMode::{Religion, Loyalty, Supply, Claims, Feudal}`, couleurs de Godot,
  `province_hatch` (couche des tenants) et `double_allegiances`.
- `provinces/shaders/hatch.wesl` : hachures partagées (inlinées dans `border.wesl` et `fill.wesl`),
  `fill.wesl` rayure la couche des tenants sur le fond du souverain.
- `provinces/shields.rs` : écu parti des doubles allégeances (noeuds UI projetés, armoiries `heraldry/*.png`
  coupées en deux).
- Touches R / N / Maj+M (actions `map_mode_religion`, `map_mode_diplomacy`, `map_toggle_unrest`),
  boutons du menu Filtres (parcourt `MapMode::ALL`), infobulles de survol (`hud/hover.rs`),
  légende (`data/ui/map_legend.json` : sections loyalty/supply/claims/feudal, couleurs de la religion mises
  à jour sur Godot).

## Écarts
- Icône du mode Féodalité : `glyph_crown` (Godot n'en a pas).
- Écu : rectangle à pointe arrondie par `border_radius`, pas le masque pixel de Godot.
- Légende : fenêtre « Légende » existante (pas de panneau permanent en bas d'écran).
- Pas de saturation/alpha renforcés propres aux modes (`MODE_TINT_*`) : le lavis multiplicatif est commun.

## Restes
- Mode Diplomatie : symboles daltoniens (lot access2) non touchés.
- Bordures : pas d'entrée `feudal` dans `faction_borders.json` `modes` (retombe sur politique).
