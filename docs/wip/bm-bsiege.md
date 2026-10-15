# bm/bsiege — batailles de siège et historiques scénarisées (Bevy)

État : terminé (voir `client/README.md` § Batailles de siège). Rendu de siège + lancement historique scripté, prouvés par dumps (`out/bsiege_*.json`) et tests.
Restes (Godot) : servants (`siege_crew_fx.gd`), animation des engins statiques (`siege_engines_fx.gd` : bascule, recul, roues), projectiles en vol (`siege_assault_fx.gd`), marques de brûlure (`siege_marks_fx.gd` en partie), kit de bâtiments (`BuildingKit`), lumières de feu, prise de point cinématique (`battle_capture_stage.gd`), échelles animées de grimpeurs (soldats sur l'échelle : lot units), historique de site de campagne (`historical_site`, `HistoricalMap::apply_site`) toujours non transmis par `flow`.
Écarts : un modèle par unité d'engin (Godot : une figurine par engin) ; effondrement = affaissement + inclinaison chronométrés ; barres en billboard de quads.
