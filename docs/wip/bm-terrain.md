# BM terrain + camera (lot Bevy migration, branche bm/terrain)

Etat : camera (rig complet, tests), pyramide, quadtree CDLOD via `relief-lod`, pages streamees (LRU 256),
maillages CPU, relief exagere (floor/state/profils lus dans game/resources/*.tres), colormap BC1 et
shader WESL portes. Verifie par `--dump-state` (`campaign_camera`, `campaign_terrain`) ; rendu non regarde
(pas de capture dans ce lot).

Restes (voir README du client) : geomorphing entre niveaux (jupes seulement), ombres, splat/biomes,
saisons/meteo/brouillard de guerre, frontieres, rivieres, vegetation, zones 1:1 (`_add_true_scale_zones`),
close_zones/floor_zones des villes (champs prets, a alimenter), suivi de cible (`follow`) a brancher aux armees.
