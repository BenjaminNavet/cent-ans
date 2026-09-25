# V4 — Fleuves, ponts et forêts de la carte de campagne (lots A1-11, A1-10)

Branche `worktree-agent-ae5685646cc8b59ac` (fusionne `main` jusqu'à 70c9e2e5 : L1, R1, CV1, CV2…).
Rendu seulement. Captures : `docs/audit/captures/v4/` (`avant_*` sur main 356a1ad, `apres_*`).
Banc : `godot --rendering-driver vulkan --disable-vsync --path game --script res://tests/v4_map_bench.gd`
(Metal plafonne à 145 i/s et ne rend pas le temps GPU ; Vulkan/MoltenVK donne le temps GPU).
`-- --hide=Rivers,Vegetation,Rivers/Crossings` masque des nœuds pour attribuer le coût.

## État : terminé (les deux lots)

### A1-11 fleuves et ponts
- Outil `tools/cent_ans_tools/geo/river_render.py` (`uv run --project tools cent-ans geo rivers-render`,
  tests `tools/tests/test_river_render.py`) → `data/map/rivers_render.json` (tronçons lissés,
  orientés vers l'aval, largeur par point selon l'importance et l'altitude, `river_styles.json`),
  `data/map/river_bed.png` (distance signée à la berge, L8) et `data/map/crossings_px.json`
  (871 passages : 79 ponts historiques et 17 gués de `crossings.json` recalés sur leur fleuve,
  775 ponts de route aux croisements route/fleuve ; structures : 634 bois, 216 pierre, 16 bacs,
  4 ponts de bateaux).
- `crossings.json` : champ `structure` (stone, wood, boats, ferry, ford) ; schéma mis à jour.
- Terrain : `game/shaders/river_bed.gdshaderinc` + `river_banks.gdshaderinc` (3 lignes dans
  `terrain.gdshader`) : lit creusé dans les tuiles de relief fin, berges (vase, roseaux).
- `game/scripts/map/rivers_renderer.gd` + `game/shaders/river_water.gdshader` : ruban d'eau avec
  test de profondeur (les murs le cachent), épaisseur d'eau lue dans la profondeur, écoulement,
  reflets du ciel (fresnel), gués ; l'eau passe **sous** les villes (coupée dans leur emprise).
- `game/scripts/map/river_crossings.gd` + `bridge_meshes.gd` : ponts 3D procéduraux (pierre à
  arches brisées et avant-becs, bois sur pilotis, bateaux, bac, gué), instanciés par tuile
  proche ; ponts-portes crénelés où un fleuve entre dans une ville fortifiée.

### A1-10 forêts
- `tools/blender_scripts/campaign_trees.py` → `game/assets/models/vegetation/campaign_trees.glb`
  (chêne 90 triangles, hêtre 90, sapin 82 ; variantes lointaines 20/20/12), procédural
  reproductible : `blender --background --python tools/blender_scripts/campaign_trees.py -- game/assets/models/vegetation/campaign_trees.glb`.
  Aucun asset tiers (les conifères Poly Haven, 30 000 triangles, sont écartés pour la carte).
- Quatre essences (`VegetationTileJob.Kind` : chêne, hêtre, conifère, haie) ; houppiers élargis au
  cœur des massifs (canopée continue) ; hêtraie par taches selon l'altitude, recul dans le Midi.
- Saisons (`foliage.gdshaderinc`) : teintes par essence pondérées par le paramètre global
  `campaign_season` du lot CV1 (`--season=winter` pour les captures) : printemps tendre, automne
  roux / cuivré, hiver dénudé ajouré. La variante `foliage_winter.gdshader` (discard) n'est
  employée qu'en hiver (poids hiver > 0,5, lu dans `CampaignLife.seasons`).
- Sources interchangeables : `data/map/forest_cover.json` (schéma `forest_cover.schema.json`) :
  couverture = splat canal B (forêts vers 1340 du lot R1), part de résineux = `forest_kind.png`
  (R1), raster d'essences complet optionnel.

## Point d'accroche L1 (Paris)
`data/map/river_styles.json` → `custom_zones` : `{"id":"paris","lonlat":[2.3499,48.853],
"radius_px":6.8,"boundary_bridges":false}` (valeurs L1). Dans le cercle : ni ruban d'eau, ni lit,
ni berge, ni pont, ni pont-porte ; la maquette L1 dessine sa Seine. Après modification :
`cent-ans geo rivers-render`. `RiversRenderer.custom_zones()` expose les zones.

## Performance (Vulkan, temps GPU, meilleur de 3 passes, alternées avec une copie de main a8e1cc7a)
| Vue | main | V4 | écart | primitives |
|---|---|---|---|---|
| large (France, d=1500) | 8,76 ms | 8,18 ms | −6,6 % | 0,29 → 0,36 M |
| très proche (Paris, d=22) | 17,39 ms | 18,79 ms | +8,1 % | 10,2 → 10,8 M |
| forêt (Orléanais, d=90) | 26,14 ms | 27,94 ms | +6,9 % | 10,8 → 11,6 M |
| Loire (Orléans, d=40) | 21,05 ms | 22,30 ms | +5,9 % | 10,9 → 11,6 M |

Attribution (masquage) : eau + ponts ≈ 0,6 à 1,2 ms selon la vue, végétation ≈ +1 ms (arbres
Blender ramenés de ≈ 150 à ≈ 90 triangles, discard réservé à l'hiver) ; terrain (lit, berges)
dans le bruit.

## Points ouverts
- Structures des ponts historiques (bois / pierre / bateaux) à vérifier par l'historien
  (`crossings.json`, listes dans l'outil de génération de la branche).
- L'eau passe sous les villes au lieu de les traverser à ciel ouvert (sauf Paris, modèle L1).
- `wetlands.png` (R1) n'est pas lu : des arbres peuvent pousser dans les marais.
- Machine partagée (≈ 10 agents) : bruit de mesure de ±10 % entre passes.
