# VT-D — retraits des maquettes dans `settlement_layer.gd`

Lot D du chantier VT (ADR 0138, `docs/wip/vt-plan.md`, sections Retraits et Recâblage).

## État
- [x] Retrait des maquettes (colonies, L1), SZ4b, DC4/DC6c, ombres.
- [x] Étiquettes au sol + décalage px au-dessus de l'emprise.
- [x] Clic et anneau sur l'emprise réelle (`radii` de `towns_1340.json`).
- [x] Accesseurs `model_*` recâblés ; exclusion de végétation par le finage.
- [x] Tests c5_settlements_ui, settlements_render, smoke.

## Tests
- OK : c5_settlements_ui, settlements_render (adapté : plus de maquette), smoke, cv1_campaign_life,
  fk_folk, fk5_incidents, sz4_prop_scale, pb3g_quadtree, rs_k_buffer_layout, m4_free_movement_ui,
  dz_diplo_borders.
- Cassés par le retrait (lot H) : dc6c_fit_scale (SettlementFit supprimé), fc1_shadows
  (`_shadow_geometries`, `model_shadow_limit`), sz4b_colonies_forests (échelle SZ4b),
  dc4_density_probe (sonde, `_models`, `_fit_scale`).
- Hors lot : da7d_overlap « declutter too slow » 5-6 ms sur machine chargée, identique sans le
  recalage d'étiquettes VT (à remesurer machine calme) ; at1_attack_order (siège au lieu d'assaut,
  règle de simulation, sans lien apparent).

## Prochaine étape
Lot fini ; recâblage des consommateurs au lot G, tests au lot H.
