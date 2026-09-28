# 0100 — Ordre public pondéré et seuil de révolte (lot RS-B)

Date : 2026-09-28. Statut : proposé.
Suivi : `docs/wip/rs-b-order.md`. Prolonge l'ADR 0082 (carte densifiée, `province_effect_percent`).

## Contexte
- Depuis la densification (lot DC, ADR 0082), une province compte ~10 places. Les effets de bâtiments
  sur toute la province ont été pondérés par `province_effect_percent` (cité 100 %, autres 50 %),
  mais d'autres sommes restaient au plein poids : garnison qui apaise, résistance à la peste,
  revenu estimé par l'IA, évaluation des bâtiments par l'IA.
- Les révoltes étaient tombées sous la cible 4-10 par partie de 200 tours (EQ1) : 2,7 à 3,6 selon
  les mesures d'EQ6, 4,1 sur main au 28/09 (`balance_probe`, graines 1-16), avec un bruit énorme
  (0 à 15 selon la graine).
- Plusieurs constantes économiques restaient codées en dur dans `economy.rs`.

## Décision
1. **Constantes économiques en données** (`data/rules/economy.json`, schéma `economy_rules`) :
   `tax_efficiency`, `tax_per_head`, `tax_rates` (multiplicateur et part prélevée de chaque barème),
   `production_tax_share`, `upkeep_months_per_season`, `garrison_upkeep_percent`,
   `garrison_relief_*`, `garrison_reinforce_*`. Valeurs inchangées.
2. **Sommes pondérées par `province_effect_percent`** :
   - garnison qui apaise : `CampaignState::weighted_garrison_strength` (celle de la cité en entier,
     celle des autres places pour moitié) ;
   - résistance à la peste : `province_building_effects` au lieu des bâtiments bruts ;
   - IA : revenu estimé d'une province avec les effets pondérés, et évaluation d'un bâtiment
     (apaisement, santé, croissance au poids de province de la place, recherche à `research_percent`).
3. **Révolte** (`data/rules/population.json`) : après **2 saisons** (3 avant) au-delà de 75 de
   mécontentement moyen. Les seuils (75, et 90 pour le passage aux rebelles) ne changent pas.
4. Sondes recalibrées : `m3_grid_ai::fifty_turns_on_eight_seeds_stay_in_the_c7a_band`, plancher des
   sièges par tour 1,5 → 1,3 (la pondération de l'IA les baisse de 1,65 à 1,49 ; 1,40 après la
   fusion de main du 28/09 : marge pour le chaos entre versions) ;
   `cv3_ai_stances::the_ai_never_gives_a_stance_order_the_core_refuses`, graine 7 → 4 (la graine 7
   ne tendait plus d'embuscade).

## Mesures (`balance_probe campaign 200`, normale, graines 1-16)
| Variante | Révoltes / partie | Guerre FR-EN | Banqueroutes / fac. / déc. | Impôt Haut |
|---|---|---|---|---|
| main + constantes | 4,1 | 74 % | 0,08 | 33 % |
| + pondérations | 2,8 | 69 % | 0,10 | 32 % |
| + 2 saisons | 4,8 | 69 % | 0,10 | 33 % |
| + 2 saisons, seuil 72 | 11,1 | 69 % | 0,08 | 30 % |
| + 2 saisons, seuil 74 | 7,8 | 69 % | 0,13 | 31 % |
| **+ 2 saisons, seuil 75 (retenu)** | **4,8** | 69 % | 0,10 | 33 % |

`century_probe` 464 tours, normale, graines 1-10 (critère EQ6 : guerre FR-EN 55-75 %) :
| Variante | Guerre FR-EN moy. [min-max] | Graines dans 55-75 | Révoltes / 200 t. |
|---|---|---|---|
| sans RS-B | 68 % [61-73] | 10/10 | 3,0 |
| RS-B, 2 saisons, seuil 74 | 73 % [67-78] | 6/10 | 7,3 |
| **RS-B, 2 saisons, seuil 75 (retenu)** | **69 % [55-75]** | **9/10** | **5,9** |

Retenu, les quatre niveaux (état final après fusion de main) :
| Niveau (graines) | Guerre FR-EN moy. [min-max] | Graines dans 55-75 | Révoltes / 200 t. | Banqueroutes |
|---|---|---|---|---|
| Facile (5) | 69 % [60-77] | 3/5 | 6,1 | 0,10 |
| Normale (10) | 69 % [55-75] | 9/10 | 5,9 | 0,05 |
| Difficile (10) | 66 % [60-70] | 10/10 | 7,8 | 0,04 |
| Très difficile (5) | 59 % [57-63] | 5/5 | 7,4 | 0,02 |

## Conséquences
- Critère EQ6 tenu : guerre FR-EN moyenne entre 55 et 75 % aux quatre niveaux, sans réglage
  supplémentaire. Écart accepté : en facile, 2 graines sur 5 dépassent légèrement 75 % (76, 77 %).
- Révoltes dans la bande 4-10 à tous les niveaux sur le siècle (5,9 en normale, 7,8 en difficile).
- Les révoltes reviennent dans la bande 4-10 (4,8 sur 200 tours en normale, 5,9 sur le siècle), en
  bas de bande : le seuil est très sensible (74 donne 7,8, 72 donne 11) mais 74 pousse la guerre
  FR-EN au-delà de 75 % sur 4 graines sur 10 (plus de révoltes affaiblissent le royaume attaqué).
- Une révolte est plus rapide à déclencher : le joueur a deux saisons pour réagir au lieu de trois
  (codex `cdx_jeu_ordre_public` mis à jour ; l'UI lit les valeurs dans les données).
- Les garnisons des châteaux et villes apaisent moins la province qu'avant : tenir l'ordre passe par
  la cité.
