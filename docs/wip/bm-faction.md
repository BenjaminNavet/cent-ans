# bm-faction : panneau de faction (client Bevy)

État : fait. `OpenPanel::Faction(id)` ouvre un panneau central empilé (`StackedPanel`, id `faction`) à trois onglets : Fiche, Budget (impôt, tableau recettes/dépenses, graphique du trésor en nœuds UI), Commerce (routes, accords).
Vue : `core/crates/game-api/src/faction_panel.rs`. Client : `client/src/campaign/panels/faction/` (`mod.rs`, `view.rs`, `chart.rs`).
Crochet de dev : `CENT_ANS_OPEN_FACTION=<id>`.

Reste (Godot) : sections Monnaie, Ordre de chevalerie, Ferveur, Obligations féodales, fenêtre des rançons, liste des biens par catégorie (`faction_panel.gd`) ; détail de la Table par province en infobulle.
