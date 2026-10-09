# 0273 — Rayon de renfort et prévision de bataille

Statut : accepté

## Contexte
Les alliés ne rejoignaient une bataille qu'à 5 km (`engage_radius_km`) alors qu'une armée parcourt ~105 km par saison (rapport WH `armees`, A3/A12).

## Décision
- `data/movement/rules.json` : `reinforce_radius_km` (25, 0 = désactivé). `battle_coalition` ajoute une armée alliée en guerre contre l'adversaire si sa distance ≤ ce rayon et si son mouvement restant (points de plaine pour la distance au-delà du rayon d'engagement) l'y porte ; une armée en posture de siège ne quitte pas son siège.
- Les mêmes coalitions servent à la résolution et à la prévision : `Reinforcement` gagne `distance_km` et `late` ; le pré-bataille affiche « à N km » et une ligne « N armée(s) alliée(s) rejoignent de loin ».
- Prévision « lève le siège » : `BattleForecast.lifts_siege` / `siege_place` quand un des camps est l'armée assiégeante d'une place ; une ligne s'ajoute aux modificateurs.

## Conséquences
- Les renforts lointains combattent à pleine puissance et ne dépensent pas leur mouvement (pas d'arrivée tardive : il faudrait des vagues dans `sim-battle`, refonte L du rapport) ; point ouvert.
