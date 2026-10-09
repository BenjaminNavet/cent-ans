class_name BattleTerrain
extends Node3D

## Champ de bataille maillé depuis `BattleSim.get_terrain()` (lot V4, rendu semi-réaliste) :
## - sol texturé (shader `battle_ground`, 9 couches PBR Poly Haven) piloté par une splatmap cuite
##   ici (sous-bois, chemins, boue, galets du lit et des gués, berges humides) ;
## - anneaux de terrain autour du champ (proche à 20 m, lointain à 200 m) : collines qui montent
##   vers l'horizon, vallée de la rivière prolongée, pour que l'horizon ne soit jamais vide ;
## - rivière (shader `battle_water` : profondeur, réfraction, reflets, écoulement) ;
## - arbres (tronc texturé + houppier en cartes alpha), buissons, rochers, bois lointains ;
## - herbe animée autour du regard (`BattleVegetation`).
## Rendu seulement : relief, zones, rivière et gués viennent de la simulation ; chemins, parcelles
## et bois hors du champ sont décoratifs.
##
## Lot B5 (champ tiré de la campagne) : le terrain de la province règle la densité des bois, le relief
## et les bois de l'horizon (`BIOMES`), la saison la teinte des feuillages, le sol (`ground`) la neige
## ou la boue ; haies des courtils et du bocage semées avec les buissons (et chênes têtards), fossés,
## mares et plage cuits dans la splatmap ; clôtures, palissades, mares, roseaux et mer
## dans `BattleSiteFeatures`. Options après `--` : `--terrain=<plains|hills|mountains|forest|marsh|heath|
## bocage|steppe|desert>`, `--season=<spring|summer|autumn|winter>`, `--coast`,
## `--ground=<dry|muddy|snowy>` (rendu seulement, comme `--weather=`)
## (réécrivent la mise en place avant la simulation, captures).
##
## SC BT7 : hauteurs et rivière en Rust (`BattleTerrainKernel`), puis un script par responsabilité :
## `BattleTerrainSplat` (textures cuites, matériau), `BattleTerrainMesh` (sol, anneaux, rivière),
## `BattleTerrainScatter` (arbres, haies, rochers, décor) ; l'état et les requêtes restent ici.
##
## Lot EP3 (eau et chemins) : rivière de largeur variable (`river.widths`), berges escarpées ou
## marécageuses, affluent et ruisseaux (rubans d'eau étroits), gués caillouteux (galets et pierres
## affleurantes), ponts du kit Blender (`BattleBridges`), routes de la simulation (`roads`)
## prolongées hors du champ (ornières et bas-côtés dans la splatmap).

## EP1 (ADR 0076) : dimensions du champ lues dans `get_terrain()` (`width`, `depth` : 1200 × 800 au
## palier « escarmouche », jusqu'à 2400 × 1600) ; la splatmap et les anneaux suivent
## (`_set_field_size`). Variables (et non constantes) aux anciens noms.
var FIELD_W := 1200.0
var FIELD_D := 800.0
## Domaine de la splatmap et de la carte de hauteurs de l'herbe (x0, z0, largeur, profondeur) :
## le champ plus 400 m de chaque côté.
var SPLAT_RECT := Rect2(-400, -400, 2000, 1600)
## CR1 : couche de rendu des surfaces qui reçoivent les décales d'interface (contour de formation,
## fantôme d'arrivée) : sol, eau, herbe. Les décales la prennent pour `cull_mask` : sans elle, leur
## boîte de 30 m projetait le trait sur les figurines, chevaux et murs debout dans la bande
## (soldats « fantômes » blancs, caparaçons rougis, bande blanche sur les piles de pont).
const DECAL_LAYER := 1 << 10
const SPLAT_TEXEL := 4.0
const HEIGHT_TEXEL := 10.0
var NEAR_RECT := Rect2(-900, -900, 3000, 2600)
const NEAR_STEP := 20.0
var FAR_RECT := Rect2(-7000, -7000, 15200, 14800)
const FAR_STEP := 200.0
## Arbres : taille des tuiles (m), distance de passage au maillage allégé, portée des buissons.
const TREE_TILE := 160.0
const TREE_LOD_DISTANCE := 260.0
const BUSH_DISTANCE := 520.0
## B5 : creusement visuel des mares (m, valeur reprise par le noyau Rust) et portée des haies.
const POOL_CARVE := 0.7
const HEDGE_DISTANCE := 1100.0
## B5 : réglages de rendu par terrain de province. `relief` = échelle des collines de l'horizon,
## `near_woods` / `far_woods` = seuil du bruit des bois décoratifs (plus bas = plus de bois),
## `rocks` = facteur des rochers, `snow_line` = altitude des neiges (hiver, montagne).
## R2 : `ridges` = part de crêtes (bruit « ridged ») dans les collines de l'horizon, `rolls` =
## amplitude (m) des ondulations moyennes qui prolongent le relief du champ au-delà du bord.
const BIOMES := {
	"plains": {"relief": 0.45, "near_woods": 0.3, "far_woods": 0.26, "rocks": 0.6, "snow_line": 10000.0, "ridges": 0.0, "rolls": 3.0},
	"heath": {"relief": 0.6, "near_woods": 0.4, "far_woods": 0.36, "rocks": 1.2, "snow_line": 10000.0, "ridges": 0.25, "rolls": 4.5},
	"bocage": {"relief": 0.7, "near_woods": 0.24, "far_woods": 0.16, "rocks": 0.6, "snow_line": 10000.0, "ridges": 0.1, "rolls": 5.0},
	"forest": {"relief": 0.9, "near_woods": 0.08, "far_woods": 0.02, "rocks": 0.8, "snow_line": 10000.0, "ridges": 0.2, "rolls": 5.5},
	"hills": {"relief": 1.5, "near_woods": 0.26, "far_woods": 0.2, "rocks": 1.3, "snow_line": 10000.0, "ridges": 0.55, "rolls": 11.0},
	"mountains": {"relief": 3.4, "near_woods": 0.22, "far_woods": 0.18, "rocks": 2.2, "snow_line": 150.0, "ridges": 0.8, "rolls": 22.0},
	"marsh": {"relief": 0.2, "near_woods": 0.42, "far_woods": 0.38, "rocks": 0.2, "snow_line": 10000.0, "ridges": 0.0, "rolls": 1.0},
}

## GA2 / TX T2c : identité des couches du sol, jamais codée en dur ici
## (`data/fx/battle_ground_layers.json`, schéma `fx_battle_ground_layers.schema.json`). `roles` fixe
## l'ordre des couches du `Texture2DArray` ; le paquet généré du biome du lieu
## (`BattleGroundTextures`) donne leurs matières et tailles de répétition ; `legacy` garde le jeu
## Poly Haven d'avant TX (`--legacy-textures`, ou repli si le paquet manque).
const GROUND_LAYERS_FILE := "fx/battle_ground_layers.json"
## Taille fixe du tableau `layer_tile_size` du shader (couches réelles ≤ cette taille).
const MAX_GROUND_LAYERS := 16
static var _ground_layers: Array = []
static var _ground_layers_loaded: bool = false
static var _ground_role_index: Dictionary = {}
## OM3 (ADR 0116) : teinte de l'herbe par terrain de province, jeu Poly Haven seulement (la steppe
## et le désert ont leur propre biome et leurs propres matières en TX).
static var _terrain_tints: Dictionary = {}


## Couches du sol du jeu Poly Haven (`legacy`) ; l'ordre est celui de `roles`.
static func ground_layers() -> Array:
	if _ground_layers_loaded:
		return _ground_layers
	_ground_layers_loaded = true
	if DataFile.exists(GROUND_LAYERS_FILE):
		var parsed: Variant = DataFile.read_json(GROUND_LAYERS_FILE)
		if parsed is Dictionary and (parsed as Dictionary).get("legacy") is Dictionary:
			var legacy: Dictionary = (parsed as Dictionary)["legacy"]
			_ground_layers = legacy.get("layers", [])
			var tints: Variant = legacy.get("terrain_tints", {})
			if tints is Dictionary:
				_terrain_tints = tints
			for i in _ground_layers.size():
				var role := str((_ground_layers[i] as Dictionary).get("role", ""))
				if role != "":
					_ground_role_index[role] = i
			return _ground_layers
	push_warning("BattleTerrain: %s introuvable, sol replié sur les couches historiques" % GROUND_LAYERS_FILE)
	return _ground_layers


## OM3 : multiplicateur de teinte de l'herbe pour un terrain de province (blanc si absent des données
## ou avec les sols générés TX, qui portent déjà la couleur régionale).
func terrain_tint(key: String) -> Color:
	ground_layers()
	if ground_tx:
		return Color(1, 1, 1)
	var rgb: Variant = _terrain_tints.get(key, null)
	if rgb is Array and (rgb as Array).size() == 3:
		return Color(float(rgb[0]), float(rgb[1]), float(rgb[2]))
	return Color(1, 1, 1)


## GA2 : index (dans le `Texture2DArray`) de la couche portant ce rôle, -1 si absente des données.
static func ground_role_index(role: String) -> int:
	ground_layers()
	return int(_ground_role_index.get(role, -1))


## TX T2c : choisit les tableaux du sol de cette bataille (paquet du biome du lieu, sinon Poly
## Haven). Pose `ground_tx`, `ground_biome`, `ground_layer_list`, `albedo_array`, `normal_array`
## et le grain fin (`micro`).
func resolve_ground() -> void:
	ground_tx = false
	ground_biome = 0
	ground_layer_list = ground_layers()
	albedo_array = ALBEDO_ARRAY
	normal_array = NORMAL_ARRAY
	micro = {}
	if not TextureQuality.use_tx():
		return
	var wanted := BattleGroundTextures.biome_for(province_id, terrain_key)
	var pack := BattleGroundTextures.pack_for(wanted)
	if pack.is_empty():
		return
	ground_tx = true
	ground_biome = int(pack["biome"])
	ground_layer_list = pack["layers"]
	albedo_array = pack["albedo"]
	normal_array = pack["normal"]
	micro = BattleGroundTextures.micro_for(ground_biome, ground_layer_list)
	print("BattleTerrain: sol TX biome %d (demandé %d), %d couches%s" % [ground_biome, wanted, ground_layer_list.size(), ", grain fin" if not micro.is_empty() else ""])


## A6-L14 : échelles de bruit et variation macro du sol (`data/fx/battle_ground.json`, schéma
## `fx_battle_ground.schema.json`) ; sans fichier, les uniforms gardent les anciennes constantes.
const GROUND_NOISE_FILE := "fx/battle_ground.json"
static var _ground_noise := JsonLookup.new(GROUND_NOISE_FILE)


static func ground_noise() -> Dictionary:
	return _ground_noise.data()


func _apply_ground_noise() -> void:
	var data := ground_noise()
	var scales: Dictionary = data.get("noise_scales", {})
	for key in scales:
		ground_material.set_shader_parameter("freq_" + str(key), float(scales[key]))
	var macro_var: Dictionary = data.get("macro_variation", {})
	if not macro_var.is_empty():
		ground_material.set_shader_parameter("freq_macro_var", float(macro_var.get("frequency", 0.0017)))
		ground_material.set_shader_parameter("macro_tint_strength", float(macro_var.get("tint_strength", 0.0)))
		ground_material.set_shader_parameter("macro_wet_strength", float(macro_var.get("wetness_strength", 0.0)))
		ground_material.set_shader_parameter("macro_albedo_strength", float(macro_var.get("albedo_strength", 0.0)))
	ground_material.set_shader_parameter("fine_contrast", float(data.get("fine_contrast", 1.0)))


const GROUND_SHADER := preload("res://shaders/battle_ground.gdshader")
const WATER_SHADER := preload("res://shaders/battle_water.gdshader")
const ALBEDO_ARRAY := preload("res://assets/textures/battle/ground_albedo_array.jpg")
const NORMAL_ARRAY := preload("res://assets/textures/battle/ground_normal_array.jpg")
## PO4 : détail proche du sol (Poly Haven CC0, `near_detail/SOURCE.md`), avec DA6.
const NEAR_DETAIL_ALBEDO := preload("res://assets/textures/battle/near_detail/grass_path_2_diff_2k.jpg")
const NEAR_DETAIL_NORMAL := preload("res://assets/textures/battle/near_detail/grass_path_2_nor_gl_2k.jpg")

## TX T2c : tableaux du sol de cette bataille (paquet du biome, sinon Poly Haven), liste des couches
## correspondante, grain fin (`micro`), biome retenu (0 = Poly Haven).
var ground_tx: bool = false
var ground_biome: int = 0
var ground_layer_list: Array = []
var albedo_array: TextureLayered = ALBEDO_ARRAY
var normal_array: TextureLayered = NORMAL_ARRAY
var micro: Dictionary = {}
var terrain: Dictionary = {}
var weather_key: String = "clear"
var tree_count: int = 0
var ground_material: ShaderMaterial
var macro_noise: Texture2D
var splat_a: ImageTexture
var splat_b: ImageTexture
var height_texture: ImageTexture
var vegetation: BattleVegetation
var roads: Array[PackedVector2Array] = []
var _heights: PackedFloat32Array
var _nx: int = 0
var _nz: int = 0
var _resolution: float = 10.0
var _mean_height: float = 0.0
var _river_points: PackedVector2Array = PackedVector2Array()  # prolongée hors du champ
## EP3 : largeur de l'eau à chaque point de `_river_points`, niveau de l'eau (ruban), sens du courant.
var _river_widths: PackedFloat32Array = PackedFloat32Array()
var _river_levels: PackedFloat32Array = PackedFloat32Array()
var river_flow: float = 1.0
## EP3 : largeur de chaque route de `roads` (même ordre), ruisseaux rééchantillonnés.
var road_widths: Array[float] = []
var _streams: Array = []  # [{points: PackedVector2Array, width, kind}]
## PB3c : tronçons des ruisseaux pour `in_water` : [{box: Rect2 (élargi de la demi-largeur),
## points: PackedVector2Array, half: demi-largeur}] ; seuls les tronçons dont la boîte contient
## le point sont mesurés (même résultat, sans parcourir tout le tracé à chaque appel).
var _stream_chunks: Array = []
const STREAM_CHUNK := 24
var bridges_view: BattleBridges
## SC PF-08/BT7 : hauteurs, rivière, grilles et maillages du sol calculés en Rust.
var _kernel := BattleTerrainKernel.new()
## SC BT7 : un script par responsabilité (l'état reste ici) : textures, maillages, semis.
var _splat := BattleTerrainSplat.new(self)
var _mesh := BattleTerrainMesh.new(self)
var _scatter := BattleTerrainScatter.new(self)
var _splat_w: int = 0
var _splat_h: int = 0
var _hills := FastNoiseLite.new()
var _woods := FastNoiseLite.new()
## R2 : crêtes et ondulations de l'horizon, carte de relief du sol (pente, creux, crêtes).
var _ridges := FastNoiseLite.new()
var _rolls := FastNoiseLite.new()
var relief_texture: ImageTexture
## B5 : site de campagne.
var terrain_key: String = "plains"
var season_key: String = "summer"
var ground_key: String = "dry"
var woodland: float = 0.5
var biome: Dictionary = BIOMES["plains"]
## Toujours vrai : `BattleVegetation` le lit encore.
var site_render: bool = true
var site_view: BattleSiteFeatures
## EP6 : décor du champ (hameaux, moulins, église, manoir, vignes, camps) et parcelles peintes au
## sol (r = nature/8 : 1 labour, 2 blé, 3 pré, 4 semis, 5 vigne, 6 chaume ; g = lacet/π ; b = bord ;
## a = parcelle), même rectangle que les splatmaps.
var decor_view: BattleDecor
var decor_fields: ImageTexture
var decor_on := false
## DA6 (bible DA § 6) : végétation de bataille — lisières douces des cultures, touffes d'herbe en
## volume, feuillus ramifiés par essence avec imposteurs au loin, détail du sol de près.
## Toujours vrai : `BattleVegetation` le lit encore.
var da6 := true
## GA2 : couches supplémentaires du sol (prairie fleurie, herbe piétinée, chaume, labour frais)
## et macro-variation de teinte/luminance (50–200 m).
var tree_view: BattleTrees = null


## DA6 (bible § 3.3) : part de saturation gardée par le sol et l'herbe ; la saison déplace la teinte,
## pas la saturation. DA7b : valeur par saison dans `data/fx/atmosphere.json` (`battle_seasons`).
func decor_saturation() -> float:
	return AtmosphereLibrary.battle_decor_saturation(season_key)
var _decor_clear: Array = []  # [centre: Vector2, demi-tailles: Vector2, lacet] (arbres écartés)
var _coast: Dictionary = {}
var _pools: Array = []
var _waves: NoiseTexture2D
## B7 : neige piétinée (null hors neige au sol).
const TRAMPLE_TEXEL := 4.0
const TRAMPLE_STEP := 0.5
var trample_image: Image = null
var _trample_texture: ImageTexture
var _trample_bytes := PackedByteArray()
## PB3e : empreintes tamponnées en Rust (`StampMap`).
var _trample_map: RefCounted = null
var _trample_timer: float = 0.0
var _trample_last: Dictionary = {}  # id -> dernière position (x, z) imprimée
var _trample_snow: bool = true  # B8 : false = carte de boue (sol détrempé)
## EP2 : horizon (relief réel lointain, panorama peint) ; province du lieu, posée par la scène.
var province_id: String = ""
## EP7 : tuile d'horizon d'un site historique (`hist_crecy`), cadrée et orientée sur le champ.
var horizon_site: String = ""
var horizon: BattleHorizon = null


## B5 : options de ligne de commande qui réécrivent la mise en place de la bataille (terrain,
## saison, côte) avant la simulation : captures des variantes sans campagne dédiée.
static func apply_site_overrides(setup: Dictionary) -> void:
	for arg in CmdArgs.args():
		if arg.begins_with("--terrain="):
			setup["terrain"] = arg.trim_prefix("--terrain=")
			setup["river"] = false
		elif arg == "--river":
			setup["river"] = true
		elif arg.begins_with("--season="):
			setup["season"] = arg.trim_prefix("--season=")
		elif arg == "--coast":
			setup["coastal"] = true
		elif arg.begins_with("--province="):
			# EP6 : paysage d'une autre province (vignoble, bocage…), captures et essais.
			setup["province"] = arg.trim_prefix("--province=")
		elif arg.begins_with("--decor-plan="):
			# EP6 : plan de décor posé à la main (schéma `battle_decor_plan`), essais et captures.
			var plan: Variant = DataFile.try_parse(arg.trim_prefix("--decor-plan="))
			if plan is Dictionary:
				# Le JSON de Godot lit tous les nombres en flottants : entiers attendus par le cœur.
				for item in (plan as Dictionary).get("items", []):
					for key in ["houses", "seed"]:
						if (item as Dictionary).has(key):
							item[key] = int(item[key])
				setup["decor_plan"] = plan
			else:
				push_warning("--decor-plan : plan illisible (%s)" % arg)


func build(p_terrain: Dictionary, weather: String) -> void:
	terrain = p_terrain
	_kernel = BattleTerrainKernel.new()
	weather_key = weather
	terrain_key = str(terrain.get("terrain", "plains"))
	season_key = str(terrain.get("season", "summer"))
	ground_key = str(terrain.get("ground", "dry"))
	woodland = float(terrain.get("woodland", 0.5))
	# Lot TF : colombage ou enduit/pierre selon la région de la province.
	BuildingKit.region_style = BuildingRegions.style_for_province(province_id)
	Ga3Kit.active = true
	ground_key = CmdArgs.value("--ground", ground_key)  # rendu seulement, comme --weather=
	biome = BIOMES.get(terrain_key, BIOMES["plains"])
	_coast = terrain.get("coast", {})
	_pools = terrain.get("pools", [])
	_heights = terrain.get("heights", PackedFloat32Array())
	_nx = int(terrain.get("nx", 0))
	_nz = int(terrain.get("nz", 0))
	_resolution = float(terrain.get("resolution", 10.0))
	_set_field_size(float(terrain.get("width", 1200.0)), float(terrain.get("depth", 800.0)))
	for child in get_children():
		child.queue_free()
	roads.clear()
	road_widths.clear()
	_streams.clear()
	_stream_chunks.clear()
	if _nx < 2 or _nz < 2:
		return
	_mean_height = 0.0
	for h in _heights:
		_mean_height += h
	_mean_height /= float(_heights.size())
	_hills.seed = 1337
	_hills.noise_type = FastNoiseLite.TYPE_SIMPLEX_SMOOTH
	_hills.frequency = 1.0 / 1400.0
	_hills.fractal_octaves = 4
	_woods.seed = 4242
	_woods.noise_type = FastNoiseLite.TYPE_SIMPLEX_SMOOTH
	_woods.frequency = 1.0 / 420.0
	_woods.fractal_octaves = 3
	_ridges.seed = 2718
	_ridges.noise_type = FastNoiseLite.TYPE_SIMPLEX_SMOOTH
	_ridges.fractal_type = FastNoiseLite.FRACTAL_RIDGED
	_ridges.frequency = 1.0 / 1600.0
	_ridges.fractal_octaves = 4
	_rolls.seed = 3141
	_rolls.noise_type = FastNoiseLite.TYPE_SIMPLEX_SMOOTH
	_rolls.frequency = 1.0 / 520.0
	_rolls.fractal_octaves = 3
	_scatter._prepare_decor()
	_extend_river()
	_setup_horizon()
	_configure_kernel()
	_plan_roads()
	_splat._build_textures()
	_splat._build_material(weather)
	_splat._apply_terrain_tint()
	_setup_trample()
	_mesh._add_mesh("Ground", _mesh._field_mesh(), true)
	_mesh._add_mesh("NearRing", _mesh._ring_mesh(NEAR_RECT, NEAR_STEP, Rect2(0, 0, FIELD_W, FIELD_D), 0.0), true)
	_mesh._add_mesh("FarRing", _mesh._ring_mesh(FAR_RECT, FAR_STEP, NEAR_RECT.grow(-2.0 * FAR_STEP), 1.5), false)
	if horizon.active:
		horizon.build_visuals(self, FAR_RECT, weather, _sea_rect())
	if terrain.has("river"):
		_mesh._build_river(terrain["river"])
		_mesh._build_ford_stones(terrain["river"])
	_mesh._build_streams()
	bridges_view = BattleBridges.new()
	bridges_view.name = "Bridges"
	add_child(bridges_view)
	bridges_view.build(self, terrain.get("bridges", []), river_flow, snowy())
	_scatter._build_trees()
	_scatter._build_rocks()
	vegetation = BattleVegetation.new()
	vegetation.name = "Vegetation"
	add_child(vegetation)
	vegetation.build(self, weather)
	site_view = BattleSiteFeatures.new()
	site_view.name = "Site"
	add_child(site_view)
	site_view.build(self, terrain, weather)
	decor_view = BattleDecor.new()
	decor_view.name = "Decor"
	add_child(decor_view)
	decor_view.build(self, terrain.get("decor", {}), weather)
	print("BattleTerrain: site %s, %s, ground %s, coast %s, %d pools, %d obstacles, %d reeds" % [
		terrain_key, season_key, ground_key,
		str(_coast.get("flank", "none")), _pools.size(), (terrain.get("obstacles", []) as Array).size(), site_view.reed_count])
	for pool in _pools:
		print("BattleTerrain: pool (%.0f, %.0f) r %.0f" % [float(pool["x"]), float(pool["z"]), float(pool["radius"])])
	for wood in terrain.get("forests", []):
		print("BattleTerrain: wood (%.0f, %.0f) r %.0f, ground %.1f m" % [float(wood["x"]), float(wood["z"]), float(wood["radius"]), height_at(float(wood["x"]), float(wood["z"]))])


## B5 : le sol est-il enneigé (neige tombante ou sol de saison) ?
## EP1 : dimensions du champ et rectangles qui en dépendent (identiques à l'ancien champ fixe
## pour 1200 × 800).
func _set_field_size(width: float, depth: float) -> void:
	FIELD_W = width
	FIELD_D = depth
	SPLAT_RECT = Rect2(-400, -400, width + 800.0, depth + 800.0)
	NEAR_RECT = Rect2(-900, -900, width + 1800.0, depth + 1800.0)
	FAR_RECT = Rect2(-7000, -7000, width + 14000.0, depth + 14000.0)


## Centre du champ (600, 400 au palier standard).
func field_center() -> Vector2:
	return Vector2(FIELD_W * 0.5, FIELD_D * 0.5)


## Largeur relative au champ standard (1 pour 1200 m).
func field_scale_x() -> float:
	return FIELD_W / 1200.0


func snowy() -> bool:
	return weather_key == "snow" or ground_key == "snowy"


## B8 : le sol est-il détrempé (pluie ou sol de saison boueux), pour le piétinement en boue ?
func muddy() -> bool:
	return weather_key == "rain" or ground_key == "muddy"


## B7/B8 : piétinement (neige ou boue). Carte L8 sur le rectangle des splatmaps
## (`TRAMPLE_TEXEL` m le texel) où chaque régiment présent imprime son emprise (rectangle
## orienté) ; la trace s'accumule au fil des passages, plus vite pour une troupe en mouvement.
## Rendu seulement, sol enneigé ou détrempé seulement (sinon aucun coût : sol sec, pas de
## texture créée, `trample_on` reste à 0 dans le shader).
func _setup_trample() -> void:
	trample_image = null
	_trample_bytes = PackedByteArray()
	_trample_last.clear()
	_trample_snow = snowy()
	if not _trample_snow and not muddy():
		return
	var w := int(SPLAT_RECT.size.x / TRAMPLE_TEXEL)
	var h := int(SPLAT_RECT.size.y / TRAMPLE_TEXEL)
	_trample_bytes.resize(w * h)
	_trample_map = ClassDB.instantiate(&"StampMap")
	_trample_map.call("setup", w, h, 1, SPLAT_RECT.position, TRAMPLE_TEXEL)
	trample_image = Image.create_from_data(w, h, false, Image.FORMAT_L8, _trample_bytes)
	_trample_texture = ImageTexture.create_from_image(trample_image)
	ground_material.set_shader_parameter("trample_map", _trample_texture)
	ground_material.set_shader_parameter("trample_on", 1.0)


## B7 : imprime les régiments sur la carte de neige piétinée, au plus tous les `TRAMPLE_STEP`
## secondes de bataille (`dt` = temps de bataille écoulé, 0 en pause).
func update_trample(units: Array, dt: float) -> void:
	if trample_image == null or dt <= 0.0:
		return
	_trample_timer += dt
	if _trample_timer < TRAMPLE_STEP:
		return
	var step := _trample_timer
	_trample_timer = 0.0
	for unit in units:
		if not bool(unit["present"]):
			continue
		var id := int(unit["id"])
		var pos := Vector2(float(unit["x"]), float(unit["z"]))
		var moved := pos.distance_to(_trample_last.get(id, pos)) / step
		_trample_last[id] = pos
		# Troupe en marche : ~0,05 par pas (trace nette après une dizaine) ; à l'arrêt, lent.
		var add := int(clampf(3.0 + moved * 7.0, 3.0, 24.0))
		if not _trample_snow:
			# B8 : la boue naît du passage, pas de l'attente (sinon toute la zone de déploiement
			# devient bourbier en une minute).
			add = int(clampf(moved * 3.0 - 2.0, 0.0, 16.0))
			if add == 0:
				continue
		var half := Vector2(float(unit["width"]), float(unit["depth"])) * 0.5 + Vector2(1.5, 1.5)
		var facing := float(unit["facing"])
		_trample_map.call("stamp_box", pos, facing, half, add, 0, 255)
	_trample_map.call("upload", trample_image, _trample_texture)


## B7 : piétinement (0-1) à un point du monde (tests, captures).
func trample_at(x: float, z: float) -> float:
	if trample_image == null:
		return 0.0
	return float(_trample_map.call("sample", x, z, 0))


## Niveau d'une mare : hauteur moyenne du sol de la simulation sur son disque, un peu relevée
## (les pieds des soldats, posés à la hauteur de la simulation, sont dans l'eau).
func pool_level(center: Vector2, radius: float) -> float:
	var total := 0.0
	var n := 0
	for k in 9:
		var ang := TAU * float(k) / 8.0
		var d := 0.0 if k == 8 else radius * 0.6
		total += height_at(center.x + cos(ang) * d, center.y + sin(ang) * d)
		n += 1
	return total / float(n) + 0.08


## Normales de vagues partagées par la rivière, les mares et la mer.
func water_waves() -> NoiseTexture2D:
	if _waves == null:
		_waves = NoiseTexture2D.new()
		_waves.seamless = true
		_waves.as_normal_map = true
		_waves.bump_strength = 6.0
		_waves.width = 256
		_waves.height = 256
		var wave_noise := FastNoiseLite.new()
		wave_noise.frequency = 0.035
		wave_noise.fractal_octaves = 3
		_waves.noise = wave_noise
	return _waves


## Reflet du ciel : entre horizon et zénith du préréglage météo.
func sky_reflection() -> Color:
	var preset: Dictionary = BattleAtmosphere.PRESETS.get(weather_key, BattleAtmosphere.PRESETS["clear"])
	return (preset["horizon"] as Color).lerp(preset["zenith"], 0.3)


## Hauteur bilinéaire du champ (même formule que la simulation) ; bornée au champ.
func height_at(x: float, z: float) -> float:
	return _kernel.height_at(x, z)


## Hauteur décorative partout : le champ dedans, collines et vallée de la rivière dehors
## (calculée en Rust : champ, collines, lit, relief réel de l'horizon, côte).
func world_height(x: float, z: float) -> float:
	return _kernel.world_height(x, z)


## SC PF-08/BT7 : confie au noyau Rust la grille de la simulation, le bruit des collines, la
## rivière prolongée, la côte, mares et relief réel (après `_extend_river` et `_setup_horizon`).
func _configure_kernel() -> void:
	_kernel.set_field(_heights, _nx, _nz, _resolution, FIELD_W, FIELD_D)
	_kernel.set_noise(_hills, _ridges, _rolls)
	_kernel.set_biome(float(biome["relief"]), float(biome.get("ridges", 0.0)), float(biome.get("rolls", 3.0)))
	_kernel.set_river(_river_points, _river_widths, float(terrain.get("river", {}).get("width", 0.0)))
	if _coast.is_empty():
		_kernel.clear_coast()
	else:
		_kernel.set_coast(str(_coast["flank"]) == "west", float(_coast["shore_x"]), BattleSiteFeatures.SEA_LEVEL)
	_kernel.set_pools(_pools)
	if horizon != null and horizon.active:
		_kernel.set_horizon(horizon.kernel_params())
	else:
		_kernel.clear_horizon()


func _in_zones(zones: Array, x: float, z: float, margin: float = 0.0) -> bool:
	for zone in zones:
		var dx: float = x - float(zone["x"])
		var dz: float = z - float(zone["z"])
		var r: float = float(zone["radius"]) + margin
		if dx * dx + dz * dz <= r * r:
			return true
	return false


## R2 : (x, z) est-il dans l'un des `count` premiers disques de `zones` ?
func _in_zones_before(zones: Array, count: int, x: float, z: float) -> bool:
	for i in count:
		var zone: Dictionary = zones[i]
		var r := float(zone["radius"])
		if Vector2(x - float(zone["x"]), z - float(zone["z"])).length_squared() <= r * r:
			return true
	return false


## R2 : profondeur (m) de (x, z) dans la réunion des disques `zones` (0 ou moins : dehors).
func _zones_edge_distance(zones: Array, x: float, z: float) -> float:
	var best := -INF
	for zone in zones:
		var d := float(zone["radius"]) - Vector2(x - float(zone["x"]), z - float(zone["z"])).length()
		best = maxf(best, d)
	return best


## Distance au lit de la rivière (prolongée au-delà du champ), INF sans rivière.
func river_distance(x: float, z: float) -> float:
	return _kernel.river_distance(x, z)


func _in_ford(x: float) -> bool:
	if not terrain.has("river"):
		return false
	for ford in terrain["river"]["fords"]:
		if absf(x - float(ford["x"])) <= float(ford["half_width"]):
			return true
	return false


## EP3 : indice (réel) de `_river_points` au droit de x (un point tous les 10 m).
func _river_index(x: float) -> float:
	if _river_points.size() < 2:
		return -1.0
	return clampf((x - _river_points[0].x) / 10.0, 0.0, float(_river_points.size() - 1))


## EP3 : largeur de l'eau au droit de x (prolongée hors du champ comme le cours).
func river_width_at(x: float) -> float:
	return _kernel.river_width_at(x)


## EP3 : demi-largeur creusée du lit (1,5 × la largeur, comme la simulation).
func river_span_at(x: float) -> float:
	return _kernel.river_span_at(x)


## EP3 : z du milieu de la rivière au droit de x.
func river_center_z(x: float) -> float:
	var f := _river_index(x)
	if f < 0.0:
		return INF
	var i := mini(int(f), _river_points.size() - 2)
	return lerpf(_river_points[i].y, _river_points[i + 1].y, f - i)


## EP3 : (x, z) dans l'eau de la rivière (largeur locale) ou d'un ruisseau.
func in_water(x: float, z: float) -> bool:
	if _river_points.size() >= 2 and river_distance(x, z) < river_width_at(x) * 0.5:
		return true
	var p := Vector2(x, z)
	for chunk in _stream_chunks:
		if not (chunk["box"] as Rect2).has_point(p):
			continue
		var points: PackedVector2Array = chunk["points"]
		var half := float(chunk["half"])
		for i in range(points.size() - 1):
			if Geometry2D.get_closest_point_to_segment(p, points[i], points[i + 1]).distance_to(p) < half:
				return true
	return false


## PB3c : découpe un ruisseau en tronçons de `STREAM_CHUNK` segments (boîtes élargies de la
## demi-largeur, plus une marge : un point hors de la boîte est à plus d'une demi-largeur).
func _add_stream_chunks(pts: PackedVector2Array, half: float) -> void:
	var start := 0
	while start < pts.size() - 1:
		var end := mini(start + STREAM_CHUNK, pts.size() - 1)
		var chunk := pts.slice(start, end + 1)
		var box := Rect2(chunk[0], Vector2.ZERO)
		for q in chunk:
			box = box.expand(q)
		_stream_chunks.append({"box": box.grow(half + 0.5), "points": chunk, "half": half})
		start = end


## EP3 : niveau de l'eau de la rivière au droit de x (-INF sans rivière).
func water_level_at(x: float) -> float:
	var f := _river_index(x)
	if f < 0.0 or _river_levels.is_empty():
		return -INF
	return _river_levels[mini(int(round(f)), _river_levels.size() - 1)]


static func _polyline_distance(points: PackedVector2Array, p: Vector2) -> float:
	var best := INF
	for i in range(points.size() - 1):
		var a := points[i]
		var b := points[i + 1]
		if absf(a.x - p.x) > 60.0 and absf(b.x - p.x) > 60.0 and absf(a.y - p.y) > 60.0 and absf(b.y - p.y) > 60.0:
			continue
		best = minf(best, Geometry2D.get_closest_point_to_segment(p, a, b).distance_to(p))
	return best


## La rivière de la simulation (x de 0 à FIELD_W, un point tous les 10 m) prolongée par symétries
## successives (onde triangulaire en x) jusqu'aux anneaux lointains.
func _extend_river() -> void:
	_river_points = PackedVector2Array()
	_river_widths = PackedFloat32Array()
	if not terrain.has("river"):
		return
	var points: PackedVector2Array = terrain["river"]["points"]
	var widths: PackedFloat32Array = terrain["river"].get("widths", PackedFloat32Array())
	river_flow = float(terrain["river"].get("flow", 1))
	if points.size() < 2:
		return
	var extended: Dictionary = _kernel.extend_river(points, widths, float(terrain["river"]["width"]), 2600.0)
	_river_points = extended["points"]
	_river_widths = extended["widths"]


## EP3 : routes de la simulation (ponts, gués, bords du champ), prolongées hors du
## champ par une marche aléatoire ; plus un chemin de traverse à l'arrière (décor) ; en siège, la
## route de la porte. Sans routes de la simulation (anciens terrains) : un chemin par gué.
func _plan_roads() -> void:
	var rng := RandomNumberGenerator.new()
	rng.seed = 90210 + _nx * 7 + int(_mean_height * 10.0)
	var sim_roads: Array = terrain.get("roads", [])
	for road in sim_roads:
		var pts: PackedVector2Array = road["points"]
		if pts.size() < 2:
			continue
		roads.append(_extend_road(pts, rng))
		road_widths.append(float(road.get("width", 4.0)))
	var crossings: Array[float] = []
	if not terrain.has("roads"):
		if terrain.has("river"):
			for ford in terrain["river"]["fords"]:
				crossings.append(float(ford["x"]))
		elif not terrain.has("siege"):
			crossings.append(rng.randf_range(FIELD_W * 0.5 - 250.0 * field_scale_x(), FIELD_W * 0.5 + 250.0 * field_scale_x()))
	for x0 in crossings:
		var pts := PackedVector2Array()
		var x := x0 + rng.randf_range(-120.0, 120.0)
		for z in range(-1400, int(FIELD_D) + 1401, 100):
			var target := x0 if absf(float(z) - FIELD_D * 0.5) < 250.0 else x
			x = lerpf(x, target, 0.5) + rng.randf_range(-35.0, 35.0)
			pts.append(Vector2(x, float(z)))
		roads.append(_smooth(pts))
		road_widths.append(4.0)
	if terrain.has("siege"):
		var siege: Dictionary = terrain["siege"]
		var center: Vector2 = siege.get("center", Vector2(600, 560))
		var pieces: Array = siege.get("pieces", [])
		var gate_index := int(siege.get("gate", -1))
		if gate_index >= 0 and gate_index < pieces.size():
			var gate: Dictionary = pieces[gate_index]
			var mid: Vector2 = ((gate["a"] as Vector2) + (gate["b"] as Vector2)) * 0.5
			var out := (mid - center).normalized()
			var pts := PackedVector2Array([center, mid])
			var p := mid
			for i in 22:
				out = out.rotated(rng.randf_range(-0.12, 0.12))
				p += out * 90.0
				pts.append(p)
			roads.append(_smooth(pts))
			road_widths.append(5.0)
	# Chemin de traverse (est-ouest) derrière l'une des lignes.
	var zr := -170.0 if rng.randf() < 0.5 else FIELD_D + 180.0
	var side := PackedVector2Array()
	for x in range(-1600, int(FIELD_W) + 1601, 120):
		side.append(Vector2(float(x), zr + rng.randf_range(-30.0, 30.0) + sin(float(x) * 0.004) * 60.0))
	roads.append(_smooth(side))
	road_widths.append(3.5)


## EP3 : une route de la simulation prolongée à ses deux bouts hors du champ (1,4 km environ),
## dans la direction de son dernier tronçon, en serpentant ; les bouts dans le champ restent tels.
func _extend_road(pts: PackedVector2Array, rng: RandomNumberGenerator) -> PackedVector2Array:
	var result := PackedVector2Array()
	var field := Rect2(0, 0, FIELD_W, FIELD_D).grow(-2.0)
	var ends := [[pts[0], pts[0] - pts[mini(3, pts.size() - 1)]], [pts[pts.size() - 1], pts[pts.size() - 1] - pts[maxi(pts.size() - 4, 0)]]]
	var tails: Array[PackedVector2Array] = []
	for e in ends:
		var p: Vector2 = e[0]
		var dir: Vector2 = (e[1] as Vector2).normalized()
		var tail := PackedVector2Array()
		if field.has_point(p) or dir == Vector2.ZERO:
			tails.append(tail)  # la route finit dans le champ (village) : pas de prolongement
			continue
		var base := dir
		for _i in 14:
			dir = dir.rotated(rng.randf_range(-0.18, 0.18))
			if dir.angle_to(base) > 0.5 or dir.angle_to(base) < -0.5:
				dir = base.rotated(clampf(dir.angle_to(base), -0.5, 0.5) * -1.0)
			p += dir * 100.0
			tail.append(p)
		tails.append(tail)
	var head := _smooth(PackedVector2Array([pts[0]]) + tails[0]) if tails[0].size() > 0 else PackedVector2Array()
	head.reverse()
	result.append_array(head)
	result.append_array(pts)
	if tails[1].size() > 0:
		var tail2 := _smooth(PackedVector2Array([pts[pts.size() - 1]]) + tails[1])
		result.append_array(tail2.slice(1))
	return result


## Lissage de Chaikin (deux passes).
static func _smooth(points: PackedVector2Array) -> PackedVector2Array:
	var result := points
	for _pass in 2:
		var next := PackedVector2Array([result[0]])
		for i in range(result.size() - 1):
			next.append(result[i].lerp(result[i + 1], 0.25))
			next.append(result[i].lerp(result[i + 1], 0.75))
		next.append(result[result.size() - 1])
		result = next
	return result


static func _commit(vertices: PackedVector3Array, normals: PackedVector3Array, indices: PackedInt32Array) -> ArrayMesh:
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_INDEX] = indices
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return mesh


## EP6 : distance du point à la route la plus proche (INF sans route).
func road_distance(p: Vector2) -> float:
	var best := INF
	for road in roads:
		for i in range(road.size() - 1):
			best = minf(best, Geometry2D.get_closest_point_to_segment(p, road[i], road[i + 1]).distance_to(p))
	return best


## NA (ADR 0219) : copie légère des arbres plantés (essence, pied, échelles), rangée par tuile de
## `TREE_TILE` m, pour retrouver l'arbre sous le curseur sans nœud ni collision.
var _decor_trees: Dictionary = {}  # Vector2i → Array de {species, position, height, radius}


## Arbres plantés à moins de `reach` m (plan horizontal) de `ground`, pour la bulle du décor.
func decor_candidates(ground: Vector2, reach: float) -> Array:
	var found: Array = []
	var lo := Vector2i(floori((ground.x - reach) / TREE_TILE), floori((ground.y - reach) / TREE_TILE))
	var hi := Vector2i(floori((ground.x + reach) / TREE_TILE), floori((ground.y + reach) / TREE_TILE))
	for ty in range(lo.y, hi.y + 1):
		for tx in range(lo.x, hi.x + 1):
			for tree: Dictionary in _decor_trees.get(Vector2i(tx, ty), []):
				var at: Vector3 = tree["position"]
				if absf(at.x - ground.x) <= reach and absf(at.z - ground.y) <= reach:
					found.append(tree)
	return found


## EP2 : dimensions du champ lues dans la grille de la simulation (EP1 les rend paramétriques).
func field_size() -> Vector2:
	return Vector2(float(_nx - 1) * _resolution, float(_nz - 1) * _resolution)


## EP2 : prépare le relief réel du lieu (avant tout appel à `world_height`).
func _setup_horizon() -> void:
	horizon = BattleHorizon.new()
	horizon.name = "Horizon"
	add_child(horizon)
	var flank := str(_coast.get("flank", "")) if not _coast.is_empty() else ""
	horizon.site_key = horizon_site  # EP7
	horizon.setup(province_id, field_size(), _mean_height, flank, terrain_key, season_key)


## EP2 : emprise de la mer du champ (B5, `BattleSiteFeatures._build_sea`), vide sans côte.
func _sea_rect() -> Rect2:
	if _coast.is_empty():
		return Rect2()
	var west := str(_coast.get("flank", "west")) == "west"
	var shore := float(_coast.get("shore_x", 0.0))
	var inland := shore + (40.0 if west else -40.0)
	var x_far := shore - 9000.0 if west else shore + 9000.0
	return Rect2(minf(inland, x_far), 400.0 - 8500.0, absf(x_far - inland), 17000.0)


## B5 : au large (sous le niveau de la mer, flanc côtier seulement).
func _in_sea(x: float, z: float) -> bool:
	if _coast.is_empty():
		return horizon != null and horizon.is_sea(x, z)  # EP2 : mer réelle au loin
	var west := str(_coast["flank"]) == "west"
	if (west and x > 0.0) or (not west and x < FIELD_W):
		return false
	return world_height(x, z) < BattleSiteFeatures.SEA_LEVEL + 0.8


