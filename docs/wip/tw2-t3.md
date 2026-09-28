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
Lot terminé une fois la vérification complète verte (fmt, clippy, cargo test, build, test headless,
smoke, pytest) ; reste la fusion dans `integration/tw2` par l'orchestrateur.

## Points ouverts
- La surprime n'apparaît pas dans le budget de l'interface (`economy.rs`, lot RS) : panneau et journal.
- Pas d'illustration propre pour les deux nouvelles unités (icônes recadrées).
- Pistes : compagnies allemandes en Italie, gallowglass, licenciement volontaire d'une compagnie.
