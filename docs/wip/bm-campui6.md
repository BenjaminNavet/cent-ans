# bm-campui6 : Tech, colonies de la province, vignette du rapport, lien Codex

Branche `bm/campui6` (base `bm/merge8`). Lot termine.

- Tech (`campaign/panels/tech`) : champ de recherche `tech:search` (nom + noms des unites/batiments debloques, estompe le reste, comme `tech_tree_view.matches_filter`) ; retrait de la file par `Order::DequeueResearch` : bouton « Retirer de la file » sur chaque carte de file (`tech:dequeue_<id>`) ou second clic sur un noeud deja en file. Test cote coeur `game-api::tech::dequeue_takes_a_technology_out_of_the_queue`. Ecart : le retrait (absent de Godot) est une amelioration.
- Province (`campaign/panels/province`) : `mates.rs` = `province_mates_overlay.gd` (pastilles UI reprojetees, anneau rubrique ville / or selection, icone du type `bld_*`, pointilles d'encre en gizmos sur le relief, cartouche « Province de X — n colonies »). Clic sur une case d'emplacement (vide surtout) : onglet Batiments de la colonie, ligne du premier palier marquee d'un cadre dore (`focus_building`, dump `campaign_panel_province.sim`). Ecart : Godot donne le focus clavier au bouton « Construire » ; ici on marque la ligne.
- Minicarte : brouillard et relief ombre etaient deja portes par hud3 (dump `minimap` : `relief_baked`, `fogged` vrais en campagne) ; seul l'en-tete de doc etait perime.
- Rapport de saison : vignette recadree en cover 536x170 (`ModalSpec::art`, `ModalArt`, `CoverFit`) a la place du `[img]` ; le titre de la planche est en dessous. Edits hors dossier : `ui/kit/modal.rs` (champ `art`, dump `ui_kit.modal.art`), `menu/faction_select.rs` (`art: None`).
- Tutoriel (`campaign/progress/tutorial.rs`) : textes et conseils passes par `codex::text::format_auto` (liens Codex cliquables, bulles) ; dump `progress_tutorial.codex_links`.

Restes : rien de bloquant. Piège : `CENT_ANS_TUTORIAL=<n>` est ignoré si les réglages enregistrés ont le guide reporté ; prévoir `CENT_ANS_SETTINGS_PATH` vers un fichier neuf pour les preuves. Fichier de réglages JSON (`{"season_report":"always","confirm_end_turn":"off"}`) pour forcer la fenêtre du rapport.
