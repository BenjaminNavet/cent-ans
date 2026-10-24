# CREDIT-UI — interface du crédit et des impôts (ADR 0780)

## État : terminé
- [x] Données : `data/ui/finance_panel.json` + schéma + test_schemas (1290 ok).
- [x] `faction/finance.rs` : 5e onglet « Finances » (barres, jauge, banquiers, emprunt, remboursement, impôts, historique), ordres via `game_api::diplomacy::submit_order`, alerte conseiller (`finance:risk`, seuil 60 %), annonce de banqueroute, raccourci Maj+J (`campaign_toggle_finances`), dump `finance_panel`.
- [x] Vérifié hors écran (`dev,system-alloc`, voir piège) : emprunt, remboursement, impôt (aide) passent ; alerte au bandeau à 100 % ; `ui_kit.layout.failures` : seuls des cadres HUD hors lot (`hud:journal`, `hud:feed`, `advisor:band`, `hud:map_filters_menu`, moins d'un pixel de marge basse) à 1280x800 et 1920x1080, 0 à l'échelle 1,25.
- [x] clippy `--features dev --all-targets` propre ; tests `campaign::panels::faction` + `settings::keymap` ok.

## Pièges
- Sous `dev` (liaison dynamique de Bevy) le client plante au démarrage (mimalloc / allocateur de la dylib) : lancer avec `CENT_ANS_FEATURES=dev,system-alloc`.

## Points ouverts
- La ligne « Cour et administration » du cœur croît avec le trésor : emprunter 124 k la fait passer de -4,7 k à -48 k par saison (à examiner côté ECO, pas ici).
- Pas de curseur : paliers seulement (ADR 0780).
