# DN ui-kit — kit d'enluminure complété (08/10)

Branche `dn/ui-kit`. Générateur procédural `tools/cent_ans_tools/ui_illumination.py` (sans GPU) :
`uv run --project tools cent-ans assets ui-illumination`. Les nouvelles pièces sont ajoutées en fin de
liste (graines par index) : les 14 PNG historiques restent identiques octet pour octet (test).

## Pièces ajoutées (`game/assets/ui/illumination/`)
- `button_primary_{normal,hover,pressed,disabled}` : champ d'azur, bande d'or, filet de blanc de plomb,
  losanges de gueules aux angles. Survol azur clair, pressé azur profond ombré, désactivé gris-bleu.
- `tab_ornate_{selected,unselected}` : onglet actif (tête d'or, languette de gueules, filet d'azur).
- `progress_frame` / `progress_fill` : auge de vélin à filets d'or, remplissage de gueules.
- `slider_grabber(_hover)` : cabochon de cire rimé d'or (icône 22 px, pas un 9-patch).
- `separator_h` : filet d'or à fleurons quadrilobés aux extrémités (9-patch horizontal).
- `inset_ornate` : cadre de lecture (encre, or, filet de gueules, losanges d'azur).
- `initial_frame` : cadre de lettrine (azur et or) ; la lettre est posée par la police du thème.

## Thème `parchment_theme.tres`
Nouvelles variations : `PrimaryButton` (Button), `OrnateProgress` (ProgressBar, remplissage gueules),
`FleuronSeparator` (HSeparator, séparation 12), `InsetFrame` et `InitialFrame` (PanelContainer).
Par défaut : TabContainer/TabBar passent aux onglets ornés ; ProgressBar prend l'auge ornée (remplissage
azur conservé) ; HSlider prend la poignée de cire.

## Boutons principaux posés (`theme_type_variation = &"PrimaryButton"`)
Confirmer (`confirm_panel.gd`), Combattre (`pre_battle_dialog.gd`, remplace les styleboxes plates),
Lancer la bataille (`custom_battle_screen.gd`), Proposer le traité (`diplomacy_negotiation_tab.gd`),
Accepter/Intervenir/Obéir (`diplomacy_offers_section.gd`). Fin de tour (cloche `end_turn_cluster.gd`) et
Commencer (menu, plaques FA5) gardent leur rendu propre.

## Tests
`game/tests/dn_ui_kit_test.gd` (variations présentes, widgets), `tools/tests/test_ui_illumination.py`.
Planche de revue (gitignorée) : `docs/img/dn/ui_kit_sheet.png` (`ui_illumination.contact_sheet`).

## Reste
Relecture visuelle par l'orchestrateur ; FleuronSeparator et InsetFrame/InitialFrame à brancher dans les
écrans (aucun usage posé) ; lettrine = cadre seulement (pas de lettre ornée peinte).
