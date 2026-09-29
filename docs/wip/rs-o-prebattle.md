# RS-O — avant-bataille qui déborde à 1280×640

Branche `feat/rs-o-prebattle`. Défaut décrit dans `docs/wip/p2d-sieges.md` (§ « Défaut
préexistant ») : `PreBattleDialog._layout()` bridait `panel.size`, mais la taille minimale des
colonnes de régiments dépassait l'écran.

## Constat à la reprise

`main` contient déjà un correctif (65dd44521, « pre-battle dialog fits 1280x640 ») : colonnes
d'armées et conditions dans un `ScrollContainer` (`_sides_scroll`), bannière réduite sur écran bas,
re-layout sur `minimum_size_changed`, C2 de `p2d_ui_test.gd` déjà bloquant (taille du panneau).

## État

- [x] C2 de `p2d_ui_test.gd` renforcé (`_check_panel_fits`) : rectangle global du panneau dans
      l'écran (position comprise), taille minimale combinée ≤ écran, boutons d'action dans le
      panneau. Mutation vérifiée : sans défilement vertical des colonnes, 4 échecs à 1280×640
      (minimum 724 px pour 711 px d'écran logique).
- [x] tests verts : p2d_ui_test, smoke, po_ui_test, nv1_naval_test, ub1_ui_test.
- [ ] `ib_plain_test` rouge **préexistant, hors lot** : littéral `tooltip_text` dans
      `game/scripts/ui/diplomacy_panel.gd` (fichier non touché ici).

## Prochaine étape

Lot terminé ; fusion par l'orchestrateur. Aucun changement de `pre_battle_dialog.gd` /
`naval_pre_battle_dialog.gd` nécessaire (correctif déjà dans `main`, 65dd44521).
