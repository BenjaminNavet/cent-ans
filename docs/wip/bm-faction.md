# bm-faction : panneau de faction (client Bevy)

État : fait (lots faction + faction2). `OpenPanel::Faction(id)` ouvre un panneau central empilé (`StackedPanel`, id `faction`) à quatre onglets (le dernier, « Cour », seulement pour la faction du joueur) :
- Fiche : seigneur, titres, vassaux, relations, objectifs, obligations féodales (tribut, ost, protection, félonies, séries d'indépendance), biens par catégorie.
- Budget : impôt, tableau recettes/dépenses, courbe du trésor en segments (nœuds UI tournés, `UiTransform`) sur barres, section Monnaie (niveau des prix, seigneuriage, refonte, sélecteur des quatre niveaux, un changement par an).
- Commerce : routes, accords en vigueur (rupture avec confirmation), nouvelles routes avec projection avant signature, biens produits.
- Cour : ordre de chevalerie (fondé ou options) et, pour la faction croisée, Ferveur (jauge à seuils et plancher, aumônes, passage à prêcher, contingents attendus).

Vue : `core/crates/game-api/src/faction_panel.rs`. Client : `client/src/campaign/panels/faction/` (`mod.rs`, `view.rs`, `sections.rs`, `chart.rs`).
Crochet de dev : `CENT_ANS_OPEN_FACTION=<id>`.

Reste (Godot) : fenêtre des rançons (`ransom_panel.gd`, vue `get_ransoms` pas encore dans `game-api`) ; détail de la Table par province en infobulle ; « Voir sur la carte » (calque des routes) de `economy_trade_tab.gd` ; liens Codex cliquables dans les textes (retirés pour l'instant) ; icônes des biens et des ordres (`IconChip`).
