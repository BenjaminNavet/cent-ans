# RS-O — avant-bataille qui déborde à 1280×640

Branche `feat/rs-o-prebattle`. Défaut décrit dans `docs/wip/p2d-sieges.md` (§ « Défaut
préexistant ») : `PreBattleDialog._layout()` bridait `panel.size`, mais la taille minimale des
colonnes de régiments dépassait l'écran.

## Constat à la reprise

`main` contient déjà un correctif (65dd44521, « pre-battle dialog fits 1280x640 ») : colonnes
d'armées et conditions dans un `ScrollContainer` (`_sides_scroll`), bannière réduite sur écran bas,
re-layout sur `minimum_size_changed`, C2 de `p2d_ui_test.gd` déjà bloquant (taille du panneau).

## État

- [ ] vérifier les tests (p2d, smoke, po_ui, ib_plain, nv1, tests PreBattleDialog)
- [ ] renforcer C2 : rectangle global du panneau dans l'écran (position comprise), pas seulement
      la taille

## Prochaine étape

Lancer les tests après `--import`.
