# BM markers : colonies, maquettes et villes emblématiques (client Bevy)

Lot `client/src/campaign/markers/` (branche bm/markers).

## État
- `catalog.rs` : `settlement_markers.json` (rangs, portées d'étiquettes) et `town_maquettes.json` (tailles, familles, gain de poids).
- `placement.rs` : repère monde (1 unité = 1 px carte, x = px.x, z = px.y), `HeightSampler`, hachage stable, collisions.
- `declutter.rs` : placement glouton de rectangles écran.
- `places.rs` : colonies (GameData + `settlements_px`), maquette choisie, villes emblématiques (`data/landmarks`, glb générés ADR 0356).
- `signs.rs` : taille constante à l'écran des ruines et chantiers.
- `mod.rs` : plugin, systèmes (construction, portée, étiquettes, signes, contrôleurs), section de dump `campaign_markers`.

## Passe 2
- Hauteur : `GroundHeight` (plus de `HeightSampler`), `GroundAnchor` + `settle_ground` (re-pose quand `ReliefState` change). Dump `ground` : 400 ancres, y 0 à 40,5.
- Étiquettes : police `UiTheme.text_font` (accents), ombre, bloc nom + écu (heraldry/<faction>.png, taille/distance de `settlement_markers.json` `shield`), rectangle mesuré sur le nœud, marge de désencombrement.
- Maquettes « invisibles » : enfouies sous le relief (y = 0). Chargement glTF OK (392/392, 8/8 villes emblématiques, tailles 6-30 unités), dump `probes`.
- Hors dossier (autres lots) : étiquettes de régions (`provinces/labels.rs`) et de rivières (`water`).

## Passe 3 (lot towns2)
- Épinglage : `Pins` (ville de la province sélectionnée, capitale du joueur ; `declutter.pin_*`) : nom toujours affiché, posé en premier dans le désencombrement, écu à toute distance.
- Écu de l'occupant : `Places::owners` (propriétaire de droit) ; écu du propriétaire au-dessus du nom, petit écu de l'occupant en bas à droite (`banner.occupant_scale/overlap`), propriétaire sans armoiries = occupant seul.
- Fondu avec la ville 1:1 : `sink_maquettes` lit `TownsState` (échelle y, masquage).
- Base des maquettes du kit : mesurée à -2,42/-2,47 unités sous l'ancre (environ 22 % de la hauteur), identique au `local` Godot (les modèles du kit se prolongent sous leur sol) ; la pose suit désormais `_pose_m` (plus bas du centre et de la moyenne de l'emprise, `GroundAnchor::footprint`).

## Restes
- Liseré de position (couleur de stance) autour des écus, survol dans le désencombrement.
- Fondu de portée, glb DN proches, icône de ruine SVG, ancrages fins, ombres par type.
- Étiquettes/écus non vérifiés à l'oeil (pas de capture).
