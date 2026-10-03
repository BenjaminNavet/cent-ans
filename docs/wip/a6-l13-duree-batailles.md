# A6-L13 : durée des batailles

## État
- Sondes `l13_duration.rs` / `l13_trace.rs`, cadence en données `battle_pace.json` (valeurs d'origine).
- Mécanisme `field_line_gap_m` livré, valeurs non appliquées : 16 tests de réglage à réétalonner (ADR 0180, Retombées).
- Mesuré : écart de lignes ×1,5 = durée ×1,23 (24/24 vainqueurs) ; + mêlée ×0,3 = ×1,27 (23/24, Azincourt/Poitiers ep7 rouges).

## Prochaine étape
- Décider d'appliquer l'écart (réétalonnage des 16 tests), ou d'autres leviers (effectifs, ADR 0180).

## L13b (en cours)
- Branche a6-l13b. Ajouts : `field_line_gap_m` 450/510/570 appliqué ; pace `historical` (cartes EP7 gardent la cadence d'origine) ;
  leviers `move_speed_factor`, `run_speed_factor`, `melee_fatigue_per_s` dans `battle_pace.json`.
- Mesure : écart + marche x0,75 = x1,61 (24/24 vainqueurs, historiques intacts), mais la phase de mêlée reste ~86 s.
- Prochaine étape : allonger la mêlée (létalité, moral progressif, recul), puis réétalonner les tests de réglage.

## L13b : fait
- Valeurs appliquées et tests réétalonnés (ADR 0180, section L13b). Reste : suite finale, pytest `-k pace`, suppression de core/target.
