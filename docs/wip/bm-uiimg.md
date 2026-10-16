# bm-uiimg : illustrations 2D dans l'UI de campagne

Fait (Godot -> Rust) :
- `settlement_buildings_view.gd` -> `client/src/campaign/panels/province/images.rs` + `view.rs` : plaque de palier (`settlement_tiers/<type>_<n>.jpg`, palier inférieur le plus proche, repli sur le bâtiment le plus haut), légende « Palier n/6 — nom », progression, une image `bld_*` par carte de bâtiment (repli : icône peinte). Dump `campaign_panel_province.sim.bound_images`.
- `season_report.gd` : vignette (kind le plus prioritaire, `ArtPlates`) en tête du rapport de saison via `[img]` dimensionné d'après l'en-tête JPEG (`hud/report.rs`). Dump `hud.last_report_vignette`.
- `chronicle_window.gd` : image `events/<event>.jpg` sur chaque décision (onglet Décisions et fenêtre de décision). Dump `panel_chronicle.decision_art`.
- `loading_screen.gd` : plaque de chargement de campagne (contexte `campaign`, `flow/loading.rs`), 3 s ou clic ; `CENT_ANS_LOADING_SECONDS=0` la supprime, aucune sous `--script`.

uiimg2 (fait) :
- `DecisionView.kind` (core sim-campaign `event_kind_key` : scène -> plague/famine/revolt/siege_started/raid/war_declared, sinon effet typé -> plague/war_declared/peace_signed/death/general_captured, sinon vide) + test `decision_view_carries_the_event_genre`.
- `hud/cover.rs` : recadrage cover via `menu::CoverFit`; vignette de repli par genre dans la fenêtre de décision et l'onglet Décisions (dump `panel_chronicle.decision_art[].kind/art`); vignette du tour le plus récent dans le journal du HUD (`ChronicleLog.newest_kinds`, dump `journal.vignette`).
- `flow/loading_veil.rs` (module séparé, `loading.rs` intact) : fond plein écran cover assombri, écu + nom + date, conseil (`front_end.json` loading.tips); dump `loading_veil`.
Restes : vignette du rapport de saison toujours en `[img]` fit (texte riche non recadrable) ; citation tirée du fonds commun et barre de progression/étapes de Godot non portées (la plaque garde sa citation) ; le genre des décisions est un repli heuristique (Godot n'avait pas de `kind` pour les événements).
