# bm-uiimg : illustrations 2D dans l'UI de campagne

Fait (Godot -> Rust) :
- `settlement_buildings_view.gd` -> `client/src/campaign/panels/province/images.rs` + `view.rs` : plaque de palier (`settlement_tiers/<type>_<n>.jpg`, palier inférieur le plus proche, repli sur le bâtiment le plus haut), légende « Palier n/6 — nom », progression, une image `bld_*` par carte de bâtiment (repli : icône peinte). Dump `campaign_panel_province.sim.bound_images`.
- `season_report.gd` : vignette (kind le plus prioritaire, `ArtPlates`) en tête du rapport de saison via `[img]` dimensionné d'après l'en-tête JPEG (`hud/report.rs`). Dump `hud.last_report_vignette`.
- `chronicle_window.gd` : image `events/<event>.jpg` sur chaque décision (onglet Décisions et fenêtre de décision). Dump `panel_chronicle.decision_art`.
- `loading_screen.gd` : plaque de chargement de campagne (contexte `campaign`, `flow/loading.rs`), 3 s ou clic ; `CENT_ANS_LOADING_SECONDS=0` la supprime, aucune sous `--script`.

Restes / écarts :
- Le repli Godot `vignette_for_kind(decision.kind)` n'a pas d'équivalent (`DecisionView` n'a pas de `kind`) ; pas de vignette dans le journal (lignes sans kind).
- Le fond assombri plein écran de la plaque de chargement (Godot) n'est pas porté ; ni FrontEndData (citation/conseil/écu de faction) de `loading_screen.gd` : seule la plaque `ArtPlates` + citation.
- Vignette en lettres encadrées (fit, pas de recadrage « cover »).
