# XXXX — Combat : rééquilibrage du tir (lot COMBAT, boucle juge 1)

## Contexte
Le juge a relevé : arcs longs intouchables, chevaliers et archers montés qui écrasent l'infanterie, approche lente.
Mesures sur la sim 3D de `main` (IA contre IA, 8 graines, `tests/combat/matchups.rs`, outil `survey`) :
- confirmé : 10 chevaliers contre 10 arcs longs 0/8 (358 pertes contre 19) ; armée française mixte contre 6 arcs +
  4 hommes d'armes 0/8 (446 contre 38) ; 10 archers montés contre 10 milices (0 perte contre 531) ; 8 chevaliers
  contre 8 hommes d'armes en marche (4 contre 231) ; approche longue (premier contact ~230 s quand le défenseur
  recule) ;
- infirmé : « 4 chevaliers contre 20 milices : 12 contre 1 552 ». La fixture `auto_resolve_scenarios.json` est
  périmée ; la sim donne 3/8 pour les chevaliers (73 contre 514) et 5/8 pour les milices dans l'autre sens.

## Décision
- `battle_pace.field.ranged_armor_exponent` = 1,15 (nouveau, défaut 1) : l'armure arrête un peu mieux les flèches.
- Archers montés : tir 65 vers 52, portée 220 vers 170, munitions 40 vers 30 (arc court de cavalier) ; archers des
  steppes portée 180 vers 160.
- Bandes d'issues larges dans `matchups::outcome_bands`.
- Essayés et écartés : munitions de l'arc long à 36-54 (le défenseur de Crécy ne gagne plus, batailles de plus de
  20 min), choc/arc-boutement/moral par perte/vitesse d'approche (cassent `bv2` et les bandes `ai_tactics`
  « symmetric_flat_battle_is_open », « crecy_like_the_english_win_most_battles »), panique des montures 40 vers 30
  (idem durées). Ces tests encodent un équilibre IA déjà réglé : un rééquilibrage plus profond demande un lot IA
  (charge groupée de la cavalerie, qui arrive aujourd'hui par à-coups sous les flèches).
- Tests à graine ajustés : `b6::battles_without_a_site_are_unchanged` (empreintes), `symmetric_flat_battle_is_open`
  (2..=7 au lieu de 3..=7) : l'exposant d'armure déplace les armées mixtes.

## Conséquences
- Effet modeste sur les arcs longs (pertes des archers ×2-3 dans les matchups) ; le constat 1 n'est pas résolu.
- Ouvert : coordination de la charge de cavalerie (IA), approche longue, fixture d'auto-résolution à régénérer.
