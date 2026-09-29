# OMR R3 — équilibre Est (commise, banqueroutes, révoltes)

Worktree `../gp-omr-r3`, branche `feat/omr-r3`. Cible cargo partagée `core/target`, profil `r3`.

## Commande de mesure
`CARGO_TARGET_DIR=<main>/core/target cargo build -p ai --example century_probe --config 'profile.r3.inherits="release"' --config 'profile.r3.debug=0' --profile r3`
puis `<main>/core/target/r3/examples/century_probe <tours> <graines…>` (`VERBOSE=1` : banqueroutes par faction).

## Référence (om-i1, 50 t. × 5)
Banqueroutes 5,25 / fac. / déc. ; petites fac. 6,03-6,47 ; 28 anciennes 0,03-0,23 ; révoltes 15,2 / 200 t. [4-36] ;
commise de Guyenne 5/5 au tour 1.

## Commise de Guyenne au tour 1 : voulu
Pas un bug ni une régression : ADR 0114 point 4 (lot FE F8), `data/rules/feudal.json`
`start_felonies` (Édouard III donne asile à Robert d'Artois, motif `harboured_felon`) ouvre le cas
de félonie au début de la campagne pour que la commise de mai 1337 soit prononcée ; la sonde FE8
après F8 donnait déjà 10/10 au tour 1 (`docs/wip/fe8-equilibre.md`). Rien à corriger.

## Diagnostic banqueroutes
- `r3_small_scan` (nouvel exemple) : 52 des 135 factions de ≤ 2 provinces ont au tour 0 un revenu
  inférieur à bâtiments + une unité de garnison. L'unité la moins chère coûte 45-50 livres / saison
  dans une cité ; un comté pauvre rapporte 40-80. L'IA ne congédie jamais la dernière unité de sa
  capitale : déficit perpétuel (Perm : revenu 47, entretien 64).
- Vassaux (Tarente, Kildare) : tribut 10 % + agents hors du compte de déficit de la simulation ;
  l'IA ne rasait donc jamais (`deficit_seasons` restait à 0) et restait en dette des décennies.

## Changements
1. ADR 0117 garde du seigneur : `settlements/rules.json` `capital_guard` (1 unité, 0 %),
   `economy::garrison_share`, `CampaignState::faction_capital_city`.
2. IA : démolition aussi quand la dette ne se rembourse pas en 8 tours au surplus net ;
   agents congédiés (le plus cher, un par saison) tant que le trésor est négatif.

## Mesures 50 t. × 5 (graines 1-5)
| version | banq. toutes | petites | 28 anciennes | révoltes / 200 t. |
|---|---|---|---|---|
| base (5d64d2bd) | 5,25 | 6,03-6,47 | 0,03-0,23 | 15,2 [4-60?] |
| v1 garde | 1,75 | 1,59-2,17 | 0,00-0,26 | 28,8 [8-60] |

## État
- [x] Commise de Guyenne : diagnostic (voulu).
- [ ] Banqueroutes des petites factions (v2 : démolition/agents en dette, mesure en cours).
- [ ] Révoltes.
- [ ] Sonde 464 t. × 10.

## Prochaine étape
Mesure v2, puis révoltes (`REVOLT_TRACE=1`).
