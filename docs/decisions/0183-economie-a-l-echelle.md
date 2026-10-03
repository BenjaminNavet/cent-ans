# 0183 — Économie à l'échelle (lot A6-L3)

Date : 2026-10-03. Contexte : audit joueur A6 (constats M4, M5) et mesures longues de la sonde A2 (`docs/audit/a6-audit-joueur.md`).

## Contexte

Mesures de départ (sonde `balance_probe campaign 200`, 4 graines, données de `main` au 03/10) :

- France : trésor 60 000 ₶ pour un solde net de +4 346 par saison (14 saisons de solde) ; 106 000 ₶ après douze tours sans rien dépenser ; plus forte rançon demandée 60 000 ₶.
- Angleterre : 45 000 ₶ pour +944 (48 saisons) ; Bourgogne : 15 000 ₶ pour -535 (armée de départ = 68 % des recettes).
- 164 factions sur 177 commençaient avec un solde structurel négatif une fois la part du trésor qui dort retirée ; l'ancienne règle `starting_budget` ne touchait que les royaumes de 5 provinces et plus.
- Révoltes 14,2 par partie (67 % en province occupée), Paix de Dieu choisie à 91 %, milice 64,9 % des recrutements, banqueroutes 1,62 par faction et par décennie.

## Décisions

Toutes les valeurs sont dans `data/` ; le Rust ne porte que le mécanisme.

1. **Trésors de départ.** France 28 000 ₶ (4,3 saisons de solde net), Angleterre 12 000 (5,0), Bourgogne 2 000 (5,8). Pour les autres : `settlements/rules.json` § `starting_budget.treasury_max_income_seasons` = 4 (aucun royaume ne commence avec plus de 4 saisons de revenu brut en trésor).
2. **Administration** (`rules/economy.json`) : `administration_per_province` 0,01 → 0,007, `administration_max` 0,35 → 0,28 (le défaut Rust suit : test `economy_rules_match_their_default`).
3. **Budget de départ** (`starting_budget`, code `setup_1337::fit_starting_garrisons`) : s'applique dès 1 province (`min_provinces` 5 → 1) ; `max_deficit_percent` accepte une valeur négative et vaut -10 (excédent d'au moins 10 % des recettes). Les garnisons hors capitale sont renvoyées d'abord (comportement JR4b) ; si le solde reste négatif, l'armée de départ perd ses unités les plus chères (elle n'est pas levée si même la dernière dépasse les recettes), puis la garnison de la capitale (sauf une unité), puis les bâtiments à l'entretien le plus lourd. Ces replis ne ramènent qu'à zéro. Garnison de départ de la Bourgogne : une unité de moins dans l'armée (4 → 3, soit -25 %), le reste vient de la règle.
4. **Rançons** (`economy.json` § `ransom`) : `income_cap_percent` 100 (une rançon ne dépasse jamais une saison de revenu du payeur) ; `sovereign_capture_only_if_routed` vrai (un souverain à la tête d'une armée battue n'est pris que si cette armée est en déroute ; `ransom::capture_allowed`, appelé par `movement::apply_battle_outcome`).
5. **Révoltes** : `edict_peace_of_god` unrest -8 → -5, recruit_slots -1 → -2, impôt -5 % ; `population.json` `occupation_unrest` 20 → 24 (+20 %) et `revolt_seasons` 2 → 5 (mesuré : 3 saisons 12,5 ; 4 saisons 14,8 ; 5 saisons 6,5 ; 6 saisons 8,7 révoltes par partie, 6 graines).
6. **Milice** (`ai/doctrines.json`) : poids de `unit_urban_militia` réduits (25 → 3 dans la doctrine par défaut). Effet faible : la part de milice suit surtout l'argent disponible (35 % tant que les banqueroutes étaient fréquentes, 41-47 % depuis). Cible < 40 % non atteinte, point ouvert (plafond de part de milice dans le code de l'IA, ou coût de levée).
7. **Banqueroutes** : la cause n'était ni le trésor de départ ni l'entretien mais `event_treasury_min_scale` (0,25 → 0,03) : un événement de chronique écrit pour un royaume de 4 000 ₶ de revenu coûtait à un comté de 40 ₶ de revenu le quart de son montant (120 ₶), soit trois saisons de recettes. À 0,03 : banqueroutes 2,1 → 0,3-0,4.
8. **Recherche** : coût des technologies de rang 3 et plus × 1,5 (24 fichiers de `data/technologies/`).
9. **Coût des bâtiments** : inchangé. Coût moyen 1 366 ₶ ; solde net d'une province de France après la règle : 250 ₶ par saison, soit 5,5 saisons (cible 3-8). Le problème de l'audit était le trésor, pas le prix.

## Mesures (sonde, 6 graines x 200 tours, avant -> après)

| Indicateur | Avant | Après | Cible |
|---|---|---|---|
| Révoltes / partie | 14,2 | 6,0 | 4-10 |
| Banqueroutes / faction / décennie | 1,62 (2,1 sur 4 graines) | 0,28 | < 0,5 |
| Milice / recrutements IA | 64,9 % | 46,6 % | < 40 % (non atteinte) |
| Paix de Dieu (provinces) | 91 % | 38 % | — |
| Édouard III capturé, 12 premiers tours (60 graines) | 1/60 | 1/60 | < 10 % |
| Souverains capturés / partie | 3,2 | 3,2 | — |
| Plus forte rançon | 60 000 | 34 661 | <= 1 saison de revenu |
| Solde net tour 0 France / Angleterre / Bourgogne | +4 346 / +944 / -535 | +6 505 / +2 378 / +342 | >= 10 % du revenu |
| Trésor de départ France / Angleterre / Bourgogne | 60 000 / 45 000 / 15 000 | 28 000 / 12 000 / 2 000 | 4-6 saisons de solde |
| Techs France au tour 200 | 40-43 | 35-38 | plus lente |

Voir le tableau du rapport de lot et `docs/wip/a6-l3-economie.md`. La sonde a gagné : `DATA_ROOT` (mesurer d'anciennes données), captures de souverains, plus forte rançon, économie au tour 0 de toutes les factions, banqueroutes par décile, flux hors revenu/entretien.

## Conséquences

- La France garde un solde structurel de +6 505 (23 % du revenu) : l'excédent vient surtout de la baisse de l'administration. Le trésor, lui, n'est plus que de 4 saisons de solde.
- Les petites factions ne commencent plus en déficit (3 sur 177 contre 164), au prix d'armées de départ et de bâtiments retirés pour les plus pauvres.
- `jr_crusade_ai` : les croisés (aucune province) restent hors de la règle.
- Hors lot (voir l'addendum A6-L3b plus bas pour la milice) : les taxes d'événement planchers, les tributs et articles d'or des traités (voir points ouverts du rapport).

## Addendum A6-L3b — plafond de milice et armées de départ en données

**Milice (décision 6 close).** Cause réelle : la composition lue par l'IA ne comptait que les armées de campagne ; une faction sans armée recrutait donc de la milice dans ses garnisons à chaque tour (les poids de doctrine restaient sans effet, la milice étant souvent la seule unité abordable). Mécanisme : `share_caps` dans `data/ai/doctrines.json` (schéma `ai_doctrine.schema.json`), appliqué par `ai::doctrine::pick_recruit` : un type dont la part dépasserait `max_share` (recrue comprise) n'est pas recruté, dès `min_field_units` régiments ; la composition comptée inclut désormais les garnisons (`ai/src/campaign.rs`). Valeurs : `unit_urban_militia` max_share 0,7, min_field_units 4. Un plafond sans compter les garnisons n'avait aucun effet (45-48 %) ; à 0,3 sans seuil la milice tombait à 5 % (factions sans armée privées de recrues) ; 0,4 à 0,7 donne 22-28 %. Mesures (6 graines x 200 tours) : milice 46,6 -> 27,6 % ; banqueroutes 0,28 -> 0,30 par faction et par décennie ; révoltes 6,0 -> 6,3 par partie. Effet de bord : le mélange de doctrine tient aussi compte des garnisons (elles sont pleines de milice, donc les autres types sont recrutés plus tôt).

**Armées de départ.** `data/rules/starting_armies.json` (schéma `starting_armies.schema.json`, test `tools/tests/test_starting_armies_schema.py`) remplace le `match` sur les identifiants de faction de `setup_1337.rs` : `default_army`, `factions` (France, Angleterre, Bourgogne, valeurs identiques) et `garrisons` par rôle de province. Repli par défaut en données ; fichier absent = pas d'armée ni de garnison de capitale (`GameData::starting_armies`).
