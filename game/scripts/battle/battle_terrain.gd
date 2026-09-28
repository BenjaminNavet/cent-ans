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
## cour du village, mares et plage cuits dans la splatmap ; maisons, clôtures, mares, roseaux et mer
## dans `BattleVillage`. Options après `--` : `--terrain=<plains|hills|mountains|forest|marsh|heath|
## bocage>`, `--season=<spring|summer|autumn|winter>`, `--village` / `--no-village`, `--coast`,
## `--ground=<dry|muddy|snowy>` (rendu seulement, comme `--weather=`)
## (réécrivent la mise en place avant la simulation, captures) ; `--no-site` coupe le rendu B5
## (comparaisons de performance A/B).
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
const SPLAT_TEXEL := 4.0
const HEIGHT_TEXEL := 10.0
var NEAR_RECT := Rect2(-900, -900, 3000, 2600)
const NEAR_STEP := 20.0
var FAR_RECT := Rect2(-7000, -7000, 15200, 14800)
const FAR_STEP := 200.0
const RIVER_CARVE := 1.6
## Arbres : taille des tuiles (m), distance de passage au maillage allégé, portée des buissons.
const TREE_TILE := 160.0
const TREE_LOD_DISTANCE := 260.0
const BUSH_DISTANCE := 520.0
const RIVER_SPAN := 27.0  # demi-largeur creusée (width * 1.5), comme la simulation
## B5 : creusement visuel des mares (m) et portée des haies.
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

## GA2 : identité des couches du sol (Poly Haven, ordre d'empilement, taille de répétition),
## jamais codée en dur ici (`data/fx/battle_ground_layers.json`, schéma
## `fx_battle_ground_layers.schema.json`) ; même repli que `BattleGore.settings()`.
const GROUND_LAYERS_FILE := "fx/battle_ground_layers.json"
## Taille fixe du tableau `layer_tile_size` du shader (couches réelles ≤ cette taille).
const MAX_GROUND_LAYERS := 16
static var _ground_layers: Array = []
static var _ground_layers_loaded: bool = false
static var _ground_role_index: Dictionary = {}


## GA2 : couches du sol depuis les données (dossier de données du jeu, puis `data/` du dépôt).
static func ground_layers() -> Array:
	if _ground_layers_loaded:
		return _ground_layers
	_ground_layers_loaded = true
	var candidates: Array[String] = []
	var tree := Engine.get_main_loop() as SceneTree
	var paths: Node = tree.root.get_node_or_null("/root/MapPaths") if tree != null else null
	if paths != null:
		candidates.append(str(paths.get("data_dir")))
	candidates.append(ProjectSettings.globalize_path("res://").path_join("../data").simplify_path())
	for dir in candidates:
		var path := dir.path_join(GROUND_LAYERS_FILE)
		if FileAccess.file_exists(path):
			var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
			if parsed is Dictionary and (parsed as Dictionary).get("layers") is Array:
				_ground_layers = (parsed as Dictionary)["layers"]
				for i in _ground_layers.size():
					var role := str((_ground_layers[i] as Dictionary).get("role", ""))
					if role != "":
						_ground_role_index[role] = i
				return _ground_layers
	push_warning("BattleTerrain: %s introuvable, sol replié sur les couches historiques" % GROUND_LAYERS_FILE)
	return _ground_layers


## GA2 : index (dans le `Texture2DArray`) de la couche portant ce rôle, -1 si absente des données.
static func ground_role_index(role: String) -> int:
	ground_layers()
	return int(_ground_role_index.get(role, -1))


const GROUND_SHADER := preload("res://shaders/battle_ground.gdshader")
const WATER_SHADER := preload("res://shaders/battle_water.gdshader")
const ALBEDO_ARRAY := preload("res://assets/textures/battle/ground_albedo_array.jpg")
const NORMAL_ARRAY := preload("res://assets/textures/battle/ground_normal_array.jpg")
## PO4 : détail proche du sol (Poly Haven CC0, `near_detail/SOURCE.md`), avec DA6.
const NEAR_DETAIL_ALBEDO := preload("res://assets/textures/battle/near_detail/grass_path_2_diff_2k.jpg")
const NEAR_DETAIL_NORMAL := preload("res://assets/textures/battle/near_detail/grass_path_2_nor_gl_2k.jpg")

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
var site_render: bool = true
var village_view: BattleVillage
## EP6 : décor du champ (hameaux, moulins, église, manoir, vignes, camps) et parcelles peintes au
## sol (r = nature/8 : 1 labour, 2 blé, 3 pré, 4 semis, 5 vigne, 6 chaume ; g = lacet/π ; b = bord ;
## a = parcelle), même rectangle que les splatmaps.
var decor_view: BattleDecor
var decor_fields: ImageTexture
var decor_on := false
## EP6 : `--no-ep6-decor` coupe le rendu du décor (banc A/B ; les règles du cœur restent).
var decor_render := true
## DA6 (bible DA § 6) : végétation de bataille — lisières douces des cultures, touffes d'herbe en
## volume, feuillus ramifiés par essence avec imposteurs au loin, détail du sol de près.
## `--no-da6` rend l'ancienne végétation (banc A/B, captures « avant »).
var da6 := true
## GA2 : couches supplémentaires du sol (prairie fleurie, herbe piétinée, chaume, labour frais)
## et macro-variation de teinte/luminance (50–200 m). `--no-ga2` coupe la macro-variation
## (comparaisons A/B ; les couches restent en place, sans coût de rendu notable si non utilisées).
var ga2 := true
var tree_view: BattleTrees = null
## DA6 : banc A/B dans un seul processus (`--bench-ab=da6,no-da6`) : les deux végétations sont
## construites, `set_da6_view` bascule de l'une à l'autre.
var da6_ab := false
var _old_trees: Node3D = null
var _old_vegetation: BattleVegetation = null
var _tree_parent: Node = null


## DA6 (bible § 3.3) : part de saturation gardée par le sol et l'herbe ; la saison déplace la teinte,
## pas la saturation. DA7b : valeur par saison dans `data/fx/atmosphere.json` (`battle_seasons`).
func decor_saturation() -> float:
	return AtmosphereLibrary.battle_decor_saturation(season_key if site_render else "summer")
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
## PB3e : empreintes tamponnées en Rust (`StampMap`) ; `--no-pb3e` : boucles GDScript d'avant.
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
## saison, village, côte) avant la simulation : captures des variantes sans campagne dédiée.
static func apply_site_overrides(setup: Dictionary) -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--terrain="):
			setup["terrain"] = arg.trim_prefix("--terrain=")
			setup["river"] = false
		elif arg == "--river":
			setup["river"] = true
		elif arg.begins_with("--season="):
			setup["season"] = arg.trim_prefix("--season=")
		elif arg == "--village":
			setup["village"] = true
		elif arg == "--no-village":
			setup["village"] = false
		elif arg == "--coast":
			setup["coastal"] = true
		elif arg.begins_with("--province="):
			# EP6 : paysage d'une autre province (vignoble, bocage…), captures et essais.
			setup["province"] = arg.trim_prefix("--province=")
		elif arg.begins_with("--decor-plan="):
			# EP6 : plan de décor posé à la main (schéma `battle_decor_plan`), essais et captures.
			var text := FileAccess.get_file_as_string(arg.trim_prefix("--decor-plan="))
			var plan: Variant = JSON.parse_string(text)
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
	weather_key = weather
	terrain_key = str(terrain.get("terrain", "plains"))
	season_key = str(terrain.get("season", "summer"))
	ground_key = str(terrain.get("ground", "dry"))
	woodland = float(terrain.get("woodland", 0.5))
	site_render = not OS.get_cmdline_user_args().has("--no-site")
	da6 = not OS.get_cmdline_user_args().has("--no-da6")
	ga2 = not OS.get_cmdline_user_args().has("--no-ga2")
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--bench-ab=") and arg.contains("da6"):
			da6_ab = true
			da6 = true
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--ground="):
			ground_key = arg.trim_prefix("--ground=")  # rendu seulement, comme --weather=
	biome = BIOMES.get(terrain_key, BIOMES["plains"]) if site_render else BIOMES["plains"]
	_coast = terrain.get("coast", {}) if site_render else {}
	_pools = terrain.get("pools", []) if site_render else []
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
	_prepare_decor()
	_extend_river()
	_setup_horizon()
	_plan_roads()
	_build_textures()
	_build_material(weather)
	_setup_trample()
	_add_mesh("Ground", _field_mesh(), true)
	_add_mesh("NearRing", _ring_mesh(NEAR_RECT, NEAR_STEP, Rect2(0, 0, FIELD_W, FIELD_D), 0.0), true)
	_add_mesh("FarRing", _ring_mesh(FAR_RECT, FAR_STEP, NEAR_RECT.grow(-2.0 * FAR_STEP), 1.5), false)
	if horizon.active:
		horizon.build_visuals(self, FAR_RECT, weather, _sea_rect())
	if terrain.has("river"):
		_build_river(terrain["river"])
		_build_ford_stones(terrain["river"])
	_build_streams()
	bridges_view = BattleBridges.new()
	bridges_view.name = "Bridges"
	add_child(bridges_view)
	bridges_view.build(self, terrain.get("bridges", []), river_flow, snowy())
	_build_trees()
	_build_rocks()
	vegetation = BattleVegetation.new()
	vegetation.name = "Vegetation"
	add_child(vegetation)
	vegetation.build(self, weather)
	if da6_ab:
		_old_vegetation = BattleVegetation.new()
		_old_vegetation.name = "VegetationNoDa6"
		_old_vegetation.visible = false
		add_child(_old_vegetation)
		_old_vegetation.build(self, weather, false)
	if site_render:
		village_view = BattleVillage.new()
		village_view.name = "Site"
		add_child(village_view)
		village_view.build(self, terrain, weather)
		if decor_render:
			decor_view = BattleDecor.new()
			decor_view.name = "Decor"
			add_child(decor_view)
			decor_view.build(self, terrain.get("decor", {}), weather)
		var village: Dictionary = terrain.get("village", {})
		print("BattleTerrain: site %s, %s, ground %s, village %s, coast %s, %d pools, %d obstacles, %d reeds" % [
			terrain_key, season_key, ground_key,
			"(%.0f, %.0f) r %.0f, %d houses" % [float(village["x"]), float(village["z"]), float(village["radius"]), village_view.house_count] if not village.is_empty() else "none",
			str(_coast.get("flank", "none")), _pools.size(), (terrain.get("obstacles", []) as Array).size(), village_view.reed_count])
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
	return weather_key == "snow" or (site_render and ground_key == "snowy")


## B8 : le sol est-il détrempé (pluie ou sol de saison boueux), pour le piétinement en boue ?
func muddy() -> bool:
	return weather_key == "rain" or (site_render and ground_key == "muddy")


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
	_trample_map = null
	if ClassDB.class_exists(&"StampMap") and not OS.get_cmdline_user_args().has("--no-pb3e"):
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
	var w := trample_image.get_width()
	var h := trample_image.get_height()
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
		if _trample_map != null:
			_trample_map.call("stamp_box", pos, facing, half, add, 0, 255)
			continue
		var axis_x := Vector2(cos(facing), -sin(facing))
		var axis_z := Vector2(sin(facing), cos(facing))
		var reach := half.length()
		var c := (pos - SPLAT_RECT.position) / TRAMPLE_TEXEL
		var r := reach / TRAMPLE_TEXEL
		for iz in range(maxi(int(c.y - r), 0), mini(int(c.y + r) + 1, h)):
			for ix in range(maxi(int(c.x - r), 0), mini(int(c.x + r) + 1, w)):
				var d := SPLAT_RECT.position + (Vector2(ix, iz) + Vector2(0.5, 0.5)) * TRAMPLE_TEXEL - pos
				if absf(d.dot(axis_x)) > half.x or absf(d.dot(axis_z)) > half.y:
					continue
				var i := iz * w + ix
				_trample_bytes[i] = mini(_trample_bytes[i] + add, 255)
	if _trample_map != null:
		_trample_map.call("upload", trample_image, _trample_texture)
		return
	trample_image.set_data(w, h, false, Image.FORMAT_L8, _trample_bytes)
	_trample_texture.update(trample_image)


## B7 : piétinement (0-1) à un point du monde (tests, captures).
func trample_at(x: float, z: float) -> float:
	if trample_image == null:
		return 0.0
	if _trample_map != null:
		return float(_trample_map.call("sample", x, z, 0))
	var c := ((Vector2(x, z) - SPLAT_RECT.position) / TRAMPLE_TEXEL).floor()
	if c.x < 0 or c.y < 0 or c.x >= trample_image.get_width() or c.y >= trample_image.get_height():
		return 0.0
	return float(_trample_bytes[int(c.y) * trample_image.get_width() + int(c.x)]) / 255.0


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


## Creusement visuel des mares en (x, z) (0 hors des mares).
func _pool_carve(x: float, z: float) -> float:
	var carve := 0.0
	for pool in _pools:
		var d := Vector2(x - float(pool["x"]), z - float(pool["z"])).length()
		var r := float(pool["radius"])
		if d < r * 1.1:
			carve = maxf(carve, POOL_CARVE * (1.0 - smoothstep(r * 0.55, r * 1.1, d)))
	return carve


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
	if _nx < 2:
		return 0.0
	var fx := clampf(x / _resolution, 0.0, float(_nx - 1))
	var fz := clampf(z / _resolution, 0.0, float(_nz - 1))
	var ix := int(floor(fx))
	var iz := int(floor(fz))
	var ix1 := mini(ix + 1, _nx - 1)
	var iz1 := mini(iz + 1, _nz - 1)
	var tx := fx - ix
	var tz := fz - iz
	var top := _heights[iz * _nx + ix] * (1.0 - tx) + _heights[iz * _nx + ix1] * tx
	var bottom := _heights[iz1 * _nx + ix] * (1.0 - tx) + _heights[iz1 * _nx + ix1] * tx
	return top * (1.0 - tz) + bottom * tz


## Hauteur décorative partout : le champ dedans, collines et vallée de la rivière dehors.
func world_height(x: float, z: float) -> float:
	var cx := clampf(x, 0.0, FIELD_W)
	var cz := clampf(z, 0.0, FIELD_D)
	var base := height_at(cx, cz)
	var d := Vector2(x - cx, z - cz).length()
	if d <= 0.0:
		return base
	var t := smoothstep(0.0, 450.0, d)
	var n := _hills.get_noise_2d(x, z) * 0.5 + 0.5
	var ridge_share := float(biome.get("ridges", 0.0))
	if ridge_share > 0.0:
		# R2 : crêtes et croupes de l'horizon (collines, montagnes), comme le champ.
		n = lerpf(n, clampf(_ridges.get_noise_2d(x, z) * 0.6 + 0.35, 0.0, 1.0), ridge_share)
	var hills := pow(n, 1.6) * lerpf(22.0, 160.0, smoothstep(300.0, 4500.0, d)) * float(biome["relief"])
	# R2 : ondulations moyennes qui prolongent le relief du champ (pas de plateau lisse au bord).
	hills += _rolls.get_noise_2d(x, z) * float(biome.get("rolls", 3.0)) * smoothstep(0.0, 250.0, d)
	var rd := river_distance(x, z)
	if rd < INF:
		hills *= smoothstep(25.0, 320.0, rd)
	var h := lerpf(base, _mean_height + hills, t)
	var span := river_span_at(x)
	if rd < span:
		# Le lit : même profil que la simulation, raccordé au bord du champ.
		h -= RIVER_CARVE * (1.0 - rd / span) * t
	if horizon != null and horizon.active:
		# EP2 : relief réel au loin, raccordé au relief généré (rivière et côte gardent la main).
		h = horizon.blend(x, z, h, rd)
	if not _coast.is_empty():
		# B5 : au-delà du bord côtier, le sol plonge sous la mer (plage puis estran).
		var west := str(_coast["flank"]) == "west"
		var beyond := -x if west else x - FIELD_W
		if beyond > 0.0:
			var offset := absf(float(_coast["shore_x"]) - (0.0 if west else FIELD_W))
			h = lerpf(base, BattleVillage.SEA_LEVEL - 4.0, smoothstep(0.0, offset * 2.5 + 40.0, beyond))
	return h


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
	if _river_points.size() < 2:
		return INF
	var best := INF
	var first_x := _river_points[0].x
	var center := int((x - first_x) / 10.0)
	for i in range(maxi(center - 4, 0), mini(center + 4, _river_points.size() - 1)):
		var p := Geometry2D.get_closest_point_to_segment(Vector2(x, z), _river_points[i], _river_points[i + 1])
		best = minf(best, p.distance_to(Vector2(x, z)))
	return best


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
	var f := _river_index(x)
	if f < 0.0 or _river_widths.is_empty():
		return float(terrain.get("river", {}).get("width", 0.0))
	var i := mini(int(f), _river_widths.size() - 2)
	return lerpf(_river_widths[i], _river_widths[i + 1], f - i)


## EP3 : demi-largeur creusée du lit (1,5 × la largeur, comme la simulation).
func river_span_at(x: float) -> float:
	var w := river_width_at(x)
	return w * 1.5 if w > 0.0 else RIVER_SPAN


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
	var span := points[points.size() - 1].x - points[0].x
	var reach := 2600.0
	var x := points[0].x - reach
	while x <= points[points.size() - 1].x + reach:
		var m := fposmod(x - points[0].x, 2.0 * span)
		if m > span:
			m = 2.0 * span - m
		var f := m / span * float(points.size() - 1)
		var i := mini(int(f), points.size() - 2)
		_river_points.append(Vector2(x, lerpf(points[i].y, points[i + 1].y, f - i)))
		if widths.size() == points.size():
			_river_widths.append(lerpf(widths[i], widths[i + 1], f - i))
		else:
			_river_widths.append(float(terrain["river"]["width"]))
		x += 10.0


## EP3 : routes de la simulation (ponts, gués, village, bords du champ), prolongées hors du
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


# --- Textures cuites ----------------------------------------------------------------------


func _build_textures() -> void:
	var sw := int(SPLAT_RECT.size.x / SPLAT_TEXEL)
	var sh := int(SPLAT_RECT.size.y / SPLAT_TEXEL)
	var a := Image.create_empty(sw, sh, false, Image.FORMAT_RGBA8)
	var b := Image.create_empty(sw, sh, false, Image.FORMAT_RGBA8)
	a.fill(Color(0, 0, 0, 0))
	b.fill(Color(0, 0, 0, 0))
	# Sous-bois et boue : disques des zones de la simulation (bord adouci).
	for zone in terrain.get("forests", []):
		_stamp_disc(a, Vector2(float(zone["x"]), float(zone["z"])), float(zone["radius"]) + 5.0, 0, 12.0)
	for zone in terrain.get("mud", []):
		_stamp_disc(a, Vector2(float(zone["x"]), float(zone["z"])), float(zone["radius"]), 2, 14.0, 0.5 if site_render and terrain_key == "marsh" else 1.0)
	if site_render:
		_stamp_site(a, b)
		_stamp_decor(a, b)
	# Rivière : galets dans le lit et sur les gués, berges humides.
	if _river_points.size() >= 2:
		var banks: Array = terrain["river"].get("banks", [])
		for i in range(_river_points.size() - 1):
			var p := _river_points[i]
			if not SPLAT_RECT.grow(40.0).has_point(p):
				continue
			var width := _river_widths[i] if i < _river_widths.size() else float(terrain["river"]["width"])
			var ford := _in_ford(p.x)
			# EP3 : gués larges et caillouteux, galets plus serrés.
			_stamp_disc(a, p, width * (1.25 if ford else 0.98), 3, 5.0)
			_stamp_disc(b, p, width * 1.25, 0, 8.0)
			# EP3 : berges marécageuses (boue, herbe humide) ou escarpées (terre nue au bord).
			for bank in banks:
				if p.x < float(bank["x0"]) or p.x > float(bank["x1"]) or ford:
					continue
				var side := 1.0 if bool(bank["north"]) else -1.0
				var edge := p + Vector2(0.0, side * (width * 0.5 + 6.0))
				if str(bank["kind"]) == "marsh":
					_stamp_disc(a, edge, 12.0, 2, 8.0, 0.8)
					_stamp_disc(b, edge, 20.0, 0, 10.0)
				else:
					_stamp_disc(a, p + Vector2(0.0, side * (width * 0.5 + 2.0)), 3.5, 3, 2.0, 0.7)
	# EP3 : ruisseaux (galets du lit, berges humides).
	for stream in terrain.get("streams", []):
		var pts: PackedVector2Array = stream["points"]
		var w := float(stream["width"])
		for i in range(pts.size() - 1):
			var steps := maxi(int(pts[i].distance_to(pts[i + 1]) / 2.0), 1)
			for s in steps:
				var p := pts[i].lerp(pts[i + 1], float(s) / float(steps))
				_stamp_disc(a, p, w * 0.55 + 0.5, 3, 1.5, 0.8)
				_stamp_disc(b, p, w * 1.3 + 3.0, 0, 4.0)
	for r in roads.size():
		var road := roads[r]
		var half := (road_widths[r] if r < road_widths.size() else 4.0) * 0.5
		for i in range(road.size() - 1):
			var p0 := road[i]
			var p1 := road[i + 1]
			var steps := maxi(int(p0.distance_to(p1) / 2.0), 1)
			for s in steps:
				var p := p0.lerp(p1, float(s) / float(steps))
				if SPLAT_RECT.grow(10.0).has_point(p):
					_stamp_disc(a, p, half + 0.7, 1, 2.0)
					_stamp_disc(b, p, half + 6.5, 1, 6.0)
					# EP3 : ornières boueuses sur les routes de terre détrempées.
					if muddy():
						_stamp_disc(a, p, half * 0.5, 2, 1.5, 0.45)
	if terrain.has("siege"):
		var siege: Dictionary = terrain["siege"]
		var center: Vector2 = siege.get("center", Vector2(600, 560))
		# Dans les murs : terre battue ; autour : pas de parcelles.
		_stamp_disc(a, center, 150.0, 1, 30.0, 0.55)
		_stamp_disc(b, center, 260.0, 1, 60.0)
	splat_a = ImageTexture.create_from_image(a)
	splat_b = ImageTexture.create_from_image(b)
	# Carte de hauteurs (herbe) : le champ à 10 m, prolongé par `world_height`.
	var hw := int(SPLAT_RECT.size.x / HEIGHT_TEXEL) + 1
	var hh := int(SPLAT_RECT.size.y / HEIGHT_TEXEL) + 1
	var hdata := PackedFloat32Array()
	hdata.resize(hw * hh)
	for iz in hh:
		var z := SPLAT_RECT.position.y + iz * HEIGHT_TEXEL
		for ix in hw:
			var x := SPLAT_RECT.position.x + ix * HEIGHT_TEXEL
			if x >= 0.0 and x <= FIELD_W and z >= 0.0 and z <= FIELD_D:
				hdata[iz * hw + ix] = height_at(x, z)
			else:
				hdata[iz * hw + ix] = world_height(x, z)
	var himage := Image.create_from_data(hw, hh, false, Image.FORMAT_RF, hdata.to_byte_array())
	height_texture = ImageTexture.create_from_image(himage)
	relief_texture = _relief_map(hdata, hw, hh)
	var noise := FastNoiseLite.new()
	noise.seed = 7
	noise.noise_type = FastNoiseLite.TYPE_SIMPLEX_SMOOTH
	noise.frequency = 1.0 / 64.0
	noise.fractal_octaves = 4
	var noise_image := noise.get_seamless_image(512, 512)
	noise_image.generate_mipmaps()
	macro_noise = ImageTexture.create_from_image(noise_image)


## R2 : carte de relief du sol, cuite sur la grille de 10 m des hauteurs (rendu seulement) :
## r = pente (0,5 = 50 %), g = creux (> 0,5) ou bosse (< 0,5) à l'échelle de 30 m (talus, crêtes
## fines), b = idem à 90 m (vallons, croupes). Le shader de sol en tire roche affleurante, terre
## et cailloux des crêtes, herbe grasse et humidité des creux, ombre des fonds.
func _relief_map(hdata: PackedFloat32Array, hw: int, hh: int) -> ImageTexture:
	var bytes := PackedByteArray()
	bytes.resize(hw * hh * 4)
	var at := func(ix: int, iz: int) -> float:
		return hdata[clampi(iz, 0, hh - 1) * hw + clampi(ix, 0, hw - 1)]
	for iz in hh:
		for ix in hw:
			var h: float = hdata[iz * hw + ix]
			var gx: float = (at.call(ix + 1, iz) - at.call(ix - 1, iz)) / (2.0 * HEIGHT_TEXEL)
			var gz: float = (at.call(ix, iz + 1) - at.call(ix, iz - 1)) / (2.0 * HEIGHT_TEXEL)
			var near: float = (at.call(ix + 3, iz) + at.call(ix - 3, iz) + at.call(ix, iz + 3) + at.call(ix, iz - 3)) * 0.25 - h
			var far: float = (at.call(ix + 9, iz) + at.call(ix - 9, iz) + at.call(ix, iz + 9) + at.call(ix, iz - 9)) * 0.25 - h
			var i := (iz * hw + ix) * 4
			bytes[i] = int(clampf(sqrt(gx * gx + gz * gz) / 0.5, 0.0, 1.0) * 255.0)
			bytes[i + 1] = int(clampf(0.5 + near / 4.0, 0.0, 1.0) * 255.0)
			bytes[i + 2] = int(clampf(0.5 + far / 10.0, 0.0, 1.0) * 255.0)
			bytes[i + 3] = 255
	var image := Image.create_from_data(hw, hh, false, Image.FORMAT_RGBA8, bytes)
	return ImageTexture.create_from_image(image)


## B5 : mares (vase et berges humides), fossés (vase), cour et ruelles du village (terre battue,
## sans parcelles), plage (sable, sans parcelles) dans les splatmaps.
func _stamp_site(a: Image, b: Image) -> void:
	for pool in _pools:
		var c := Vector2(float(pool["x"]), float(pool["z"]))
		var r := float(pool["radius"])
		_stamp_disc(a, c, r * 1.15, 2, 6.0)
		_stamp_disc(b, c, r * 1.5, 0, 8.0)
	for o in terrain.get("obstacles", []):
		var kind := str(o["kind"])
		var p0: Vector2 = o["a"]
		var p1: Vector2 = o["b"]
		var steps := maxi(int(p0.distance_to(p1) / 2.0), 1)
		for s in steps + 1:
			var p := p0.lerp(p1, float(s) / float(steps))
			if kind == "ditch":
				_stamp_disc(a, p, 1.6, 2, 2.0)
				_stamp_disc(b, p, 3.5, 0, 3.0)
			# Pas de labours sous les haies et les clôtures (lisière d'herbe).
			_stamp_disc(b, p, 4.0, 1, 4.0)
	var village: Dictionary = terrain.get("village", {})
	if not village.is_empty():
		var c := Vector2(float(village["x"]), float(village["z"]))
		var r := float(village["radius"])
		_stamp_disc(b, c, r + 30.0, 1, 20.0)
		for house in village.get("houses", []):
			var p := Vector2(float(house["x"]), float(house["z"]))
			_stamp_disc(a, p, float(house["width"]) * 0.5 + 2.5, 1, 3.0, 0.7)
			# Ruelle de terre battue jusqu'au centre du hameau.
			var steps := maxi(int(p.distance_to(c) / 2.5), 1)
			for s in steps + 1:
				_stamp_disc(a, p.lerp(c, float(s) / float(steps)), 1.8, 1, 2.0, 0.65)
	if not _coast.is_empty():
		var west := str(_coast["flank"]) == "west"
		var beach := float(_coast["beach"])
		var shore := float(_coast["shore_x"])
		var x0 := shore if west else FIELD_W - beach - 12.0
		var x1 := beach + 12.0 if west else shore
		# Au-delà de la ligne de rivage, sable mouillé jusqu'au bord de la splatmap.
		x0 = minf(x0, SPLAT_RECT.position.x) if west else x0
		x1 = maxf(x1, SPLAT_RECT.end.x) if not west else x1
		var ix0 := maxi(int((x0 - SPLAT_RECT.position.x) / SPLAT_TEXEL), 0)
		var ix1 := mini(int((x1 - SPLAT_RECT.position.x) / SPLAT_TEXEL), a.get_width() - 1)
		for iz in a.get_height():
			for ix in range(ix0, ix1 + 1):
				var x := SPLAT_RECT.position.x + ix * SPLAT_TEXEL
				var inland := (x - shore) if west else (shore - x)
				var edge := beach - (x if west else FIELD_W - x)
				var v := clampf(edge / 12.0 + 0.5, 0.0, 1.0)
				var ca := a.get_pixel(ix, iz)
				ca.g = maxf(ca.g, v)
				a.set_pixel(ix, iz, ca)
				var cb := b.get_pixel(ix, iz)
				cb.g = maxf(cb.g, v)
				if inland < 6.0:
					cb.r = maxf(cb.r, clampf(1.0 - inland / 6.0, 0.0, 1.0) * 0.8)
				b.set_pixel(ix, iz, cb)


## Disque adouci dans le canal `channel` de `image` (coordonnées monde) ; garde le maximum.
func _stamp_disc(image: Image, center: Vector2, radius: float, channel: int, feather: float, strength: float = 1.0) -> void:
	var cx := (center.x - SPLAT_RECT.position.x) / SPLAT_TEXEL
	var cz := (center.y - SPLAT_RECT.position.y) / SPLAT_TEXEL
	var r := (radius + feather) / SPLAT_TEXEL
	var x0 := maxi(int(cx - r), 0)
	var x1 := mini(int(cx + r) + 1, image.get_width() - 1)
	var z0 := maxi(int(cz - r), 0)
	var z1 := mini(int(cz + r) + 1, image.get_height() - 1)
	for iz in range(z0, z1 + 1):
		for ix in range(x0, x1 + 1):
			var d := Vector2(ix - cx, iz - cz).length() * SPLAT_TEXEL
			if d > radius + feather:
				continue
			var v := (1.0 - smoothstep(radius - feather * 0.5, radius + feather, d)) * strength
			var c := image.get_pixel(ix, iz)
			if v > c[channel]:
				c[channel] = v
				image.set_pixel(ix, iz, c)


func _build_material(weather: String) -> void:
	ground_material = ShaderMaterial.new()
	ground_material.shader = GROUND_SHADER
	ground_material.set_shader_parameter("albedo_array", ALBEDO_ARRAY)
	ground_material.set_shader_parameter("normal_array", NORMAL_ARRAY)
	ground_material.set_shader_parameter("macro_noise", macro_noise)
	ground_material.set_shader_parameter("splat_a", splat_a)
	ground_material.set_shader_parameter("splat_b", splat_b)
	ground_material.set_shader_parameter("splat_rect", Vector4(SPLAT_RECT.position.x, SPLAT_RECT.position.y, SPLAT_RECT.size.x, SPLAT_RECT.size.y))
	var calm := Vector4(150.0, 60.0, 1050.0, 740.0)
	ground_material.set_shader_parameter("calm_rect", calm)
	ground_material.set_shader_parameter("da6_on", 1.0 if da6 else 0.0)
	ground_material.set_shader_parameter("near_detail_albedo", NEAR_DETAIL_ALBEDO)
	ground_material.set_shader_parameter("near_detail_normal", NEAR_DETAIL_NORMAL)
	ground_material.set_shader_parameter("near_detail_on", 1.0 if da6 else 0.0)
	ground_material.set_shader_parameter("decor_saturation", decor_saturation())
	# GA2 : identité des couches (nombre, taille de répétition) et index des rôles ajoutés
	# (prairie fleurie, herbe piétinée, chaume, labour frais), lus depuis les données
	# (`data/fx/battle_ground_layers.json`), jamais codés en dur dans le shader.
	var ground_layer_list := ground_layers()
	ground_material.set_shader_parameter("layer_count", ground_layer_list.size())
	var tile_sizes := PackedFloat32Array()
	tile_sizes.resize(MAX_GROUND_LAYERS)
	for i in mini(ground_layer_list.size(), MAX_GROUND_LAYERS):
		tile_sizes[i] = float((ground_layer_list[i] as Dictionary).get("tile_size_m", 6.0))
	ground_material.set_shader_parameter("layer_tile_size", tile_sizes)
	ground_material.set_shader_parameter("idx_flowering_meadow", ground_role_index("flowering_meadow"))
	ground_material.set_shader_parameter("idx_trodden_grass", ground_role_index("trodden_grass"))
	ground_material.set_shader_parameter("idx_stubble", ground_role_index("stubble"))
	ground_material.set_shader_parameter("idx_fresh_plough", ground_role_index("fresh_plough"))
	ground_material.set_shader_parameter("ga2_on", 1.0 if ga2 else 0.0)
	if decor_on:
		ground_material.set_shader_parameter("decor_fields", decor_fields)
		ground_material.set_shader_parameter("decor_on", 1.0)
	# R2 : relief de détail (rendu seulement) : carte de relief (texels centrés sur la grille de
	# 10 m des hauteurs), roche affleurante selon le terrain, force des normales de détail.
	var hw := int(SPLAT_RECT.size.x / HEIGHT_TEXEL) + 1
	var hh := int(SPLAT_RECT.size.y / HEIGHT_TEXEL) + 1
	ground_material.set_shader_parameter("relief_map", relief_texture)
	ground_material.set_shader_parameter("relief_rect", Vector4(SPLAT_RECT.position.x - HEIGHT_TEXEL * 0.5, SPLAT_RECT.position.y - HEIGHT_TEXEL * 0.5, hw * HEIGHT_TEXEL, hh * HEIGHT_TEXEL))
	ground_material.set_shader_parameter("relief_on", 1.0)
	ground_material.set_shader_parameter("outcrops", clampf((float(biome["rocks"]) - 0.7) / 1.5, 0.0, 1.0))
	ground_material.set_shader_parameter("detail_bump", lerpf(0.5, 1.0, clampf(float(biome["relief"]) / 3.4, 0.0, 1.0)))
	if site_render:
		# B5 : sol de saison (neige au sol sans chute de neige, sol détrempé sans pluie), neiges
		# des sommets en montagne, herbe d'hiver ou de plein été.
		var snow_line := float(biome["snow_line"])
		if season_key != "winter" and snow_line < 10000.0:
			snow_line *= 2.2
		if horizon != null and horizon.active:
			# EP2 : limite des neiges en altitude réelle ; le sol détaillé (neige uniforme) la prend
			# un peu plus haut que l'anneau d'horizon (neige en plaques).
			snow_line = horizon.snow_line_world() + 450.0
		ground_material.set_shader_parameter("snow_line", snow_line)
		if ground_key == "snowy" and weather != "snow":
			ground_material.set_shader_parameter("snow", 0.75)
			ground_material.set_shader_parameter("grass_tint", Color(0.85, 0.82, 0.7))
			return
		if ground_key == "muddy" and weather != "rain":
			ground_material.set_shader_parameter("wetness", 0.45)
		if weather == "clear":
			match season_key:
				"winter":
					ground_material.set_shader_parameter("grass_tint", Color(0.92, 0.9, 0.72))
					return
				"autumn":
					ground_material.set_shader_parameter("grass_tint", Color(1.0, 0.95, 0.72))
					return
				"summer":
					if terrain_key != "marsh":
						ground_material.set_shader_parameter("grass_tint", Color(0.94, 1.0, 0.78))
						return
	match weather:
		"rain":
			ground_material.set_shader_parameter("wetness", 0.75)
			ground_material.set_shader_parameter("grass_tint", Color(0.9, 0.95, 0.85))
		"snow":
			ground_material.set_shader_parameter("snow", 0.85)
			ground_material.set_shader_parameter("grass_tint", Color(0.85, 0.85, 0.78))
		"fog":
			ground_material.set_shader_parameter("wetness", 0.25)
		_:
			ground_material.set_shader_parameter("grass_tint", Color(0.9, 1.0, 0.8))


# --- Maillages du sol ---------------------------------------------------------------------


func _add_mesh(node_name: String, mesh: ArrayMesh, shadows: bool) -> MeshInstance3D:
	var instance := MeshInstance3D.new()
	instance.name = node_name
	instance.mesh = mesh
	instance.material_override = ground_material
	instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON if shadows else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(instance)
	return instance


## Grille du champ (10 m, hauteurs de la simulation) avec une jupe verticale sur le pourtour.
func _field_mesh() -> ArrayMesh:
	var vertices := PackedVector3Array()
	var normals := PackedVector3Array()
	vertices.resize(_nx * _nz)
	normals.resize(_nx * _nz)
	for iz in _nz:
		for ix in _nx:
			var i := iz * _nx + ix
			var carve := _pool_carve(ix * _resolution, iz * _resolution) if not _pools.is_empty() else 0.0
			vertices[i] = Vector3(ix * _resolution, _heights[i] - carve, iz * _resolution)
			var hl := _heights[iz * _nx + maxi(ix - 1, 0)]
			var hr := _heights[iz * _nx + mini(ix + 1, _nx - 1)]
			var hd := _heights[maxi(iz - 1, 0) * _nx + ix]
			var hu := _heights[mini(iz + 1, _nz - 1) * _nx + ix]
			normals[i] = Vector3(hl - hr, 2.0 * _resolution, hd - hu).normalized()
	var indices := PackedInt32Array()
	for iz in range(_nz - 1):
		for ix in range(_nx - 1):
			var a := iz * _nx + ix
			var b := a + 1
			var c := a + _nx
			var d := c + 1
			indices.append_array([a, b, c, b, d, c])
	# Jupe : chaque sommet du bord dupliqué 3 m plus bas.
	var border: Array[int] = []
	for ix in _nx:
		border.append(ix)
	for iz in range(1, _nz):
		border.append(iz * _nx + _nx - 1)
	for ix in range(_nx - 2, -1, -1):
		border.append((_nz - 1) * _nx + ix)
	for iz in range(_nz - 2, -1, -1):
		border.append(iz * _nx)
	var base := vertices.size()
	for k in border.size():
		vertices.append(vertices[border[k]] - Vector3(0, 3.0, 0))
		normals.append(normals[border[k]])
	for k in border.size():
		var k1 := (k + 1) % border.size()
		var top0 := border[k]
		var top1 := border[k1]
		indices.append_array([top0, base + k, top1, top1, base + k, base + k1])
	return _commit(vertices, normals, indices)


## Anneau de terrain (grille `step`) autour de `hole` (quads entièrement dedans omis).
func _ring_mesh(rect: Rect2, step: float, hole: Rect2, sink: float) -> ArrayMesh:
	var nx := int(rect.size.x / step) + 1
	var nz := int(rect.size.y / step) + 1
	var vertices := PackedVector3Array()
	var normals := PackedVector3Array()
	vertices.resize(nx * nz)
	normals.resize(nx * nz)
	var heights := PackedFloat32Array()
	heights.resize(nx * nz)
	for iz in nz:
		for ix in nx:
			var x := rect.position.x + ix * step
			var z := rect.position.y + iz * step
			var h := world_height(x, z)
			if hole.has_point(Vector2(x, z)):
				h -= sink
			heights[iz * nx + ix] = h
			vertices[iz * nx + ix] = Vector3(x, h, z)
	for iz in nz:
		for ix in nx:
			var hl := heights[iz * nx + maxi(ix - 1, 0)]
			var hr := heights[iz * nx + mini(ix + 1, nx - 1)]
			var hd := heights[maxi(iz - 1, 0) * nx + ix]
			var hu := heights[mini(iz + 1, nz - 1) * nx + ix]
			normals[iz * nx + ix] = Vector3(hl - hr, 2.0 * step, hd - hu).normalized()
	var indices := PackedInt32Array()
	for iz in range(nz - 1):
		for ix in range(nx - 1):
			var x0 := rect.position.x + ix * step
			var z0 := rect.position.y + iz * step
			if hole.encloses(Rect2(x0, z0, step, step)):
				continue
			var a := iz * nx + ix
			var b := a + 1
			var c := a + nx
			var d := c + 1
			indices.append_array([a, b, c, b, d, c])
	return _commit(vertices, normals, indices)


static func _commit(vertices: PackedVector3Array, normals: PackedVector3Array, indices: PackedInt32Array) -> ArrayMesh:
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_INDEX] = indices
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return mesh


# --- Rivière ------------------------------------------------------------------------------


## Ruban d'eau le long du lit prolongé ; niveau pris sur les berges (non creusées) de part et
## d'autre, lissé, pour que l'eau affleure les rives et reste peu profonde sur les gués.
func _build_river(river: Dictionary) -> void:
	var points := _river_points
	if points.size() < 2:
		return
	# Niveau : au-dessus du fond du lit (0,5 m d'eau, 0,2 m sur les gués), lissé le long du cours.
	var levels := PackedFloat32Array()
	levels.resize(points.size())
	var dirs: Array[Vector2] = []
	for i in points.size():
		var a := points[maxi(i - 1, 0)]
		var b := points[mini(i + 1, points.size() - 1)]
		var dir := (b - a).normalized()
		dirs.append(Vector2(-dir.y, dir.x))
		var p := points[i]
		var inside := p.x >= 0.0 and p.x <= FIELD_W
		levels[i] = world_height(p.x, p.y) + (0.2 if inside and _in_ford(p.x) else 0.5)
	for _pass in 8:
		var smoothed := levels.duplicate()
		for i in range(1, points.size() - 1):
			smoothed[i] = (levels[i - 1] + levels[i] * 2.0 + levels[i + 1]) * 0.25
		levels = smoothed
	# Largeur de chaque rive : jusqu'où le sol reste sous l'eau, plus une marge sous la berge.
	var vertices := PackedVector3Array()
	var uvs := PackedVector2Array()
	var normals := PackedVector3Array()
	var indices := PackedInt32Array()
	var along := 0.0
	var width := 0.0
	for i in points.size():
		var p := points[i]
		var n := dirs[i]
		var extent := [0.0, 0.0]
		var reach := (_river_widths[i] if i < _river_widths.size() else 18.0) * 0.5 + 8.0
		for s in 2:
			var sign := 1.0 if s == 0 else -1.0
			var d := 1.0
			while d < reach:
				var q := p + n * sign * d
				if world_height(q.x, q.y) > levels[i] + 0.08:
					break
				d += 1.0
			extent[s] = d + 2.5
		if i > 0:
			along += p.distance_to(points[i - 1])
		var left := p + n * float(extent[0])
		var right := p - n * float(extent[1])
		width = maxf(width, float(extent[0]) + float(extent[1]))
		vertices.append(Vector3(left.x, levels[i], left.y))
		vertices.append(Vector3(right.x, levels[i], right.y))
		uvs.append(Vector2(0.0, along))
		uvs.append(Vector2(1.0, along))
		normals.append(Vector3.UP)
		normals.append(Vector3.UP)
		if i > 0:
			var k := i * 2
			indices.append_array([k - 2, k - 1, k, k - 1, k + 1, k])
	var half := width * 0.5
	_river_levels = levels
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_TEX_UV] = uvs
	arrays[Mesh.ARRAY_INDEX] = indices
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	var mat := ShaderMaterial.new()
	mat.shader = WATER_SHADER
	mat.set_shader_parameter("river_width", half * 2.0)
	# EP3 : l'eau coule vers le bas du champ (UV.y croît avec x).
	mat.set_shader_parameter("flow_speed", 0.55 * river_flow)
	var waves := NoiseTexture2D.new()
	waves.seamless = true
	waves.as_normal_map = true
	waves.bump_strength = 6.0
	waves.width = 256
	waves.height = 256
	var wave_noise := FastNoiseLite.new()
	wave_noise.frequency = 0.035
	wave_noise.fractal_octaves = 3
	waves.noise = wave_noise
	mat.set_shader_parameter("wave_normal", waves)
	mat.set_shader_parameter("macro_noise", macro_noise)
	# Reflet : le ciel entre horizon et zénith du préréglage météo.
	var preset: Dictionary = BattleAtmosphere.PRESETS.get(weather_key, BattleAtmosphere.PRESETS["clear"])
	mat.set_shader_parameter("sky_color", (preset["horizon"] as Color).lerp(preset["zenith"], 0.3))
	if weather_key == "rain":
		mat.set_shader_parameter("turbidity", 0.75)
		mat.set_shader_parameter("ripple", 1.0)
	elif weather_key == "snow":
		mat.set_shader_parameter("deep_color", Color(0.05, 0.09, 0.11))
	var instance := MeshInstance3D.new()
	instance.name = "River"
	instance.mesh = mesh
	instance.material_override = mat
	instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(instance)


## EP3 : affluent et ruisseaux, rubans d'eau étroits (même shader que la rivière) ; le niveau suit
## le lit creusé par la simulation, lissé, sous les berges.
func _build_streams() -> void:
	for stream in terrain.get("streams", []):
		var raw: PackedVector2Array = stream["points"]
		if raw.size() < 2:
			continue
		var width := float(stream["width"])
		# Rééchantillonnage tous les 4 m.
		var pts := PackedVector2Array([raw[0]])
		for i in range(raw.size() - 1):
			var steps := maxi(int(raw[i].distance_to(raw[i + 1]) / 4.0), 1)
			for s in range(1, steps + 1):
				pts.append(raw[i].lerp(raw[i + 1], float(s) / float(steps)))
		_streams.append({"points": pts, "width": width, "kind": str(stream["kind"])})
		_add_stream_chunks(pts, width * 0.5)
		var levels := PackedFloat32Array()
		levels.resize(pts.size())
		for i in pts.size():
			levels[i] = height_at(pts[i].x, pts[i].y) + (0.22 if str(stream["kind"]) == "tributary" else 0.14)
		for _pass in 6:
			var smoothed := levels.duplicate()
			for i in range(1, pts.size() - 1):
				smoothed[i] = minf(levels[i], (levels[i - 1] + levels[i] * 2.0 + levels[i + 1]) * 0.25)
			levels = smoothed
		var vertices := PackedVector3Array()
		var uvs := PackedVector2Array()
		var normals := PackedVector3Array()
		var indices := PackedInt32Array()
		var along := 0.0
		for i in pts.size():
			var a := pts[maxi(i - 1, 0)]
			var b := pts[mini(i + 1, pts.size() - 1)]
			var dir := (b - a).normalized()
			var n := Vector2(-dir.y, dir.x)
			var half := width * 0.5 + 0.8
			if i > 0:
				along += pts[i].distance_to(pts[i - 1])
			var left := pts[i] + n * half
			var right := pts[i] - n * half
			vertices.append(Vector3(left.x, levels[i], left.y))
			vertices.append(Vector3(right.x, levels[i], right.y))
			uvs.append(Vector2(0.0, along))
			uvs.append(Vector2(1.0, along))
			normals.append(Vector3.UP)
			normals.append(Vector3.UP)
			if i > 0:
				var k := i * 2
				indices.append_array([k - 2, k - 1, k, k - 1, k + 1, k])
		var arrays := []
		arrays.resize(Mesh.ARRAY_MAX)
		arrays[Mesh.ARRAY_VERTEX] = vertices
		arrays[Mesh.ARRAY_NORMAL] = normals
		arrays[Mesh.ARRAY_TEX_UV] = uvs
		arrays[Mesh.ARRAY_INDEX] = indices
		var mesh := ArrayMesh.new()
		mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
		var mat := ShaderMaterial.new()
		mat.shader = WATER_SHADER
		mat.set_shader_parameter("river_width", width + 1.6)
		mat.set_shader_parameter("flow_speed", 0.8)
		mat.set_shader_parameter("clarity", 1.6)
		mat.set_shader_parameter("turbidity", 0.15)
		mat.set_shader_parameter("macro_noise", macro_noise)
		mat.set_shader_parameter("wave_normal", water_waves())
		mat.set_shader_parameter("sky_color", sky_reflection())
		var instance := MeshInstance3D.new()
		instance.name = "Stream"
		instance.mesh = mesh
		instance.material_override = mat
		instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		instance.visibility_range_end = 1400.0
		add_child(instance)


## EP3 : gués visibles : pierres et galets qui affleurent en travers du courant.
func _build_ford_stones(river: Dictionary) -> void:
	var fords: Array = river.get("fords", [])
	if fords.is_empty():
		return
	var rng := RandomNumberGenerator.new()
	rng.seed = 4711
	var transforms: Array[Transform3D] = []
	for ford in fords:
		var fx := float(ford["x"])
		var half := float(ford["half_width"])
		for _i in int(half * 5.0):
			var x := fx + rng.randf_range(-half, half)
			var w := river_width_at(x)
			var z := river_center_z(x) + rng.randf_range(-0.55, 0.55) * w
			var s := rng.randf_range(0.18, 0.55)
			var basis := Basis(Vector3.UP, rng.randf() * TAU).scaled(Vector3(s * rng.randf_range(1.0, 1.6), s * rng.randf_range(0.4, 0.7), s))
			var level := water_level_at(x)
			var y := height_at(x, z) + s * 0.1
			if level != -INF:
				y = minf(y, level + s * 0.15)
			transforms.append(Transform3D(basis, Vector3(x, y, z)))
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.mesh = BattleMeshes.rock()
	mm.instance_count = transforms.size()
	for i in transforms.size():
		mm.set_instance_transform(i, transforms[i])
	var instance := MultiMeshInstance3D.new()
	instance.name = "FordStones"
	instance.multimesh = mm
	instance.visibility_range_end = 700.0
	add_child(instance)


# --- Arbres, buissons, rochers -----------------------------------------------------------


## EP6 : distance du point à la route la plus proche (INF sans route).
func road_distance(p: Vector2) -> float:
	var best := INF
	for road in roads:
		for i in range(road.size() - 1):
			best = minf(best, Geometry2D.get_closest_point_to_segment(p, road[i], road[i + 1]).distance_to(p))
	return best


# --- EP6 : décor du champ -------------------------------------------------------------------


## Emprises du décor dont les arbres et buissons semés s'écartent : bâtiments (+ 4 m), zones
## (hameaux, cimetières, manoirs, fermes, vignes, labours, prés, vergers : ceux-ci ont leurs
## propres arbres), accessoires, camps et convois.
func _prepare_decor() -> void:
	_decor_clear.clear()
	decor_on = false
	decor_render = not OS.get_cmdline_user_args().has("--no-ep6-decor")
	var decor: Dictionary = terrain.get("decor", {})
	if decor.is_empty() or not site_render or not decor_render:
		return
	for b in decor.get("buildings", []):
		_decor_clear.append([Vector2(float(b["x"]), float(b["z"])), Vector2(float(b["length"]), float(b["width"])) * 0.5 + Vector2(4, 4), float(b["yaw"])])
	for a in decor.get("areas", []):
		_decor_clear.append([Vector2(float(a["x"]), float(a["z"])), Vector2(float(a["length"]), float(a["width"])) * 0.5 + Vector2(2, 2), float(a["yaw"])])
	for p in decor.get("props", []):
		_decor_clear.append([Vector2(float(p["x"]), float(p["z"])), Vector2(float(p["length"]), float(p["depth"])) * 0.5 + Vector2(1.5, 1.5), float(p["yaw"])])
	for camp in decor.get("camps", []):
		var a: Dictionary = camp["area"]
		_decor_clear.append([Vector2(float(a["x"]), float(a["z"])), Vector2(float(a["length"]), float(a["width"])) * 0.5 + Vector2(6, 6), float(a["yaw"])])
		for w in camp.get("convoy", []):
			_decor_clear.append([Vector2(float(w["x"]), float(w["z"])), Vector2(float(w["length"]), float(w["depth"])) * 0.5 + Vector2(2, 2), float(w["yaw"])])


func _in_decor(p: Vector2) -> bool:
	for r in _decor_clear:
		var local: Vector2 = (p - (r[0] as Vector2)).rotated(-float(r[2]))
		var half: Vector2 = r[1]
		if absf(local.x) <= half.x and absf(local.y) <= half.y:
			return true
	return false


## Parcelles du décor peintes au sol (texture `decor_fields`) ; terre battue des cours, des abords
## des maisons et du camp ; boue du fossé du manoir ; pas de parcelles procédurales sous le décor.
func _stamp_decor(a: Image, b: Image) -> void:
	var decor: Dictionary = terrain.get("decor", {})
	if decor.is_empty() or not decor_render:
		return
	var sw := a.get_width()
	var sh := a.get_height()
	var fields := Image.create_empty(sw, sh, false, Image.FORMAT_RGBA8)
	fields.fill(Color(0, 0, 0, 0))
	var codes := {"ploughed": 1, "crop": 2, "sown": 4, "stubble": 6}
	for area in decor.get("areas", []):
		var kind := str(area["kind"])
		var code := 0
		match kind:
			"ploughland":
				code = int(codes.get(str(area.get("state", "ploughed")), 1))
			"meadow", "orchard":
				code = 3
			"vineyard":
				code = 5
		var c := Vector2(float(area["x"]), float(area["z"]))
		var half := Vector2(float(area["length"]), float(area["width"])) * 0.5
		var yaw := float(area["yaw"])
		var yaw01 := fposmod(yaw, PI) / PI
		var reach := half.length()
		var ix0 := maxi(int((c.x - reach - SPLAT_RECT.position.x) / SPLAT_TEXEL), 0)
		var ix1 := mini(int((c.x + reach - SPLAT_RECT.position.x) / SPLAT_TEXEL) + 1, sw - 1)
		var iz0 := maxi(int((c.y - reach - SPLAT_RECT.position.y) / SPLAT_TEXEL), 0)
		var iz1 := mini(int((c.y + reach - SPLAT_RECT.position.y) / SPLAT_TEXEL) + 1, sh - 1)
		for iz in range(iz0, iz1 + 1):
			for ix in range(ix0, ix1 + 1):
				var w := Vector2(SPLAT_RECT.position.x + ix * SPLAT_TEXEL, SPLAT_RECT.position.y + iz * SPLAT_TEXEL)
				var local := (w - c).rotated(-yaw)
				var edge := minf(half.x - absf(local.x), half.y - absf(local.y))
				if edge < 0.0:
					continue
				# Pas de parcelles procédurales sous le décor.
				var cb := b.get_pixel(ix, iz)
				cb.g = maxf(cb.g, 1.0)
				b.set_pixel(ix, iz, cb)
				if code == 0:
					# Hameau, ferme, cimetière, manoir, camp : herbe foulée et terre par endroits.
					var ca := a.get_pixel(ix, iz)
					ca.g = maxf(ca.g, 0.25 if kind in ["hamlet", "church"] else 0.35)
					a.set_pixel(ix, iz, ca)
					continue
				# DA6 : rampe de lisière sur 8 m (fondu, bord bruité dans les shaders), 3 m sinon.
				fields.set_pixel(ix, iz, Color(float(code) / 8.0, yaw01, clampf(edge / (8.0 if da6 else 3.0), 0.0, 1.0), 1.0))
	# Cours de ferme, abords des maisons et des moulins : terre battue.
	for bld in decor.get("buildings", []):
		var p := Vector2(float(bld["x"]), float(bld["z"]))
		_stamp_disc(a, p, float(bld["width"]) * 0.5 + 3.0, 1, 3.0, 0.7)
	for camp in decor.get("camps", []):
		var area: Dictionary = camp["area"]
		var c := Vector2(float(area["x"]), float(area["z"]))
		var r := minf(float(area["length"]), float(area["width"])) * 0.5
		_stamp_disc(a, c, r, 1, r * 0.6, 0.55)
		for item in camp.get("items", []):
			if str(item["kind"]) == "campfire":
				_stamp_disc(a, Vector2(float(item["x"]), float(item["z"])), 1.6, 1, 1.5)
	for moat in decor.get("moats", []):
		var c := Vector2(float(moat["x"]), float(moat["z"]))
		var yaw := float(moat["yaw"])
		var hl := float(moat["length"]) * 0.5
		var hw := float(moat["width"]) * 0.5
		var ring := float(moat["ring"])
		var corners: Array[Vector2] = [Vector2(-hl, -hw), Vector2(hl, -hw), Vector2(hl, hw), Vector2(-hl, hw)]
		for s in 4:
			var p0: Vector2 = corners[s] * (1.0 - ring * 0.5 / maxf(minf(hl, hw), 1.0))
			var p1: Vector2 = corners[(s + 1) % 4] * (1.0 - ring * 0.5 / maxf(minf(hl, hw), 1.0))
			var steps := maxi(int(p0.distance_to(p1) / 2.0), 1)
			for k in steps + 1:
				var q := c + p0.lerp(p1, float(k) / float(steps)).rotated(yaw)
				_stamp_disc(a, q, ring * 0.6, 2, 2.0)
	decor_fields = ImageTexture.create_from_image(fields)
	decor_on = true


## Vergers : pommiers et poiriers en quinconce (6,5 m), petits houppiers ; en fleurs au printemps.
func _plant_orchards(sets: Dictionary, tints: Dictionary) -> void:
	var decor: Dictionary = terrain.get("decor", {})
	var blossom := bool(decor.get("orchard_blossom", false))
	var rng := RandomNumberGenerator.new()
	rng.seed = 6606
	for area in decor.get("areas", []):
		if str(area["kind"]) != "orchard":
			continue
		var c := Vector2(float(area["x"]), float(area["z"]))
		var yaw := float(area["yaw"])
		var hl := float(area["length"]) * 0.5 - 3.0
		var hw := float(area["width"]) * 0.5 - 3.0
		var step := 6.5
		var v := -hw
		var row := 0
		while v <= hw:
			var u := -hl + (step * 0.5 if row % 2 == 1 else 0.0)
			while u <= hl:
				var p := c + Vector2(u + rng.randf_range(-0.6, 0.6), v + rng.randf_range(-0.6, 0.6)).rotated(yaw)
				u += step
				if rng.randf() < 0.06 or _near_road(p, 3.0):
					continue  # un arbre mort arraché, ou le chemin
				# DA6 : fruitiers à part (essence « fruit », taille réelle : échelle 0,85-1,1).
				var fruit := "fruit" if da6 else "oak"
				var t := _tree_transform(rng, p.x, p.y, 0.85, 1.1) if da6 else _tree_transform(rng, p.x, p.y, 0.38, 0.52)
				t.origin.y = height_at(p.x, p.y) - 0.2
				sets[fruit].append(t)
				if blossom:
					# Fleurs blanc rosé mêlées aux jeunes feuilles : éclairci, pas blanc pur.
					var w := rng.randf_range(1.12, 1.35)
					tints[fruit].append(Color(w * 1.08, w * rng.randf_range(0.92, 1.0), w * rng.randf_range(0.82, 0.92)))
				else:
					tints[fruit].append(_tree_tint(rng) * Color(0.95, 1.05, 0.9))
			v += step
			row += 1


func _near_road(p: Vector2, margin: float) -> bool:
	for road in roads:
		for i in range(road.size() - 1):
			var a := road[i]
			var b := road[i + 1]
			if absf(a.y - p.y) > 200.0 and absf(b.y - p.y) > 200.0 and absf(a.x - p.x) > 200.0:
				continue
			if Geometry2D.get_closest_point_to_segment(p, a, b).distance_to(p) < margin:
				return true
	return false


## Place un arbre : transformée (échelle, lacet) + teinte par instance.
func _tree_transform(rng: RandomNumberGenerator, x: float, z: float, scale_min: float, scale_max: float) -> Transform3D:
	var s := rng.randf_range(scale_min, scale_max)
	var basis := Basis(Vector3.UP, rng.randf() * TAU).scaled(Vector3(s, s * rng.randf_range(0.9, 1.15), s))
	return Transform3D(basis, Vector3(x, world_height(x, z) - 0.25, z))


func _tree_tint(rng: RandomNumberGenerator) -> Color:
	# B5 : feuillage de la saison (roux d'automne, feuilles sèches et brunes des chênes l'hiver).
	var autumn_share := 0.12
	if site_render:
		match season_key:
			"autumn":
				autumn_share = 0.55
			"summer":
				autumn_share = 0.04
			"winter":
				if da6:
					# DA6 : feuillus nus (ramilles grises) ; la teinte ne module que la luminance.
					var g := rng.randf_range(0.85, 1.12)
					return Color(g, g * 0.98, g * 0.95)
				var w := rng.randf_range(0.62, 0.85)
				if snowy():
					w *= 1.15
				return Color(w * 1.95, w * 0.78, w * 0.36)
	var autumn := rng.randf() < autumn_share
	if autumn and site_render and season_key == "autumn":
		# Automne marqué : roux, ocre et or (la texture des feuilles est verte : forte modulation).
		return Color(rng.randf_range(1.5, 2.0), rng.randf_range(0.8, 1.05), rng.randf_range(0.3, 0.45))
	if autumn:
		return Color(rng.randf_range(1.05, 1.25), rng.randf_range(0.9, 1.0), rng.randf_range(0.55, 0.7))
	var v := rng.randf_range(0.78, 1.12)
	return Color(v * rng.randf_range(0.9, 1.05), v, v * rng.randf_range(0.85, 1.05))


func _build_trees() -> void:
	var rng := RandomNumberGenerator.new()
	rng.seed = 1337
	var sets := {"oak": [], "poplar": [], "bush": [], "far": [], "hedge": [], "fruit": []}
	var tints := {"oak": [], "poplar": [], "bush": [], "far": [], "hedge": [], "fruit": []}
	var siege_center := Vector2(-1e6, -1e6)
	if terrain.has("siege"):
		siege_center = terrain["siege"].get("center", Vector2(600, 560))
	var near_woods := float(biome["near_woods"])
	var far_woods := float(biome["far_woods"])
	# B5 : densité des bois selon le terrain (forêt serrée, lande clairsemée).
	var area_per_tree := lerpf(85.0, 36.0, woodland) if site_render else 55.0
	# Bois de la simulation : denses, lisière de buissons. R2 : un bois est une grappe de disques
	# qui se chevauchent (ancre, lobes, bosquets) : un arbre tiré dans un disque déjà couvert par
	# un disque précédent est écarté (densité uniforme), les buissons de lisière ne restent que
	# sur le pourtour de la réunion ; les arbres des lisières sont plus petits et plus clairsemés.
	var forests: Array = terrain.get("forests", [])
	for k in forests.size():
		var zone: Dictionary = forests[k]
		var r := float(zone["radius"])
		var count := clampi(int(PI * r * r / area_per_tree), 4, 1000)
		for _i in count:
			var angle := rng.randf() * TAU
			var dist := sqrt(rng.randf()) * r
			var x := float(zone["x"]) + cos(angle) * dist
			var z := float(zone["z"]) + sin(angle) * dist
			if _in_zones_before(forests, k, x, z):
				continue
			# EP3 : routes et ruisseaux restent dégagés dans les bois.
			if _near_road(Vector2(x, z), 4.0) or in_water(x, z):
				continue
			var edge := _zones_edge_distance(forests, x, z)
			if edge < 6.0 and rng.randf() < 0.35:
				continue
			var kind := "poplar" if rng.randf() < 0.15 else "oak"
			var grow := lerpf(0.75, 1.0, clampf(edge / 14.0, 0.0, 1.0))
			sets[kind].append(_tree_transform(rng, x, z, 0.75 * grow, 1.3 * grow))
			tints[kind].append(_tree_tint(rng))
		for _i in int(TAU * r / 6.0):
			var angle := rng.randf() * TAU
			var x := float(zone["x"]) + cos(angle) * (r + rng.randf_range(-2.0, 5.0))
			var z := float(zone["z"]) + sin(angle) * (r + rng.randf_range(-2.0, 5.0))
			if _zones_edge_distance(forests, x, z) > 1.0:
				continue
			sets["bush"].append(_tree_transform(rng, x, z, 0.6, 1.4))
			tints["bush"].append(_tree_tint(rng))
	var field := Rect2(0, 0, FIELD_W, FIELD_D)
	# Bois décoratifs de l'anneau proche, haies et arbres isolés.
	var step := 13.0
	var z := NEAR_RECT.position.y
	while z < NEAR_RECT.end.y:
		var x := NEAR_RECT.position.x
		while x < NEAR_RECT.end.x:
			var px := x + rng.randf_range(-5.0, 5.0)
			var pz := z + rng.randf_range(-5.0, 5.0)
			x += step
			var p := Vector2(px, pz)
			if field.grow(25.0).has_point(p) or p.distance_to(siege_center) < 280.0:
				continue
			var n := _woods.get_noise_2d(px, pz)
			var isolated := rng.randf() < 0.004
			if n < near_woods and not isolated:
				continue
			if _in_sea(px, pz) or river_distance(px, pz) < river_span_at(px) + 6.0 or _near_road(p, 9.0):
				continue
			if n >= near_woods and n < near_woods + 0.06 and rng.randf() < 0.6:
				sets["bush"].append(_tree_transform(rng, px, pz, 0.7, 1.5))
				tints["bush"].append(_tree_tint(rng))
				continue
			var kind := "poplar" if rng.randf() < (0.35 if isolated else 0.1) else "oak"
			sets[kind].append(_tree_transform(rng, px, pz, 0.8, 1.35))
			tints[kind].append(_tree_tint(rng))
		z += step
	# Quelques buissons épars dans le champ, hors du centre.
	for _i in int(140.0 * FIELD_W * FIELD_D / 960000.0):
		var p := Vector2(rng.randf_range(0.0, FIELD_W), rng.randf_range(0.0, FIELD_D))
		if absf(p.x - FIELD_W * 0.5) < 420.0 * field_scale_x() and absf(p.y - FIELD_D * 0.5) < FIELD_D * 0.5 - 120.0:
			continue
		if river_distance(p.x, p.y) < river_span_at(p.x) or _near_road(p, 6.0) or p.distance_to(siege_center) < 200.0 or in_water(p.x, p.y):
			continue
		if _in_site_clearing(p):
			continue
		sets["bush"].append(_tree_transform(rng, p.x, p.y, 0.6, 1.3))
		tints["bush"].append(_tree_tint(rng))
	if site_render:
		_plant_hedges(rng, sets, tints)
	# Bois lointains (anneau lointain) : arbres simplifiés, plus gros, sans ombre.
	step = 42.0
	z = FAR_RECT.position.y
	while z < FAR_RECT.end.y:
		var x := FAR_RECT.position.x
		while x < FAR_RECT.end.x:
			var px := x + rng.randf_range(-15.0, 15.0)
			var pz := z + rng.randf_range(-15.0, 15.0)
			x += step
			if NEAR_RECT.grow(-60.0).has_point(Vector2(px, pz)):
				continue
			if _woods.get_noise_2d(px * 0.6, pz * 0.6) < _far_woods_at(px, pz, far_woods) or _in_sea(px, pz):
				continue
			sets["far"].append(_tree_transform(rng, px, pz, 1.3, 2.0))
			tints["far"].append(_tree_tint(rng))
		z += step
	tree_count = 0
	if site_render and decor_render:
		_plant_orchards(sets, tints)
	for kind in sets:
		tree_count += (sets[kind] as Array).size()
	if da6:
		_plant_da6(sets, tints)
		if not da6_ab:
			return
		_old_trees = Node3D.new()
		_old_trees.name = "TreesNoDa6"
		_old_trees.visible = false
		add_child(_old_trees)
		_tree_parent = _old_trees
		# Ancienne végétation du banc A/B : fruitiers remis parmi les chênes, à l'ancienne échelle.
		for i in (sets["fruit"] as Array).size():
			var t: Transform3D = sets["fruit"][i]
			t.basis = t.basis * 0.45
			sets["oak"].append(t)
			tints["oak"].append(tints["fruit"][i])
		sets["fruit"] = []
	for kind in sets:
		_tree_layer(kind, sets[kind], tints[kind])
	_tree_parent = null


## DA6 : bascule du banc A/B (`--bench-ab=da6,no-da6`) entre la végétation DA6 et l'ancienne.
func set_da6_view(on: bool) -> void:
	if not da6_ab:
		return
	tree_view.visible = on
	_old_trees.visible = not on
	vegetation.visible = on
	_old_vegetation.visible = not on
	ground_material.set_shader_parameter("da6_on", 1.0 if on else 0.0)
	ground_material.set_shader_parameter("near_detail_on", 1.0 if on else 0.0)


## DA6 : essences des feuillus (chêne, hêtre, frêne ; saule et peuplier près de l'eau), tuiles par
## niveau de détail (choix par instance dans les shaders) et imposteurs au-delà de 300 m.
func _plant_da6(sets: Dictionary, tints: Dictionary) -> void:
	tree_view = BattleTrees.new()
	tree_view.name = "Trees"
	add_child(tree_view)
	var by_species := {}
	var impostors := {}  # tuile (640 m) -> [transforms, tints, rows]
	for kind in sets:
		var transforms: Array = sets[kind]
		for i in transforms.size():
			var t: Transform3D = transforms[i]
			var species := str(kind)
			match kind:
				"oak", "far":
					species = _broadleaf_species(t.origin, kind == "oak")
				"hedge":
					species = "bush"
			if kind == "far":
				# Anneau lointain : arbres ramenés à la taille réelle, imposteurs seuls.
				t.basis = t.basis * 0.62
			if not by_species.has(species):
				by_species[species] = [[], []]
			if kind != "far":
				by_species[species][0].append(t)
				by_species[species][1].append(tints[kind][i])
			var row := BattleTrees.impostor_row(species)
			if row >= 0:
				var key := Vector2i(floori(t.origin.x / (TREE_TILE * 4.0)), floori(t.origin.z / (TREE_TILE * 4.0)))
				if not impostors.has(key):
					impostors[key] = [[], [], []]
				impostors[key][0].append(t)
				impostors[key][1].append(tints[kind][i])
				impostors[key][2].append(row)
	var winter := site_render and season_key == "winter"
	var lod_k := RenderQuality.battle_lod_scale
	var lod1 := BattleTrees.LOD1_DISTANCE * lod_k
	var far := BattleTrees.IMPOSTOR_DISTANCE
	var slack := TREE_TILE * 0.75 + BattleTrees.LOD_BAND
	for species in by_species:
		var transforms: Array = by_species[species][0]
		var tile_tints: Array = by_species[species][1]
		if transforms.is_empty():
			continue
		var tiles := {}
		for i in transforms.size():
			var o: Vector3 = (transforms[i] as Transform3D).origin
			var key := Vector2i(floori(o.x / TREE_TILE), floori(o.z / TREE_TILE))
			if not tiles.has(key):
				tiles[key] = []
			(tiles[key] as Array).append(i)
		for key in tiles:
			var members: Array = tiles[key]
			if species == "bush":
				# Buissons et haies : maillage unique ; portée des tuiles comme avant.
				var reach := (HEDGE_DISTANCE if sets["hedge"].size() > 0 else BUSH_DISTANCE) * lod_k
				var bush_lod1 := BattleTrees.BUSH_LOD1_DISTANCE * lod_k
				_da6_tile(species, 0, winter, 0.0, bush_lod1, members, transforms, tile_tints, key, 0.0, bush_lod1 + slack, false)
				_da6_tile(species, 1, winter, bush_lod1, 100000.0, members, transforms, tile_tints, key, maxf(bush_lod1 - slack, 0.0), reach, false)
				continue
			_da6_tile(species, 0, winter, 0.0, lod1, members, transforms, tile_tints, key, 0.0, lod1 + slack, true)
			_da6_tile(species, 1, winter, lod1, far, members, transforms, tile_tints, key, maxf(lod1 - slack, 0.0), far + slack, false)
	for key in impostors:
		var entry: Array = impostors[key]
		tree_view.add_impostor_tile("Impostors_%d_%d" % [key.x, key.y], entry[0], entry[1], entry[2])
	tree_view.bake_impostors.call_deferred(winter)


func _da6_tile(species: String, lod: int, winter: bool, lod_near: float, lod_far: float, members: Array, transforms: Array, tints: Array, key: Vector2i, range_begin: float, range_end: float, shadows: bool) -> void:
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_colors = true
	mm.mesh = BattleTrees.mesh(species, lod, winter, lod_near, lod_far)
	mm.instance_count = members.size()
	for k in members.size():
		var i: int = members[k]
		mm.set_instance_transform(k, transforms[i])
		mm.set_instance_color(k, tints[i])
	var instance := MultiMeshInstance3D.new()
	instance.name = "Trees_%s_%d_%d_%d" % [species, lod, key.x, key.y]
	instance.multimesh = mm
	instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON if shadows else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	instance.visibility_range_begin = range_begin
	instance.visibility_range_end = range_end
	tree_view.add_child(instance)


## DA6 : essence d'un feuillu selon le lieu (tirage haché, stable) : saules et peupliers au bord de
## l'eau ; chênes, hêtres (forêts, collines) et frênes (bocage, fonds frais) ailleurs.
func _broadleaf_species(o: Vector3, near: bool) -> String:
	var h := fposmod(sin(o.x * 12.9898 + o.z * 78.233) * 43758.5453, 1.0)
	var wet := near and (river_distance(o.x, o.z) < river_span_at(o.x) + 28.0 or _in_zones(_pools, o.x, o.z, 20.0))
	if wet or terrain_key == "marsh":
		if h < 0.55:
			return "willow"
		if h < 0.75:
			return "poplar"
		return "ash"
	var beech := 0.35 if terrain_key in ["forest", "hills", "mountains"] else 0.2
	var ash := 0.3 if terrain_key == "bocage" else 0.2
	if h < beech:
		return "beech"
	if h < beech + ash:
		return "ash"
	return "oak"


## EP2 : seuil des bois lointains ; au loin, les forêts réelles de la tuile d'horizon.
func _far_woods_at(x: float, z: float, biome_threshold: float) -> float:
	if horizon == null or not horizon.active:
		return biome_threshold
	var w := horizon.weight(x, z)
	if w <= 0.0:
		return biome_threshold
	return lerpf(biome_threshold, lerpf(0.55, -0.9, horizon.forest_at(x, z)), w)


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


## EP2 : emprise de la mer du champ (B5, `BattleVillage._build_sea`), vide sans côte.
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
	return world_height(x, z) < BattleVillage.SEA_LEVEL + 0.8


## B5 : pas de buissons épars dans les mares, le village et sur la plage.
func _in_site_clearing(p: Vector2) -> bool:
	if not site_render:
		return false
	if _in_zones(_pools, p.x, p.y, 4.0):
		return true
	if not _decor_clear.is_empty() and _in_decor(p):
		return true
	var village: Dictionary = terrain.get("village", {})
	if not village.is_empty() and p.distance_to(Vector2(float(village["x"]), float(village["z"]))) < float(village["radius"]) + 8.0:
		return true
	if not _coast.is_empty():
		var west := str(_coast["flank"]) == "west"
		var from_edge := p.x if west else FIELD_W - p.x
		if from_edge < float(_coast["beach"]) + 5.0:
			return true
	return false


## B5 : haies vives (buissons serrés tous les 1,8 m, portée longue), chênes têtards dans les
## haies du bocage ; quelques arbres fruitiers dans les courtils du village.
func _plant_hedges(rng: RandomNumberGenerator, sets: Dictionary, tints: Dictionary) -> void:
	var bocage := terrain_key == "bocage"
	for o in terrain.get("obstacles", []):
		if str(o["kind"]) != "hedge":
			continue
		var a: Vector2 = o["a"]
		var b: Vector2 = o["b"]
		var length := a.distance_to(b)
		var steps := maxi(int(length / 1.8), 1)
		var normal := (b - a).normalized().orthogonal()
		var since_tree := rng.randf_range(0.0, 20.0)
		for k in steps + 1:
			var p := a.lerp(b, float(k) / float(steps)) + normal * rng.randf_range(-0.4, 0.4)
			var s := rng.randf_range(0.8, 1.1)
			var basis := Basis(Vector3.UP, rng.randf() * TAU).scaled(Vector3(s, s * rng.randf_range(1.1, 1.45), s))
			sets["hedge"].append(Transform3D(basis, Vector3(p.x, height_at(p.x, p.y) - 0.3, p.y)))
			tints["hedge"].append(_tree_tint(rng) * Color(0.92, 0.95, 0.9))
			since_tree += 1.8
			if since_tree > (18.0 if bocage else 40.0) and rng.randf() < 0.35:
				since_tree = 0.0
				var t := _tree_transform(rng, p.x, p.y, 0.6, 0.95)
				t.origin.y = height_at(p.x, p.y) - 0.25
				sets["oak"].append(t)
				tints["oak"].append(_tree_tint(rng))
	var village: Dictionary = terrain.get("village", {})
	if village.is_empty():
		return
	var c := Vector2(float(village["x"]), float(village["z"]))
	var r := float(village["radius"])
	for _i in int(r * 0.25):
		var ang := rng.randf() * TAU
		var p := c + Vector2(cos(ang), sin(ang)) * r * rng.randf_range(0.9, 1.25)
		var near_house := false
		for house in village.get("houses", []):
			if p.distance_to(Vector2(float(house["x"]), float(house["z"]))) < float(house["length"]) * 0.6 + 3.0:
				near_house = true
				break
		if near_house:
			continue
		var fruit := "fruit" if da6 else "oak"
		var t := _tree_transform(rng, p.x, p.y, 0.9, 1.2) if da6 else _tree_transform(rng, p.x, p.y, 0.4, 0.6)
		t.origin.y = height_at(p.x, p.y) - 0.2
		sets[fruit].append(t)
		tints[fruit].append(_tree_tint(rng))


## Arbres par tuiles (lot V4b) : une tuile de `TREE_TILE` m par MultiMesh pour que le moteur
## écarte ce qui est hors champ ou hors des ombres. Chênes et peupliers ont deux niveaux de
## détail par distance (`visibility_range`) : houppier complet avec ombre portée près, maillage
## allégé sans ombre au-delà de `TREE_LOD_DISTANCE`.
func _tree_layer(kind: String, transforms: Array, tints: Array) -> void:
	if transforms.is_empty():
		return
	var tile := TREE_TILE * (4.0 if kind == "far" else 1.0)
	var tiles := {}
	for i in transforms.size():
		var t: Transform3D = transforms[i]
		var key := Vector2i(floori(t.origin.x / tile), floori(t.origin.z / tile))
		if not tiles.has(key):
			tiles[key] = []
		(tiles[key] as Array).append(i)
	# PF1 : distances de LOD des arbres et buissons selon le préréglage de qualité.
	var lod_k := RenderQuality.battle_lod_scale
	for key in tiles:
		var members: Array = tiles[key]
		match kind:
			"oak", "poplar":
				_tree_tile(kind, members, transforms, tints, key, 0.0, TREE_LOD_DISTANCE * lod_k, true)
				_tree_tile(kind + "_lod", members, transforms, tints, key, TREE_LOD_DISTANCE * lod_k, 0.0, false)
			"bush":
				_tree_tile(kind, members, transforms, tints, key, 0.0, BUSH_DISTANCE * lod_k, false)
			"hedge":
				_tree_tile(kind, members, transforms, tints, key, 0.0, HEDGE_DISTANCE * lod_k, false)
			_:
				_tree_tile(kind, members, transforms, tints, key, 0.0, 0.0, false)


func _tree_tile(kind: String, members: Array, transforms: Array, tints: Array, key: Vector2i, range_begin: float, range_end: float, shadows: bool) -> void:
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_colors = true
	mm.mesh = BattleMeshes.tree("bush" if kind == "hedge" else kind)
	mm.instance_count = members.size()
	for k in members.size():
		var i: int = members[k]
		mm.set_instance_transform(k, transforms[i])
		mm.set_instance_color(k, tints[i])
	var instance := MultiMeshInstance3D.new()
	instance.name = "Trees_%s_%d_%d" % [kind, key.x, key.y]
	instance.multimesh = mm
	instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON if shadows else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	instance.visibility_range_begin = range_begin
	instance.visibility_range_end = range_end
	(_tree_parent if _tree_parent != null else self).add_child(instance)


## Rochers : sur les pentes raides du champ et dans les collines de l'anneau proche.
func _build_rocks() -> void:
	var rng := RandomNumberGenerator.new()
	rng.seed = 777
	var transforms: Array[Transform3D] = []
	for _i in 2600:
		var x := rng.randf_range(NEAR_RECT.position.x, NEAR_RECT.end.x)
		var z := rng.randf_range(NEAR_RECT.position.y, NEAR_RECT.end.y)
		var h := world_height(x, z)
		var slope := Vector2(world_height(x + 4.0, z) - world_height(x - 4.0, z), world_height(x, z + 4.0) - world_height(x, z - 4.0)).length() / 8.0
		var in_field := x >= 0.0 and x <= FIELD_W and z >= 0.0 and z <= FIELD_D
		var chance := (smoothstep(0.12, 0.35, slope) + (0.0 if in_field else 0.04)) * float(biome["rocks"])
		if rng.randf() > chance or river_distance(x, z) < river_span_at(x) or _in_sea(x, z) or in_water(x, z):
			continue
		if in_field and absf(x - FIELD_W * 0.5) < 380.0 * field_scale_x() and absf(z - FIELD_D * 0.5) < FIELD_D * 0.5 - 150.0:
			continue
		var s := rng.randf_range(0.5, 2.6)
		var basis := Basis(Vector3.UP, rng.randf() * TAU).rotated(Vector3.RIGHT, rng.randf_range(-0.3, 0.3)).scaled(Vector3(s * rng.randf_range(0.8, 1.5), s * rng.randf_range(0.5, 0.9), s))
		transforms.append(Transform3D(basis, Vector3(x, h - s * 0.25, z)))
	if transforms.is_empty():
		return
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.mesh = BattleMeshes.rock()
	mm.instance_count = transforms.size()
	for i in transforms.size():
		mm.set_instance_transform(i, transforms[i])
	var instance := MultiMeshInstance3D.new()
	instance.name = "Rocks"
	instance.multimesh = mm
	add_child(instance)
