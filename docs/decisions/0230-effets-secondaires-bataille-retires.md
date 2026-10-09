# 0230 — Effets secondaires de bataille retirés

Statut : accepté (chantier SC, lot BT11).

## Contexte
Quatre modules de `game/scripts/battle/` ajoutaient de l'ambiance sans porter aucune règle de
jeu : duels appariés (`BattleDuels`, BV3), oiseaux (`BattleBirds`, EP8), ombres de nuages
(`BattleCloudShadows`, EP8) et mouvement secondaire des étoffes et crins (`BattleSecondaryMotion`,
AN1a, ADR 0096). Ils coûtaient du code, des données, des tests et des appels dans la scène de
bataille, pour un gain peu lisible à la distance de jeu.

## Décision
- Supprimés : ces quatre classes (+ `battle_bird.gdshader`), leur câblage (`battle_scene.gd`,
  `battle_staging.gd`, `battle_soldiers.gd`, `battle_standards.gd`), le bloc `sm_*` du shader
  `battle_soldier_skinned.gdshader` (uniforme `move_speed` compris), les sections `duels`
  (`battle_finish.json`), `cloud_shadows` et `birds` (`battle_staging.json`), `secondary_motion`
  (`atmosphere.json`), leurs schémas, le test `an1a_motion_test`, les tests de durée de duel
  (`test_battle_finish_schema.py`) et le volet oiseaux de `ep8_staging_test`.
- Conservé : `BattleQueueTip` (CB-M3). Ce n'est pas un effet : c'est l'aide d'interface qui
  explique au joueur pourquoi le curseur est interdit quand Maj est tenue et que la file
  d'ordres est pleine (texte de `BattleInput`, borne du cœur). Couvert par `cb_m3_queue_test`.
- Les drapeaux des étendards gardent leur ondulation de shader (`battle_standard_flag.gdshader`,
  vent de `BattleStandards`) : seul le réglage `setup_flag` d'AN1a disparaît, les valeurs par
  défaut du shader s'appliquent.

## Conséquences
- Environ 800 lignes de GDScript et de shader en moins, plus aucun réglage ni drapeau associé.
- Les surcots, caparaçons, crinières et queues sont rigides ; plus de duels isolés, d'oiseaux ni
  d'ombres de nuages sur le champ de bataille. Aucune règle ni sauvegarde touchée.
- Les ombres de nuages de la carte de campagne et les oiseaux de la carte (`MapBirdFlocks`) sont
  distincts et restent.
