# TW ai-camp — l'IA de campagne utilise les actions m2a/m2b

Branche `tw/ai-camp` (base `tw/int3`), ADR 0334.

## État
- (a) Héritier : `dynasty::ai_designate_heir`, appelé au printemps (`ai/campaign/characters.rs`). Seuil
  `ai_heir_min_skill_gap` dans `data/rules/dynasty.json`. Test `tw_ai_camp` (sim-campaign/tests/diplomacy).
- (b) Conversion : déjà couverte (passive + prédicateur IA) ; rien de nouveau.
- (c) Croisade : branche `war_target` + `ai_max_participants` déjà en place (ADR 0327) ; mesure de participation
  ajoutée à `campaign_probe`.
- (d) Réconciliation : `religion.json` `ai_reconcile`, `diplomacy/ai.rs` ; couvre l'interdit seul.

## Mesures campaign_probe (120 tours, graines 1-2, `campaign_probe` étendu : église + désignations)
| | avant (graine 1 / 2) | après (graine 1 / 2) |
|---|---|---|
| guerres déclarées | 238 / 247 | 225 / 212 |
| guerres actives (moy.) | 33,2 / 40,8 | 34,4 / 38,4 |
| révoltes | 2 / 2 | 2 / 2 |
| faillites | 115 / 124 | 89 / 125 |
| éliminées | 19 / 22 | 19 / 27 |
| croisade : événements / participants IA max | 27 / 1 et 44 / 1 | 25 / 1 et 53 / 1 |
| désignations d'héritier par l'IA | 0 | 6 (2 graines) |
Pas de dérive grossière (les trajectoires divergent : chaos normal après un changement d'héritier). Seuil d'héritier
25 puis 8 : 0 désignation (total de compétences initial 0-23, médiane 14) ; retenu 6. La croisade atteint 1
participant IA sur `ai_max_participants` = 3 : borne non saturée, comportement voulu par l'ADR 0327.
Piège : `DynastyRules::bundled()` ignore `CENT_ANS_DATA_DIR` (le « avant » est donc le seuil 25 inopérant).

## Prochaine étape
Fusion par l'orchestrateur.
