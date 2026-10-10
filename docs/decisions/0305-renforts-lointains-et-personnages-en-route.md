# 0305 — Renforts lointains affaiblis, personnages libres en route

Statut : accepté

## Contexte
Lot WR « armies », restes du lot WH armya (ADR 0272, 0273). Une armée alliée à jusqu'à 25 km rejoignait une bataille à pleine force sans dépenser son mouvement. Et les personnages n'avaient pas de déplacement propre : seul un général suivait son armée ; l'IA ne trouvait un chef que parmi les personnages déjà dans la province de la place.

## Décision
**Renforts.**
- `data/movement/rules.json` : `reinforce_full_radius_km` (10) et `reinforce_min_percent` (40). Part engagée d'un renfort : 100 % dans la même place ou jusqu'au rayon plein, puis décroissance linéaire jusqu'au plancher au `reinforce_radius_km` (25) ; au-delà il n'est pas renfort (ADR 0273).
- `movement::committed_percent` : la part est appliquée à la force de chaque régiment du renfort, en résolution automatique (`coalition_side`) et dans la description de la bataille 3D (`coalition_army`). Le moteur 3D n'a pas d'arrivée datée (les réserves `Unit.reserve` ne se libèrent que par remplacement, sans durée) : on retient donc la réduction d'effectif aux deux endroits. Les pertes sont bornées par la force engagée et reportées sur les vrais régiments ; la part non engagée est intacte.
- `apply_battle_result` (voie commune auto/3D) retire au renfort le mouvement du trajet (coût en points de grille de la distance au-delà du rayon d'engagement, `saturating_sub`).
- Prévision : `Reinforcement.committed_percent` ; le pré-bataille affiche « à N km, ~X % ».

**Personnages libres.**
- Pas de position/mouvement propre dans le moteur : solution simple, `Order::SendCharacter { character, to }` pose `CharacterState.journey { to, turns_left }` ; `turns_left = ceil(distance des centroïdes / km_per_turn)` (`data/rules/armies.json` → `character_travel.km_per_turn`, 150 ; au moins 1). `location` reste la province de départ jusqu'à l'arrivée (phase personnages, `char_travel::advance_journeys`). En route, le personnage ne peut être ni nommé général (`CharacterBusy`) ni renvoyé. Destination : province dont la faction est propriétaire.
- IA (`ai/src/campaign/char_moves.rs`) : au plus 4 envois par tour, du meilleur commandant libre (ni armée, ni gouvernorat, ni nommé ce tour) vers : les provinces d'une armée sans chef (d'abord), puis celles d'une ville dont la garnison dépasse son rôle quand la faction a encore des armées gratuites. Les convois (au plus 2 régiments, ADR 0272) ne réclament pas de chef ; le souverain ne part qu'en dernier recours. Rien n'est envoyé si quelqu'un attend déjà sur place ou est en route. `assign_generals` nomme ensuite le chef à l'arrivée ; la réserve de meilleur commandant (hors gouvernorats) vaut aussi tant qu'une armée est sans chef.

## Conséquences
- Les renforts lointains comptent moins et paient leur marche ; l'arrivée datée en bataille 3D reste possible plus tard (vagues dans `sim-battle`).
- Pas encore d'interface joueur pour `SendCharacter` (ordre disponible via le pont). Sauvegardes anciennes : `journey` absent = pas de trajet.

## Mesure (campaign_probe, 60 tours, graines 1-6)
- La sonde compte maintenant les batailles et les armées de campagne (plus de 2 régiments) sans chef. Avant → après : batailles 550 → 651 au total (+18 %) ; armées de combat sans chef : moyenne 3,2 → 2,9, fin de partie 14 → 12.
- Les écarts par graine (40 → 90 sur la graine 1) viennent du chaos de la simulation : désactiver l'affaiblissement, le mouvement dépensé et les envois (mêmes graines) donne déjà 95 batailles sur la graine 1 ; l'affaiblissement seul retire ~8 % de batailles, les envois n'en changent aucune.
- Les armées sans chef restent surtout des convois et des armées de factions dont le seul adulte libre est le souverain ou n'existe pas.
