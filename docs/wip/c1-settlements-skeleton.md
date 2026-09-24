# C1 — squelette des colonies

Spec : `docs/design/2026-09-24-echelle-colonies.md` § 8 (lot C1). ADR 0005.

## État : terminé

- [x] `data-model` : `SettlementId` (`set_`), `SettlementKind`, `Settlement`, `SettlementRules`,
  `SettlementGraph`/`SettlementEdge` (`entities/settlement.rs`), chargement + repli + validation
  (`settlement_load.rs`), `GameData.settlements`, `settlements_by_province`, `settlement_rules`,
  `settlement_graph`, `GameData::province_city`, `GameData::province_settlements`.
- [x] `data/settlements/rules.json` + `data/schemas/settlement_rules.schema.json`.
- [x] `sim-campaign` : `SettlementState`, `CampaignState.settlements` (serde default),
  `init_settlements` dans setup 1337, accesseurs `settlement_state`, `province_settlements`,
  test `tests/c1_settlements.rs`. `STATE_VERSION` reste 4.
- [x] Test Python `tools/tests/test_settlements_schema.py` (vérifié avec un fichier Normandie
  temporaire, supprimé).
- [x] Pont Godot : `CampaignSim.settlements()` (`campaign_sim_settlements.rs`).
- [x] fmt/clippy/test, pytest (89 ok), build.sh, smoke Godot.

## Points ouverts

- Smoke Godot : échec **préexistant** (reproduit sur 64dec0b sans ce lot) :
  `projected_income should increase after construction: 22307 -> best 22307`.
- La garnison de la cité est vide dans `SettlementState` (la garnison de province reste la vraie
  jusqu'à C4) ; `starting_garrison.city` de `rules.json` sera appliquée par C4.
- Règle « au moins un port par province côtière » (§ 3.1) non vérifiée par le test Python (non
  demandée pour C1 ; à ajouter quand C2 aura écrit les fichiers).
- Coordonnées dans la province (lecture de `province_ids.png`) : pour C3.

## Prochaine étape

Lot C2 (fichiers de colonies) et C3 (graphe), puis C4.
