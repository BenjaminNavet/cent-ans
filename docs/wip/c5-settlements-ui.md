# Lot C5 : pont et interface des colonies

Spec : `docs/design/2026-09-24-echelle-colonies.md` § 4.6 et § 6. Suit C4
(`docs/wip/c4-core-settlements.md`) et C6 (`docs/wip/c6-zoom-tiers.md`).
Branche : `worktree-agent-ac38aa74f75611f21` (partie de `main` `cda86a9`).

## État

- [x] Pont : `settlement_buildable(id)` ; `settlement_detail` enrichi (`buildings_info`,
  `recruit_slots`, `recruit_slots_free`, `garrison_strength`, `port`, `province_name`).
  Recrutement d'une colonie : `get_recruitable(settlement_id)` (existait déjà).
- [x] Tests Rust `core/crates/sim-campaign/tests/c5_settlement_panel.rs` (constructions filtrées
  par type de colonie, ordres de recrutement et de construction sur une ville non-cité).
- [x] `PanelWidgets` (`game/scripts/map/panel_widgets.gd`) : lignes partagées (garnison,
  recrutement, bâtiments, constructions) ; `ProvincePanel` s'en sert.
- [x] `SettlementPanel` (construit en code) ; onglet « Colonies » du panneau de province.
- [x] `SettlementController` : ouverture du panneau, ordres par colonie, clic droit sur une
  colonie, anneaux des colonies atteignables (`ReachableMarkers`), aperçu de chemin sur le graphe.
- [x] Test headless `game/tests/c5_settlements_ui_test.gd` (OK) ; smoke et `settlements_render_test` verts.
- [x] Captures `docs/img/colonies/c5-panneau-colonie.png`, `c5-ordres-armee.png`
  (`--stage=settlement`, `--stage=settlement_orders`, fenêtre non headless).
- [x] Documentation : `docs/godot-map.md` § « Interface des colonies (lot C5) ».

## Décisions

- Le panneau de colonie est construit en code (pas de `.tscn`) et prend la place du panneau de
  province (même ancrage) ; les deux ne sont jamais ouverts ensemble.
- Toute la logique C5 vit dans de nouveaux scripts ; `campaign_map.gd` ne reçoit que des
  crochets d'une ou deux lignes (création du contrôleur, `refresh`, sélection d'armée, aperçu).
- Sans API de colonies (mock), le contrôleur est inactif et la carte garde le comportement v1.
- Aperçu de chemin : segments droits entre colonies successives du chemin (arêtes du graphe),
  subdivisés et posés sur le relief ; il ne suit pas le tracé exact des routes.
- Clic droit sur une colonie = cible ; ailleurs, comportement v1 (cité de la province).
- Survol : la colonie sous le curseur est prioritaire sur la province survolée.

## Points ouverts

- L'aperçu de chemin trace des segments droits entre colonies, pas les routes réelles.
- Portée d'une saison : depuis Paris, 233 colonies atteignables (jusqu'à Villeneuve-sur-Lot, 14
  étapes) ; à vérifier à l'équilibrage (C7, `points_per_step`, routes ÷ 2).
- Le panneau (comme celui de province) recouvre en partie la minicarte.
- Pas d'onglet Colonies ni de panneau de colonie avec le mock (pas d'API de colonies).

## Prochaine étape

Terminé : fusion par l'orchestrateur.
