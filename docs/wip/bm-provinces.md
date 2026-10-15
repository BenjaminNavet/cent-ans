# bm/provinces — couche provinces du client Bevy

État : FAIT pour la vague 1 (client/src/campaign/provinces/). data (geojson + raster), projection (MapMeta), picker, modes
(politique, terrain, diplomatie, mécontentement, richesse, population), frontières (ruban, signes EN/DZ), remplissage
(texture 1/4), noms de région, LabelPlacer, dump `campaign_provinces`. 26 tests.

Restes (fichiers Godot) : hachures d'occupation et halo respirant du joueur (faction_borders.gd), modes religion /
loyauté / ravitaillement / revendications (map_mode_controller.gd, besoin de vues dans game-api), filtre féodalité
(feudal_map_lens.gd, `get_feudal_map` à exposer), menu de filtres + légende + infobulle (lot hud), province_mates_overlay.gd
(lot colonies), parchemin lointain, option Réglages `map/stance_fill`, drapage des bandeaux sur le relief (SurfaceHeight).

## Lot drape (bm/drape)
Fait : fond de province (grille 16 px) et frontières draperés sur le relief dynamique par le vertex shader
(`provinces/drape.rs`, `shaders/{fill,border}.wesl`, altitude en mètres dans `position.y`, `ReliefGpu` copié par
`water::drape::sync_drape`). Alpha frontières = mode x zoom_alpha x `ZoomView.border_alpha`. Étiquettes de région en
police `UiTheme` (italique IM Fell), placées sur `GroundHeight`. Pièges trouvés : l'`import` WESL du module relief
reste « en attente » sans erreur (le module est donc inliné, `water::drape::with_relief`) ; culling des rubans
(`cull_mode = None`) ; AABB périmée (`NoFrustumCulling`).
Restes : hachures d'occupation, halo du joueur, ponts placeholders non replacés au zoom (CPU), noms de rivières sans
collision avec les autres étiquettes.
