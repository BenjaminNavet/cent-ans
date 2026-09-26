# ADR 0068 — Direction de la déroute et contagion de moral

Date : 2026-09-26. Statut : accepté. Lot EP10 (suivi : `docs/wip/ep10-deroute-contagion.md`),
renvoyé par SG5 (ADR 0046 § Suite SG5).

## Contexte

Diagnostic de la session de nuit (SG5) sur la crête miroir avec pieux (`sg4_balance`) : la première
déroute est celle d'une aile de cavalerie du défenseur ; ses fuyards refluent le long de la ligne et
des régiments intacts cèdent l'un après l'autre. Deux règles de `sim.rs` en étaient la cause :

1. **Contagion aveugle.** Chaque ami en déroute à moins de 120 m retirait 0,4 point de moral par
   seconde (au plus trois), qu'il cède à côté, passe devant ou soit déjà loin derrière.
2. **Fuite latérale.** Un régiment en déroute fuyait « à l'opposé de l'ennemi le plus proche + vers
   son bord ». En bout de ligne, l'ennemi est sur le flanc : la déroute courait à 45° le long de la
   ligne et la brisait régiment par régiment.

Reproduction (`sim-battle/tests/ep10_rout.rs`, sonde `probe`) : ligne de 8 régiments d'hommes
d'armes à pied, ligne ennemie miroir à 90 m, chevaliers ennemis à 70 m sur le flanc de l'aile, aile
en déroute, reste de la ligne ébranlé (moral = plafond, pas de récupération au contact de l'ennemi).

| Moral de la ligne | 24 | 27 | 30 | 35 | 40 |
|---|---|---|---|---|---|
| régiments qui cèdent (sur 7), avant | 7 | 7 | 7 | 7 | 4 |
| après | 7 | 7 | 0 | 0 | 0 |
| course de l'aile le long de la ligne, avant → après | 113 m → 0 | | | | |

(Une ligne à 24-27, au seuil de déroute de 20, cède encore : un voisin qui lâche pied à côté reste un
choc.)

## Décision

Règles dans `data/rules/battle_rout.json` (schéma `battle_rout_rules.schema.json`, pytest
`tools/tests/test_battle_rout_schema.py`), code dans `sim-battle/src/rout.rs`
(`RoutRules`, `ContagionRules::weight`, `FlightRules::direction`) et `sim.rs`
(`flight_direction`, `resolve_morale_and_fatigue`). **Batailles rangées seulement** : un siège garde
exactement l'ancienne règle (fuite loin de l'ennemi le plus proche, tout ami à 120 m compte 1).

1. **La fuite part vers l'arrière de son camp**, droit loin du front (l'attaquant tient le bord bas en
   z, le défenseur le bord haut, sur les champs générés comme sur les cartes EP7). L'ennemi ne compte
   que pour ne pas le traverser : un ennemi en état de combattre sur la route (moins de 80 m devant,
   moins de 60 m de côté) fait obliquer le fuyard (`enemy_weight` 2 : 63° devant un ennemi droit sur la route). Les amis en
   ordre sur la route (40 m devant, 25 m de côté) sont contournés d'un écart léger
   (`friend_weight` 0,25, soit 14° au plus), plafonné quel que soit leur nombre.
2. **La contagion est pondérée par la position.** Chaque ami en déroute à moins de 120 m pèse 1 s'il
   cède à côté ou devant le régiment (jusqu'à 15 m en arrière le long de l'axe du front), 0,15 au-delà
   de 50 m en arrière, linéairement entre les deux. Perte de moral : 0,5 par seconde et par unité de
   poids (au lieu de 0,4 par ami), poids plafonné à 3.

Pourquoi relever le taux de 0,4 à 0,5 : à 0,4, les vagues françaises des cartes historiques, que les
fuyards de la vague précédente dépassent, ne s'ébranlaient presque plus (Crécy 10/20, Azincourt 13/20,
Poitiers 7/20 victoires anglaises, hors des fourchettes d'EP7) ; relever le poids « devant / à côté »
garde le choc d'un fuyard qui traverse ou longe un régiment, et ne rend pas sa force à celui qui
s'éloigne derrière.

Pourquoi plafonner le contournement des amis : sans plafond (0,8 par ami), les cavaliers que l'IA
masse au même point envoyaient la fuite de côté ; sur la bataille miroir d'`ep9b_duel`, l'attaquant
gagnait 29 fois sur 30.

## Mesures (avant = main 222080a4 avec SG5, après = EP10)

| Mesure | Avant | Après |
|---|---|---|
| Crécy (`ep7_historical`, graines 1-20 / 1-30) | 16/20, 23/30 | 16/20, 26/30 |
| Azincourt | 15/20, 24/30 | 19/20, 29/30 |
| Poitiers | 14/20, 20/30 | 12/20, 18/30 |
| Durées EP7 min/méd/max (s) : Crécy, Azincourt, Poitiers | 824/849/1044, 520/796/931, 771/1158/1289 | 814/848/1088, 520/849/905, 587/1123/1313 |
| `ep9b_duel` miroir plat sans pieux, attaquant (graines 1-10 / 1-30) | 7/10, 18/30 | 4/10, 7/30 |
| `ep9_decisive::survey_crecy` | 12/12 anglais | 12/12 anglais |
| `ep9_decisive::survey` (27 lignes × 12 graines) | toutes finies, max 969 s | toutes finies, max 1161 s |
| sg4 plat avec / sans pieux | 3/7, 7/3 | 1/9, 4/6 |
| sg4 crête avec / sans pieux | 0/10, 0/10 | 0/10, 0/10 |
| sg4 crête, pertes att. / déf., durée | 43 % / 21 %, 466 s | 30 % / 14 %, 292 s |
| sg4 plaine générée avec / sans pieux | 1/9, 6/4 | 3/7, 5/5 |
| sg4 comme `ep1_scale` | 5/5 | 2/8 |
| `b6` petite bataille (6 contre 4), Français sur graines 0-63 | 29/64 | 34/64 |

`ep9_decisive` (tests), `ep1_scale`, `ai` : verts sans modification. Toutes les batailles se
concluent.

## Conséquences

- Empreintes `b6.rs` recalculées (graine 3 : vainqueur changé, les hommes d'armes français tiennent
  au lieu de céder à côté de leurs chevaliers en déroute ; graine 11 : même fin), justifiées dans le
  commentaire du test.
- Sur la crête miroir, le défenseur gagne toujours, plus vite et à moindre coût : la ligne en
  profondeur de SG5 n'est plus seule à la protéger de la contagion.
- La bataille miroir d'`ep9b_duel` penche désormais vers le défenseur (7/30 au lieu de 18/30) ; le
  test (3 à 7 sur les graines 1-10) passe à 4/10. Ces batailles miroirs sont chaotiques : 10 graines
  valent une ou deux batailles indépendantes (ADR 0046 § SG5), et un écart de 0,1 sur le contournement
  des amis fait passer l'attaquant de 7 à 25 victoires sur 30.
- Azincourt est à 19/20, au bord haut de la fourchette du test (14 à 19).
