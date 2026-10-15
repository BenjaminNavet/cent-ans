# bm/vegvis : parité visuelle de la végétation de campagne (Bevy)

État : en cours.

## Constats (Godot actif = style généralisé)
- `forest_detail.gd` est éteint en style généralisé (`use_forest_detail and not generalised`) : rien à porter pour la parité.
- `field_layer.gd` : `data/art/dn_fields.json` a `enabled: false` ; les champs vus dans Godot sont peints dans le shader de sol (`terrain.gdshader`, `hb_ground`, `fine_parcels`, `landuse`) : lot campvis.
- Les grandes taches sombres de forêt à 200-600 sont le couvert forestier du shader de sol (`terrain.gdshader` `canopy_field`, `canopy_*`) : lot campvis, pas les arbres.
- Toutes les 41 essences ont un glb DN (lod1) : le repli `meshes.rs` n'est jamais exercé.

## Fait
- Imposteurs : niveau de mip borné (`max_lod` 2,5) et compensation d'alpha (`1 + lod × 0,35`) comme `campaign_tree_impostor.gdshaderinc` : les arbres lointains n'étaient plus que des points gris épars (alpha moyen sous le seuil).
- Teinte par instance (`species_tint` : luminosité + teinte du massif) transmise au lieu de blanc.
- Modèles proches : `lod1` (comme `DnTreeModels.MODEL_LOD`, lod2 trop clairsemé), dessinés par le pipeline de végétation (mode 3 de `billboard.wesl`) : saturation `model_chroma`, gains `tree_model_gains.json` × `MODEL_BOOST` 1,55, teinte, saison, vent ; plus le pipeline instancié des batailles.

- Ombres au sol (`shadows.rs`, mode 4) : ellipse douce projetée le long du soleil depuis le houppier, lot jumeau de chaque lot d'arbres, fondu 60 → 90 de distance (Godot : cartes d'ombre jusqu'à `generalised_shadow_distance` 70). Écart : pas de vraie carte d'ombre (le pipeline instancié n'a pas de passe d'ombre).
- Éclairage (`light.rs`) : les arbres reprennent soleil, ambiance et brume du matériau de terrain (`TerrainParams`) au lieu du PBR de Bevy (exposition différente du sol) ; Lambert enveloppé, contre-jour (`BACKLIGHT`), AO (`AO_LIGHT_AFFECT` 0,4).
- Bande tramée modèles ↔ imposteurs (`model_fade` 0,35, poids de zone dans l'enregistrement).

## Prochaine étape
- Raster `landuse` exposé pour campvis.
