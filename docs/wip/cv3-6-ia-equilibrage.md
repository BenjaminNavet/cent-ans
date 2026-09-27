# CV3-6 — IA des postures et des rencontres, équilibrage CV3

Branche : `worktree-agent-abcec86700c5ee43d` (worktree privé, base 5704fa8c).
Spec : `docs/design/2026-09-27-campagne-vivante.md` § 0 et § 5. ADR 0094 (addendum CV3-6), ADR 0085 (bande EQ6).

## État : TERMINÉ (prêt à fusionner)

## Choix
- L'IA de campagne réelle est `core/crates/ai/src/campaign.rs` (`plan_armies`) + `grid.rs` ;
  `sim_campaign::ai_minimal` ne sert qu'aux tests de la simulation : non modifié.
- Module `core/crates/ai/src/stances.rs`, appelé par `plan_armies` :
  - **embuscade** (`ambush_orders`, `keep_ambush`) : armée plus faible (puissance 0,25-1,0 × celle
    de l'ennemi) qu'une armée ennemie dont la marche de plusieurs tours (`planned_path`, coupée à
    `route_turns` = 2 tours pleins) traverse une province qu'elle possède ; case couverte à moins de
    0,9 × rayon de ZdC de cette route et hors de la ZdC actuelle de l'ennemi : sur place, ou après
    un déplacement qui laisse les 25 % de mouvement requis. Garde la posture tant que la menace dure
    et qu'aucun ennemi ne l'a découverte, sinon `SetStance Normal`. Pas d'attaque en embuscade.
    Poids par personnalité : 800 ‰ − 10 ‰ par point d'agressivité au-dessus de 50.
  - **marche forcée** (`forced_march_orders`) : objectif Défendre (place amie assiégée) ou Siège
    (place que notre camp assiège déjà), coût de route > allocation et ≤ 0,9 × allocation de marche
    forcée ; jamais si la puissance ennemie à 12 km de la cible dépasse la sienne ; pour rejoindre
    un siège, arrêt au pied des murs (`move_to_point`, la marche forcée n'entre pas dans une place
    ennemie) ; pas de traversée maritime.
  - **camp retranché** (`should_entrench`) : armée en rase campagne, dans une province à elle et
    frontalière, puissance ennemie autour ≥ la sienne, sans abri atteignable ce tour (pas
    d'objectif, ou repli/regroupement hors de portée).
  - **rencontres** (`encounter_detour`) : armée sans objectif (ou simple regroupement), royaume en
    paix (`detour_at_war` faux), aucun siège en cours pour ou contre elle ; site vu (≤ 30 km), hors
    terres fermées et hostiles, atteint ce tour (prévisualisation exacte) ; 300 ‰ par tour.
- Réglages : sections `postures` et `encounters` de `data/ai/grid.json` (schéma
  `ai_grid.schema.json`, tests `tools/tests/test_ai_grid_schema.py`). Défauts du cœur = aucune
  posture CV3 (comportement antérieur).
- Aucun ordre refusé : `posture::validate_stance_change` et `CampaignState::preview_march_to_point`
  (nouvelle, `march.rs`, même `simulate` que la vraie marche).
- `GridPlanner` ignore les armées ennemies cachées en embuscade (ni attaquées ni évitées).
- Chances = tirages purs (graine, tour, armée ; `alignment::campaign_roll`) : RNG intact.
- `century_probe` : `CV3_STATS=1` imprime le tableau « CV3 » (ordres de posture, embuscades
  réussies/éventées, rencontres résolues, batailles, classes de résultat, révoltes, taux par 20
  tours). Limite : les embuscades déclenchées pendant la phase « joueur » de la France (ordres
  soumis hors fin de tour) ne sont pas comptées.

## Mesures (binaires release, graines EQ6 ; avant = main 5704fa8c, après = réglages retenus)

`century_probe` 464 tours (`CV3_STATS=1 DIFFICULTY=…`) :

| Niveau | Guerre FR-EN avant | Guerre FR-EN après | Trêves / siècle av. → ap. | Tentatives d'embuscade / 20 t. (réussies + éventées) | Marches forcées / 20 t. | Camps retranchés / 20 t. | Rencontres / 20 t. av. → ap. |
|---|---|---|---|---|---|---|---|
| Facile (5) | 71 % [68-77], 4/5 | 68 % [64-75], **5/5** | 11,2 → 11,0 | 0,65 + 0,29 | 0,18 | 0,00 | 0,22 → 0,63 |
| Normale (10) | 71 % [65-75], 10/10 | 68 % [61-73], **10/10** | 13,0 → 13,3 | 0,90 + 0,45 | 0,27 | 0,06 | 0,20 → 0,76 |
| Difficile (10) | 69 % [66-76], 9/10 | 67 % [59-71], **10/10** | 14,6 → 15,1 | 1,31 + 0,78 | 0,34 | 0,06 | 0,16 → 0,79 |
| Très difficile (5) | 63 % [55-69], 5/5 | 60 % [47-72], **4/5** | 15,6 → 16,2 | 1,06 + 0,72 | 0,31 | 0,15 | 0,23 → 0,98 |

Ordres d'embuscade par partie (464 tours, toutes factions) : facile 13-35, normale 30-62,
difficile 36-94, très difficile 45-77. Classes de résultat (camps, normale après) : héroïque 8,6 %,
décisive 10,6 %, à la Pyrrhus 0 %, victoire 30,8 %, défaite honorable 0,5 %, désastre 11,5 %,
défaite 38,0 % — six classes dans chaque partie (avant : mêmes proportions à ±1,5 point).

`balance_probe campaign 200` (normale), lots 1-8 / 9-16 :

| Mesure | Avant | Après | Cible |
|---|---|---|---|
| Guerre FR-EN | 72 / 69 % | 74 / 73 % | 55-75 % |
| Révoltes / partie (hors rebelles) | 3,8 / 3,4 (3,6) | 3,5 / 4,6 (4,1) | 4-10 (pas aggraver) |
| Banqueroutes / fac. / déc. | 0,05 / 0,10 | 0,09 / 0,08 | < 0,5 |
| Impôt Haut | 30 / 31 % | 32 / 33 % | < 40 % |
| Milice | 28,3 / 27,7 % | 27,1 / 28,0 % | < 40 % |

## Essais (très difficile, 5 graines, même binaire)
| Essai | Guerre FR-EN | Graines dans la bande |
|---|---|---|
| Tout actif, embuscade sur terres possédées ou occupées, détours en guerre sans menace | 56 % [49-68] | 2/5 |
| Sans embuscade | 58 % [52-66] | 4/5 |
| Embuscade seule | 64 % [61-71] | 5/5 |
| Sans marche forcée | 57 % [49-66] | 4/5 |
| Sans détours | 62 % [51-71] | 4/5 |
| Détours en paix seulement | 60 % [51-69] | 4/5 |
| **Retenu** : détours en paix, embuscade sur terres possédées | 60 % [47-72] | 4/5 |

Le bruit entre essais (±4 points) domine ; les nouveautés renforcent un peu l'Angleterre IA contre
la France « joueur » handicapée (Angleterre dominant le royaume plus souvent), d'où des guerres un
peu plus courtes. Aucun réglage de `battle_outcome.json`, `postures.json` ni `encounters.json` n'a
été nécessaire : la bande tient, les nouveautés sont présentes sans être omniprésentes.

## Mesures exploratoires (century_probe 120 tours, graines 1-2, normale, tout à 1000 ‰)
- Entonnoir de l'embuscade (appels / menace / rapport de force / case couverte) :
  4432 / 86 / 26 / 13 avec les réglages initiaux (80 km, 1 tour, 0,35-0,9) → 0,67 ordre / 20 tours ;
  4181 / 174 / 79 / 19 avec 150 km, 2 tours, 0,25-1,0 → 1,5 ordre / 20 tours. Le goulot : une
  armée ennemie en marche de plusieurs tours vers nos terres, et l'embusqué assez près de sa route.
- Classe « à la Pyrrhus » : 0 % en auto-résolution même au seuil 0,35 (0,3 % à 0,2) ; les
  vainqueurs perdent rarement 20 % : seuil de la spec (0,5) gardé, la classe reste propre à la 3D.

## Points ouverts
- Très difficile à 4/5 (graine 1 à 47 %) : dans le critère EQ6 (4/5) mais sous le 5/5 d'avant.
- Camp retranché rare (0-0,15 / 20 tours) : les armées de l'IA stationnent presque toujours dans
  une place ; la condition « sans abri atteignable » le réserve aux armées surprises en campagne.
- « À la Pyrrhus » jamais atteinte en auto-résolution (voir plus haut).
- Une armée ennemie sans marche de plusieurs tours n'a pas de route prévisible : pas d'embuscade
  contre elle (fidèle à la spec, limite le nombre d'embuscades).
