# Lot campui — mise en page des panneaux de campagne (post-playtest 17/10)

Branche `fix/campui`. Sources : `docs/wip/playtest-2026-10-17/ui-campagne.md`, `ux.md`.

## État (final)
- 1 Recrutement : corrigé (cartes min 120 px, « Bientôt » liste défilante 52 px, pied non écrasable, page défilante en dernier recours). Sonde rec:panel/rec:cards OK à 1280x800.
- 2 Économie, 3 Traditions : corrigés (`tab_pane_fit`, `shrinkable`, liste d'options défilante). Sondes EconomyPanel/Diplomacy/Court/Faction OK.
- 4 Échelle : `Settings::fitted_ui_scale` plafonne l'échelle pour garder une fenêtre logique >= 1150x640 ; barre du haut tient (`top_bar_fits`) y compris ui 1.6.
- 5 Sélection de faction : racine/colonne `min_height: 0`, boutons visibles (dump `menu.faction_choice.layout.actions_visible`) ; cartouche du fond retiré (`menu_map_plain.jpg`) ; Échap = retour au menu.
- 6 Lieu de l'armée : nom affiché. 7 `close_button` (« Fermer »).
## Restes
- Sondes HUD en échec avant comme après (padding < marges de texture) : hud:top_bar, hud:minimap, hud:turn_wait, hud:hint, hud:journal, hud:map_filters_menu.
- Texte 1.3 sur zone courte : le recrutement défile (pas de débordement) mais reste à l'étroit.
- Traditions : une ligne de légende dépasse de ~11 px dans le padding droit.
