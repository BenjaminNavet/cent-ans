# WH diploa — diplomatie lisible

Branche wh/diploa. Spec : docs/wip/wh/diplomatie.md § 3 points 1, 2, 3, 4, 10. ADR 0278.

## État : FAIT (à fusionner)
Core, pont, UI, tests Rust (`sim-campaign/tests/diplomacy/wh_diploa.rs`), smoke UI (`game/tests/wh_diploa_test.gd`), ADR.
## Reste / points ouverts
- L'IA ne retire jamais un accès (ordre joueur seulement). Valeurs −40/−20/−30 de `declare_war` toujours en Rust.
