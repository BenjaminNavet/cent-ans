# EQ6 — guerre France-Angleterre 55-75 % à tous les niveaux de difficulté

Branche : `feat/eq6-war-all-difficulties` (main fusionné).

Décision du joueur (2026-09-26) : la cible « France et Angleterre en guerre 55-75 % du siècle »
vaut à tous les niveaux, pas seulement en Normale.

## État : en cours

- [x] Mesure de référence (century_probe 464 tours, mêmes graines qu'EQ4/EQ5) : identique à EQ5.
- [x] Sonde : tableau « EQ6 » (guerres ouvertes par l'Angleterre / la France, tours de paix et
      obstacles à la déclaration anglaise), `WAR_TRACE=1` (détail tous les 5 ans).
- [x] Diagnostic par niveau (ci-dessous).
- [x] Correctif v1 : `war.main_claim_first` + `war.claim_war_ignores_difficulty`
      (`data/ai/diplomacy.json`, cœur `diplomacy.rs::war_target`).
- [ ] Mesure v1, réglage, non-régression EQ4/EQ5.
- [ ] ADR 0085.

## Diagnostic (référence main)

Obstacles comptés à chaque tour de paix FR-EN (plusieurs par tour possibles) :

- **Facile** : l'attitude de l'Angleterre envers la France dépasse 20 (seuil de la guerre de
  prétention) dans 60-70 % des tours de paix. La raison « Niveau de difficulté » (+10 envers le
  joueur) s'ajoute à « Même foi » (+10), aux mariages (+30) ou aux ambassades de hérauts (+18) :
  sans elle, l'attitude reste sous 20. Graine 4 : paix de 1420 à 1449.
- **Difficile / très difficile** : le « repos » de 12 tours après toute déclaration de guerre
  bloque 50-80 % des tours de paix. L'Angleterre, plus riche, déclare sans cesse d'autres guerres
  (croisade contre Grenade, prétention héritée par mariage sur Vérone tous les 3 ans de 1403 à
  1448 en difficile graine 4) ; `war_target` classe les prétentions par rapport de forces, donc la
  plus petite couronne revendiquée passe avant la France. En très difficile, guerres courtes
  (15 trêves) : la France affaiblie cède vite des provinces.
- **Normale** : même mécanisme, moins marqué (l'Angleterre moins riche déclare moins ailleurs) ;
  graines 4 et 8 bloquées par l'attitude (mariages, hérauts).

- **Facile, guerres de 60 ans** (graine 1, déjà 72 ans sur main) : l'Angleterre est à 100 de
  fatigue et -82 de score pendant 50 ans, la France gagne sans rien pouvoir prendre (aucune
  province anglaise tenue) et refuse toute paix payée ; le vainqueur ne se lasse jamais
  (+1 par guerre, -4 de récupération par tour).
- **Très difficile** : la France de la sonde (IA avec les handicaps du joueur) est dominée par
  l'Angleterre 80-90 % du siècle dès 1342 ; réduite à sa capitale, elle est « acculée » et signe
  la paix le tour même de chaque déclaration anglaise (graine 4 : une paix tous les 2 ans de 1399
  à 1448, 21 guerres de 0 tour).

## Essais

| Essai | Facile (5) | Normale (10) | Difficile (10) | Très difficile (5) |
|---|---|---|---|---|
| main (référence) | 48 % [32-70], 1/5, 6,4 trêves | 62 % [42-75], 8/10, 11,1 | 56 % [39-65], 5/10, 13,0 | 50 % [41-60], 1/5, 15,4 |
| v1 : prétention principale d'abord + indépendante de la difficulté | 74 % [64-83], 3/5 | 68 % [62-75], 10/10, 13,6 | 60 % [50-67], 8/10, 16,5 | 48 % [35-58], 1/5 |
| v1 + très difficile moral IA 10 → 7 | — | — | — | 56 % [36-67], 4/5, 17,8 ; 1re faction jusqu'à 46 % |
| v4 : v1 + « Guerre interminable » 10 ans / 3 pts / 30 max + moral 7 | 71 % [62-78], 3/5, 12,0 | 67 % [59-75], 10/10, 13,3 | 62 % [50-73], 7/10, 14,8 | 50 % [40-61], 1/5, 16,8 |
| v6 : v4 avec 8 ans / 8 pts / 100 max + très difficile revenus IA 130 | 65 % [55-73], **5/5**, 12,6 | — | — | 54 % [44-69], 1/5, 19,4 |
| v7 : v6, données de difficulté rendues à DF1, acculé attend un score < 0 | — | — | 60 % [52-68], 8/10, 15,2 | 59 % [51-70], 3/5, 17,2 |
| v8 : v7, acculé attend d'être battu (score ≤ -25) | (en cours) | (en cours) | (en cours) | **59 % [52-67], 4/5, 15,6** ; 1re faction ≤ 34 % |

Lecture des essais :

- La difficulté des données (moral, revenus de l'IA en très difficile) ne change rien de net :
  l'écart entre graines l'emporte ; rendues aux valeurs DF1 (aucune donnée de difficulté changée).
- Très difficile : les guerres « de 0 tour » (8 à 21 par siècle et par graine) venaient de la
  France acculée qui achetait la paix la saison même de la déclaration ; la règle « l'acculé
  attend d'être battu » les supprime (0-1 par graine).

## Prochaine étape
Mesure v8 complète (normale, facile, difficile), `balance_probe` 16 × 200, `cargo test`,
puis ADR 0085 et addendum.
