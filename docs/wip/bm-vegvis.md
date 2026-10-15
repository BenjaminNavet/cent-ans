# bm/vegvis : parité visuelle de la végétation de campagne (Bevy)

État : FAIT (sous-ensemble ci-dessous), dump vérifié à Paris distance 25/60/200.

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

- `countryside/landuse.rs` : raster d'occupation du sol (`VegetationFields.landuse`, 128², RVBA8 linéaire, repère carte, R vigne / G sécheresse / B bocage) construit sur un thread, publié en ressource `LanduseTexture { image }` pour le shader de sol (lot campvis : uniforme `landuse` + `terroir_landuse` de `terrain.gdshader`). Dump `landuse` : 213 provinces, 188 ms.

## Mesures (dump, Paris, distance 60)
34 508 imposteurs, 139 modèles lod1 (15 essences), 35 lots d'ombres (opacité 0,42), assemblage 1,1 ms.

## Reste
- Vraies ombres portées (passe d'ombre pour le pipeline instancié) ; ombres de nuages sur les arbres (`campaign_cloud_shadow`) ; voile du brouillard de guerre sur les arbres ; brume de bord `ce_haze` (prise sur les paramètres du terrain seulement via sa brume de distance).
- Repli `meshes.rs` non branché : aucune essence sans glb (41/41 ont un lod1).
- Le rendu reste délavé tant que la brume/teinte politique du sol (lot campvis) écrase le contraste : les arbres prennent la même brume que le sol.
- Taches sombres de forêt à 200-600 = couvert forestier du shader de sol (`canopy_field`), lot campvis.
