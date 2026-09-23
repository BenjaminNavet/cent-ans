# WIP — sim-campaign (M2)

Suivi de l'implémentation du modèle de campagne (`core/crates/sim-campaign`, `core/crates/ai`).
Mis à jour à chaque commit `wip:` pour qu'un agent qui reprend puisse continuer.

## État
- API publique complète et compilée : `CampaignState::new_1337`, `submit_order`, `end_turn(data)`,
  `reachable`, `find_path`, `recruitable`, `faction_summary`, `province_state`, `army`, `armies`,
  `events`, `date_label`, `turn`, `player_faction`, `save_json`, `load_json`.
- Modules : state, orders, movement (Dijkstra + phase de mouvement + batailles), battle_auto (tests
  unitaires), siege (sièges, prises, chevauchées), economy (taxes, entretien, attrition, décroissance),
  characters (mort naturelle, succession), setup_1337, save, turn, ai_minimal, rng, events.
- `ai::plan_turn` ré-exporte `sim_campaign::ai_minimal::plan_turn`.
- Le bridge (`godot-bridge`) appelle encore `end_turn()` sans argument : il casse jusqu'à ce que
  l'agent bridge passe à `end_turn(&data)`. Les autres anciens items (`new(seed)`, `turn`, `year`,
  `date_label()`) sont conservés.

- Tests d'intégration `core/crates/sim-campaign/tests/campaign.rs` (17 tests) verts sur `data/`.
- Équilibrage observé : France 24 133 livres/saison, entretien total 6 900, armée royale (8 unités)
  3 120 (12,9 %). 40 tours tout-IA : ~60 prises, ~13 batailles, 14 armées, 15 factions vivantes.

## Prochaine étape
- Rien de bloquant côté sim. Pistes : IA qui dépense ses excédents (France/Empire thésaurisent),
  ordre d'assaut, évolution de la population (M3).

## Décisions
- `ai` dépend de `sim-campaign` : le planificateur minimal vit dans `sim_campaign::ai_minimal`
  (évite un cycle de dépendances).
- Revenu : formule du spec × `TAX_EFFICIENCY` (sinon France ≈ 268 000 livres/saison).
- Toutes les factions ont une armée principale (3 unités pour les mineures) pour que l'IA agisse.
- Vassal/suzerain = alliés (territoire ami pour le ravitaillement).
- Pas d'ordre d'assaut : un siège se résout uniquement par sa durée.
