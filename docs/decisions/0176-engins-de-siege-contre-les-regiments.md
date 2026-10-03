# 0176 — Engins de siège contre les régiments

## Contexte
Retour joueur (2026-10-03) : en bataille, les engins de siège ne semblent pas blesser les unités ennemies.
Cause : depuis le premier jet de `sim-battle`, un tir d'engin sur un régiment était calculé comme une volée
d'archers de « équipage × 3 » traits. Un trébuchet (4 servants) tirait donc 12 « flèches » toutes les 12 s,
armure pleine : environ 0,3 mort par tir, une dizaine sur toute une bataille, soit 1/25 d'une compagnie d'archers.

## Décision
- Un tir d'engin sur un régiment abat un nombre d'hommes fixe :
  `siege_attack × kills_per_siege_attack × part d'équipage × précision relative`, où la précision relative
  vaut 1 à bout portant et 0,5 à portée maximale.
- L'armure de la cible ne compte qu'à `armor_weight` (un boulet ne s'arrête pas sur une cotte).
- Chaque tir qui touche ôte `morale_shock` points de moral, en plus de la panique habituelle des projectiles.
- Les autres modificateurs de tir (forêt, village, haies, merlons, tir indirect, fumée…) restent appliqués.
- Valeurs dans `data/rules/siege_works.json` (`engine`) : 0,08 / 0,3 / 4.

## Conséquences
- À 150 m sur des hommes d'armes à pied : un trébuchet abat environ 3 hommes par tir (16 en 60 s), un
  mangonneau un peu moins de la moitié, une bombarde un peu plus. Cela reste en dessous d'une compagnie
  d'archers par minute ; la valeur de l'engin tient à sa portée et au choc moral.
- Les engins en batterie contre un mur (`fire_at_wall`) ne changent pas.
- Test : `core/crates/sim-battle/tests/es_engine_vs_units.rs`.
