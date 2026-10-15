# bm-pdiplo : écran de diplomatie (Bevy)

État : game-api `diplomacy.rs` + `client/src/campaign/panels/diplomacy/` portés (liste, fiche, négociation, guerre, traités, offres, actions).
Preuve : `client/out/pdiplo.txt` (script) -> dump section `diplomacy`.
Reste : carte diplomatique centrale, bandeau de cour, ligne de chancellerie/don à l'Église, arbitrage féodal, recherche texte, saisies libres d'or/tribut.

## diplo2 (vague 4)
Fait : bandeau de cour (`game_api::diplomacy::court_banner`, offres / trêves et pactes <= 2 tours / ligue), recherche texte au clavier (`DiplomacyUi::query`, filtre `visible_rows`), dump `query`/`court`. Preuve : `client/src/campaign/panels/diplomacy/proof_diplo2.txt` (frappe `e`,`n` -> 13/164 visibles).
Reste : carte diplomatique centrale, tendances d'attitude (flèches), ligne de chancellerie / don à l'Église, arbitrage féodal, saisies libres d'or/tribut.
