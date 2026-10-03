# ADR 0180 — Garnison par habitant, prime d'occupation et plafond d'opinion à la lecture (lot LR-07)

Date : 2026-10-03. Statut : accepté.
Suivi : `docs/wip/lr-07.md`. Prolonge l'ADR 0100 (RS-B, ordre public pondéré) et le lot RS-C
(plafond d'opinion par motif).

## Contexte
- Le chantier LR reprenait un point ouvert d'EQ6 : « révoltes 2,7 par partie, sous la bande
  4-10 ». Mesure du 03/10 sur main (`balance_probe campaign 200`, normale, graines 1-16) :
  **14,8 révoltes par partie** (18,2 / 11,4 selon la moitié), au-dessus de la bande. La carte
  a grandi depuis (OM, FE : 443 provinces, ~177 factions). 57-70 % des révoltes éclatent en
  province occupée, et la plupart dans une province **sans garnison** (l'IA prend une place et
  repart).
- L'apaisement de la garnison valait 1 point par 100 hommes (pondérés par place, ADR 0100),
  plafonné à 10, sans égard au nombre d'habitants : 700 hommes calmaient Paris autant qu'un
  comté de montagne.
- Le bonus « Mariage entre nos maisons » était plafonné à l'ajout depuis RS-C, mais pas à la
  lecture : les sauvegardes d'avant RS-C, les opinions données par les événements et toute
  écriture directe s'empilaient encore.

## Décision
1. **Garnison par habitant** (`PopulationRules::garrison_relief`, `data/rules/population.json`) :
   apaisement = `garrison_relief_per_100_men` × (hommes pondérés / 100) × poids, plafonné à
   `garrison_relief_max` ; poids = `garrison_relief_reference_population` / population de la
   province, plafonné à `garrison_relief_max_weight`. Valeurs : 1,5 par 100 hommes, plafond 15,
   référence 100 000 habitants, poids ≤ 3. Une petite province bien gardée s'apaise plus, une
   grande ville moins ; référence 0 = règle d'avant.
2. **Prime d'occupation 20 → 12** (`occupation_unrest`) : c'est elle, pas la garnison, qui
   fait les révoltes en trop (province prise et laissée sans garnison).
3. **Plafond d'opinion lu partout** : `CampaignState::attitude` additionne les modificateurs
   d'un motif plafonné (`opinion_caps`) et borne leur total au plafond, sur une seule ligne ;
   les opinions des événements (`chronicle::apply_effect`) passent par
   `add_capped_modifier`. Règle générale : vaut pour tout motif de `opinion_caps`.

## Mesures (`balance_probe campaign 200`, normale, graines 1-16, même binaire, règles par
`POPULATION_RULES`)
| Variante | Révoltes / partie | Tenant 1-2 prov. | 3-6 | 7-15 | 16+ | Guerre FR-EN | Impôt Haut |
|---|---|---|---|---|---|---|---|
| main | 14,8 | 6,6 | 2,7 | 3,0 | 2,6 | 68 % | 20 % |
| garnison par habitant seule | 14,8 | 5,6 | 2,4 | 5,2 | 1,7 | 66 % | 23 % |
| + occupation 15 | 11,6 | 3,9 | 2,0 | 4,0 | 1,8 | 63 % | 23 % |
| **+ occupation 12 (retenu)** | **7,2** | **2,2** | 2,0 | 2,5 | 0,6 | **68 %** | 24 % |
| + occupation 10 | 7,4 | 3,7 | 1,3 | 1,7 | 0,7 | 63 % | 23 % |

Graines 1-8 seules, premier tour : garnison par habitant sans bonus (poids ≤ 1, baisse
seulement) 20,4 ; 2 par 100 hommes, poids ≤ 2 : 16,4 ; 2 par 100 hommes sans pondération :
15,9 (main 18,2). Écart entre graines : 3 à 63 révoltes.

## Conséquences
- Révoltes dans la bande 4-10 (7,2), guerre FR-EN inchangée (68 %), les petites factions
  (1-2 provinces) se révoltent trois fois moins (6,6 → 2,2).
- Une conquête laissée sans garnison gronde moins (+12 au lieu de +20) ; la garnison compte
  davantage dans les petites provinces.
- Le total d'opinion d'un motif plafonné ne dépasse jamais le plafond, quelle que soit la source.
- Banqueroutes 1,6 par faction et décennie (cible < 0,5) avant comme après : hors du lot.
