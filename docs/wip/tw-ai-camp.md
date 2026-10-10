# TW ai-camp — l'IA de campagne utilise les actions m2a/m2b

Branche `tw/ai-camp` (base `tw/int3`), ADR 0334.

## État
- (a) Héritier : `dynasty::ai_designate_heir`, appelé au printemps (`ai/campaign/characters.rs`). Seuil
  `ai_heir_min_skill_gap` dans `data/rules/dynasty.json`. Test `tw_ai_camp` (sim-campaign/tests/diplomacy).
- (b) Conversion : déjà couverte (passive + prédicateur IA) ; rien de nouveau.
- (c) Croisade : branche `war_target` + `ai_max_participants` déjà en place (ADR 0327) ; mesure de participation
  ajoutée à `campaign_probe`.
- (d) Réconciliation : `religion.json` `ai_reconcile`, `diplomacy/ai.rs` ; couvre l'interdit seul.

## Mesures campaign_probe (120 tours, graines 1-2)
Voir le rapport de lot (ci-dessous, rempli à la fin).

## Prochaine étape
Fusion par l'orchestrateur.
