class_name Vegetation
extends Node3D

## Végétation de la carte de campagne (lot V3, ADR 0004) : forêts, bosquets et haies en
## `MultiMeshInstance3D`, par tuile (même découpage 16 × 16 que `TerrainBuilder`).
##
## - Construction paresseuse : seules les tuiles proches de la caméra sont semées, dans des
##   tâches `WorkerThreadPool` (`VegetationTileJob`) ; les tuiles construites restent en cache
##   (au plus `max_cached_tiles`, les plus anciennes hors champ sont libérées).
## - Distance : deux maillages par essence (détaillé de près, ≈ 20 triangles au loin),
##   éclaircissement progressif (`foliage.gdshader`). VT3 (ADR 0138) : arbres à l'échelle 1:1,
##   plus aucun arbre au-delà de la portée où ils font ≈ 1 px (`MapPropScale.tree_view_range`,
##   `tree_max_distance`) : la canopée du terrain (`terrain.gdshader`) porte la forêt au loin.
## - Autonome : se branche seul sur la scène parente (`map_data`, `load_ok`, `terrain`) et sur le
##   rig de caméra ; `build(map_data)` peut aussi être appelé directement.
## - Lot C7b : les arbres sont posés sur la surface du maillage de terrain affiché
##   (`TerrainBuilder.surface_grid`) et recalés quand une tuile change de niveau (relief fin
##   8192², LOD proche ou lointain, signal `chunk_surface_changed`) : nouveaux tampons calculés
##   dans une tâche `VegetationGroundJob`, installés en une fois (pas d'à-coup).
## - Lot V4 (A1-10) : quatre essences (chêne, hêtre, conifère de montagne, haie ; maillages Blender
##   `campaign_trees.glb`), houppiers élargis au cœur des massifs (canopée continue), teinte
##   saisonnière par essence (`foliage.gdshaderinc`, poids globaux `campaign_season` du lot CV1 ;
##   `--season=winter` les force pour les captures). Couverture forestière : `data/map/forest_cover.json`.
## Purement visuel : aucune règle de jeu.

const FOLIAGE_SHADER := preload("res://shaders/foliage.gdshader")
const FOLIAGE_WINTER_SHADER := preload("res://shaders/foliage_winter.gdshader")
const IMPOSTOR_SHADER := preload("res://shaders/campaign_tree_impostor.gdshader")
const CARDS_SHADER := preload("res://shaders/foliage_cards.gdshader")
## Niveaux de détail d'une partie de tuile (`_apply_lod`).
enum Lod { FAR, DETAILED, NEAR }

@export var camera_rig_path: NodePath = ^"../CameraRig"
## Pas (px de carte) de la grille de candidats ; plus petit = forêts plus denses.
@export var spacing: float = 1.35
@export var tree_scale: float = 1.0
## Au-delà de cette distance caméra → point visé, plus d'arbres (VT3 : plafond, la portée réelle
## est `MapPropScale.tree_max_distance`, ≈ 30).
@export var max_camera_distance: float = 700.0
## Lot L5 : portée selon le préréglage (`veg_max_distance`, Basse 500 … Ultra 1100), appliquée
## seulement avec les imposteurs (FC2) ; sans eux, `max_camera_distance` (VT3 : plafonds).
var quality_max_distance: float = -1.0
## Tuiles (point le plus proche) à moins de cette distance de la caméra : maillage détaillé ;
## au-delà, les arbres font moins de ≈ 10 pixels et la variante ≈ 20 triangles suffit.
@export var detail_distance: float = 170.0
@export var max_concurrent_jobs: int = 5
## Tuiles semées d'un coup (et attendues) au premier affichage. `WorkerThreadPool` ne sert les
## tâches basse priorité que sur ~4 fils quel que soit le nombre de cœurs (mesuré) : demander
## plus que `max_concurrent_jobs` d'un coup ne fait qu'ajouter des salves d'attente bloquante en
## série sans plus de parallélisme réel. Les tuiles restantes arrivent ensuite normalement (même
## budget que le chargement en tâche de fond), en général en une poignée de frames.
@export var warm_start_tiles: int = 5
## Lot PB2 : semis natif (`VegetationScatter`, Rust) quand l'extension l'expose ; sinon tout le
## semis tourne en GDScript dans le `WorkerThreadPool`.
@export var use_native_scatter: bool = true
## Lot SZ4b : forêt dense autour du point visé (`ForestDetail`, semis natif requis).
@export var use_forest_detail: bool = true
@export var max_cached_tiles: int = 64
## VT3 : tuiles semées (sans être affichées) jusqu'à cette distance caméra : la portée des arbres
## 1:1 (≈ 30) est plus courte qu'une tuile, et la forêt dense a besoin des grilles de la tuile.
@export var tile_prefetch_distance: float = 120.0
@export var cast_shadows: bool = true
## Au-delà de cette distance caméra (zoom global, pas la distance d'une tuile), plus aucune
## ombre de végétation : à cette échelle les ombres portées des arbres/haies ne sont plus
## discernables individuellement mais restent payées en pleine géométrie d'ombre côté GPU (V6,
## perf ; zoom moyen d≈300-500).
@export var shadow_camera_distance: float = 300.0
## Recalages (lot C7b) simultanés au plus.
@export var max_ground_jobs: int = 2
## Lot FC2 : au-delà de `detail_distance`, chênes, hêtres et conifères en imposteurs (quadrilatère
## texturé par les atlas cuits sous Blender, 2 triangles) au lieu du maillage bas (≈ 20 triangles) ;
## haies inchangées. `--no-fc2` rend les maillages bas (comparaisons A/B).
@export var use_impostors: bool = true
## Lot FC5 : parties à moins de `near_distance` (× `veg_detail`) : chênes, hêtres et sapins en
## cartes de feuillage (`VegetationMeshes.essence_mid`, ≈ 145-250 triangles) ; entre
## `near_distance` et `detail_distance`, imposteurs (ombres gardées) au lieu des maillages
## détaillés de 90 triangles. `--no-fc5` rend les maillages détaillés.
@export var use_near_cards: bool = true
@export var near_distance: float = 45.0
## Lot FC6 : largeur du fondu tramé cartes → imposteur, par arbre, en deçà de `near_distance`.
@export var near_fade: float = 8.0
## GA3-L2 : imposteurs générés à toutes distances (cartes FC5 retirées, ≈ 250 → 2 triangles par
## arbre proche) ; faux avec `--no-ga3-veg` / `--no-ga3-near` ou sans imposteurs.
var ga3_near_impostors: bool = false

## PF1 : préréglage de qualité (`apply_render_quality`) : part des arbres, portée du détail,
## zoom maximal des ombres.
var quality_density: float = 1.0
var quality_detail: float = 1.0
var quality_shadow_distance: float = -1.0

var map_data: MapData
var mask: VegetationMask
## Lot SZ4b : forêts denses autour du point visé aux paliers vallée et site (semis natif requis).
var forest_detail: ForestDetail
## Terrain affiché (lot C7b) : null → arbres sur la heightmap 4096 bilinéaire, sans recalage.
var terrain: TerrainBuilder
## Lot C6 : cercles d'exclusion supplémentaires (colonies, hameaux) : Vector3(x, y, rayon) px carte.
var extra_exclusions: PackedVector3Array = PackedVector3Array()
var chunk_px: int = 0
## Grille des tuiles de terrain (`TerrainBuilder.chunk_grid_for`, ADR 0115).
var chunks_x: int = TerrainBuilder.LEGACY_CHUNKS
var chunks_y: int = TerrainBuilder.LEGACY_CHUNKS
var enabled: bool = true
## Statistiques : tuiles construites, instances, temps de semis (ms, somme et max).
var stats: Dictionary = {"tiles": 0, "instances": 0, "build_ms_total": 0.0, "build_ms_max": 0.0, "source": "", "regrounds": 0, "reground_ms_max": 0.0}

var _material: ShaderMaterial
## Lot FC2 : matériau des imposteurs (null : atlas absents ou `--no-fc2`) ; mêmes uniformes de
## feuillage que `_material` (`_set_foliage_param`).
var _impostor_material: ShaderMaterial
## Lot FC5 : matériau des cartes de feuillage (null : texture absente ou `--no-fc5`).
var _cards_material: ShaderMaterial
var _tiles: Dictionary = {}  # index → {"node": Node3D, "mmis": Array[MultiMeshInstance3D], "counts", "last_seen"}
var _jobs: Dictionary = {}  # index → {"task": int, "job": VegetationTileJob, "level": int[, "native": id]}
## Lot PB2 : pool natif (`VegetationScatter`) ou null ; requêtes en cours (id → index de tuile).
var _native: Object = null
var _native_ids: Dictionary = {}
var _native_ground_ids: Dictionary = {}  # recalages : id → index de tuile
var _native_serial: int = 0
var _native_floor_version: int = -1
## Lot C7b : recalages en cours (index → {"task", "job": VegetationGroundJob}) et tuiles à recaler.
var _ground_jobs: Dictionary = {}
var _ground_dirty: Dictionary = {}
var _generation: int = 0
var _exclusions := PackedVector3Array()
var _rig: Node3D
var _frame: int = 0
var _log_bursts := false
var _warm := false
## Saison du feuillage : 3 en hiver (variante ajourée), 1 sinon ; -1 : pas encore lue.
var _season: int = -1


func apply_render_quality(p: Dictionary) -> void:
	quality_density = float(p.get("veg_density", 1.0))
	quality_detail = float(p.get("veg_detail", 1.0))
	quality_shadow_distance = float(p.get("veg_shadow_distance", shadow_camera_distance))
	quality_max_distance = float(p.get("veg_max_distance", -1.0))
	for entry: Dictionary in _tiles.values():
		for part: Dictionary in entry["parts"]:
			part.erase("lod")  # LOD réévalué à la prochaine mise à jour


func _ready() -> void:
	add_to_group(RenderQuality.CLIENT_GROUP)
	apply_render_quality(RenderQuality.preset())
	_rig = get_node_or_null(camera_rig_path) as Node3D
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--screenshot") or arg == "--vegetation-stats":
			_log_bursts = true
		elif arg == "--no-native-vegetation":  # PB2 : comparaisons avec le semis GDScript
			use_native_scatter = false
		elif arg == "--no-forest-detail":  # SZ4b : captures « avant », mesures A/B
			use_forest_detail = false
		elif arg == "--no-fc2":  # FC2 : maillages bas au loin au lieu des imposteurs (A/B)
			use_impostors = false
		elif arg == "--no-fc5":  # FC5 : maillages détaillés de près au lieu des cartes (A/B)
			use_near_cards = false



func _exit_tree() -> void:
	_wait_all_jobs()


func build(data: MapData) -> void:
	clear()
	map_data = data
	_native = _make_native(data) if use_native_scatter else null
	_native_floor_version = -1
	var grid := TerrainBuilder.chunk_grid_for(data.size)
	chunk_px = grid.x
	chunks_x = grid.y
	chunks_y = grid.z
	mask = VegetationMask.new()
	mask.setup(data)
	stats["source"] = mask.source
	_material = ShaderMaterial.new()
	_material.shader = FOLIAGE_SHADER
	_impostor_material = _make_impostor_material() if use_impostors else null
	_cards_material = null
	# GA3-L2 : imposteurs générés aussi pour les arbres proches (pas de cartes de feuillage).
	ga3_near_impostors = _impostor_material != null and Ga3Vegetation.near_impostors()
	if use_near_cards and not ga3_near_impostors and ResourceLoader.exists(VegetationMeshes.CARD_TEXTURE) and VegetationMeshes.essence_mid("oak") != null:
		_cards_material = ShaderMaterial.new()
		_cards_material.shader = CARDS_SHADER
		_cards_material.set_shader_parameter("card_texture", load(Ga3Vegetation.pick(Ga3Vegetation.LEAF_CARDS, VegetationMeshes.CARD_TEXTURE)))
		if _impostor_material != null:  # FC6 : quadrilatère d'imposteur des arbres proches
			for param in ["albedo_atlas", "normal_atlas", "views", "rows"]:
				_cards_material.set_shader_parameter(param, _impostor_material.get_shader_parameter(param))
	_bind_forest_cover(data)
	_season = -1
	_exclusions.clear()
	# Sans colonies (C6), clairière autour de chaque capitale de province.
	for index in data.provinces if extra_exclusions.is_empty() else {}:
		var capital: Vector2 = data.provinces[index].get("capital_px", Vector2(-1, -1))
		if capital.x >= 0.0:
			_exclusions.append(Vector3(capital.x, capital.y, 11.0))
	_exclusions.append_array(extra_exclusions)
	if _native != null and use_forest_detail:
		forest_detail = ForestDetail.new()
		add_child(forest_detail)
		forest_detail.setup(self, terrain)


## Lot FC2 : matériau des imposteurs, null si un atlas manque (repli sur les maillages bas).
## GA3-L2 : grille générée (`Ga3Vegetation`) sauf `--no-ga3-veg`, même cadrage.
static func _make_impostor_material() -> ShaderMaterial:
	var ga3 := Ga3Vegetation.enabled() and Ga3Vegetation.has_impostors()
	var albedo_path := Ga3Vegetation.IMPOSTOR_ALBEDO if ga3 else VegetationMeshes.IMPOSTOR_ALBEDO
	var normal_path := Ga3Vegetation.IMPOSTOR_NORMAL if ga3 else VegetationMeshes.IMPOSTOR_NORMAL
	if not ResourceLoader.exists(albedo_path) or not ResourceLoader.exists(normal_path):
		return null
	var material := ShaderMaterial.new()
	material.shader = IMPOSTOR_SHADER
	material.set_shader_parameter("albedo_atlas", load(albedo_path))
	material.set_shader_parameter("normal_atlas", load(normal_path))
	material.set_shader_parameter("views", VegetationMeshes.IMPOSTOR_VIEWS)
	material.set_shader_parameter("rows", VegetationMeshes.IMPOSTOR_ROWS.size())
	return material


## Lot L5 : zoom au-delà duquel plus aucun arbre (préréglage si les imposteurs sont actifs).
## VT3 (ADR 0138) : arbres à l'échelle 1:1, plus dessinés au-delà de
## `MapPropScale.tree_max_distance` (≈ 1 px) ; les portées d'avant (`max_camera_distance`,
## `veg_max_distance` du préréglage) ne sont plus que des plafonds.
## Distance du rig au-delà de laquelle les arbres ne portent plus d'ombre : préréglage
## (`shadow_camera_distance`), et VT3 `MapPropScale.tree_shadow_distance` (arbres 1:1 de quelques
## pixels : ombres indiscernables mais payées dans chaque cascade).
func tree_shadow_limit() -> float:
	var limit := shadow_camera_distance if quality_shadow_distance < 0.0 else quality_shadow_distance
	return minf(limit, MapPropScale.shared().tree_shadow_distance)


func effective_max_distance() -> float:
	var legacy := max_camera_distance
	if quality_max_distance > 0.0 and use_impostors and (_impostor_material != null or map_data == null):
		legacy = quality_max_distance
	return minf(legacy, MapPropScale.shared().tree_max_distance)


## Lot FC2 : imposteurs actifs (matériau prêt).
func impostors_active() -> bool:
	return _impostor_material != null


func impostor_material() -> ShaderMaterial:
	return _impostor_material


## Lot FC2 (tests, bancs) : MultiMesh d'arbres (haies exclues) des parties visibles, par maillage :
## {"impostor", "low", "detailed", "impostor_triangles", "low_triangles"} (triangles des instances
## visibles).
func lod_census() -> Dictionary:
	var census := {"impostor": 0, "low": 0, "detailed": 0, "near": 0, "impostor_triangles": 0, "low_triangles": 0, "detailed_triangles": 0, "near_triangles": 0}
	for entry: Dictionary in _tiles.values():
		if not (entry["node"] as Node3D).visible:
			continue
		for part: Dictionary in entry["parts"]:
			if not (part["node"] as Node3D).visible:
				continue
			var mmis: Array = part["mmis"]
			for kind in mini(mmis.size(), VegetationTileJob.Kind.HEDGE):
				var mmi: MultiMeshInstance3D = mmis[kind]
				if mmi == null:
					continue
				var multimesh := mmi.multimesh
				var shown := multimesh.visible_instance_count if multimesh.visible_instance_count >= 0 else multimesh.instance_count
				var triangles := shown * _triangles(multimesh.mesh)
				if mmi.material_override == _impostor_material and _impostor_material != null:
					census["impostor"] += 1
					census["impostor_triangles"] += triangles
				elif mmi.material_override == _cards_material and _cards_material != null:
					census["near"] += 1
					census["near_triangles"] += triangles
				elif int(part.get("lod", Lod.FAR)) != Lod.FAR:
					census["detailed"] += 1
					census["detailed_triangles"] += triangles
				else:
					census["low"] += 1
					census["low_triangles"] += triangles
	return census


static func _triangles(mesh: Mesh) -> int:
	var total := 0
	for s in mesh.get_surface_count():
		var indices: Variant = mesh.surface_get_arrays(s)[Mesh.ARRAY_INDEX]
		total += (indices as PackedInt32Array).size() / 3 if indices != null else (mesh.surface_get_arrays(s)[Mesh.ARRAY_VERTEX] as PackedVector3Array).size() / 3
	return total


## Uniforme commune du feuillage (`foliage_common.gdshaderinc`) : maillages et imposteurs.
func _set_foliage_param(param: String, value: Variant) -> void:
	_material.set_shader_parameter(param, value)
	if _cards_material != null:
		_cards_material.set_shader_parameter(param, value)
	if _impostor_material != null:
		_impostor_material.set_shader_parameter(param, value)


## Lot PO3 : couverture forestière réduite (1024 px) pour les lisières du feuillage
## (`foliage.gdshaderinc` : arbres plus bas et clairsemés sur la rampe de la couverture).
func _bind_forest_cover(data: MapData) -> void:
	var image := mask.forest_cover_image(1024)
	_set_foliage_param("has_forest_cover", image != null)
	if image == null:
		return
	var params := mask.forest_cover_params()
	var channel := Vector4.ZERO
	channel[int(params["channel"])] = 1.0
	_set_foliage_param("forest_cover", ImageTexture.create_from_image(image))
	_set_foliage_param("cover_map_size", Vector2(data.size))
	_set_foliage_param("cover_channel", channel)
	_set_foliage_param("cover_low", float(params["low"]))
	_set_foliage_param("cover_high", float(params["high"]))


## Lot PB2 : pool natif de semis, partageant la heightmap et le lit des fleuves de `data`.
static func _make_native(data: MapData) -> Object:
	if not ClassDB.class_exists("VegetationScatter") or data.height_bytes.is_empty():
		return null
	var native: Object = ClassDB.instantiate("VegetationScatter")
	var river := PackedByteArray()
	var river_size := Vector2i.ZERO
	if data.river_bed_image != null:
		var image: Image = data.river_bed_image.duplicate()
		image.clear_mipmaps()
		if image.get_format() != Image.FORMAT_R8:
			image.convert(Image.FORMAT_R8)
		river = image.get_data()
		river_size = image.get_size()
	native.call("set_map", data.height_bytes, data.size.x, data.size.y, data.height_bpp, data.height_little_endian,
		data.height_min_m, data.height_max_m, river, river_size.x, river_size.y)
	native.call("start", clampi(OS.get_processor_count() / 2, 2, 6))
	return native


func clear() -> void:
	_wait_all_jobs()
	if forest_detail != null:
		forest_detail.clear()
		forest_detail.queue_free()
		forest_detail = null
	for entry in _tiles.values():
		(entry["node"] as Node).queue_free()
	_tiles.clear()
	_warm = false
	stats["tiles"] = 0
	stats["instances"] = 0


func tile_count() -> int:
	return _tiles.size()


func instance_count() -> int:
	return int(stats["instances"])


func pending_jobs() -> int:
	return _jobs.size() + (forest_detail.pending() if forest_detail != null else 0)


# --- Lot SZ4b : accès pour la couche de forêt dense (`ForestDetail`) -----------------------


func has_native() -> bool:
	return _native != null


func foliage_material() -> ShaderMaterial:
	return _material


## Grilles grossières d'une tuile semée (paramètres de `VegetationScatter.request` sans le semis
## lui-même), {} si la tuile n'est pas construite.
func tile_coarse(index: int) -> Dictionary:
	var entry: Dictionary = _tiles.get(index, {})
	return entry.get("coarse", {})


func exclusions_for(rect: Rect2) -> PackedVector3Array:
	return _exclusions_for(rect)


## Requête de semis native (identifiant, -1 si refusée).
func native_submit(params: Dictionary) -> int:
	if _native == null:
		return -1
	_sync_native_floor()
	_native_serial += 1
	return _native_serial if _native.call("request", _native_serial, params) else -1


## Requête de recalage native (identifiant, -1 si refusée).
func native_submit_reground(buffers: Array, grid: Dictionary, origin: Vector2) -> int:
	if _native == null:
		return -1
	_sync_native_floor()
	_native_serial += 1
	var untyped: Array = []
	untyped.assign(buffers)
	if _native.call("request_reground", _native_serial, untyped, grid, origin, MapData.vertical_scale(), MapData.relief_gain(), MapData.relief_squash()):
		return _native_serial
	return -1


## Relève les résultats natifs jusqu'à ce que la forêt dense n'attende plus rien (captures).
func poll_native_blocking() -> void:
	var guard := 0
	while forest_detail != null and forest_detail.pending() > 0 and guard < 20000:
		guard += 1
		OS.delay_usec(200)
		_poll_native()


func _process(_delta: float) -> void:
	if map_data == null:
		_try_autobind()
		if map_data == null:
			return
	var camera := get_viewport().get_camera_3d()
	if camera == null:
		return
	var distance: float = _rig.get("distance") if _rig != null else camera.global_position.y
	update_view(camera.global_position, distance)
	if forest_detail != null:
		var focus: Variant = _rig.get("focus") if _rig != null else null
		var at := Vector2(focus.x, focus.z) if focus is Vector3 else Vector2(camera.global_position.x, camera.global_position.z)
		forest_detail.update_view(at, distance, cast_shadows and distance < tree_shadow_limit())
	if _frame % 30 == 1:
		_update_season()


## Hiver (poids `campaign_season.w` du lot CV1 > 0,5) → variante ajourée du feuillage ; le reste
## de l'année, le shader sans discard garde le test de profondeur anticipé (moins de surdessin).
func _update_season() -> void:
	if _material == null:
		return
	# Poids tenus par CampaignLife.seasons (la lecture du paramètre global est réservée à l'éditeur).
	var life: Variant = get_parent().get("life") if get_parent() != null else null
	var seasons: Variant = (life as Object).get("seasons") if life is Object else null
	var value := 3 if seasons is SeasonVisuals and (seasons as SeasonVisuals).weights.w > 0.5 else 1
	if value != _season:
		_season = value
		_material.shader = FOLIAGE_WINTER_SHADER if value == 3 else FOLIAGE_SHADER


func _try_autobind() -> void:
	var parent := get_parent()
	if parent == null or parent.get("load_ok") != true:
		return
	var data: Variant = parent.get("map_data")
	if data is MapData:
		var parent_terrain: Variant = parent.get("terrain")
		if parent_terrain is TerrainBuilder:
			bind_terrain(parent_terrain)
		build(data)


## Lot C7b : pose les arbres sur la surface affichée de `terrain_builder` et les recale quand une
## tuile change de niveau. À appeler avant `build`.
func bind_terrain(terrain_builder: TerrainBuilder) -> void:
	if terrain != null and terrain.chunk_surface_changed.is_connected(_on_chunk_surface_changed):
		terrain.chunk_surface_changed.disconnect(_on_chunk_surface_changed)
	terrain = terrain_builder
	if terrain != null:
		terrain.chunk_surface_changed.connect(_on_chunk_surface_changed)


func _on_chunk_surface_changed(index: int) -> void:
	if _tiles.has(index):
		_ground_dirty[index] = true


## Tuiles en attente de recalage ou en cours (tests).
func pending_regrounds() -> int:
	return _ground_dirty.size() + _ground_jobs.size()


## Écart maximal (unités monde) entre le pied des arbres des tuiles construites et la surface
## affichée (tests) ; ne regarde qu'une instance sur `stride`.
func max_ground_error(stride: int = 7) -> float:
	var worst := 0.0
	if terrain == null:
		return worst
	for index in _tiles:
		var entry: Dictionary = _tiles[index]
		for buffer: PackedFloat32Array in entry["buffers"]:
			var k := 0
			while k < buffer.size():
				var height := Vector3(buffer[k + 1], buffer[k + 5], buffer[k + 9]).length()
				var foot := buffer[k + 7] + VegetationTileJob.GROUND_SINK * height
				worst = maxf(worst, absf(foot - terrain.surface_height_at(buffer[k + 3], buffer[k + 11])))
				k += VegetationTileJob.FLOATS_PER_INSTANCE * stride
	return worst


## Met à jour tuiles, LOD et éclaircissement pour une caméra en `camera_position`, à
## `camera_distance` de son point visé.
func update_view(camera_position: Vector3, camera_distance: float) -> void:
	_frame += 1
	_collect_jobs()
	_collect_ground_jobs()
	var max_distance := effective_max_distance()
	var active := enabled and camera_distance < max_distance
	visible = active
	if not active or _material == null:
		return
	# VT3 : arbres 1:1 dessinés jusqu'à la distance caméra où ils font ≈ 1 px
	# (`MapPropScale.tree_view_range`, éteints par graine sur `tree_view_fade`) ; au voisinage de
	# la portée du rig, la portée se referme : les arbres s'effacent (la canopée du terrain reste).
	var props := MapPropScale.shared()
	var closing := props.range_weight(camera_distance, max_distance)
	var fade_end := props.tree_view_range * quality_detail * closing
	var fade_start := fade_end * (1.0 - props.tree_view_fade)
	_set_foliage_param("view_origin", camera_position)
	_set_foliage_param("fade_start", fade_start)
	_set_foliage_param("fade_end", maxf(fade_end, fade_start + 1.0))
	# VT3 : plus d'éclaircissement au dézoom (arbres coupés à d ≈ 30) : part du préréglage seule.
	var density := quality_density
	_set_foliage_param("density", density)
	if _cards_material != null:
		# Sans imposteurs (`--no-fc2`), cartes jusqu'au bout des parties proches.
		var near_end := near_distance * quality_detail if _impostor_material != null else 1e9
		_cards_material.set_shader_parameter("near_end", near_end)
		_cards_material.set_shader_parameter("near_start", near_end - near_fade)
	var wanted: Array = []
	var camera_xz := Vector2(camera_position.x, camera_position.z)
	# Même métrique que le shader : distance horizontale + moitié de la hauteur de la caméra.
	var lift := absf(camera_position.y) * 0.5
	for cy in chunks_y:
		for cx in chunks_x:
			var index := cy * chunks_x + cx
			var d := _rect_distance(Rect2(cx * chunk_px, cy * chunk_px, chunk_px, chunk_px), camera_xz) + lift
			var in_range := d < fade_end
			if _tiles.has(index):
				var entry: Dictionary = _tiles[index]
				(entry["node"] as Node3D).visible = in_range
				if d < maxf(fade_end, tile_prefetch_distance):
					entry["last_seen"] = _frame
				if in_range:
					for part: Dictionary in entry["parts"]:
						var part_d := _rect_distance(part["rect"], camera_xz) + lift
						(part["node"] as Node3D).visible = part_d < fade_end
						if part_d < fade_end:
							_apply_lod(part, part_d, fade_start, fade_end, density, camera_distance)
			elif d < maxf(fade_end, tile_prefetch_distance) and not _jobs.has(index):
				wanted.append([d, index])
	_start_ground_jobs()
	wanted.sort_custom(func(a: Array, b: Array) -> bool: return a[0] < b[0])
	# Premier affichage : les tuiles les plus proches sont semées en parallèle et attendues
	# (pas d'apparition progressive au lancement ni dans les captures).
	var budget := max_concurrent_jobs if _warm else maxi(max_concurrent_jobs, warm_start_tiles)
	for item in wanted:
		if _jobs.size() >= budget:
			break
		_start_job(item[1])
	if not _warm and not wanted.is_empty():
		_warm = true
		var t0 := Time.get_ticks_msec()
		for item: Dictionary in _jobs.values():
			if not item.has("native"):
				WorkerThreadPool.wait_for_task_completion(item["task"])
		if _native != null:
			# Grilles prêtes : semis natif en parallèle, attendu ici.
			for index in _jobs.keys():
				if not (_jobs[index] as Dictionary).has("native"):
					_submit_native(index, _jobs[index])
			while not _native_ids.is_empty():
				OS.delay_usec(200)
				_poll_native()
		var ready_jobs := _jobs.duplicate()
		_jobs.clear()
		for index in ready_jobs:
			_install_tile(index, ready_jobs[index]["job"], ready_jobs[index]["level"])
		stats["warm_start_ms"] = Time.get_ticks_msec() - t0
		if _log_bursts:
			print("Vegetation (warm start): %s" % JSON.stringify(stats))
	_evict()


static func _rect_distance(rect: Rect2, point: Vector2) -> float:
	var nearest := Vector2(clampf(point.x, rect.position.x, rect.end.x), clampf(point.y, rect.position.y, rect.end.y))
	return nearest.distance_to(point)


## Maillage selon la distance et nombre d'instances visibles : les graines (triées) inférieures
## au seuil d'éclaircissement du point le plus proche de la tuile sont invisibles partout.
func _apply_lod(entry: Dictionary, d: float, fade_start: float, fade_end: float, density: float, camera_distance: float) -> void:
	var detailed := d < detail_distance * quality_detail
	var lod: int = Lod.FAR
	if detailed:
		lod = Lod.NEAR if _cards_material != null and d < near_distance * quality_detail else Lod.DETAILED
	var mmis: Array = entry["mmis"]
	if entry.get("lod", -1) != lod:
		entry["lod"] = lod
		var meshes := _meshes(lod)
		for kind in mmis.size():
			var mmi: MultiMeshInstance3D = mmis[kind]
			if mmi != null and mmi.multimesh.mesh != meshes[kind]:
				mmi.multimesh = _with_mesh(mmi.multimesh, meshes[kind], _tiles[entry["tile"]]["buffers"][entry["slot0"] + kind])
			if mmi != null:
				mmi.material_override = _material_for(kind, lod)
	# Ombres portées des tuiles proches seulement (au loin elles ne se voient plus) et seulement
	# au zoom global le plus rapproché (`shadow_camera_distance`) : au zoom moyen, des ombres
	# d'arbres individuelles ne se distinguent déjà plus mais coûtent toujours plein tarif côté
	# GPU. Réévalué chaque image (pas seulement au changement de LOD) : ne dépend pas de `detailed`
	# seul, mais aussi du zoom global qui peut varier sans que `detailed` change.
	var shadow_on := cast_shadows and detailed and camera_distance < tree_shadow_limit()
	var shadow_setting := GeometryInstance3D.SHADOW_CASTING_SETTING_ON if shadow_on else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	for mmi in mmis:
		if mmi != null:
			(mmi as MultiMeshInstance3D).cast_shadow = shadow_setting
	var t := clampf((d - fade_start) / maxf(fade_end - fade_start, 1.0), 0.0, 1.0)
	var fraction := clampf(minf(1.0 - t, density) + 0.02, 0.0, 1.0)
	for mmi in entry["mmis"]:
		if mmi != null:
			var multimesh: MultiMesh = (mmi as MultiMeshInstance3D).multimesh
			multimesh.visible_instance_count = ceili(multimesh.instance_count * fraction)


## SZ6 : même MultiMesh avec un autre maillage. Changer `MultiMesh.mesh` après `buffer` fait
## relire le tampon au GPU par le serveur de rendu pour recalculer la boîte englobante (image
## bloquée jusqu'à 70 ms en zoomant) : on recrée le MultiMesh depuis la copie processeur du tampon
## (`buffers` de la tuile, tenue à jour par les recalages), boîte calculée sur le processeur.
static func _with_mesh(old: MultiMesh, mesh: Mesh, buffer: PackedFloat32Array) -> MultiMesh:
	var multimesh := MultiMesh.new()
	multimesh.transform_format = old.transform_format
	multimesh.use_custom_data = old.use_custom_data
	multimesh.mesh = mesh
	multimesh.instance_count = old.instance_count
	multimesh.buffer = buffer
	multimesh.visible_instance_count = old.visible_instance_count
	return multimesh


## Un maillage par essence (ordre de `VegetationTileJob.Kind`) ; lot V4 : chêne, hêtre et
## conifère modélisés sous Blender (`VegetationMeshes.essence`).
## Lot FC2 : au loin, imposteurs pour les trois essences (haies : maillage bas).
## Lot FC5 : de près, cartes de feuillage ; entre les deux portées, imposteurs si les cartes sont
## actives (sinon maillages détaillés). Haies : maillage détaillé dès `detail_distance`.
func _meshes(lod: int) -> Array:
	var detailed := lod != Lod.FAR
	var hedge := VegetationMeshes.hedge() if detailed else VegetationMeshes.hedge_low()
	if lod == Lod.NEAR and _cards_material != null:
		return [VegetationMeshes.essence_mid("oak"), VegetationMeshes.essence_mid("beech"), VegetationMeshes.essence_mid("fir"), hedge]
	if _impostor_material != null and (lod == Lod.FAR or _cards_material != null or ga3_near_impostors):
		return [VegetationMeshes.impostor("oak"), VegetationMeshes.impostor("beech"), VegetationMeshes.impostor("fir"), hedge]
	return [VegetationMeshes.essence("oak", detailed), VegetationMeshes.essence("beech", detailed),
		VegetationMeshes.essence("fir", detailed), hedge]


## Lot FC5 : maillages de la forêt dense (`ForestDetail`) : cartes de feuillage au plus près
## (`near`), imposteurs sinon ; `--no-fc5` : maillages bas partout (comportement SZ4b).
func forest_meshes(near: bool) -> Array:
	if _cards_material == null and not ga3_near_impostors:
		return [VegetationMeshes.essence("oak", false), VegetationMeshes.essence("beech", false),
			VegetationMeshes.essence("fir", false), VegetationMeshes.hedge_low()]
	return _meshes(Lod.NEAR if near else Lod.FAR)


func forest_material(kind: int, near: bool) -> ShaderMaterial:
	return _material if _cards_material == null and not ga3_near_impostors else _material_for(kind, Lod.NEAR if near else Lod.FAR)


func near_cards_active() -> bool:
	return _cards_material != null


## Matériau d'un MultiMesh selon l'essence (`VegetationTileJob.Kind`) et le niveau de détail.
func _material_for(kind: int, lod: int) -> ShaderMaterial:
	if kind == VegetationTileJob.Kind.HEDGE:
		return _material
	if lod == Lod.NEAR and _cards_material != null:
		return _cards_material
	if _impostor_material != null and (lod == Lod.FAR or _cards_material != null or ga3_near_impostors):
		return _impostor_material
	return _material


func _start_job(index: int) -> void:
	var job := VegetationTileJob.new()
	job.mask = mask
	job.tile_index = index
	job.origin_px = Vector2i((index % chunks_x) * chunk_px, (index / chunks_x) * chunk_px)
	job.size_px = chunk_px
	job.spacing = spacing * float(chunk_px) / 256.0 if chunk_px < 256 else spacing
	job.tree_scale = tree_scale
	job.exclusions = _exclusions_for(Rect2(Vector2(job.origin_px), Vector2(chunk_px, chunk_px)))
	var level := -1
	if terrain != null:
		level = terrain.chunk_level(index)
		job.ground_grid = terrain.surface_grid(index)
	job.coarse_only = _native != null
	var task := WorkerThreadPool.add_task(job.run, false, "vegetation tile %d" % index)
	_jobs[index] = {"task": task, "job": job, "level": level}


## Exclusions qui touchent une tuile (le semis teste chaque candidat contre toute la liste).
func _exclusions_for(rect: Rect2) -> PackedVector3Array:
	var result := PackedVector3Array()
	for e in _exclusions:
		if rect.grow(e.z).has_point(Vector2(e.x, e.y)):
			result.append(e)
	return result


func _collect_jobs() -> void:
	for index in _jobs.keys():
		var item: Dictionary = _jobs[index]
		if item.has("native") or not WorkerThreadPool.is_task_completed(item["task"]):
			continue
		WorkerThreadPool.wait_for_task_completion(item["task"])
		if (item["job"] as VegetationTileJob).coarse_only:
			_submit_native(index, item)
			continue
		_jobs.erase(index)
		_install_tile(index, item["job"], item["level"])
		if _jobs.is_empty() and _log_bursts:
			print("Vegetation: %s" % JSON.stringify(stats))
	_poll_native()


## Lot PB2 : recopie le fond de vallée (ZG8) dans le pool natif quand il a été republié.
func _sync_native_floor() -> void:
	var grid := MapData.relief_floor_grid()
	if int(grid["version"]) == _native_floor_version:
		return
	_native_floor_version = grid["version"]
	var side: Vector2i = grid["side"]
	_native.call("set_floor", grid["data"], side.x, side.y, grid["cell"])
	# SZ1 : base et écrasement des montagnes (même grille).
	_native.call("set_relief_fields", grid["base"], grid["squash"])


## Lot PB2 : grille grossière prête → semis natif (conversion des données sur le fil principal).
func _submit_native(index: int, item: Dictionary) -> void:
	_sync_native_floor()
	_native_serial += 1
	var job: VegetationTileJob = item["job"]
	if not _native.call("request", _native_serial, job.native_params()):
		# Requête refusée (données malformées) : semis GDScript complet dans le fil principal.
		job.coarse_only = false
		job.run()
		_jobs.erase(index)
		_install_tile(index, job, item["level"])
		return
	item["native"] = _native_serial
	_native_ids[_native_serial] = index


## Installe les tuiles semées par le pool natif ; les résultats périmés (tuile vidée par
## `clear` entre-temps) sont ignorés.
func _poll_native() -> void:
	if _native == null or (_native_ids.is_empty() and _native_ground_ids.is_empty() and (forest_detail == null or forest_detail.pending() == 0)):
		return
	for result: Dictionary in _native.call("poll", 64):
		var id: int = result["id"]
		if forest_detail != null and forest_detail.owns(id):
			forest_detail.on_result(result)  # SZ4b : cellule de forêt dense
			continue
		if _native_ground_ids.has(id):
			var ground_index: int = _native_ground_ids[id]
			_native_ground_ids.erase(id)
			var ground_item: Dictionary = _ground_jobs.get(ground_index, {})
			if ground_item.get("native", -1) == id:
				_ground_jobs.erase(ground_index)
				var ground_job: VegetationGroundJob = ground_item["job"]
				ground_job.results.clear()
				for buffer: PackedFloat32Array in result["buffers"]:
					ground_job.results.append(buffer)
				ground_job.build_ms = float(result["ms"])
				_apply_ground(ground_index, ground_job)
			continue
		if not _native_ids.has(id):
			continue
		var index: int = _native_ids[id]
		_native_ids.erase(id)
		var item: Dictionary = _jobs.get(index, {})
		if item.get("native", -1) != id:
			continue
		var job: VegetationTileJob = item["job"]
		job.apply_native(result)
		_jobs.erase(index)
		_install_tile(index, job, item["level"])
		if _jobs.is_empty() and _log_bursts:
			print("Vegetation: %s" % JSON.stringify(stats))


func _wait_all_jobs() -> void:
	for item: Dictionary in _jobs.values():
		if not item.has("native"):  # tâche déjà attendue avant la requête native
			WorkerThreadPool.wait_for_task_completion(item["task"])
	_jobs.clear()
	_native_ids.clear()
	for item: Dictionary in _ground_jobs.values():
		if not item.has("native"):
			WorkerThreadPool.wait_for_task_completion(item["task"])
	_ground_jobs.clear()
	_native_ground_ids.clear()
	_ground_dirty.clear()


## Lance le recalage des tuiles visibles dont la surface a changé (au plus `max_ground_jobs`).
func _start_ground_jobs() -> void:
	if terrain == null or _ground_dirty.is_empty():
		return
	for index in _ground_dirty.keys():
		if _ground_jobs.size() >= max_ground_jobs:
			return
		if _ground_jobs.has(index):
			continue
		var entry: Dictionary = _tiles.get(index, {})
		if entry.is_empty():
			_ground_dirty.erase(index)
			continue
		if not (entry["node"] as Node3D).visible:
			continue  # recalée quand elle reviendra dans le champ
		_ground_dirty.erase(index)
		var job := VegetationGroundJob.new()
		job.tile_index = index
		job.generation = entry["generation"]
		job.level = terrain.chunk_level(index)
		job.grid = terrain.surface_grid(index)
		job.origin = Vector2((index % chunks_x) * chunk_px, (index / chunks_x) * chunk_px)
		job.buffers = entry["buffers"]
		if _native != null:
			_sync_native_floor()
			_native_serial += 1
			var untyped: Array = []  # le pont Rust attend un Array non typé
			untyped.assign(job.buffers)
			if _native.call("request_reground", _native_serial, untyped, job.grid, job.origin, MapData.vertical_scale(), MapData.relief_gain(), MapData.relief_squash()):
				_native_ground_ids[_native_serial] = index
				_ground_jobs[index] = {"native": _native_serial, "job": job}
				continue
		var task := WorkerThreadPool.add_task(job.run, false, "vegetation ground %d" % index)
		_ground_jobs[index] = {"task": task, "job": job}


func _collect_ground_jobs(block: bool = false) -> void:
	for index in _ground_jobs.keys():
		var item: Dictionary = _ground_jobs[index]
		if item.has("native"):
			continue  # relevé par `_poll_native`
		if not block and not WorkerThreadPool.is_task_completed(item["task"]):
			continue
		WorkerThreadPool.wait_for_task_completion(item["task"])
		_ground_jobs.erase(index)
		_apply_ground(index, item["job"])
	while block and not _native_ground_ids.is_empty():
		OS.delay_usec(200)
		_poll_native()


## Installe les tampons recalés d'une tuile (ignorés si la tuile a été resemée entre-temps).
func _apply_ground(index: int, job: VegetationGroundJob) -> void:
	var entry: Dictionary = _tiles.get(index, {})
	if entry.is_empty() or int(entry["generation"]) != job.generation:
		return
	var slots: Array = entry["slots"]
	for slot in slots.size():
		var mmi: MultiMeshInstance3D = slots[slot]
		if mmi != null:
			mmi.multimesh.buffer = job.results[slot]
	entry["buffers"] = job.results
	entry["level"] = job.level
	# Le niveau a encore changé pendant le calcul : un nouveau recalage suivra.
	if terrain.chunk_level(index) != job.level:
		_ground_dirty[index] = true
	stats["regrounds"] = int(stats["regrounds"]) + 1
	stats["reground_ms_max"] = maxf(float(stats["reground_ms_max"]), job.build_ms)


## Recale tout de suite les tuiles en attente, visibles ou non (tests, captures).
func flush_ground() -> void:
	for _i in 8:
		var saved := max_ground_jobs
		max_ground_jobs = 1 << 20
		for index in _ground_dirty.keys():
			var entry: Dictionary = _tiles.get(index, {})
			if not entry.is_empty():
				(entry["node"] as Node3D).visible = true
		_start_ground_jobs()
		max_ground_jobs = saved
		_collect_ground_jobs(true)
		if _ground_dirty.is_empty():
			return


func _install_tile(index: int, job: VegetationTileJob, level: int = -1) -> void:
	var node := Node3D.new()
	node.name = "Tile_%d" % index
	var parts: Array = []
	var meshes := _meshes(Lod.FAR)
	var part_px := float(chunk_px) / VegetationTileJob.PARTS_SIDE
	var slots: Array = []
	for part_index in VegetationTileJob.PARTS:
		var part_node := Node3D.new()
		part_node.name = "Part_%d" % part_index
		var mmis: Array = []
		for kind in VegetationTileJob.KIND_COUNT:
			var slot := part_index * VegetationTileJob.KIND_COUNT + kind
			var count: int = job.counts[slot]
			if count == 0:
				mmis.append(null)
				slots.append(null)
				continue
			var multimesh := MultiMesh.new()
			multimesh.transform_format = MultiMesh.TRANSFORM_3D
			multimesh.use_custom_data = true
			multimesh.mesh = meshes[kind]
			multimesh.instance_count = count
			multimesh.buffer = job.buffers[slot]
			var mmi := MultiMeshInstance3D.new()
			mmi.name = ["Oak", "Beech", "Conifer", "Hedge"][kind]
			mmi.multimesh = multimesh
			mmi.material_override = _material_for(kind, Lod.FAR)
			mmi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
			part_node.add_child(mmi)
			mmis.append(mmi)
			slots.append(mmi)
		node.add_child(part_node)
		var cell := Vector2(part_index % VegetationTileJob.PARTS_SIDE, part_index / VegetationTileJob.PARTS_SIDE)
		parts.append({"node": part_node, "mmis": mmis, "tile": index, "slot0": part_index * VegetationTileJob.KIND_COUNT, "rect":Rect2(Vector2(job.origin_px) + cell * part_px, Vector2(part_px, part_px))})
	add_child(node)
	_generation += 1
	# Tampons CPU gardés pour le recalage (lot C7b) : 64 octets par instance.
	_tiles[index] = {"node": node, "parts": parts, "counts": job.counts, "last_seen": _frame, "buffers": job.buffers, "slots": slots, "generation": _generation, "level": level,
		"coarse": job.coarse_params()}
	if terrain != null and terrain.chunk_level(index) != level:
		_ground_dirty[index] = true
	stats["tiles"] = _tiles.size()
	stats["instances"] = int(stats["instances"]) + job.instance_total()
	stats["build_ms_total"] = float(stats["build_ms_total"]) + job.build_ms
	stats["build_ms_max"] = maxf(float(stats["build_ms_max"]), job.build_ms)


## Libère les tuiles hors champ les plus anciennes au-delà de `max_cached_tiles`.
func _evict() -> void:
	if _tiles.size() <= max_cached_tiles:
		return
	var idle: Array = []
	for index in _tiles:
		var entry: Dictionary = _tiles[index]
		if not (entry["node"] as Node3D).visible:
			idle.append([int(entry["last_seen"]), index])
	idle.sort_custom(func(a: Array, b: Array) -> bool: return a[0] < b[0])
	var excess := _tiles.size() - max_cached_tiles
	for i in mini(excess, idle.size()):
		var index: int = idle[i][1]
		var entry: Dictionary = _tiles[index]
		var total := 0
		for count in entry["counts"]:
			total += count
		stats["instances"] = int(stats["instances"]) - total
		(entry["node"] as Node).queue_free()
		_tiles.erase(index)
		_ground_dirty.erase(index)
	stats["tiles"] = _tiles.size()
