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

## Diagnostic révoltes
- `REVOLT_TRACE` + `r3_unrest_scan` (`TRACE_PROV`, `INCITE`) : les provinces qui se soulèvent
  (Syrte, Andalousie…) ont un équilibre de mécontentement vers 34 mais montent de +10 à +19 par
  saison sans événement : 395 ordres « Soulever » d'espions en 50 tours (graine 3). Un espion posté
  dans une cité ennemie la soulève saison après saison (+15, la jauge ne revient que de 20 % par
  saison vers sa cible). Expérience `incite_unrest` 0 : 0,8 révolte / 200 t. (contre 24).

## Changements
1. ADR 0117 garde du seigneur : `settlements/rules.json` `capital_guard` (1 unité, 0 %),
   `economy::garrison_share`, `CampaignState::faction_capital_city`.
2. IA : démolition aussi quand la dette ne se rembourse pas en 8 tours au surplus net (tribut et
   agents compris) ; agents congédiés (le plus cher, un par saison) tant que le trésor est négatif.
3. `rules/agents.json` `incite_unrest` 15 → 10 (défaut du code aligné ; spec agents mise à jour).
4. Exemples `r3_small_scan` (marge structurelle au tour 0) et `r3_unrest_scan`.
5. Tests `sim-campaign/tests/omr_r3_small_realms.rs` ; `c6_agents` lit la valeur de la donnée.

## Mesures 50 t. × 5 (graines 1-5)
| version | banq. toutes | petites | 28 anciennes | révoltes / 200 t. |
|---|---|---|---|---|
| base (5d64d2bd) | 5,25 | 6,03-6,47 | 0,03-0,23 | 15,2 [4-36] |
| v1 garde | 1,75 | 1,59-2,17 | 0,00-0,26 | 28,8 [8-60] |
| v2 + démolition/agents | 1,18 | 1,08-1,54 (1,30) | 0,03-0,11 | 24,0 [8-36] |
| incite 0 (expérience) | 1,20 | 1,14-1,68 | 0,00-0,57 | 0,8 |
| incite 8 | 1,09 | 0,93-1,40 | 0,00-0,11 | 0,8 [0-4] |
| **incite 10 (retenu)** | 1,26 | 1,20-1,62 (1,41) | 0,00-0,29 | 8,8 [0-16] |

## État
- [x] Commise de Guyenne : diagnostic (voulu).
- [x] Banqueroutes des petites factions.
- [x] Révoltes (incite 10).
- [ ] Sonde 464 t. × 10 (base et final, en cours) ; tests workspace + pytest.

## Prochaine étape
Relever les sondes 464 × 10 (`/private/tmp/claude-501/r3/{base,final}_464.txt`), rapport, commit final.
Disque : 13 Go libres pendant le lot (autres lots) ; empreinte R3 ≈ 1,5 Go (profils r3/r3dev).
