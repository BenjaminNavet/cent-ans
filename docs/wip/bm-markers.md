# BM markers : colonies, maquettes et villes emblématiques (client Bevy)

Lot `client/src/campaign/markers/` (branche bm/markers).

## État
- `catalog.rs` : `settlement_markers.json` (rangs, portées d'étiquettes) et `town_maquettes.json` (tailles, familles, gain de poids).
- `placement.rs` : repère monde (1 unité = 1 px carte, x = px.x, z = px.y), `HeightSampler`, hachage stable, collisions.
- `declutter.rs` : placement glouton de rectangles écran.
- `places.rs` : colonies (GameData + `settlements_px`), maquette choisie, villes emblématiques (`data/landmarks`, glb générés ADR 0356).
- `signs.rs` : taille constante à l'écran des ruines et chantiers.
- `mod.rs` : plugin, systèmes (construction, portée, étiquettes, signes, contrôleurs), section de dump `campaign_markers`.

## Prochaine étape / restes
- Pas de caméra 3D de campagne encore (lot camera) : étiquettes et portées vérifiées par tests seulement, `labels_visible`/`maquettes_visible` = 0 au dump.
- Écus et bannières de contrôleur (heraldry atlas), glb générés DN proches (`dn_campaign_models.json` table), fondu de portée, ombres par type.
- Ancrages fins (`fine_anchors.json`, grille réelle) et hauteur du sol (brancher `HeightSampler` au terrain).
- Icône de ruine SVG (`delapouite-castle-ruins.svg`) : quad uni provisoire.
- Couches 1:1 (`town_far_*`, `landmark_city_layer`) : abandonnées par l'ADR 0158, non portées.
