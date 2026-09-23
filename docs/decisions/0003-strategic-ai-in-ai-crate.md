# ADR 0003 — IA stratégique dans la crate `ai`, IA minimale dans la simulation

Date : 2026-09-23. Statut : accepté.

## Contexte

`sim-campaign::end_turn` doit faire jouer les factions IA, mais la crate `ai` dépend de `sim-campaign`
(elle en lit l'état) : l'IA ne peut pas être appelée depuis `sim-campaign` sans dépendance circulaire.

## Décision

- `sim-campaign::ai_minimal` (M2) reste le planificateur par défaut de `CampaignState::end_turn`, utilisé
  par les tests de la simulation : simple, stable, il garde les tests indépendants des réglages de l'IA.
- L'IA stratégique (M9) est `ai::plan_turn`, fonction pure `(state, data, faction) -> Vec<Order>`, branchée
  par `CampaignState::end_turn_with(data, ai::plan_turn)` dans le pont GDExtension : c'est elle que
  rencontre le joueur. Diplomatie (`diplomacy::plan_diplomacy`) et recherche (`research::ai_choose_research`)
  restent dans `sim-campaign` et sont réutilisées par les deux planificateurs.
- L'IA de bataille vit dans `sim-battle` (commandes émises par la même API que le joueur).

## Conséquences

Les tests de `sim-campaign` ne couvrent pas l'IA stratégique ; `ai/tests/campaign_ai.rs` et les sondes
`ai_probe`/`war_probe` le font. Toute nouvelle mécanique doit rester jouable par les deux planificateurs.
