# DN V1 — audit des éléments visuels de la carte de campagne

Lecture seule (08/10), sans Godot ni capture. Chemins relatifs à `game/` sauf mention. « GLB ? » =
candidat au remplacement par un glb généré (image → 3D, `docs/pipeline-assets-3d.md`).
Brief = une ligne anglaise, 1337-1453, semi-réaliste, objet isolé sur fond gris (cf. `style_prefix`/`style_suffix` de `data/art/ga3_decor.json`).

## 1. Inventaire

| Élément | Rendu actuel | Fichier / point d'entrée | GLB ? |
|---|---|---|---|
| Figurines d'armée (lord à cheval + escorte, 2-6 fantassins/archers/cavaliers) | Figurines skinnées V2 (`battle_skinned`, mesh.bin + atlas d'os), un MultiMesh par figurine, shader `battle_soldier_skinned`, livrée uniforme + armoiries ; variantes GA3 (Nano Banana → glb, `battle_ga3/`) déjà fusionnées pour archer/cavalier/infanterie | `scripts/map/army_figures.gd` (`_make_material`, `FIGURE_SCALE` 2.3), `scripts/battle/battle_skinned.gd` (`_merge_ga3`) | Déjà GA3 ; manque variantes (voir §3) |
| Bannière d'armée (hampe + étendard armoiries) | Quad/ShaderMaterial `map_banner.gdshader` + PNG `assets/heraldry/banners/<id>_banner.png`, hampe procédurale (8,4) | `scripts/map/army_marker.gd` (`POLE_HEIGHT`), `army_markers.gd` | Non (2D + shader) ; hampe/pennon possible |
| Anneau/plaque d'effectif, badge de posture | Decal + Control 2D + Sprite3D billboard | `army_marker.gd`, `stance_badge.gd`, `reachable_markers.gd` | Non (UI) |
| Villes/châteaux/abbayes/villages (style `maquette`, défaut) | Kit de maquettes stylisées `settlements/<type>[_<famille>]_<a|b>.glb` (Blender procédural, 6 familles : west, med, byz, rus, steppe, isl), MultiMesh par (modèle, tuile), `maquette_kit.gdshader`, teintes cuites en couleur de sommet, bannière de faction | `scripts/map/town_maquette_layer.gd`, `town_maquette_data.gd`, `data/art/town_maquettes.json` (tailles : city 14, town 8, castle 4,6, abbey 4,4, village 4 ; `native_width`) | OUI, fort impact |
| Villes 1:1 / emblématiques (Paris, London, Rouen, Bordeaux, Bruges, Avignon, Calais) | Maquette Blender `landmarks/<id>[_siege].glb` drapée sur le relief (`landmark.gdshader`) ; villes v2 `data/landmarks_v2/` ; plans procéduraux `TownBuilder`/`TownFarBuilder` | `landmark_model.gd`, `landmark_city_layer.gd`, `town_builder.gd`, `town_far_builder.gd`, `town_layer.gd` | Partiel : monuments (cathédrale, tour) en glb ; ville entière non |
| Villages/hameaux, fermes, moulins, vignobles, mines, salines, abbayes, marchés, ports hors murs | Maquettes `outbuildings/*.glb` + kit `buildings/*.glb` (cottage, timber, townhouse, church, barn, watermill, windmill, well, stall…), atlas `TownBuilder.material`, MultiMesh | `scripts/map/outbuilding_layer.gd`, `data/map/building_models.json`, `assets/models/outbuildings/manifest.json` | OUI (ferme, moulin, port, mine, saline, vigne) |
| Hameaux (rendu de repli) | `ModelLibrary.hamlet_meshes()` (`village.glb`) | `model_library.gd` | Faible |
| Châteaux (hors maquette) | `castle.glb`, `cathedral.glb`, `town.glb`, `village.glb` M10 (placeholders Blender) | `model_library.gd` (`CITY_SCALE` 4.8) | Remplacés par maquettes ; secondaires |
| Navires / flottes | `fleet/cog.glb`, `nef.glb` (Blender `campaign_fleet.py`), voile teintée `campaign_sail.gdshader`, tangage, ≤ 3 par flotte | `army_figures.gd` (`SHIP_SCALE` 5,5, `_dress_sail`) ; bateaux ambiants `life_ambient.gd` | OUI |
| Camp de siège | `siege_camp.glb` (tentes, procédural) + fumée GPUParticles | `army_figures.gd` (`_build_camp`, `SIEGE_CAMP_SLOT`) | OUI |
| Bivouac | `fleet/bivouac.glb` procédural | idem (`BIVOUAC_DISTANCE` 190) | OUI |
| Engins de siège (campagne) | Aucun engin séparé : seulement le camp ; engins `siege/*.glb` (GA3 bélier/trébuchet + procéduraux) en bataille uniquement (`battle_siege.gd`, `SiegeEnginesFx`) | `siege_controller.gd` (UI) | Non sur carte (voir §3 : tour/échelles du camp) |
| Chantier | `outbuildings/worksite_1.glb` en signe de taille écran constante | `construction_markers.gd` | OUI (échafaudage) |
| Ruines | Icône sprite `delapouite-castle-ruins` | `ruin_markers.gd`; modèles `*_ruin_*.glb` dans `buildings/` | Moyen |
| Ponts, gués, bacs | Mesh 100 % procédural (arches avec avant-becs, bois sur pilotis, bateaux, gué, pont-porte) ; aussi `buildings/bridge_*` | `scripts/map/bridge_meshes.gd`, `river_crossings.gd` | OUI (pierre, bois) |
| Moulins (à vent / à eau) | `buildings/windmill_0`, `watermill_*` ; ailes animées `life_windmill.gdshader` | `life_effects.gd` (`WINDMILL_SHADER`), `outbuilding_layer.gd` | OUI (déjà brief GA3 `windmill` dans ga3_decor.json, bataille) |
| Arbres / forêts | Imposteurs billboard 3 essences (chêne, hêtre, sapin) × 8 azimuts, atlas `textures/vegetation/ga3/ga3_impostors_*.png` ; cartes de feuillage proches ; maillages procéduraux de repli ; `campaign_trees.glb` | `vegetation.gd`, `vegetation_tile_job.gd`, `ga3_vegetation.gd`, `tree_species.gd` + `data/art/tree_species.json`, `forest_detail.gd`, `shaders/campaign_tree_impostor.gdshader` | OUI mais par rendu atlas (image → 3D → bake imposteur, `tools/blender_scripts/campaign_tree_impostors.py`) |
| Herbe/broussailles | Touffe sprite `ga3_grass_tuft.png`, MultiMesh par cellule | `ground_clutter.gd`, `shaders/ground_clutter.gdshader` | Non (texture) |
| Rochers | Rochers TRELLIS `vegetation/ga3/ga3_rock_{a,b,c}_lod{0,1,2}.glb` (120/60/18 tris) via `GroundClutter` ; affleurements `models/rocks/hb/<id>_lod{0,1,2}.glb` (6 types) via catalogue | `ga3_vegetation.gd` (`rock_meshes`), `rock_outcrops.gd`, `data/art/rock_outcrops.yaml` | Fait (GA3/HB) ; densifier |
| Champs / haies / vignes | Parcellaire dans le shader de terrain (`field_at`) + haies semées ; `vine_row_*.glb`, `haystack_*` | `vegetation_fields.gd`, `shaders/terrain.gdshader`, `fine_parcels.gdshaderinc` | Non (shader) ; props possibles |
| Routes | Traits (moyen) + rubans de terre drapés (proche), ancrés `road.gdshader`, `road_fine.gdshader` | `road_renderer.gd`, `map_paths.gd` | Non (texture/shader) |
| Eau : rivières, lacs, côte, mer | Rubans/shaders `river_water`, `river_fine`, `water.gdshader`, `coast_band`, bassins `sea_basins` ; mousse | `rivers_renderer.gd`, `lakes_renderer.gd`, `coast_renderer.gd`, `sea.gd` | Non (shader) |
| Relief / sol | Pyramide de relief + `satellite_ground`, colormap, nuages/ombres, brume | `terrain_builder.gd`, `relief_*.gd`, `campaign_clouds.gdshader` | Non |
| Vie : paysans, charrettes, bétail, processions, bûchers, échoppes | Figurines `villager_*` + accessoires Blender `folk/*.glb` (vache, mouton, bœuf, cheval, charrues, charrette marchande/funèbre/pierre, étal, croix, bannière de procession, échafaud, torche, bûcher) | `scripts/map/life_folk/folk_models.gd`, `assets/models/folk/manifest.json`, `folk_prop.gdshader` | OUI (animaux et charrettes) |
| Fumées, incendies, oiseaux, précipitations, cicatrices de guerre | GPU particles/shaders (`life_smoke`, `fire_flame`, `life_birds`, `campaign_precipitation`) ; `war_scars.gd`, `war_scar_meshes.gd` | `life_effects.gd`, `life_ambient.gd`, `campaign_weather_view.gd`, `war_scars.gd` | Non (effets) ; cicatrices : props possibles |
| Frontières, routes maritimes, signes TB2 | Rubans/décals/shaders (`faction_borders`, `sea_lane`), icônes sprites | `faction_borders.gd`, `sea_lane_layer.gd`, `stance_fill.gd` | Non |
| Étiquettes | Label3D / 2D | `region_labels.gd`, `label_placer.gd` | Non |

## 2. Comment brancher un nouveau glb (intégrateur)

1. **Dépôt et import** : glb sous `assets/models/<famille>/` ; `godot --headless --path game --import` une fois. `ModelLibrary.instantiate("<dossier>/<nom>", échelle)` charge `res://assets/models/<nom>.glb` (cache `_scenes`), tolérant si absent (retombe sur le placeholder). Échelle : CITY_SCALE 4,8, ARMY_SCALE 3,8, SHIP_SCALE 5,5 ; unités glb = mètres Blender, +X devant, origine au sol (`folk/SOURCE.md`).
2. **Maquettes villes** : nom `settlements/<type>[_<famille>]_<a|b>.glb`, repli famille par défaut ; une surface `Kit`, teintes en couleur de sommet (`maquette_kit.gdshader`) ; taille monde = `data/art/town_maquettes.json` `sizes` ÷ `native_width` du modèle (mesurer la largeur native et la reporter) ; bannière de faction = surface `Banner` (`tint_banner`). Un glb TRELLIS (texture, une surface) demande de baker un matériau unique ou d'adapter le shader.
3. **Bâtiments/hors murs** : `outbuildings/<nom>.glb` une surface `Building` (atlas `TownBuilder.material`), enregistré dans `outbuildings/manifest.json` (triangles, boîte) et `data/map/building_models.json` (niveau 1-3).
4. **GA3 props (bataille/kit)** : `props_ga/ga3_<id>_lod{0,1,2}.glb` + albédo/normal jpg, `manifest.json` généré, catalogue `data/art/ga3_decor.json` (`wired`, `share`, `kit_kind`, `fit`). `Ga3Kit` (`scripts/visual/ga3_kit.gd`) : tuiles `CELL` 120 m, LOD0 < 90 m, LOD1 < 280 m, LOD2 au-delà, enfoncement 0,35 m, anisotropie ≤ 1,15. Hors bataille `Ga3Kit.active` est faux : à étendre pour la campagne.
5. **Rochers/végétation** : `models/vegetation/ga3/<nom>_lod{0,1,2}.glb` (premier `MeshInstance3D`, une surface, `Ga3Vegetation._load_mesh`) ; `models/rocks/hb/<id>_lod{0,1,2}.glb` + entrée dans `data/art/rock_outcrops.yaml`. Arbres : imposteurs atlas 3 lignes × 8 azimuts de 256² (`ga3_impostors_albedo/normal.png`), cadrage `ortho`/`foot` ; ajouter une essence = une ligne `tree_species.yaml` puis `ga3_vegetation_l2.py species`.
6. **Figurines (recoloration livrée)** : maillage `battle_ga3/<kind>_<n>_lod{0,1,2}.mesh.bin` + `<kind>_<n>_albedo.png` + entrée de `battle_ga3/manifest.json` (`faces`, `head_y`, `ga3_lum`, `lods`, `tris`, `unit`, `variants`). Chaîne : `tools/blender_scripts/ga3_figures.py` (UV `cam_uvs`). Livrée : le **canal alpha de l'albédo** marque la zone teintable (code C_LIVERY surcot/jaque/écu, C_CLOTH étoffe) ; le shader `battle_soldier_skinned.gdshader` (`#ifdef GA3_TEX`) mélange `livery × (lum/ga3_lum.x)` ; le vert pur du prompt (livrée) et le bleu pur (chausses) sont les couleurs-clés servant à fabriquer ce masque alpha au cuisson, pas des couleurs lues à l'exécution. Paramètres carte : `livery`, `trim`, `livery_share` (0,95 chef / 0,7 escorte), `interp_distance` 150, `far_start` 260 ; LOD0 < 90, LOD1 < 320 (`army_figures.gd`). Budget (bible § 6) : LOD0 ≤ 12 000 tris à pied, 17 500 monté ; LOD1 ≤ 1 350 / 2 100 ; LOD2 ≤ 260 / 550.
7. **Voiles** : `campaign_sail.gdshader`, teinte via `tint_banner` ; surfaces nommées `Banner`/voile requises.

## 3. Liste priorisée à générer (haut = plus visible à l'écran)

Brief : « X, 14th-century, semi-realistic, three-quarter view, plain grey background ». Cible tris campagne : ≤ 3-8 k par objet (LOD0), MultiMesh.

| # | Asset | Remplace / emplacement | Brief anglais |
|---|---|---|---|
| 1 | Lord à cheval, livrée verte/chausses bleues, bras écartés | variante lord `army_figures.gd` (cavalry_*) | Mounted 14th-century French knight lord in full plate with surcoat, horse caparison, arms stretched out horizontally, empty open palms, white background, semi-realistic |
| 2 | Ville moyenne ouest (town_west) | `settlements/town_west_{a,b}` | Medieval walled market town of 1350 seen from above at three-quarter angle, stone curtain wall with towers, clustered timber and slate roofs, central church spire, semi-realistic miniature |
| 3 | Grande ville ouest (city_west) | `city_west_{a,b}` | Large medieval French walled city circa 1400, double ring of crenellated walls, dense roofs, gothic cathedral with twin towers, river gate, semi-realistic miniature |
| 4 | Château ouest | `castle_west_{a,b}` | 14th-century French stone castle on a mound, round corner towers with conical slate roofs, keep, curtain wall and gatehouse with drawbridge, semi-realistic |
| 5 | Village ouest | `hamlet_*`, village | Cluster of 14th-century French peasant houses with thatched roofs, small stone church, timber barns, fenced gardens, miniature three-quarter view |
| 6 | Cathédrale / abbaye ouest | `abbey_west_{a,b}`, monuments des villes | Gothic abbey with cloister, long nave and bell tower, grey limestone and slate, 14th century, semi-realistic |
| 7 | Ville Angleterre (family isl) | `town_isl_*`, `city_isl_*` | English medieval walled town 1360, half-timbered houses, stone parish church with square tower, wooden gate, semi-realistic miniature |
| 8 | Château anglais | `castle_isl_*` | English Norman-style stone castle with square keep, curtain wall and towers on a motte, 14th century |
| 9 | Ville Méditerranée (med) | `town_med_*`, `city_med_*` | Mediterranean medieval hill town with terracotta roofs, tower houses, stone walls and a campanile, 14th century |
| 10 | Cogue de commerce/guerre | `fleet/cog.glb` | 14th-century Hanseatic cog with single square sail, high stern castle, clinker hull, no flag, semi-realistic |
| 11 | Nef/carrack | `fleet/nef.glb` | Late medieval two-masted sailing ship with forecastle and sterncastle, red-ochre hull, plain furled sails, 15th century |
| 12 | Camp de siège | `siege_camp.glb` | Medieval siege camp with canvas tents, wooden palisade, trebuchet, siege ladders and stacked stone shot, muddy ground, 14th century, miniature |
| 13 | Bivouac d'armée | `fleet/bivouac.glb` | Small medieval army bivouac with three canvas tents, campfire ring, stacked spears and hobbled horses, miniature |
| 14 | Pont de pierre à arches | `bridge_meshes.gd` (stone) | 14th-century stone bridge with three round arches, cutwaters, fortified tower at one end, weathered limestone, semi-realistic |
| 15 | Pont de bois | `bridge_wood_*` | Medieval wooden trestle bridge on timber piles with plank deck and side rails, weathered oak |
| 16 | Pont-porte fortifié | pont-porte des murailles | Medieval fortified gate bridge with crenellated stone towers on both ends and a portcullis, 14th century |
| 17 | Moulin à vent sur pivot | `buildings/windmill_0` | Medieval wooden post mill windmill with lattice sails, weathered oak planks, tail pole, 14th century (brief repris de `ga3_decor.json`) |
| 18 | Moulin à eau | `watermill_*` | Medieval stone and timber watermill with wooden undershot wheel, thatched roof, small millpond |
| 19 | Ferme fortifiée / grange | `outbuildings/farm_*`, `barn_*` | 14th-century French farmstead with long timber-framed barn, thatched roofs, courtyard wall, haystacks |
| 20 | Vignoble avec cabane | `vine_row_*`, outbuilding vigne | Medieval vineyard terrace with staked vine rows and small stone hut, France 14th century |
| 21 | Port médiéval (quai + grue) | outbuilding `port` | Medieval harbour with stone quay, wooden treadwheel crane, warehouses and moored small boats, 14th century |
| 22 | Mine / saline | outbuildings mine, saline | Medieval salt works with rectangular evaporation pans, wooden shed and stacked sacks; second: small timber-propped mine entrance with ore cart |
| 23 | Tour de siège de campagne | à ajouter au camp | Medieval wooden siege tower on four wheels with hide-covered walls and drawbridge, 14th century |
| 24 | Trébuchet | camp de siège | Medieval counterweight trebuchet on an A-frame, weathered timber, stone shot piles, 14th century |
| 25 | Échafaudage de chantier | `worksite_1`, `scaffold.glb` | Stone cathedral construction worksite with timber scaffolding, wooden crane and stacked ashlar blocks, 14th century |
| 26 | Chêne de campagne (3 variantes) | imposteurs `ga3_impostors_*` | Mature European oak tree full summer foliage, broad crown, semi-realistic, isolated (puis bake en imposteur 8 azimuts) |
| 27 | Hêtre, sapin, pin maritime, saule | idem, nouvelles lignes `tree_species.yaml` | Beech, spruce, maritime pine, willow: same brief per species |
| 28 | Rochers/affleurements supplémentaires (calcaire, falaise côtière) | `rocks/hb`, `rock_outcrops.yaml` | Weathered limestone cliff outcrop with grass tufts at the base, semi-realistic |
| 29 | Vaches / moutons / bœufs | `folk/cow`, `sheep`, `ox` | Medieval-breed cow standing in profile, brown and white coat, semi-realistic (idem mouton, bœuf) |
| 30 | Charrette marchande / chariot | `folk/merchant_cart`, `buildings/wagon_*` | Covered wooden merchant cart with two spoked wheels pulled by an ox, 14th century |
| 31 | Étal de marché | `folk/market_stall` | Medieval market stall with striped canvas awning and baskets of produce |
| 32 | Procession (croix, bannière, bière) | `folk/procession_*` | Medieval processional wooden cross on a pole; banner of a saint; litter with covered coffin |
| 33 | Ruine de château / ville rasée | `ruin_markers.gd` (sprite) | Ruined burnt medieval castle with collapsed tower and charred beams, 14th century, semi-realistic |
| 34 | Hampe d'étendard + pennon sculptés | `army_marker.gd` hampe procédurale | Medieval banner pole with carved finial, cloth banner wavy and flat for texture painting |
| 35 | Fort/tour de guet, peu de villes | outbuildings `watchtower` | Medieval square stone watchtower with wooden hoarding and beacon platform |
| 36 | Prieuré/chapelle isolée | hors-mur, églises | Small stone romanesque chapel with bellcote, rural France 14th century |
| 37 | Cavalier d'escorte et archer variantes | `battle_ga3/cavalry_*`, `archer_*` | Longbowman / mounted man-at-arms with livery, arms outstretched, empty hands (voir règles figurines) |
| 38 | Villes byz/rus/steppe (lot ultérieur) | `town_byz_*`, `town_rus_*`, `town_steppe_*` | Domed Byzantine town / wooden Rus kremlin / yurt and timber steppe camp, 14th century |

## 4. Observations

- Les maquettes stylisées (GC2) et les navires/camps sont le plus visible à toutes les hauteurs : priorité campagne.
- `GA3` couvre déjà la bataille (maison, église, moulin, tente, puits, charrette, palissade, bélier, trébuchet) ; la campagne n'emploie que rochers et imposteurs d'arbres. `Ga3Kit.active` est bataille seulement.
- Les glb TRELLIS sont mono-surface texturés : villes à teintes cuites en sommet (maquette_kit) exigent un second matériau ou un shader de variante.
- Points ouverts pour l'intégration : mesure de `native_width` par modèle, budget MultiMesh (≤ 8 k tris par ville LOD0), recoloration des bannières sur modèles texturés (surface `Banner` séparée à ajouter à la main).
