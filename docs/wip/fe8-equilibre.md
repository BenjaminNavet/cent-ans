# FE F8 — Équilibre (branche `feat/fe8-equilibre`, worktree `../game_project-fe8`)

Issue de `main` 639b7e49 (F0-F6, 91 factions, 186 provinces). Cible cargo `core/target-fe8`.

## État
- [x] Sonde `century_probe` : tableau « FE8 » (commise de Guyenne, 1re guerre FR-EN, commises,
      félonies, ost impérial, Italiens en guerre contre la France, Empire allié à son vassal,
      recettes France t ≤ 50, banqueroutes des petites factions et des 28 factions d'avant FE).
- [ ] 1. Ost effectif (règle cœur + ADR).
- [ ] 2. Commise de Guyenne.
- [ ] 3. Banqueroutes.
- [ ] 4. Rapport avant/après, plancher c7a (ADR 0113).

## Lancer les mesures
`cd core && CARGO_TARGET_DIR=$PWD/target-fe8 cargo build --release -p ai --example century_probe`
puis `target-fe8/release/examples/century_probe <tours> <graines...>` (graines en parallèle ;
50 tours × 5 graines ≈ 45 s, 464 tours × 10 graines ≈ 10 min).

## Mesures de référence (main 639b7e49, 50 tours, graines 1-5)
| graine | commise Guyenne | 1re guerre FR-EN | commises | félonies | ost impérial (c. France) | Italiens en guerre c. France (tours×fac.) | Empire allié à son vassal (tours) | recettes France t ≤ 50 | trésor France t50 | banqueroutes petites fac. / fac. / déc. |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | - | 1 | 0 | 14 | 67 (67) | 130 | 50 | 1 556 656 | 26 866 | 8,04 |
| 2 | - | 1 | 0 | 22 | 34 (34) | 70 | 50 | 1 558 020 | 35 435 | 7,90 |
| 3 | - | 1 | 0 | 13 | 35 (35) | 65 | 50 | 1 675 165 | 28 166 | 8,59 |
| 4 | - | 1 | 0 | 15 | 35 (35) | 66 | 50 | 1 706 216 | 36 652 | 8,50 |
| 5 | - | 1 | 0 | 20 | 67 (67) | 112 | 50 | 1 429 725 | 17 205 | 9,12 |

Banqueroutes toutes factions sur 50 tours : 7,22 / fac. / déc. (pires : Urgell, Berg, Saluces,
Urbino, Leinster, ≈ 33 / déc., soit 80 % des tours). La guerre FR-EN est ouverte au tour 1 dans
toutes les graines (déclaration anglaise), la commise n'est jamais prononcée.

### Banqueroutes : la colonne
Même définition qu'avant FE (un événement `Bankruptcy` par tour de trésor négatif, divisé par le
nombre de factions hors rebelles et par décennie), mais la population a changé : 28 factions avant
FE, 91 maintenant, dont beaucoup d'un comté. Nouvelle colonne « 28 factions d'avant FE » pour
comparer à population égale.

Trace `ECON_TRACE=fac_urgell` : trésor initial 6 000 pour un revenu de 80 / saison ; l'impôt
d'opulence (20 % au-delà de 6 saisons de revenu) le ronge en 7 tours, l'IA dépense le reste
(agents, recherche, édits, recrues de garnison) puis l'entretien de la garnison (≈ 50-100) dépasse
le revenu (≈ 70) : trésor négatif en continu.

## Référence 464 tours (1337-1453), graines 1-10, main 639b7e49
| graine | commise Guyenne | commises / exécutées | félonies | ost impérial (c. France) | Italiens c. France t ≤ 50 | recettes France t ≤ 50 | banqueroutes petites fac. | banqueroutes 28 fac. d'avant FE |
|---|---|---|---|---|---|---|---|---|
| 1 | - | 0 / 0 | 113 | 89 (67) | 130 | 1 556 656 | 7,31 | 0,15 |
| 2 | - | 0 / 0 | 83 | 58 (58) | 70 | 1 558 020 | 7,17 | 0,04 |
| 3 | - | 4 / 0 | 186 | 141 (104) | 65 | 1 675 165 | 7,16 | 0,10 |
| 4 | - | 2 / 1 | 126 | 103 (67) | 66 | 1 706 216 | 6,97 | 0,16 |
| 5 | - | 1 / 1 | 131 | 192 (82) | 112 | 1 429 725 | 7,27 | 0,09 |
| 6 | - | 3 / 1 | 104 | 93 (66) | 66 | 1 510 560 | 7,71 | 0,11 |
| 7 | - | 3 / 1 | 119 | 111 (52) | 64 | 1 551 034 | 7,22 | 0,05 |
| 8 | - | 4 / 0 | 162 | 158 (71) | 79 | 1 802 172 | 7,62 | 0,10 |
| 9 | - | 4 / 0 | 170 | 58 (49) | 63 | 1 601 573 | 7,50 | 0,09 |
| 10 | - | 4 / 1 | 151 | 102 (95) | 84 | 1 599 416 | 7,25 | 0,06 |

Synthèse : guerre FR-EN 65 % [56-72], 10/10 dans 55-75 % ; trêves 12,2 / siècle ; révoltes 14,3 / 200 t.
[9,1-21,1] ; banqueroutes toutes factions 6,28 / fac. / déc. [5,98-6,62] ; 4 majeures en vie en
1400 : 10/10 ; Empire allié à un vassal direct 464/464 tours (alliance d'état initial).

## Après F8 (branche 40c360c3 ; 464 tours, graines 1-10, release)
Changements : ADR 0114 (ost effectif à 200 km / indépendance de fait à 0,5 ; lien féodal au lieu
d'alliance ; rapport de force de la commise entre coalitions ; félonie de 1337 ; `commise.min_power_ratio`
2,0 → 1,2 ; réseau d'agents de l'IA dès 500 livres de revenu ; garnisons dans la garde de mutation
monétaire) ; plancher c7a 15 000 → 20 000 (note à l'ADR 0113).

| graine | commise Guyenne (tour) | commises / exécutées | félonies | ost impérial (c. France) | Italiens c. France t ≤ 50 | recettes France t ≤ 50 | banqueroutes petites fac. | banqueroutes 28 fac. d'avant FE |
|---|---|---|---|---|---|---|---|---|
| 1 | 1 | 27 / 9 | 186 | 94 (25) | 25 | 1 631 571 | 3,42 | 0,02 |
| 2 | 1 | 34 / 10 | 189 | 67 (0) | 0 | 1 769 966 | 3,89 | 0,24 |
| 3 | 1 | 31 / 9 | 212 | 168 (15) | 37 | 1 609 564 | 3,95 | 0,12 |
| 4 | 1 | 29 / 12 | 224 | 81 (13) | 13 | 1 665 118 | 3,35 | 0,07 |
| 5 | 1 | 31 / 6 | 201 | 137 (17) | 51 | 1 696 219 | 3,45 | 0,10 |
| 6 | 1 | 31 / 12 | 196 | 121 (17) | 45 | 1 746 358 | 3,84 | 0,13 |
| 7 | 1 | 30 / 4 | 185 | 154 (15) | 29 | 1 741 778 | 3,85 | 0,09 |
| 8 | 1 | 32 / 4 | 284 | 62 (15) | 38 | 1 686 708 | 4,15 | 0,19 |
| 9 | 1 | 18 / 2 | 109 | 149 (30) | 22 | 1 779 149 | 3,43 | 0,14 |
| 10 | 1 | 24 / 11 | 104 | 57 (16) | 37 | 1 798 705 | 4,01 | 0,10 |

### Avant / après (moyennes, 10 graines)
| mesure | avant (639b7e49) | après |
|---|---|---|
| commise de Guyenne | 0/10 | 10/10, au tour 1 (mai 1337) ; la guerre FR-EN est ouverte par les relations du scénario dès le tour 0, la commise lui donne son motif féodal le premier tour |
| commises prononcées / exécutées (toutes) | 2,5 / 0,5 | 28,7 / 7,9 |
| ost impérial contre la France (réponses, siècle) | 71 | 16 |
| Italiens en guerre contre la France (tours × fac., t ≤ 50) | 80 | 30 |
| Empire allié à un vassal direct (tours) | 464 | 0 |
| recettes France t ≤ 50 | 1 599 054 | 1 712 514 (+7 %) |
| banqueroutes / fac. / déc. (toutes) | 6,28 | 3,21 |
| banqueroutes petites factions (≤ 2 prov.) | 7,3 | 3,7 |
| banqueroutes 28 factions d'avant FE (réf. RS-B 0,05) | 0,10 | 0,12 |
| guerre FR-EN (ADR 0085, 55-75 %) | 65 % [56-72], 10/10 | 66 % [51-76], 6/10 |
| trêves FR-EN / siècle | 12,2 | 12,4 |
| révoltes / 200 tours | 14,3 | 8,7 |
| 4 majeures en vie en 1400 | 10/10 | 10/10 |
| sonde c7a (France / Angleterre, 8 graines, 50 t.) | 24 336 / 20 316 (RS) | 32 603 / 13 251 |

Les colonnes de banqueroute gardent leur définition ; la hausse venait de la population (91 factions,
dont une trentaine d'un comté). Reste : des comtés de 50-80 livres de revenu (rois irlandais, Isles,
Luna, Urbino) ne paient pas une seule unité de garnison ; donnée d'économie provinciale (FE4), hors F8.

### Bandes de la spec § 5 (après ; pas de mesure « avant », colonnes ajoutées en F8)
| graine | vassaux directs de la France (1337 / t80 / fin) | France 1er royaume par population (t80 / fin) | guerres de succession | mineure la plus grande à la fin (provinces) |
|---|---|---|---|---|
| 1 | 7 / 7 / 4 | oui / oui | 3 | Namur (13) |
| 2 | 7 / 5 / 0 | oui / non | 3 | Namur (17) |
| 3 | 7 / 7 / 1 | oui / non | 2 | Pavie (11) |
| 4 | 7 / 7 / 0 | oui / oui | 3 | Gueldre (18) |
| 5 | 7 / 7 / 1 | oui / oui | 2 | Flandre (13) |
| 6 | 7 / 7 / 1 | oui / non | 4 | Gueldre (14) |
| 7 | 7 / 7 / 2 | non / non | 5 | Savoie (12) |
| 8 | 7 / 6 / 0 | oui / non | 2 | Trèves (25) |
| 9 | 7 / 7 / 5 | oui / oui | 2 | Trèves (25) |
| 10 | 7 / 6 / 0 | oui / oui | 2 | Flandre (29) |

- France 1er royaume : 9/10 après 20 ans, 5/10 à la fin (l'Angleterre ou l'Empire la dépassent). Partiel.
- Absorption des grands fiefs sans tout avaler en 20 ans : tenue (7 vassaux en 1337, 5-7 en 1357,
  0-5 en 1453).
- Guerres de succession : 2 à 5 par partie. Tenue.
- Aucune mineure n'explose : non tenue (une ancienne petite faction finit à 11-29 provinces ; Trèves,
  Flandre, Gueldre, Namur). Une cinquantaine de factions éliminées par partie, avant comme après.

## Points ouverts
- ADR 0085 : moyenne tenue (66 %), dispersion accrue (6/10 dans la bande ; un essai intermédiaire à
  code presque identique donnait 9/10 : forte sensibilité des trajectoires). Angleterre éliminée après
  1400 dans la graine 10.
- Mineures qui explosent et France dépassée en population en fin de siècle : à traiter avec l'économie
  des comtés et la doctrine « survie d'abord » (F8 bis ou lot dédié).
- Comtés trop pauvres pour une garnison (voir plus haut).

## Prochaine étape
Lot terminé côté agent ; fusion et partie pilote par l'orchestrateur.
