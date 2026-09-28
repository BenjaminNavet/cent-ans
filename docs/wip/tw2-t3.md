# TW2-T3 — Compagnies de mercenaires

Branche `feat/tw2-t3` (depuis `integration/tw2`, SB fusionné). Spec :
`docs/design/2026-09-28-tw2-mecaniques-total-war.md` § T3. ADR 0103.

## Fait
- [x] Données : `data/rules/mercenaries.json` + schéma `mercenary_rules.schema.json` + test pytest
  `tools/tests/test_mercenary_rules_schema.py` ; `data-model` `MercenaryRules` (défaut = fichier embarqué).
- [x] Nouvelles fiches `unit_brabancons`, `unit_scots_archers` (marquées `mercenary`) + gore, étendards,
  catalogue d'icônes et PNG dérivés d'illustrations existantes.
- [x] `sim-campaign/src/mercenaries.rs` : réserves par bande × région, marché d'une armée, ordre
  `HireMercenary`, surprime d'entretien, impayés (désertion / pillage), recharge. Branché dans le tour
  après `resolve_economy`. Le recrutement en ville refuse les unités `mercenary`.
- [x] Tests Rust `sim-campaign/tests/tw2_t3_mercenaries.rs` (7) ; `ur1_units.rs` et `campaign.rs` adaptés.
- [x] IA : `ai/src/mercenaries.rs` (riche + menacée), appelée avant `plan_armies` ; test
  `ai/tests/tw2_t3_mercenaries_ai.rs` ; sonde `ai/examples/t3_mercenary_probe.rs` → tableau ADR 0103.
- [x] Pont `get_mercenaries(army)`, `hire_mercenary(army, unit)` ; bouton « Mercenaires » du bandeau
  d'armée + `MercenaryPanel` (lignes de `PanelWidgets.fill_recruitable`).
- [x] Test headless `game/tests/tw2_t3_mercenaries_test.gd` (OK), smoke.

## Prochaine étape
Lot terminé : `integration/tw2` (SB + doc/uids) fusionné sans conflit, vérification complète verte
(fmt, clippy, cargo test workspace, build, import, smoke, les 3 tests headless TW2, pytest). Reste la
fusion dans `integration/tw2` par l'orchestrateur.

## Vérification finale (2026-09-28)
- `git merge integration/tw2` : pas de conflit (3 `.uid` non suivis identiques au contenu fusionné,
  supprimés avant merge) ; commit `96ca4112`.
- `cargo fmt --all --check` : OK.
- `cargo clippy --workspace --all-targets -- -D warnings` : OK.
- `cargo test --workspace` : 154 suites, 0 échec.
- `core/build.sh` : OK, dylib copiée.
- `godot --headless --path game --import` : OK.
- `godot --headless --path game --script res://tests/smoke.gd` : `smoke OK`.
- `tw2_t3_mercenaries_test.gd`, `tw2_t1_capture_test.gd`, `tw2_t2_replenish_test.gd` : `OK` (exit 0
  chacun).
- `uv run --project tools pytest -q` : 885 passed, 2 skipped.
- Aucune correction de code nécessaire.

## Points ouverts
- La surprime n'apparaît pas dans le budget de l'interface (`economy.rs`, lot RS) : panneau et journal.
- Pas d'illustration propre pour les deux nouvelles unités (icônes recadrées).
- Pistes : compagnies allemandes en Italie, gallowglass, licenciement volontaire d'une compagnie.
