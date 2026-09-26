# EQ5 — IA de campagne : banqueroutes chroniques et intrusions

Branche : `worktree-agent-a745acdb29a9168ed` (main + EQ4 fusionnés). ADR 0068.

## État : terminé (à fusionner)

- [x] Sonde `century_probe` : `ECON_TRACE=fac_x` (budget d'une faction par tour, événements,
      écart inexpliqué du trésor), `TRESPASS_TRACE=1` (armées en intrusion en fin de tour),
      `DEBUG_ARMY=army_x` (ordres d'une armée et, pour la France, leur résultat).
- [x] Référence = EQ4 à l'identique.
- [x] Budget (`ai/src/campaign.rs`) : tributs et agents dans le revenu net ; marge de 5 % ;
      bâtiments plafonnés à 30 % du revenu brut (sauf ceux qui se paient) ; impôt Haut tenu
      tant que le budget au taux normal serait en déficit avec moins d'une saison en caisse
      (trouble toléré 40 au lieu de 30) ; pas de monnaie affaiblie au-delà d'1/3 en bâtiments.
- [x] Armées (`grid.rs`, `campaign.rs`, `diplomacy_eval.rs`) : sortie des terres fermées,
      places propres jamais fermées, retour au pays des armées sans objectif (table élargie,
      même composante de grille), garnison en paix dans sa propre place à l'étranger, plus de
      poursuite en terre fermée, routes qui mordent sur une province fermée fermées, coût ×2
      des terres ouvertes par tempérament (`data/ai/grid.json` `trespass_route_factor`),
      demande de droit de passage.
- [x] Tests `ai/tests/eq5.rs` (3, échouent sans le lot) ; cargo test, clippy, pytest, smoke OK.

## Avant / après (century_probe 464 tours, mêmes graines qu'EQ4)

| Mesure | Facile (5) | Normal (10) | Difficile (10) | Très difficile (5) |
|---|---|---|---|---|
| Banqueroutes / fac. / déc. | 0,42 → **0,08** | 0,17 → **0,04** | 0,19 → **0,03** | 0,09 → **0,01** |
| Pire faction, une graine (/ déc.) | Suisses 31,4 → 1,1 | Grenade 6,7 → 1,1 | Grenade 7,8 → 2,0 | Grenade 5,4 → 0,3 |
| Saisons d'intrusion / siècle | 593 → **251** | 1312 → **474** | 2216 → **657** | 2472 → **745** |
| Casus belli d'intrusion | 48 → 17 | 101 → 43 | 177 → 54 | 193 → 73 |
| Guerre FR-EN | 56 % → 48 % [32-70] | 60 % [52-67] → **62 % [42-75]** | 56 % → 56 % [39-65] | 51 % → 49 % |
| Trêves FR-EN / siècle | 8,2 → 6,4 | 12,5 → **11,1** | 13,6 → 13,0 | 15,0 → 15,4 |
| Révoltes / 200 tours (siècle) | 3,9 → 5,0 | 3,0 → 3,6 | 3,1 → 4,4 | 3,0 → 7,2 |
| 1re faction fin (moy., max) | 24 %, 29 → 24 %, 27 | 25 %, 34 → **22 %, 26** | 26 %, 31 → 24 %, 34 | 31 %, 38 → 33 %, 36 |
| Majeures en vie en 1400 | 5/5 → 5/5 | 10/10 → 10/10 | 10/10 → 10/10 | 5/5 → 5/5 |
| Sièges engagés / siècle | 245 → 222 | 438 → 517 | 699 → 650 | 725 → 734 |

`balance_probe` 16 × 200 (normal) : guerre FR-EN 61-65 % (EQ4 63-64), impôt Haut 30-32 %
(EQ4 28-30, cible < 40), milice 28 %, banqueroutes 0,07-0,11 (EQ4 0,15-0,17), révoltes
4,2 (graines 1-8) et 3,5 (9-16), moyenne 3,9 (EQ4 5,1 ; cible 4-10).

Normal graine par graine (guerre FR-EN / trêves / intrusions) : 1 : 65 % / 10 / 327 ;
2 : 75 % / 11 / 618 ; 3 : 71 % / 12 / 369 ; 4 : 49 % / 7 / 461 ; 5 : 65 % / 13 / 302 ;
6 : 73 % / 13 / 559 ; 7 : 68 % / 13 / 539 ; 8 : 42 % / 8 / 510 ; 9 : 61 % / 13 / 578 ;
10 : 56 % / 11 / 480.

## Diagnostic

- Suisses (facile, graine 2) : bâtiments ≈ 60-75 % du revenu après la peste (bâtis sur le trésor
  initial), impôt qui oscille Haut / Normal autour de 30 de trouble, événements à -300 / -500,
  monnaie affaiblie qui gonfle l'entretien des bâtiments (157 → 201) pour toujours.
- Grenade : guerres lointaines (Angleterre, Écosse, Holstein) soldées par des paix payantes ;
  les tributs par saison (200-550) n'entraient pas dans le budget.
- Intrusions : 38 % des saisons = armées dans leur propre place d'une province étrangère ;
  armées coincées (les terres fermées bloquaient aussi la sortie) ; poursuites en terre neutre ;
  routes du graphe qui mordent sur une province voisine (le Véronais) ; route « par l'eau »
  (Amersfoort → Stavoren, `NoPath` chaque tour).

## Essais rejetés

- Budget avec rançons et Table : l'Écosse (difficile, graine 9) licenciait toutes ses garnisons
  après Neville's Cross et disparaissait en 1354.
- Impôt Haut tenu jusqu'à la réserve de 3 saisons : impôt Haut 44-47 % (> 40 %).
- Sans plafond des bâtiments : Écosse 5,8 banqueroutes / déc. (facile), éliminée (difficile).
- Atteignabilité par `find_path` (A*) : tour 2,5 × plus long ; la composante de grille suffit
  (mêmes décisions sur les 30 parties).
- La demande de droit de passage seule ne change presque rien (≈ -4 %) ; gardée, bon marché.

## Points ouverts

- Chaos : chaque changement de l'IA redistribue les guerres ; les écarts par graine sont
  larges (guerre FR-EN normal 42-75 % ; EQ4 52-67 %), la moyenne reste 62 %.
- Facile : trêves 6,4 (EQ4 8,2), une graine à 32 % de guerre FR-EN.
- Très difficile : la France (jouée par l'IA avec les handicaps du joueur) disparaît après 1400
  sur 1 graine / 5 (Angleterre 36 %) ; effet DF1 à surveiller.
- Révoltes `balance_probe` 3,9 par partie, juste sous la bande 4-10.
- Intrusions restantes : surtout des marches de guerre délibérées (IA agressives) et des
  retours au pays après une paix.
- Suisses encore 2,0 / déc. sur une graine difficile : bâtiments antérieurs au plafond ; il
  n'existe pas d'ordre de démolition.

## Commandes

```
cd core && cargo build --release -p ai --example century_probe --example balance_probe
DIFFICULTY=normal target/release/examples/century_probe 464 1 2 3 4 5 6 7 8 9 10
ECON_TRACE=fac_swiss DIFFICULTY=easy target/release/examples/century_probe 464 2
TRESPASS_TRACE=1 target/release/examples/century_probe 464 3
target/release/examples/balance_probe campaign 200 9 10 11 12 13 14 15 16
```

## Prochaine étape
Fusion dans main par le coordinateur.
