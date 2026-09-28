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
3. **Révolte** (`data/rules/population.json`) : après **2 saisons** (3 avant) au-delà de **74** de
   mécontentement moyen (75 avant). Le seuil de passage aux rebelles (90) ne change pas. L'IA,
   qui lit ce seuil pour sa marge de prudence fiscale, suit.

## Mesures (`balance_probe campaign 200`, normale, graines 1-16)
| Variante | Révoltes / partie | Guerre FR-EN | Banqueroutes / fac. / déc. | Impôt Haut |
|---|---|---|---|---|
| main + constantes | 4,1 | 74 % | 0,08 | 33 % |
| + pondérations | 2,8 | 69 % | 0,10 | 32 % |
| + 2 saisons | 4,8 | 69 % | 0,10 | 33 % |
| + 2 saisons, seuil 72 | 11,1 | 69 % | 0,08 | 30 % |
| **+ 2 saisons, seuil 74 (retenu)** | **7,8** | 69 % | 0,13 | 31 % |

`century_probe` 464 tours par niveau : voir `docs/wip/rs-b-order.md`.

## Conséquences
- Les révoltes reviennent au milieu de la bande 4-10 ; le seuil est très sensible (72 donne 11).
- Une révolte est plus rapide à déclencher : le joueur a deux saisons pour réagir au lieu de trois
  (codex `cdx_jeu_ordre_public` mis à jour ; l'UI lit les valeurs dans les données).
- Les garnisons des châteaux et villes apaisent moins la province qu'avant : tenir l'ordre passe par
  la cité.
