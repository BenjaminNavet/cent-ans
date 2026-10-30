# ECO-R1d — « L'impôt consenti » : interface et textes

Spec : `docs/audits/2026-10-29-economie-politique.md` § 8 R1-d. ADR 0813, `docs/wip/eco-r1ab.md`.

## État
- [x] game-api : `assembly::{assembly_panel, forecast_view}`, `FinanceView::assembly_house` (aucune règle).
- [x] textes `data/ui/assembly_panel.json` + schéma + `test_schemas.py`
- [x] onglet client `faction/assembly.rs` (compile; à tester)
- [x] boutons d impôt de Finances remplacés (`faction:fin:levy_assembly`)
- [ ] codex, conseiller
- [ ] scénario smoke `faction_assembly`

## Prochaine étape
Textes + onglet client.
