# EQ6 — guerre France-Angleterre 55-75 % à tous les niveaux de difficulté

Branche : `feat/eq6-war-all-difficulties` (main fusionné). ADR 0085 (0077 pris sur main).

Décision du joueur (2026-09-26) : la cible « France et Angleterre en guerre 55-75 % du siècle »
vaut à tous les niveaux, pas seulement en Normale. La difficulté change la dureté de la guerre
pour le joueur, pas le fait qu'elle ait lieu.

## État : TERMINÉ (reprise 2026-09-27) — prêt pour la fusion ff dans main

### Reprise 2026-09-27

- main fusionné (372 commits, sans conflit ; commit de fusion après 96accf8f).
- **Mesure après fusion, sans nouvelle règle** (`century_probe` 464 tours, binaire `cp_m3`) :
  facile 62 % [55-73] 4/5 ; normale 65 % [31-78] 8/10 ; difficile 65 % [59-69] 10/10 (le
  problème d'avant la fusion a disparu) ; très difficile 57 % [50-65] **3/5** ✗.
- **Diagnostic** (tableau « EQ6 » + `WAR_TRACE=1`, qui affiche désormais les autres guerres de
  l'Angleterre, les déclarations qui la concernent et les paix blanches évaluées) :
  - normale graine 10 (31 %) : l'obstacle « attitude » bloque 293 tours de paix sur 321.
    L'attitude anglaise envers la France monte à 100 : « Mariage entre nos maisons » (+15,
    80 tours) empilé jusqu'à 10 fois, « Liens matrimoniaux » +15, « Même maison régnante » +20.
  - très difficile graines 1-2 (50-52 %) : l'obstacle « weary » bloque ~165 tours de paix.
    En paix avec la France, l'Angleterre reste à 90-100 de fatigue : trop lasse pour déclarer
    elle-même, elle répond pourtant à chaque appel aux armes du Portugal contre la Castille et
    de l'Empire contre la Bohême (graine 1 : t99, 147, 173, 197, 227, 253, 275…). Ces guerres
    d'appoint, au score nul, l'entretiennent dans la fatigue pendant des décennies.
- **Deux règles génériques** (réglages de `data/ai/diplomacy.json`, défaut faux = avant) :
  - `war.claim_war_ignores_kinship` : pour la guerre de prétention principale, les raisons de
    parenté (mariages, liens matrimoniaux, même maison) sortent de l'attitude qui la décide :
    la prétention naît de cette parenté (Édouard III, petit-fils de Philippe IV par sa mère).
  - `join_war.weary_stay_out` : un royaume trop las pour déclarer lui-même une guerre
    (`max_weariness_to_declare`, +20 pour un prétendant) ne répond plus à l'appel aux armes,
    comme un royaume ruiné (trésor < 0) : l'alliance est rompue.
- Aucun changement des données de difficulté ni du combat tactique.

### Tableau final (v9, `century_probe` 464 tours, mêmes graines)

| Niveau | Guerre FR-EN moy. [min-max] | Graines dans la bande | Trêves / siècle moy. [max] | 1re faction fin moy. [max] | Banqueroutes | Majeures 1400 | Angl. domine le royaume |
|---|---|---|---|---|---|---|---|
| Facile (5) | 71 % [66-74] | **5/5** | 11,4 [12] | 24 % [27] | 0,09 | 5/5 | 0 % |
| Normale (10) | 69 % [59-77] | **8/10** (75,4 et 76,7 %, juste au-dessus) | 12,7 [14] | 22 % [29] | 0,05 | 10/10 | 0-25 % |
| Difficile (10) | 66 % [59-75] | **10/10** | 14,5 [17] | 24 % [30] | 0,04 | 10/10 | 0-74 % |
| Très difficile (5) | 65 % [58-71] | **5/5** | 15,8 [17] | 28 % [31] | 0,03 | 5/5 | 47-76 % |

Graines : facile 72 73 74 66 72 ; normale 59 69 73 72 75 70 70 77 61 66 ; difficile 60 66 69 61
75 63 68 68 66 59 ; très difficile 58 62 64 69 71. Aucune faction éliminée.

`balance_probe campaign` 200 tours (normale), lots 1-8 / 9-16 :

| Mesure | Après fusion, sans règle | v9 (retenu) | Cible |
|---|---|---|---|
| Guerre FR-EN | 72 / 70 % | 69 / 72 % | 55-75 % |
| Révoltes / partie (hors rebelles) | 4,0 / 3,1 (3,6) | 3,2 / 2,2 (2,7) | 4-10 |
| Banqueroutes / fac. / déc. | 0,05 / 0,05 | 0,05 / 0,05 | < 0,5 |
| Impôt Haut | 32 / 34 % | 29 / 34 % | < 40 % |
| Milice | 27,8 / 28,2 % | 28,2 / 28,5 % | < 40 % |

### Essais de la reprise

| Essai | Facile (5) | Normale (10) | Difficile (10) | Très difficile (5) | Révoltes balance |
|---|---|---|---|---|---|
| m3 : main fusionné, règles v8 | 62 % [55-73], 4/5, 11,4 | 65 % [31-78], 8/10, 11,6 | 65 % [59-69], 10/10, 14,0 | 57 % [50-65], 3/5, 15,4 | 4,0 / 3,1 |
| parenté seule | — | — | — | — | 4,0 / 3,2 |
| fatigue seule (seuil de déclaration) | — | — | — | — | 3,2 / 2,6 |
| **v9 (retenu)** : parenté + fatigue (seuil de déclaration) | 71 % [66-74], 5/5, 11,4 | 69 % [59-77], 8/10, 12,7 | 66 % [59-75], 10/10, 14,5 | 65 % [58-71], 5/5, 15,8 | 3,2 / 2,2 |
| v10 : parenté + fatigue au seuil `sue_weariness` (70) | 72 % [65-78], 4/5, 11,6 | 70 % [61-76], 8/10, 13,3 | 67 % [58-71], 10/10, 14,4 (1re faction 39 %) | 62 % [51-71], 4/5, 15,4 | 2,4 / 3,9 |

v10 ne relève pas les révoltes de façon mesurable (3,2 contre 2,7 : dans le bruit, ±0,8 entre
lots) et tient les critères de guerre plus juste : v9 retenu.

### Points ouverts

- **Révoltes `balance_probe` sous la bande 4-10** : 3,6 déjà après la fusion de main, sans
  règle EQ6 (dette connue du lot DC, `docs/archive/chantiers.md` : 3,2 contre 5,7 avant DC) ;
  2,7 avec v9. La règle de fatigue retire des guerres d'appoint, donc des provinces occupées
  (la moitié des révoltes éclatent en province occupée). Relever les révoltes relève de l'ordre
  public (lot DC/EQ1 : pondérer la garnison qui apaise), hors du périmètre d'EQ6.
- Très difficile : l'Angleterre domine le royaume 47-76 % du siècle (la France de la sonde est
  une IA avec les handicaps du joueur) ; trêves 15,8 en moyenne, 17 sur deux graines.
- Le bonus « Mariage entre nos maisons » s'empile sans plafond (jusqu'à +150) ; les
  ambassades de héraut aussi (6 à la fois vues en trace). EQ6 le neutralise pour la guerre de
  prétention seulement ; un plafond général par motif changerait toute la diplomatie : à
  traiter à part si besoin.
- Chaos entre graines toujours large ; en normale, graines 5 et 8 à 75,4 et 76,7 % (hors bande
  par le haut, de peu) : la guerre est désormais plutôt trop présente que trop rare.

### Vérifications finales (2026-09-27, après fusion + v9)

- `cargo fmt --all --check` : OK.
- `cargo clippy --workspace --all-targets -- -D warnings` : OK.
- `cargo test --workspace` : OK (aucun échec ; `eq6_main_claim` 6/6).
- `core/build.sh` : OK ; `godot --headless --path game --import` : OK ;
  `smoke.gd` : « smoke OK » (sortie 0 ; seuls des avertissements de fixtures manquantes).
- `uv run --project tools pytest` : 775 passés, 2 ignorés.
- `balance_probe campaign 200` (normale), lots 1-8 et 9-16 : voir tableau ci-dessus.
- Combat tactique non touché (seules la diplomatie de l'IA et la sonde changent).

## État antérieur (2026-09-26) : EN PAUSE (session suspendue par le joueur, 2026-09-26) — pas prêt à fusionner

### Où l'on en est (lire d'abord)

- main fusionné une seconde fois (commit de fusion après 4f07e4db) : main apportait des
  changements de campagne (Q5 : négociation, sièges, mouvements) qui redistribuent les guerres.
- Après cette fusion, l'Écosse disparaissait en facile (graine 2, 1396) : la règle 4
  (« l'acculé attend d'être battu ») s'appliquait aussi à l'Écosse entrée en guerre à l'appel de
  la France. Correctif (commit 7931b970) : la règle ne vaut que contre un prétendant au trône de
  l'acculé (l'Angleterre contre la France) ; l'Écosse traite aussitôt comme avant. Vérifié sur
  la graine 2 : l'Écosse vit.
- **Mesure finale après fusion + correctif** (`century_probe` 464 tours, binaire `cp_m2`) :

| Niveau | Guerre FR-EN | Trêves | Majeures 1400 | 1re faction fin (max) | Banqueroutes |
|---|---|---|---|---|---|
| Facile (5) | 66 % [52-72], **4/5** | 12,2 | 5/5 | 30 % | 0,07 |
| Normale (10) | 67 % [59-73], **10/10** | 13,1 | 10/10 | 30 % | 0,07 |
| Difficile (10) | 56 % [44-64], **5/10** ✗ | 16,0 | 10/10 | 39 % | 0,03 |
| Très difficile (5) | 57 % [39-68], **4/5** | 18,6 ✗ | 5/5 | 35 % | 0,01 |

  `balance_probe` 16 × 200 (normale, même binaire) : guerre FR-EN 65 / 70 %, révoltes 5,2 / 4,9
  par partie (cible 4-10 ✓), banqueroutes 0,07 / 0,06, impôt Haut 30 / 32 %, milice 27 %.

- **Ce qui manque** : le niveau difficile retombe à 5/10 graines après la fusion de main
  (8/10 avant la fusion, v8) ; trêves très difficile 18,6 (> 16) ; 1re faction jusqu'à 39 % en
  difficile (Angleterre dominant le royaume 59-86 % du siècle). Les tableaux « avant / après »
  plus bas sont ceux d'avant la seconde fusion (v8).
- **Vérifications** : `cargo fmt`, `cargo clippy --all-targets -D warnings` et `cargo test`
  (workspace) verts après la fusion mais AVANT le correctif Écosse ; après le correctif, seuls
  `cargo fmt` et les tests `eq6_main_claim` (4/4) ont tourné. `core/build.sh`, import Godot et
  `smoke.gd` OK après le correctif. `pytest` vert avant la seconde fusion. À refaire : clippy +
  `cargo test` complets.

### Prochaine étape (reprise)

1. `cargo clippy --all-targets -- -D warnings` et `cargo test` complets.
2. Difficile : diagnostiquer avec le tableau « EQ6 » de `century_probe`
   (`DIFFICULTY=hard … 464 1 … 10`) pourquoi la part retombe après la fusion (obstacles
   dominants : trêve, fatigue, guerres courtes contre une France dominée ?). Pistes : l'Angleterre
   domine davantage la France en difficile depuis Q5 (sièges) → guerres courtes ; essayer
   `min_war_turns` 20 → 24 ou le seuil de l'acculé, en vérifiant l'Écosse et la normale.
3. Remettre à jour les tableaux « avant / après » et l'ADR 0085 (section Conséquences) avec la
   mesure finale, puis `git merge main` et rapport.

Commandes de reprise :

```
cd core && cargo build --release -p ai --example century_probe --example balance_probe
DIFFICULTY=hard target/release/examples/century_probe 464 1 2 3 4 5 6 7 8 9 10
```

## Historique (avant la seconde fusion de main)

- [x] Sonde `century_probe` : tableau « EQ6 » (guerres ouvertes par l'Angleterre / la France,
      tours de paix et obstacles à la déclaration anglaise), `WAR_TRACE=1` (en paix : raisons et
      garde-fous de l'Angleterre tous les 5 ans ; en guerre : fatigue, score, trésor, provinces
      et évaluation d'une paix blanche des deux côtés).
- [x] Diagnostic par niveau (ci-dessous).
- [x] Quatre règles derrière des réglages de `data/ai/diplomacy.json` (schéma à jour, défaut
      sans le fichier = comportement antérieur) ; aucune donnée de difficulté changée.
- [x] Tests `sim-campaign/tests/eq6_main_claim.rs` (4) ; mesures ci-dessous.
- [x] ADR 0085.

## Règles (ADR 0085)

| Réglage | Valeur | Effet |
|---|---|---|
| `war.main_claim_first` | vrai | La prétention principale (trône du plus grand royaume revendiqué : la France pour l'Angleterre) passe avant toute autre et n'attend pas le repos de 12 tours après une autre déclaration. |
| `war.claim_war_ignores_difficulty` | vrai | Pour cette guerre, ni l'attitude ni le rapport de forces du niveau de difficulté ne comptent. |
| `negotiation.long_war_years` / `_points_per_year` / `_max_points` | 8 / 8 / 100 | « Guerre interminable » : au-delà de 8 ans, +8 points par an (plafond 100) en faveur d'un traité de paix, vainqueur compris. |
| `peace.cornered_waits_for_defeat` | vrai | Une couronne acculée ne demande la paix avant `min_war_turns` qu'une fois battue (score ≤ -25), plus la saison même de la déclaration. |

## Diagnostic (référence main)

Obstacles comptés à chaque tour de paix FR-EN :

- **Facile** : l'attitude anglaise envers la France dépasse 20 (seuil de la guerre de
  prétention) dans 60-70 % des tours de paix. La raison « Niveau de difficulté » (+10 envers le
  joueur) s'ajoute à « Même foi » (+10), aux mariages (+30) ou aux ambassades de hérauts (+18) :
  sans elle, l'attitude reste sous 20. Graine 4 : paix de 1420 à 1449.
- **Difficile / très difficile** : le « repos » de 12 tours après toute déclaration bloque
  50-80 % des tours de paix. L'Angleterre, plus riche, déclare sans cesse d'autres guerres
  (croisade contre Grenade ; prétention héritée par mariage sur Vérone tous les 3 ans de 1403 à
  1448 en difficile graine 4) ; `war_target` classait les prétentions par rapport de forces, donc
  la plus petite couronne revendiquée passait avant la France.
- **Normale** : même mécanisme, moins marqué ; graines 4 et 8 bloquées par l'attitude (mariages,
  hérauts).
- **Guerres de 60 ans** (facile graine 1, déjà 72 ans sur main) : l'Angleterre est à 100 de
  fatigue et -82 de score pendant 50 ans ; la France gagne sans rien pouvoir prendre, refuse
  toute paix blanche (article « Paix » à -88 pour elle) et ne se lasse jamais (+1 par guerre,
  -4 de récupération par tour).
- **Très difficile** : la France de la sonde (IA avec les handicaps du joueur) est dominée par
  l'Angleterre 80-90 % du siècle ; réduite à sa capitale, elle est « acculée » et achète la paix
  la saison même de chaque déclaration anglaise (graine 4 : une paix tous les 2 ans de 1399 à
  1448 ; jusqu'à 21 guerres de 0 tour par siècle).

## Avant / après (century_probe 464 tours, mêmes graines qu'EQ4/EQ5)

| Mesure | Facile (5) | Normale (10) | Difficile (10) | Très difficile (5) | Cible |
|---|---|---|---|---|---|
| Guerre FR-EN | 48 % [32-70], 1/5 → **63 % [50-70], 4/5** | 62 % [42-75], 8/10 → **69 % [60-72], 10/10** | 56 % [39-65], 5/10 → **60 % [45-72], 8/10** | 50 % [41-60], 1/5 → **59 % [52-67], 4/5** | 55-75 %, 8/10 (4/5) |
| Trêves FR-EN / siècle | 6,4 → 11,6 | 11,1 → 12,7 | 13,0 → 16,2 | 15,4 → 15,6 | 6-16 |
| Plus longue guerre (ans) | 6-72 → 9-12 | 9-18 → 7-14 | 6-11 → 5-10 | 5-10 → 6-9 | — |
| Banqueroutes / fac. / déc. | 0,08 → 0,06 | 0,04 → 0,04 | 0,03 → 0,04 | 0,01 → 0,02 | ≤ 0,25 |
| Révoltes / 200 tours (siècle) | 5,0 → 4,6 | 3,6 → 3,9 | 4,4 → 4,5 | 7,2 → 4,9 | — |
| 1re faction fin (moy., max) | 24 %, 27 → 23 %, 27 | 22 %, 26 → 23 %, 32 | 23 %, 34 → 25 %, 33 | 33 %, 36 → 30 %, 34 | ≤ ~35 % |
| Majeures en vie en 1400 | 5/5 → 5/5 | 10/10 → 10/10 | 10/10 → 10/10 | 5/5 → 5/5 | toutes |
| France éliminée | 0 → 0 | 0 → 0 | 0 → 0 | 1/5 (1434) → **0/5** | — |
| Angleterre dominant le royaume (part du siècle) | 0 % → 0 % | 0-18 % → 0-23 % | 0-77 % → 4-85 % | 42-90 % → 58-81 % | — |

`balance_probe campaign` 16 × 200 (normale) : guerre FR-EN 66 % / 64 % (EQ5 61-65),
révoltes par partie **5,4 / 7,5, moyenne 6,5** (EQ5 3,9 ; cible 4-10), banqueroutes 0,12 / 0,08
(EQ5 0,07-0,11), impôt Haut 34 / 37 % (EQ5 30-32 ; cible < 40), milice 28,8 %.

### Graine par graine (guerre FR-EN / trêves / révoltes / 1re faction fin / éliminées)

Facile :

| graine | main | EQ6 |
|---|---|---|
| 1 | 70 % / 3 / 16 / France 25 % / — | 70 % / 12 / 15 / France 20 % / Bohême |
| 2 | 44 % / 8 / 1 / France 23 % / — | 66 % / 14 / 13 / France 20 % / — |
| 3 | 47 % / 7 / 2 / France 27 % / — | 59 % / 12 / 2 / France 23 % / Écosse (1436) |
| 4 | 32 % / 6 / 8 / France 23 % / — | 70 % / 13 / 9 / France 27 % / Portugal |
| 5 | 48 % / 8 / 31 / France 23 % / — | 50 % / 7 / 14 / France 24 % / Bohême, Portugal |

Normale :

| graine | main | EQ6 |
|---|---|---|
| 1 | 65 % / 10 / 5 / France 26 % | 68 % / 13 / 9 / France 24 % |
| 2 | 75 % / 11 / 5 / France 20 % | 60 % / 13 / 12 / Angl. 27 % |
| 3 | 71 % / 12 / 15 / France 21 % | 72 % / 14 / 8 / Angl. 16 % |
| 4 | 49 % / 7 / 4 / France 21 % | 71 % / 13 / 9 / Angl. 22 % |
| 5 | 65 % / 13 / 3 / Empire 25 % | 70 % / 13 / 10 / France 21 % |
| 6 | 73 % / 13 / 16 / Angl. 20 % | 71 % / 12 / 4 / Angl. 20 % |
| 7 | 68 % / 13 / 3 / Angl. 21 % | 68 % / 12 / 9 / France 32 % |
| 8 | 42 % / 8 / 16 / France 23 % | 67 % / 12 / 2 / France 28 % |
| 9 | 61 % / 13 / 12 / Angl. 18 % | 72 % / 11 / 19 / France 27 % |
| 10 | 56 % / 11 / 4 / Angl. 21 % | 69 % / 14 / 9 / France 17 % |

Difficile :

| graine | main | EQ6 |
|---|---|---|
| 1 | 62 % / 14 / 1 / Empire 20 % | 64 % / 16 / 8 / Empire 18 % |
| 2 | 55 % / 17 / 7 / Angl. 34 % | 64 % / 16 / 11 / Angl. 33 % |
| 3 | 39 % / 11 / 6 / Empire 17 % | 72 % / 15 / 14 / Angl. 26 % |
| 4 | 48 % / 9 / 13 / Angl. 25 % | 62 % / 19 / 10 / Angl. 33 % |
| 5 | 52 % / 14 / 10 / Angl. 23 % | 66 % / 16 / 15 / Angl. 17 % |
| 6 | 49 % / 9 / 31 / Angl. 21 % | 47 % / 14 / 14 / Angl. 23 % |
| 7 | 63 % / 12 / 6 / France 23 % | 59 % / 16 / 3 / Angl. 23 % |
| 8 | 65 % / 14 / 9 / Angl. 22 % | 45 % / 18 / 11 / Angl. 29 % |
| 9 | 61 % / 14 / 6 / Angl. 22 % | 61 % / 14 / 14 / Angl. 24 % |
| 10 | 62 % / 16 / 14 / Angl. 28 % | 61 % / 18 / 4 / Angl. 22 % |

Très difficile :

| graine | main | EQ6 |
|---|---|---|
| 1 | 60 % / 14 / 12 / Angl. 31 % / — | 61 % / 16 / 12 / Angl. 33 % / Gueldre |
| 2 | 45 % / 13 / 6 / Angl. 36 % / **France (1434)** | 60 % / 15 / 5 / Angl. 34 % / — |
| 3 | 48 % / 19 / 46 / Angl. 33 % / — | 58 % / 16 / 13 / Angl. 32 % / — |
| 4 | 41 % / 16 / 11 / Angl. 32 % / — | 52 % / 15 / 5 / Angl. 17 % / — |
| 5 | 53 % / 15 / 9 / Angl. 31 % / — | 67 % / 16 / 22 / Angl. 32 % / — |

## Essais

| Essai | Facile (5) | Normale (10) | Difficile (10) | Très difficile (5) |
|---|---|---|---|---|
| main (référence) | 48 % [32-70], 1/5, 6,4 trêves | 62 % [42-75], 8/10, 11,1 | 56 % [39-65], 5/10, 13,0 | 50 % [41-60], 1/5, 15,4 |
| v1 : prétention principale d'abord + indépendante de la difficulté | 74 % [64-83], 3/5 | 68 % [62-75], 10/10, 13,6 | 60 % [50-67], 8/10, 16,5 | 48 % [35-58], 1/5 |
| v1 + très difficile moral IA 10 → 7 | — | — | — | 56 % [36-67], 4/5, 17,8 ; 1re faction jusqu'à 46 % |
| v4 : v1 + « Guerre interminable » 10 ans / 3 pts / 30 max + moral 7 | 71 % [62-78], 3/5, 12,0 | 67 % [59-75], 10/10, 13,3 | 62 % [50-73], 7/10, 14,8 | 50 % [40-61], 1/5, 16,8 |
| v6 : 8 ans / 8 pts / 100 max + très difficile revenus IA 130, moral 7 | 65 % [55-73], 5/5, 12,6 | — | — | 54 % [44-69], 1/5, 19,4 |
| v7 : données de difficulté rendues à DF1, acculé attend un score < 0 | — | — | 60 % [52-68], 8/10, 15,2 | 59 % [51-70], 3/5, 17,2 |
| **v8 (retenu)** : acculé attend d'être battu (score ≤ -25) | 63 % [50-70], 4/5, 11,6 | 69 % [60-72], 10/10, 12,7 | 60 % [45-72], 8/10, 16,2 | 59 % [52-67], 4/5, 15,6 |

- Les retouches des données de difficulté (moral, revenus de l'IA en très difficile) ne donnent
  rien de net : l'écart entre graines l'emporte. Rendues aux valeurs DF1.
- « Guerre interminable » à 30 points ne suffisait pas (l'article « Paix » vaut -88 pour un
  vainqueur à +82) : 100 points au plus, atteints après ~20 ans de guerre.
- La règle de l'acculé supprime les guerres « de 0 tour » (8-21 par graine en très difficile →
  0-1) ; au seuil -25, la France tient assez pour que la guerre compte.

## Points ouverts

- Chaos : chaque changement redistribue les guerres ; écarts par graine larges (difficile
  45-72 %). Les critères sont atteints de justesse en facile (4/5) et en très difficile (4/5).
- Trêves en difficile 16,2 par siècle (borne haute 16) : guerres courtes (5-10 ans) contre une
  France affaiblie.
- Très difficile : l'Angleterre domine encore le royaume 58-81 % du siècle ; la France de la
  sonde n'est plus éliminée, mais c'est une IA avec les handicaps du joueur, pas un joueur.
  Pas de retouche des données DF1 (aucun effet net mesuré) ; à revoir avec des parties réelles.
- Facile : l'Écosse tombe en 1436 sur une graine (après 1400) ; mineures éliminées un peu plus
  souvent (Bohême, Portugal).
- Impôt Haut 34-37 % dans `balance_probe` (EQ5 30-32, cible < 40) : plus de guerre, plus
  d'impôt ; à surveiller.
- La prétention principale est choisie par taille du royaume revendiqué : une IA qui hériterait
  d'un trône plus grand que la France (improbable) changerait de guerre principale.

## Commandes

```
cd core && cargo build --release -p ai --example century_probe --example balance_probe
DIFFICULTY=easy target/release/examples/century_probe 464 1 2 3 4 5
DIFFICULTY=hard target/release/examples/century_probe 464 1 2 3 4 5 6 7 8 9 10
WAR_TRACE=1 DIFFICULTY=very_hard target/release/examples/century_probe 464 4
target/release/examples/balance_probe campaign 200 9 10 11 12 13 14 15 16
```

## Prochaine étape
Fusion ff dans main par le coordinateur (branche `feat/eq6-war-all-difficulties`).
