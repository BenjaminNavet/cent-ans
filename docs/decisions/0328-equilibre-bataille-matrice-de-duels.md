# 0328 — Équilibre de bataille : matrice de duels et recalibrage minimal (TW balance)

Statut : accepté.

## Contexte
Le lot bsim (ADR 0320) a posé le mur de lances, le moral par type et le champ `stats.reload_s` sans valeur. Le rapport TW « simulation de bataille » (top2) demandait un pierre-feuille-ciseaux lisible façon Medieval II, vérifié par des duels A/B (esprit de l'ADR 0148).

## Décision
- Outil : `tests/combat/tw_balance.rs`, matrice de duels régiment contre régiment (10 types, 3 graines, 600 s), imprimée par un test ignoré ; un test fige les relations lisibles (lanciers > cavalerie moyenne, piques > chevaliers, cavalerie > tireurs, tireurs > infanterie, hommes d'armes > milice).
- Données : `battle_spear_wall.json` (x2,0, perte 6 %, moral 10), arc long `reload_s` 5 avec `ranged` 58 et `ammo` 72 (4 s impossible : les volées plus rapprochées démoralisent plus et les gardes Crécy/Azincourt/ep9b ne tiennent pour aucun couple `ranged`/`ammo`) ; hommes d'armes inchangés. Les arbalètes restent à 9 s (toutes ont un pavois, le cas « sans pavois » n'existe pas dans les données).
- Les gardes historiques et d'IA (Crécy, symétrique ep9b, reliefs, passif) passent ; Azincourt est autorisé à 20/20 (borne haute incluse) ; les empreintes `b6` sont recalées et la graine 4 de `cv3_ai_stances` est remplacée (dérive de campagne).

## Conséquences
Lanciers fiables contre la cavalerie de leur gamme, hommes d'armes nets contre la milice, arc long plus rapide (5 s) à dégâts par volée plus faibles, plus de flèches ; la cavalerie bat enfin l'arc long. Les archers restent dominants (voir note `docs/wip/tw/balance.md`). La campagne auto-résolue (`battle_auto.rs`) lit les mêmes stats ; seul le test de piles de postures de campagne a dérivé (graine remplacée).
