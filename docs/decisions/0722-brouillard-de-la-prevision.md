# 0722 - Brouillard de la prévision de bataille

Statut : accepté (lot M5, `tx/m5-prevision`)

## Contexte
`battle_forecast.rs` affiche une prévision exacte (échantillons de l'auto-résolution). TWW3 bruite la prévision selon la précision (`prediction_variation` 0,15 / 0,10 / 0,05 / 0,01).

## Décision
- Le bruit touche **uniquement l'affichage** de la prévision (probabilité, barre, pertes) ; `battle_auto.rs` et la bataille 3D ne changent pas, aucun recalibrage `balance_probe`.
- Niveau de connaissance de l'armée ennemie : non vue (0,15), vue (éclaireurs / masque de vision, 0,10), espion proche (0,05), espion + place amie en vue (0,01). Armées de son camp ou alliées : exactes.
- Bruit multiplicatif `1 ± spread`, déterministe (graine = identité de la bataille + saison), sans toucher au RNG de campagne ; l'IA (`assault_win_chance`, IA contre IA) lit la valeur exacte.
- Valeurs dans `data/rules/auto_resolve.json` (`forecast_fog`).

## Conséquences
La prévision peut se tromper dans la fourchette ; les modificateurs contextuels (embuscade, gué, siège, nuit) existent déjà dans l'auto-résolveur et sont listés dans la prévision ; la courbe de vitesse des tirailleurs reste hors lot.
