# VT-D — retraits des maquettes dans `settlement_layer.gd`

Lot D du chantier VT (ADR 0138, `docs/wip/vt-plan.md`, sections Retraits et Recâblage).

## État
- [x] Retrait des maquettes (colonies, L1), SZ4b, DC4/DC6c, ombres.
- [x] Étiquettes au sol + décalage px au-dessus de l'emprise.
- [x] Clic et anneau sur l'emprise réelle (`radii` de `towns_1340.json`).
- [x] Accesseurs `model_*` recâblés ; exclusion de végétation par le finage.
- [ ] Tests c5_settlements_ui, settlements_render, smoke.

## Prochaine étape
c5 OK ; lancer settlements_render et smoke, lister les tests cassés.
