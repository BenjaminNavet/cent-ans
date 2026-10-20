# 0711 — Le choc des charges est un modèle en données (élan, poids, arc-boutement, paliers)

Statut : accepté (20/10, chantier choc).

## Contexte
Le choc d'une charge reposait sur des constantes du code (`CHARGE_IMPACT`, `KNOCKDOWN_TIME`, pertes des
pieux et des piques) et ne tenait compte ni de la distance courue, ni du poids de la cible, ni de la
tenue d'une infanterie à l'arrêt. Un régiment au contact relançait aussi un impact à chaque retour en
mêlée, ce qui rendait le bonus de charge presque permanent.

## Décision
- Tous les nombres du choc vivent dans `data/rules/battle_impact.json` (schéma
  `battle_impact_rules.schema.json`) et sont lus par `ImpactRules` (`sim-battle/src/impact.rs`).
- Élan : mètres courus avant le contact (`run_up`), plein élan à 40 m à cheval, 15 m à pied ; un
  seuil minimal et un plancher d'élan sont réglables.
- Poids : masse du chargeur (lance, cavalerie, pied) comparée au poids de la cible (homme + armure),
  rapport borné.
- Arc-boutement graduel : seule une infanterie à l'arrêt (ou déjà en mêlée), face à la charge, résiste
  selon sa profondeur en rangs ; un schiltron ou un mur de piques résiste en entier. Il réduit les
  renversés, la perte de cohésion et émousse le bonus d'une charge à cheval.
- Paliers : selon l'intensité du choc, la cible recule ou est renversée (durée de chute variable).
- Une immunité courte après un choc et un bonus de charge décroissant sont prévus par le code.

## Conséquences
- L'équilibre actuel (Crécy, Poitiers, duels symétriques, IA contre passive) a été calé avec ce
  modèle, mais il dépendait des impacts répétés : le seuil d'élan, le plancher d'élan < 1, l'immunité
  et la décroissance du bonus restent **neutres dans les données** (`min_impact_run_up_m` 0,
  `elan.min` 1, `shock_immunity_s` 0, `charge_bonus_fade_s` 0). Les activer demande un rééquilibrage
  de l'IA et du rythme de mêlée, à faire dans un chantier séparé.
- Le poids, l'arc-boutement et les paliers sont actifs.
