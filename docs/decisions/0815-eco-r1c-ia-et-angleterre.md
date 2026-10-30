# 0815 — ECO-R1c : l'impôt consenti, IA et Angleterre

Statut : accepté (30/10/2026, lot ECO-R1c de la boucle juge 2). Complète l'ADR 0813.

## Contexte
R1-a/b (ADR 0813) ont livré les assemblées et une IA minimale : convoquer quand le trésor passe sous N saisons de revenu,
le plus petit palier, la première forme. L'audit demandait en plus la maltôte de la laine et la capitation du Parlement,
des promesses tenues par l'IA, et une convocation sur prévision de guerre.

## Décision
1. **Formes à rendement ou à assiette propres** (`data/rules/assemblies.json`, schéma mis à jour) : `yield_factor`
   (multiplicateur du palier propre à la forme) et `export_good` (le subside ne pèse que sur les provinces couvertes qui
   produisent le bien, et son montant suit l'ouverture des routes d'exportation).
   - `wool_custom` (maltôte, Parlement, `export_good: res_wool`) ; `poll_tax` (capitation, dès 1377, rendement x2, poids
     rural 3 : l'agitation locale existante la rend risquée).
2. **Ouverture des routes** : `trade::export_openness(state, data, faction, bien)` = part (pondérée par `base_value`
   et sécurité) des routes de `trade.json` portant le bien entre un comptoir de la maison et un comptoir étranger qui ne
   sont pas coupées (guerre, embargo, siège, escadre). Le vote la lit (`per_season` nul : refus) et la perception la relit
   chaque saison : un embargo flamand assèche la maltôte en cours. Au départ de 1337 l'embargo flamand de 1336 est en
   vigueur : la maltôte est fermée jusqu'à sa levée.
3. **Honneur** : `ai_honourable` (`ai_personality.diplomacy` >= `ai.honour_min_diplomacy`) ; l'IA qui doit une monnaie forte ne
   l'affaiblit pas (`ai_may_debase`, branché dans `coinage::ai_choose_coinage`) sauf dette de plus de `honour_debt_seasons`
   de revenu ; elle ne promet une trêve que si elle n'est pas l'agresseur (`ai_can_keep`), sauf trésor sous
   `need_seasons` de revenu (nécessité chiffrée : elle accepte et trahira) ; une trêve promise la fait traiter chaque
   saison (`peace_promised`, `diplomacy/ai.rs`).
4. **Convocation sur prévision** (`war_shortfall`) : coût de la guerre = entretien des armées et sièges x `war_cost_factor`
   sur `forecast_seasons`, contre trésor et excédent ordinaire (part de cour de l'administration, sans la ponction
   d'épargne) ; l'IA convoque aussi dès que le trésor passe sous `campaign_below_seasons` en guerre. Le palier est le plus
   petit qui couvre le manque (au moins `target_seasons`). Elle choisit le couple (assemblée, forme) qui rapporte le
   plus parmi les votes non refusés, refuse de redemander une forme encore en cours, et ne demande la capitation que
   pour un grand manque et des provinces calmes.

## Conséquences
- Écart à l'audit : le revenu flamand n'est pas lié à la maltôte par une règle propre (les routes coupées lui coûtent déjà
  la moitié de leur valeur par la douane de `trade.rs`).
- Les mesures et l'équilibre sont dans `docs/wip/eco-r1c.md`.
