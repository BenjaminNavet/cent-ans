# 0710 — La carte de campagne vise l'atmosphère de Total War

Statut : accepté (20/10, chantier TS).

## Contexte
Comparaison de captures de Warhammer III et de Cent Ans aux mêmes distances (`docs/wip/ts-style-total-war.md`) :
notre carte est lisible mais claire, saturée et uniforme ; la neige couvre tout en aplat, les forêts perdent leur
forme en vue moyenne, aucune brume ne structure les vallées.

## Décision
- La lisibilité vient du contraste de valeurs et de l'atmosphère, pas de la saturation : terrain désaturé,
  couleurs vives réservées aux éléments de jeu (bannières, feux, villes, zones d'armée).
- La neige dépend de l'altitude, de la pente et d'un bruit en plaques ; elle n'est jamais blanc pur ; les pentes
  raides restent en roche sombre.
- Une forêt se lit comme une masse sombre à lisière franche à toutes les distances de la vue 3D.
- La brume suit le relief (vallées) et la distance ; elle est plus froide et plus dense dans les régions froides.
- Les deux vues de l'ADR 0124 restent : la vue parchemin n'est pas modifiée.
- Toutes les nouvelles valeurs vivent dans les données (`data/`, `game/resources/`), pas dans le code.

## Conséquences
Les réglages de couleur existants (saison, neige, brume, impostors) sont repris lot par lot ; les captures de
référence se refont avec `tools/compare_views.sh`.
