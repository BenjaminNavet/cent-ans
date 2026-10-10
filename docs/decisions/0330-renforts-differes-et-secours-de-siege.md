# 0330 — Renforts à arrivée différée et armée de secours de siège (lot TW reinf)

Statut : accepté

## Contexte
ADR 0305 (WR armies) : un renfort allié lointain (jusqu'à 25 km) combat avec une part réduite de son effectif, mais présent dès le premier tour de la bataille 3D, car le moteur n'avait pas d'arrivée datée. Le critique `bataille-ia-sieges` (top5, top9) demande des renforts qui arrivent sur un bord de la carte à T+x, et une armée de secours qui frappe les assiégeants pendant l'assaut.

## Décision
- **Moteur** : `UnitSetup.arrival_s` (seconde de bataille, `None` = présent dès le début) et `UnitSetup.entry_edge` (`EntryEdge` : `own` par défaut, `west`, `east`, `rear` = derrière la ligne ennemie), copiés sur `Unit`. `hold_reserves` met en réserve tout régiment à arrivée datée (hors règle du surplus 40/80) ; `release_reserves` les fait entrer par `march_in` dès `elapsed >= arrival_s`, quel que soit le nombre de régiments sur le terrain, au bord indiqué ; les réserves ordinaires ne puisent jamais dans les arrivées datées. `BattleSim::next_arrival_in(side)` donne le délai restant (pont : `get_next_arrival`, -1 si rien). Alerte `Reinforcements` et journal existants à l'entrée.
- **Données** (`data/movement/rules.json`, schéma mis à jour) : `reinforce_base_delay_s` (90) et `reinforce_seconds_per_km` (60) ; arrivée = délai de base + (distance − `engage_radius_km`) × s/km (25 km : ~23 min). Dans le rayon d'engagement ou dans la même place : présent dès le début. La distance est celle de l'ADR 0305 (`army_distance_km`) ; l'affaiblissement de force de 0305 est conservé en plus du retard (le retard ne le remplace pas : les résultats 3D se rapportent toujours régiment par régiment à `coalition_army`).
- **Campagne** : `movement::coalition_arrivals` (une entrée par régiment de `coalition_army`, même ordre), `arrival_for`, `apply_arrivals`. Bord d'entrée : `west`/`east` quand l'armée alliée est surtout à l'ouest/à l'est du chef (|dx| > 1,2 |dy|), sinon `own` ; le champ 3D n'a pas de boussole, c'est une approximation.
- **Secours de siège** : `siege::siege_defence` : à l'assaut 3D, les armées alliées du contrôleur de la place, en guerre avec l'assiégeant, hors de la place et à portée (`reinforce_radius_km`, mouvement restant, comme `is_late_reinforcement`) rejoignent le côté défenseur après la garnison, avec la part de force de 0305, arrivée datée et bord `rear` (derrière l'assiégeant). `resolve_pending_battle` valide les pertes sur garnison + secours ; `apply_relief_losses` reporte les pertes au-delà de la garnison sur les armées de secours (général intact, pas de capture).
- **HUD** : étiquette « Renforts dans MM:SS » (`BattleHud.set_reinforcements`) à côté de l'horloge, alimentée par `get_next_arrival` pour le camp du joueur (le plus proche en spectateur).

## Conséquences
- Une armée lointaine peut manquer la bataille si elle finit avant son heure ; une armée battue n'est pas « finie » tant que des renforts attendent (comme les réserves existantes), la limite de 3600 s borne tout.
- L'IA de siège n'a pas encore de garde dédiée contre l'arrivée du secours (les renforts rejoignent la ligne défensive comme les réserves) ; les armées de secours ne dépensent pas de mouvement de campagne ; seul l'assaut 3D les prend en compte (pas la résolution automatique ni la prévision pré-bataille).
- Sauvegardes anciennes : champs `#[serde(default)]`, rien à migrer.
