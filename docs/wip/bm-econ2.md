# bm-econ2
Fait : clic Trésor -> économie (déjà câblé), cloche/filet/fit_to déjà portés (uileft). Ajouté : `hud/layers.rs` (couches Routes commerciales [V] et Signes, règles `signs` de campaign_map.json, rubans drapés sur le relief, brouillard, masqués en vue vallée), cases dans le menu Filtres de la minicarte.
Écarts : pas de routes maritimes (ligne droite port à port), pas d'infobulle de route au survol.
Preuve : client/src/campaign/hud/proof_layers.txt (map_layers : 10 routes, 9 tracées, 4 coupées ; panel_economy ouvert par hud:treasury). 4 tests layers.
