# EQ4 — sonde d'équilibre combinée (EQ1 + EQ2 + EQ3 + DP2 + DF1 + C4/C5 + SG4)

Branche : `worktree-agent-a2b33defc792e3eab` (main e384848c fusionné).

## État : terminé (à fusionner) — aucune règle ni donnée changée

- [x] `century_probe` étendu (tableau « EQ4 — tableau combiné ») : révoltes, sièges engagés et
      leur issue, prises directes, boule de neige (1re faction en part des provinces contrôlées)
      en 1437 et en fin de partie, factions éliminées, accès militaires accordés, saisons
      d'intrusion et casus belli d'intrusion (DP2).
- [x] Mesure combinée : normal 10 graines × 464 tours, difficile 10, facile et très difficile 5 ;
      `balance_probe campaign` 16 × 200 (graines 1-8 et 9-16).
- [x] Deux essais de correctif par les données, annulés (voir plus bas).
- [x] Contre-essai DP2 désactivé (normal 10 graines, difficile 10).
- [x] Addendum à l'ADR 0054.

Mesures identiques avant et après la fusion de main (SV2, SV4, EP6) : la campagne IA contre IA
est déterministe et ces lots n'en changent pas le cours.

## Tableau combiné (main e384848c)

| Mesure | Facile (5) | Normal (10) | Difficile (10) | Très difficile (5) | Cible (source) |
|---|---|---|---|---|---|
| Guerre FR-EN (part du siècle) | 56 % [47-69], 2/5 | **60 % [52-67], 8/10** | 56 % [45-63], 5/10 | 51 % [45-59], 2/5 | 55-75 % (EQ3 : 67 %) |
| Trêves FR-EN / siècle | 8,2 [6-11] | **12,5 [10-16]** | 13,6 [10-17] | 15,0 [12-17] | 9-15 (EQ3) |
| Révoltes / 200 tours (siècle) | 3,9 | 3,0 | 3,1 | 3,0 | — |
| Révoltes / partie (`balance_probe` 16 × 200) | — | **5,1** (3,1 et 7,1) | — | — | 4-10 (EQ1) |
| Banqueroutes / fac. / déc. | 0,42 [0,06-1,48] | **0,17 [0,05-0,35]** | 0,19 | 0,09 | < 0,5 (EQ1) ; EQ3 0,12-0,17 |
| Sièges engagés / siècle | 245 | 438 | 699 | 725 | — |
| Sièges réussis (prise / siège résolu) | 37 % | **36 % [29-43]** | 34 % | 34 % | — |
| 1re faction en 1437 (% prov.) | 24 % | **25 % [23-29]** | 26 % | 30 % [20-38] | pas de boule de neige |
| 1re faction en 1453 (% prov.) | 24 % [20-29] | **25 % [19-34]** | 26 % [17-31] | 31 % [19-38] | pas de boule de neige |
| Factions éliminées / partie | 0,2 | 0,3 (Gueldre, Portugal) | 0,2 | 0 | — |
| 4 majeures en vie en 1400 | 5/5 | 10/10 | 10/10 | 5/5 | toutes |
| Accès militaires accordés / siècle | 67 | 124 | 217 | 176 | — |
| Saisons d'intrusion (casus belli) / siècle | 593 (48) | 1312 (101) | 2216 (177) | 2472 (193) | — |

`balance_probe` normal, 16 graines : guerre FR-EN 63-64 %, trouble moyen 20,4, milice 27-28 %,
impôt Haut 28-30 %, banqueroutes 0,15-0,17, Paix de Dieu dans 71-73 % des provinces.

Définitions (sonde) : un siège « résolu » est un siège vu en fin de tour qui disparaît ; il
est réussi si l'assiégeant tient la place au tour suivant. Les « prises directes » sont les
changements de contrôleur sans siège vu au tour précédent (garnison vide, assaut dans le tour,
bataille de secours ; hors rebelles). La 1re faction est celle qui contrôle le plus de
provinces (villes) ; la France part à ~23 %.

### Normal, graine par graine

| graine | guerre FR-EN | trêves | révoltes | banqueroutes | sièges (réussis) | 1437 | fin | éliminées | accès | intrusions (CB) |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 59 % | 10 | 7 | 0,30 | 386 (35 %) | France 26 % | France 22 % | Gueldre, Portugal | 86 | 991 (73) |
| 2 | 58 % | 12 | 5 | 0,35 | 443 (33 %) | Angl. 24 % | Angl. 19 % | — | 142 | 1181 (106) |
| 3 | 61 % | 14 | 2 | 0,11 | 517 (38 %) | France 24 % | France 27 % | — | 112 | 1351 (109) |
| 4 | 61 % | 15 | 14 | 0,05 | 465 (37 %) | Angl. 27 % | Angl. 24 % | — | 156 | 1500 (132) |
| 5 | 59 % | 12 | 4 | 0,20 | 338 (31 %) | France 23 % | France 23 % | Portugal | 68 | 1025 (75) |
| 6 | 63 % | 13 | 7 | 0,16 | 540 (38 %) | Angl. 29 % | Angl. 32 % | — | 200 | 1751 (136) |
| 7 | 53 % | 10 | 2 | 0,11 | 404 (36 %) | Angl. 23 % | Angl. 23 % | — | 128 | 1637 (113) |
| 8 | 67 % | 12 | 16 | 0,14 | 366 (29 %) | France 23 % | France 24 % | — | 64 | 1032 (73) |
| 9 | 63 % | 11 | 7 | 0,24 | 384 (39 %) | France 24 % | France 27 % | — | 124 | 1349 (84) |
| 10 | 52 % | 16 | 6 | 0,08 | 539 (43 %) | Angl. 29 % | Angl. 34 % | — | 156 | 1300 (111) |

## Lecture

- **Guerre FR-EN, trêves** : au niveau normal, dans la bande (60 %, 8/10 graines, 12,5 trêves).
  Plus bas qu'à la mesure EQ3 (67 %) : les graines diffèrent et DP2 (fusionné après EQ3)
  ajoute des incidents de passage ; aucune guerre sans fin (plus longue guerre 6-13 ans).
- **Révoltes** : la sonde du siècle en donne ~3 par 200 tours, mais la cible EQ1 se lit sur
  `balance_probe` 200 tours : 3,1 sur les graines 1-8, **7,1 sur les graines 9-16**, 5,1 en
  moyenne. Pas de dérive : l'écart vient des graines.
- **Banqueroutes** : 0,17 au niveau normal, dans la cible. Granada reste la pire (2-7 / déc.).
- **Sièges (SG4)** : SG4 ne change que la bataille de siège 3D ; la campagne IA contre IA se
  résout par la famine, les sorties et l'auto-résolution (`siege.rs`, `battle_auto.rs`), qui ne
  font pas d'assaut. Un siège sur trois se termine par une prise, le reste est levé.
  Pas de boule de neige : la 1re faction plafonne à 25 % en moyenne (34 % au pire, l'Angleterre
  de la graine 10), aucune majeure ne tombe.
- **Droit de passage (DP2)** : l'IA l'utilise (124 accords d'accès militaire par siècle au
  niveau normal), mais les intrusions restent très nombreuses (1 300 saisons, 100 casus belli
  par siècle, toutes factions). Contre-essai `passage.enabled = false` : normal guerre FR-EN
  59 % [44-72] 6/10 (sans) contre 60 % [52-67] 8/10 (avec), banqueroutes 0,11 → 0,17 ;
  difficile 60 % 8/10 (sans) contre 56 % 5/10 (avec). DP2 ne rend pas les guerres
  interminables ; il les raccourcit un peu au niveau difficile.
- **Difficulté** : en facile, trêves rares (8,2) et banqueroutes chroniques d'une petite IA
  (Suisses 31 / déc. graine 2) ; en difficile / très difficile, la guerre FR-EN descend sous
  55 % et l'Angleterre grossit (jusqu'à 38 % des provinces) : c'est l'effet voulu de DF1 sur une
  France jouée par l'IA avec les handicaps du joueur ; la cible 55-75 % est celle du niveau
  normal.

## Essais annulés

- **A — Paix de Dieu** (trouble -8 → -6, piété 2 → 1) : `balance_probe` révoltes 3,1 → 5,4
  (graines 1-8) mais 7,1 → 7,2 (graines 9-16) ; Paix de Dieu 73 → 57 % des provinces, marchés
  francs 11 → 27 %. Le déficit de révoltes des graines 1-8 était du bruit : annulé.
- **B — facile, `ai_upkeep_percent` 100 → 90** contre les banqueroutes chroniques : 10 graines,
  0,44 / fac. / déc., les boucles (Suisses, Gueldre) persistent : annulé. La cause est une petite
  IA qui reste en négatif malgré `disband_for_debt` (`ai/src/campaign.rs`), pas le niveau.

## Points ouverts

- Banqueroutes chroniques d'une petite IA (Granada à tous les niveaux, Suisses / Gueldre en
  facile) : chaque saison en négatif compte ; piste côté IA (licencier plus, ou garnisons
  minimales), hors données.
- Intrusions DP2 très fréquentes (1 300 saisons / siècle) : `ai_may_trespass` laisse camper les
  armées en terre neutre ; à surveiller si l'on veut que le passage pèse sur la diplomatie.
- Guerre FR-EN en difficile / très difficile sous 55 % : décider si la cible doit valoir à tous
  les niveaux (sinon rien à faire).
- Commerce C5 : 0,3 % du revenu total, négligeable dans l'équilibre.

## Commandes

```
cd core && cargo build --release -p ai --example century_probe --example balance_probe
DIFFICULTY=normal target/release/examples/century_probe 464 1 2 3 4 5 6 7 8 9 10
target/release/examples/balance_probe campaign 200 9 10 11 12 13 14 15 16
```

## Prochaine étape
Fusion dans main par le coordinateur.
