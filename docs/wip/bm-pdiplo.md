# bm-pdiplo : écran de diplomatie (Bevy)

État : game-api `diplomacy.rs` + `client/src/campaign/panels/diplomacy/` portés (liste, fiche, négociation, guerre, traités, offres, actions).
Preuve : `client/out/pdiplo.txt` (script) -> dump section `diplomacy`.
Reste : carte diplomatique centrale, bandeau de cour, ligne de chancellerie/don à l'Église, arbitrage féodal, recherche texte, saisies libres d'or/tribut.

## diplo2 (vague 4)
Fait : bandeau de cour (`game_api::diplomacy::court_banner`, offres / trêves et pactes <= 2 tours / ligue), recherche texte au clavier (`DiplomacyUi::query`, filtre `visible_rows`), dump `query`/`court`. Preuve : `client/src/campaign/panels/diplomacy/proof_diplo2.txt` (frappe `e`,`n` -> 13/164 visibles).
Reste : carte diplomatique centrale, tendances d'attitude (flèches), ligne de chancellerie / don à l'Église, arbitrage féodal, saisies libres d'or/tribut.

## diplo3 (vague 5)
Fait : ligne de chancellerie + don à l'Église (`game_api::diplomacy::{religion_view, chancery_text, DONATION_AMOUNT}`, bouton `dip:donate`), arbitrage féodal (boutons `dip:arbitrate:<offre>:<parti>` via `game_api::feudal::arbitrate`, "Soutenir X"), flèches de tendance d'attitude (cache `previous`/`seen` par tour, `format::trend`), saisies libres d'or et de tribut (`dip:field:<gold|tribute>:<offer|demand>` puis chiffres + Entrée, ou `dip:addamt:*`; plafond trésorerie / 20000), touche F3 via `KeyMap` (`map_toggle_diplomacy`), Échap via `KeyEdges` avant la pause (ferme d'abord un champ en saisie). Carte : à l'ouverture le mode `Diplomacy` est demandé (`SetMapMode`) et `MapModeState::focus_faction` suit la faction choisie ; le mode précédent revient à la fermeture.
Preuve : `proof_diplo3.txt` (dump `diplomacy` : `chancery`, `can_donate`, `field`, `trends`, `arbitration_calls`).
Reste : carte diplomatique centrale propre (`diplomacy_map_view.gd`), lien « Commerce → Économie », champ de recherche sans widget texte du kit.
