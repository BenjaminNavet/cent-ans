# Lot « tests d'UI en échec » (branche `fix/ui-tests`)

## État
- `q6_ui_test` : passe. Test périmé (dépendance de compilation vers `DiplomacyPanel` avant les
  autoloads ; choix de décision cherchés dans le corps défilant alors que Q7 les en a sortis) et
  un défaut du jeu (double connexion `size_changed` dans `ChronicleWindow.show_decision`).
- `fe_ui_test` : en cours (cadrage du sélecteur de faction sur carte après OM).

## Prochaine étape
- Dériver l'attente de cadrage de `fe_ui_test` de l'emprise des provinces jouables.
- Vérifier `smoke`, `q6_diplomacy_test`, `vn_ui_720_test`, `ub1_ui_test`, puis mettre à jour `docs/wip/vn.md`.
