# DZ — frontières diplomatiques (2026-09-28, nuit)

Demande : en mode Diplomatie, frontières rouges pour les ennemis ; en sélectionnant une faction,
voir ses ennemis / neutres / alliés.

## Fait
- Cœur (pont) : `get_province_stances_for(viewer, ids)`, `get_faction_stances_for(viewer)`
  (rebelles toujours « guerre ») ; `get_province_stances` délègue au joueur.
- `FactionBorders.set_color_override(colors, highlight)` : palette des traits imposée (couleur de
  position), halo respirant sur la faction observée.
- `MapModeController` : `focus_faction` ; clic sur une province en mode Diplomatie = son
  contrôleur devient observé ; clic sur nos terres / hors carte = retour. Légende et survol
  nommés. Opacité des frontières en Diplomatie 0.75 → 1.0.
- Panneau de diplomatie : carte = relations de la faction choisie (bascule « Vue de la faction
  choisie »).
- Tests : smoke (pont), `game/tests/dz_diplo_borders_test.gd`.

## Prochaine étape
Build, tests, capture de contrôle, fusion ff-only dans main.
