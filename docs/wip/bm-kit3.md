# bm-kit3
Fait : `ui/kit/text_field.rs` (champ de texte, `TextInputFocus`, `text_input_active`/`keyboard_free`, messages `TextFieldChanged`/`TextFieldSubmitted`, hook `CENT_ANS_TYPE`), Échap du `PanelStack` via `KeyEdges` (consomme la touche, avant `pause_escape`, ignoré pendant la saisie).
Son : l'audio lit déjà `ButtonPressed` et joue `ui_click` (audio/mod.rs `button_clicks`) ; pas de `UiSound` nécessaire.
Restes : migrer les champs maison (menu/saves.rs `NameField`, codex `SearchText`, diplomatie, menu/custom_battle.rs) ; les raccourcis des autres lots doivent tester `TextInputFocus` ; commande agent `type <texte>` (agent/ non possédé).
