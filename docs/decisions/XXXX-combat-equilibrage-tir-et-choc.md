# XXXX — Combat : rééquilibrage du tir et du choc (lot COMBAT, boucle juge 1)

## Contexte
Le juge a relevé : arcs longs intouchables (France 0/6, pertes 247 contre 3), chevaliers et archers montés qui
écrasent l'infanterie, approche trop lente. Mesures sur la sim 3D (IA contre IA, `tests/combat/matchups.rs`) :
- confirmé : 10 chevaliers contre 10 arcs longs, 0/8 victoires, 358 pertes contre 19 ; armée française mixte
  contre 6 arcs + 4 hommes d'armes, 0/8, 446 contre 38 ; 10 archers montés contre 10 milices, 0 perte contre 531 ;
  8 chevaliers contre 8 hommes d'armes à pied, 4 contre 231 ;
- infirmé : « 4 chevaliers contre 20 milices : 12 contre 1 552 ». La fixture `auto_resolve_scenarios.json` est
  périmée ; la sim actuelle donne 3/8 pour les chevaliers, 73 contre 514 pertes (les milices tiennent déjà).

## Décision
- `battle_pace.field.ranged_armor_exponent` (1,25) : l'armure arrête mieux les flèches que la règle linéaire.
- `mounted_panic_per_loss` 40 vers 30 : les destriers caparaçonnés ne paniquent pas à la première volée.
- Arc long : munitions 72 vers 36 (3 minutes de tir, Poitiers : flèches épuisées).
- Archers montés : tir 65 vers 52, portée 220 vers 170, munitions 40 vers 30 ; archers des steppes portée 180 vers 160.
- Choc (`battle_impact.json`) : cohésion 8/15 vers 5/10, arc-boutement plus fort (0,25 + 0,08 par rang, max 0,75),
  renversés au plus 40 % ; pace : moral par perte 60 vers 50, résolution en mêlée 0,4 vers 0,6.
- Approche : `move_speed_factor` 0,45 vers 0,75.
- Bandes d'issues larges dans `matchups::outcome_bands` (plausibilité, pas de chiffre).

## Conséquences
- Archers toujours dominants contre l'infanterie sans armure (12 milices contre 12 arcs : 0 victoire), mais
  l'armée française gagne parfois et les archers saignent (pertes ~144 contre 38).
- Reste ouvert : charge de chevaliers contre hommes d'armes en marche (232 contre 9) ; l'approche reste longue
  quand le défenseur recule (IA, hors lot) ; fixture d'auto-résolution à régénérer.
