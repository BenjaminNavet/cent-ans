# Lot battleui : correctifs post-playtest du 17/10

Branche `fix/battleui`. État : en cours.

## Fait (à vérifier en capture / tests)
- 1. Dialogue pré-bataille : corps défilant (`scroll_body` du kit), bannière et pied (Retraite / Résolution automatique / Combattre) fixes ; grille des cartes à largeur définie (plus de chevauchement). Capture OK.
- 2. Écran de résultat : corps défilant avec barre, pied fixe (« Revoir la bataille », « Retour à la campagne »). Doublon `replay:watch` supprimé.
- 3. Titre et sous-titre en encre (rubrique pour une défaite) : `illuminated_title_colored`.
- 4. Survivants : `SideSummary.pursuit`, ligne « dont N perdus dans la poursuite » ; spectateur : « Fin de la bataille ».
- 5. `fr::de_place` (testé), utilisé pour les titres de bataille / siège.
- 6. « côte ouest » : cause = `coastal` de la province (Île-de-France a l'estuaire de la Somme) ; core `battle_request.rs` : un champ de bataille n'est côtier que près d'un port.
- 7. Aide F1 au-dessus du didacticiel et défilante ; icônes de repli pour « Unité ralliée » / « La place est menacée » ; « Nuit dans » sans retour à la ligne.

- 6 (suite). « ÉDOUARD III » au siège de Paris : non reproduit comme erreur d'orateur. Le discours du joueur vient bien de son général (« LE CAPITAINE » dans la démo, camp français) ; la signature d'Édouard III est le discours adverse (étape `Enemy`, vérifié par dumps `battle_speech`). Il est maintenant marqué « (ennemi) » (`speaker_label`).
- 7 (suite). Cartes du bilan : l'illustration existait mais était écrasée dans la carte portrait ; recadrée comme les cartes du HUD.
- Test core : `an_inland_field_battle_has_no_coast`.

## Reste / hors lot
- Siège / Île-de-France dans les panneaux de campagne (`campaign/panels/siege/view.rs:153`, `Siège de {settlement}`) : hors lot, élision à voir si un nom de colonie commence par une voyelle (utiliser `fr::de_place`).
- Bulle du conseiller par-dessus les modales : lot `overlays`.
- Mode « Regarder » : titre neutre couvert par test unitaire ; pas rejoué en jeu (15 min de simulation).
