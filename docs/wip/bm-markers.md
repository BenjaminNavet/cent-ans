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

## Restes
- Épinglage capitale/sélection/survol dans le désencombrement (besoin de la sélection de carte), écu d'occupant (bannière RJ-c), liseré de position.
- Maquettes du kit : base de l'AABB à -2 unités sous l'ancre (à vérifier contre `local` Godot).
- Fondu de portée, glb DN proches, icône de ruine SVG, ancrages fins, ombres par type.
- Étiquettes/écus non vérifiés à l'oeil (pas de capture).
