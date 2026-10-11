# RG1 — fenêtres redimensionnables

État : implémenté, en test.
- `game/scripts/ui/window_resize_grip.gd` : poignée coin bas droit (top_level, curseur FDIAGSIZE), posée par `PanelStack.register` pour CENTRAL et COMPANION.
- Méta `ui_user_sized` ; `MapUI._keep_on_screen` réduit aussi `custom_minimum_size` si l'écran rétrécit ; `HoldingsController._layout` laisse la liste remplir la taille choisie.
- Taille mémorisée par nom de panneau (statique, session).
- Test : `game/tests/rg1_resize_test.gd`.
Reste : bords droit/bas non faits (coin seul) ; panneaux à largeur fixe autres que Colonies à vérifier à l'œil.
