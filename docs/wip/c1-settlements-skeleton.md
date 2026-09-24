# C1 — squelette des colonies

Spec : `docs/design/2026-09-24-echelle-colonies.md` § 8 (lot C1). ADR 0005.

## État

- [x] `data-model` : `SettlementId` (`set_`), `SettlementKind`, `Settlement`, `SettlementRules`,
  `SettlementGraph`/`SettlementEdge` (`entities/settlement.rs`), chargement + repli + validation
  (`settlement_load.rs`), `GameData.settlements`, `settlements_by_province`, `settlement_rules`,
  `settlement_graph`.
- [x] `data/settlements/rules.json` + `data/schemas/settlement_rules.schema.json`.
- [x] `sim-campaign` : `SettlementState`, `CampaignState.settlements` (serde default), `init_settlements` dans setup 1337, accesseurs `settlement_state`, `province_settlements`, test `tests/c1_settlements.rs`.
- [x] Test Python `tools/tests/test_settlements_schema.py` (vérifié avec un fichier Normandie temporaire, supprimé).
- [ ] Pont Godot : `settlements()`.
- [ ] fmt/clippy/test, pytest, build.sh, smoke Godot.

## Prochaine étape

Pont Godot `settlements()` dans `godot-bridge`.
