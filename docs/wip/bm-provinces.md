# bm/provinces — couche provinces du client Bevy

État : FAIT pour la vague 1 (client/src/campaign/provinces/). data (geojson + raster), projection (MapMeta), picker, modes
(politique, terrain, diplomatie, mécontentement, richesse, population), frontières (ruban, signes EN/DZ), remplissage
(texture 1/4), noms de région, LabelPlacer, dump `campaign_provinces`. 26 tests.

Restes (fichiers Godot) : hachures d'occupation et halo respirant du joueur (faction_borders.gd), modes religion /
loyauté / ravitaillement / revendications (map_mode_controller.gd, besoin de vues dans game-api), filtre féodalité
(feudal_map_lens.gd, `get_feudal_map` à exposer), menu de filtres + légende + infobulle (lot hud), province_mates_overlay.gd
(lot colonies), parchemin lointain, option Réglages `map/stance_fill`, drapage des bandeaux sur le relief (SurfaceHeight).
