# TW2-T3 — Compagnies de mercenaires

Branche `feat/tw2-t3` (depuis `integration/tw2`). Spec : `docs/design/2026-09-28-tw2-mecaniques-total-war.md` § T3. ADR 0103.

## Fait
- [x] Données : `data/rules/mercenaries.json` + schéma `mercenary_rules.schema.json` + test pytest
  `tools/tests/test_mercenary_rules_schema.py` ; `data-model` `MercenaryRules` (défaut = fichier embarqué).
- [x] Nouvelles fiches `unit_brabancons`, `unit_scots_archers` (marquées `mercenary`) + gore, étendards,
  catalogue d'icônes (dérivées d'illustrations existantes).
- [x] `sim-campaign/src/mercenaries.rs` : réserves par bande × région, marché d'une armée, ordre
  `HireMercenary`, surcoût d'entretien, impayés (désertion / pillage), recharge. Branché dans le tour
  après `resolve_economy`. Le recrutement en ville refuse les unités `mercenary`.
- [x] Tests Rust `sim-campaign/tests/tw2_t3_mercenaries.rs` (7) ; `ur1_units.rs` adapté.
- [x] IA : `ai/src/mercenaries.rs` (riche + menacée), appelée avant `plan_armies` ; test `ai/tests/tw2_t3_mercenaries_ai.rs`.
- [ ] Pont `get_mercenaries(army)` ; UI bouton « Mercenaires » (bandeau d'armée) + panneau.
- [ ] Icônes PNG des deux nouvelles unités (build partiel `entity_icons`).
- [ ] Test headless `game/tests/tw2_t3_mercenaries_test.gd`, smoke.
- [ ] Sonde IA (effectifs, trésors) → ADR 0103.

## Prochaine étape
Pont `get_mercenaries`, puis UI.
