# Q6 — avis (zone TOASTS) au-dessus des fenêtres, repli du texte

Test : `game/tests/q6_toasts_test.gd` (vert).

## État
- [x] Test rouge : registre des agents couvert par les avis/journal, « Colonies » aussi, journal et avis plus larges que la zone en vue étroite.
- [x] Ordre d'affichage : zone TOASTS à l'étage HUD (`ui_layout.gd`), BANNER seulement pendant le bandeau de fin de tour (`map_ui.gd`) ; fenêtre bloquante hors pile = étage PANEL (`panel_stack.gd`).
- [x] Largeur : `LogScroll` sans largeur minimale (`map_ui.gd`), encart « Conseil » sans largeur minimale et titre replié (`next_hint_card.gd`).
- [x] Non-régression : po_ui, p2c/p2d/p2g, ux2, ui_panel_stack, vo1, q6_ui, smoke (sortie 0) ; q3_playtest en cours.

## Prochaine étape
Terminé (reste : résultat du pilote q3_playtest).
