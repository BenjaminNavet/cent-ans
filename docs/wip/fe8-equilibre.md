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

## Prochaine étape
Référence 464 tours × 10 graines en cours ; puis règle de l'ost effectif.
