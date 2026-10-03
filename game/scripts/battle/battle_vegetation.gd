class_name BattleVegetation
extends Node3D

## Herbe animée des batailles (lot V4) : deux grilles de touffes (proche dense, lointaine plus
## clairsemée et plus grande) en `MultiMesh`, déplacées par pas entiers de leur maille pour suivre
## le point regardé par la caméra. Tout le placement est dans `battle_grass.gdshader`.
## Masquée quand la caméra est très haute (l'herbe n'y serait qu'un bruit de pixels).
## Lot B5 : teinte et neige selon la saison et le sol du site (`BattleTerrain.snowy()`).

const GRASS_SHADER := preload("res://shaders/battle_grass.gdshader")
const GRASS_TEXTURE := preload("res://assets/textures/battle/grass_clump.png")
## DA6 : touffe en éventail (luminance et alpha seuls), luminance moyenne linéaire de la carte.
const GRASS_TEXTURE_DA6 := preload("res://assets/textures/battle/grass_blades.png")
const GRASS_TEX_LUM_DA6 := 0.21
const MAX_CAMERA_HEIGHT := 170.0
## FA7 : atlas de touffes faites de vrais brins photographiés (`build_fa_grass.py`), catalogue et
## réglages du rendu dans `data/art/battle_grass.json` (schéma `art_battle_grass.schema.json`).
const FA_TEXTURE_PATH := "res://assets/textures/battle/grass_tufts.png"
const FA_FILE := "art/battle_grass.json"
const FA_MAX_VARIANTS := 16  # taille des tableaux du shader
const MAP_PATHS_SCRIPT := preload("res://scripts/map/map_paths.gd")

static var _fa_catalogue: Dictionary = {}
static var _fa_loaded: bool = false

## [maille (m), rayon (m), échelle des touffes, rayon intérieur (m)]
const LAYERS := [[0.5, 40.0, 1.0, 0.0], [1.1, 95.0, 1.35, 34.0]]

var _layers: Array[MultiMeshInstance3D] = []
var _materials: Array[ShaderMaterial] = []
var _ground_y: float = 0.0
var _fa_ab: bool = false  # banc A/B : les deux herbes sont construites
var _fa_view: bool = true


## `da6_on` : −1 = selon le terrain (`BattleTerrain.da6`), 0/1 forcé (banc A/B DA6).
func build(terrain: BattleTerrain, weather: String, da6_on: int = -1) -> void:
	_ground_y = terrain.height_at(terrain.FIELD_W * 0.5, terrain.FIELD_D * 0.5)
	var da6 := terrain.da6 if da6_on < 0 else da6_on == 1
	# FA7 : touffes de vrais brins (au-dessus de DA6). `--no-fa-grass` après `--` : herbe d'avant.
	var use_fa := da6 and fa_grass_enabled()
	_build_set(terrain, weather, da6, use_fa)
	for arg in OS.get_cmdline_user_args():
		if use_fa and arg.begins_with("--bench-ab=") and arg.contains("fa-grass"):
			# Banc A/B (`--bench-ab=fa-grass,no-fa-grass`) : l'herbe d'avant, masquée.
			_build_set(terrain, weather, da6, false)
			_fa_ab = true
			set_fa_view(true)


## FA7 : bascule du banc A/B entre l'herbe en vrais brins et celle d'avant.
func set_fa_view(on: bool) -> void:
	if _fa_ab:
		_fa_view = on


func _build_set(terrain: BattleTerrain, weather: String, da6: bool, use_fa: bool) -> void:
	var mesh := _clump_mesh_da6() if da6 else _clump_mesh()
	var fa: Dictionary = fa_catalogue() if use_fa else {}
	var render: Dictionary = fa.get("render", {})
	var layers: Array = LAYERS
	if not fa.is_empty():
		mesh = _clump_mesh_fa(render["card"])
		layers = render["layers"]
	for layer in layers:
		var spacing: float = layer[0]
		# PF1 : rayon (donc nombre de touffes, au carré) selon le préréglage de qualité.
		var radius: float = float(layer[1]) * float(RenderQuality.preset().get("grass", 1.0))
		var mat := ShaderMaterial.new()
		mat.shader = GRASS_SHADER
		mat.set_shader_parameter("grass_texture", GRASS_TEXTURE_DA6 if da6 else GRASS_TEXTURE)
		if da6:
			mat.set_shader_parameter("da6_on", 1.0)
			mat.set_shader_parameter("decor_saturation", terrain.decor_saturation())
			mat.set_shader_parameter("tex_lum", GRASS_TEX_LUM_DA6)
		if not fa.is_empty():
			_apply_fa(mat, fa)
		mat.set_shader_parameter("height_map", terrain.height_texture)
		var hr := terrain.SPLAT_RECT
		var t := BattleTerrain.HEIGHT_TEXEL
		var hw := float(int(hr.size.x / t) + 1) * t
		var hh := float(int(hr.size.y / t) + 1) * t
		mat.set_shader_parameter("height_rect", Vector4(hr.position.x - t * 0.5, hr.position.y - t * 0.5, hw, hh))
		mat.set_shader_parameter("albedo_array", BattleTerrain.ALBEDO_ARRAY)
		mat.set_shader_parameter("macro_noise", terrain.macro_noise)
		mat.set_shader_parameter("splat_a", terrain.splat_a)
		mat.set_shader_parameter("splat_b", terrain.splat_b)
		if terrain.decor_on:
			# EP6 : parcelles du décor (labours sans herbe, blé haut, chaume ras).
			mat.set_shader_parameter("decor_fields", terrain.decor_fields)
			mat.set_shader_parameter("decor_on", 1.0)
		mat.set_shader_parameter("splat_rect", Vector4(hr.position.x, hr.position.y, hr.size.x, hr.size.y))
		mat.set_shader_parameter("spacing", spacing)
		mat.set_shader_parameter("radius", radius)
		mat.set_shader_parameter("blade_scale", float(layer[2]))
		mat.set_shader_parameter("inner_radius", float(layer[3]) * float(RenderQuality.preset().get("grass", 1.0)))
		# B5 : sol de saison (neige au sol sans chute de neige), herbe d'hiver et d'automne
		# plus sèche, joncs plus sombres du marais.
		var ground_weather := "snow" if terrain.snowy() else weather
		if terrain.site_render and ground_weather == "clear":
			match terrain.season_key:
				"winter":
					mat.set_shader_parameter("base_tint", Color(0.95, 0.88, 0.7))
				"autumn":
					mat.set_shader_parameter("base_tint", Color(1.0, 0.92, 0.72))
			if terrain.terrain_key == "marsh":
				mat.set_shader_parameter("base_tint", Color(0.82, 0.9, 0.7))
		match ground_weather:
			"snow":
				mat.set_shader_parameter("snow", 1.0 if weather == "snow" else 0.8)
				mat.set_shader_parameter("base_tint", Color(0.85, 0.85, 0.75))
			"rain":
				mat.set_shader_parameter("base_tint", Color(0.8, 0.88, 0.75))
				mat.set_shader_parameter("wind_strength", 1.6)
			"fog":
				mat.set_shader_parameter("wind_strength", 0.4)
		var n := int(ceil(radius * 2.0 / spacing))
		var mm := MultiMesh.new()
		mm.transform_format = MultiMesh.TRANSFORM_3D
		mm.mesh = mesh
		mm.instance_count = n * n
		var buffer := PackedFloat32Array()
		buffer.resize(n * n * 12)
		var half := float(n) * spacing * 0.5
		var i := 0
		for iz in n:
			for ix in n:
				var o := i * 12
				buffer[o] = 1.0
				buffer[o + 3] = float(ix) * spacing - half
				buffer[o + 5] = 1.0
				buffer[o + 10] = 1.0
				buffer[o + 11] = float(iz) * spacing - half
				i += 1
		mm.buffer = buffer
		# Les sommets sont déplacés dans le shader : boîte englobante généreuse.
		mm.mesh.custom_aabb = AABB(Vector3(-4, -200, -4), Vector3(8, 600, 8))
		var instance := MultiMeshInstance3D.new()
		instance.name = "Grass%d" % _layers.size()
		instance.multimesh = mm
		instance.material_override = mat
		instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		instance.custom_aabb = AABB(Vector3(-radius, -300, -radius), Vector3(radius * 2.0, 900, radius * 2.0))
		instance.layers |= BattleTerrain.DECAL_LAYER  # CR1 : le contour de formation passe sur l'herbe
		add_child(instance)
		instance.set_meta("spacing", spacing)
		instance.set_meta("fa", not fa.is_empty())
		_layers.append(instance)
		_materials.append(mat)


## BV3 : herbe couchée et tachée de sang (`BattleGrassFlatten`), sur toutes les couches.
func set_flatten(flatten: BattleGrassFlatten) -> void:
	for mat in _materials:
		mat.set_shader_parameter("flatten_map", flatten.texture)
		mat.set_shader_parameter("flatten_rect", flatten.rect_vec())
		mat.set_shader_parameter("flatten_on", 1.0)


## BV3 : vent de la bataille (même direction que les drapeaux).
func set_wind(direction: Vector2, strength: float) -> void:
	for mat in _materials:
		mat.set_shader_parameter("wind_dir", direction.normalized())
		mat.set_shader_parameter("wind_strength", strength)


func _process(_delta: float) -> void:
	var camera := get_viewport().get_camera_3d()
	if camera == null or _layers.is_empty():
		return
	var focus := _focus_point(camera, _ground_y)
	var high := camera.global_position.y - focus.y > MAX_CAMERA_HEIGHT
	for k in _layers.size():
		var instance := _layers[k]
		instance.visible = not high and (not _fa_ab or bool(instance.get_meta("fa")) == _fa_view)
		if not instance.visible:
			continue
		var spacing: float = instance.get_meta("spacing")
		instance.global_position = Vector3(snappedf(focus.x, spacing), 0.0, snappedf(focus.z, spacing))
		_materials[k].set_shader_parameter("focus", focus)


## Point du sol regardé (intersection du rayon central avec le plan moyen), rapproché de la
## caméra pour que l'herbe couvre surtout le premier plan.
static func _focus_point(camera: Camera3D, ground_y: float) -> Vector3:
	var origin := camera.global_position
	var forward := -camera.global_transform.basis.z
	var t := 200.0
	if forward.y < -0.05:
		t = (ground_y - origin.y) / forward.y
	var hit := origin + forward * clampf(t, 0.0, 600.0)
	var cam_ground := Vector3(origin.x, hit.y, origin.z)
	return cam_ground.lerp(hit, 0.72)


## FA7 : catalogue de l'herbe (dossier de données du jeu) ; vide s'il manque ou si l'atlas manque.
static func fa_catalogue() -> Dictionary:
	if _fa_loaded:
		return _fa_catalogue
	_fa_loaded = true
	var dir := MAP_PATHS_SCRIPT.default_data_dir()
	var tree := Engine.get_main_loop() as SceneTree
	if tree != null and tree.root != null:
		var map_paths := tree.root.get_node_or_null("MapPaths")
		if map_paths != null:
			dir = str(map_paths.get("data_dir"))
	var path := dir.path_join(FA_FILE)
	if FileAccess.file_exists(path) and ResourceLoader.exists(FA_TEXTURE_PATH):
		var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
		if parsed is Dictionary and (parsed as Dictionary).get("render") is Dictionary:
			_fa_catalogue = parsed
	if _fa_catalogue.is_empty():
		push_warning("BattleVegetation: %s or %s missing, drawn grass kept" % [path, FA_TEXTURE_PATH])
	return _fa_catalogue


static func fa_grass_enabled() -> bool:
	return not OS.get_cmdline_user_args().has("--no-fa-grass")


## FA7 : paramètres du shader tirés du catalogue (cases de l'atlas, tirage, fondu dans le sol).
static func _apply_fa(mat: ShaderMaterial, fa: Dictionary) -> void:
	var atlas: Dictionary = fa["atlas"]
	var render: Dictionary = fa["render"]
	var variants: Array = fa["variants"]
	var heights := PackedFloat32Array()
	var cum_open := PackedFloat32Array()
	var cum_clump := PackedFloat32Array()
	heights.resize(FA_MAX_VARIANTS)
	cum_open.resize(FA_MAX_VARIANTS)
	cum_clump.resize(FA_MAX_VARIANTS)
	var total_open := 0.0
	var total_clump := 0.0
	var index_of := {}
	for i in variants.size():
		total_open += float(variants[i]["weight_open"])
		total_clump += float(variants[i]["weight_clump"])
		index_of[str(variants[i]["name"])] = i
	var sum_open := 0.0
	var sum_clump := 0.0
	for i in FA_MAX_VARIANTS:
		if i < variants.size():
			heights[i] = float(variants[i]["height_m"])
			sum_open += float(variants[i]["weight_open"]) / maxf(total_open, 0.001)
			sum_clump += float(variants[i]["weight_clump"]) / maxf(total_clump, 0.001)
		cum_open[i] = sum_open
		cum_clump[i] = sum_clump
	mat.set_shader_parameter("grass_texture", load(FA_TEXTURE_PATH))
	mat.set_shader_parameter("tex_lum", float(render["tex_lum"]))
	mat.set_shader_parameter("fa_on", 1.0)
	mat.set_shader_parameter("fa_grid", Vector2(float(atlas["columns"]), float(atlas["rows"])))
	mat.set_shader_parameter("fa_pad", Vector2(float(atlas["padding"]) / float(atlas["cell_width"]), float(atlas["padding"]) / float(atlas["cell_height"])))
	mat.set_shader_parameter("fa_count", variants.size())
	mat.set_shader_parameter("fa_height", heights)
	mat.set_shader_parameter("fa_cum_open", cum_open)
	mat.set_shader_parameter("fa_cum_clump", cum_clump)
	var fields: Dictionary = render["fields"]
	mat.set_shader_parameter("fa_wheat", Vector2i(int(index_of[fields["wheat_variants"][0]]), int(index_of[fields["wheat_variants"][1]])))
	mat.set_shader_parameter("fa_stubble", Vector2i(int(index_of[fields["stubble_variants"][0]]), int(index_of[fields["stubble_variants"][1]])))
	mat.set_shader_parameter("fa_field_height", Vector4(float(fields["wheat_height"]), float(fields["fallow_height"]), float(fields["stubble_height"]), float(fields["seedling_height"])))
	mat.set_shader_parameter("fa_size", Vector2(float(render["size"][0]), float(render["size"][1])))
	mat.set_shader_parameter("fa_luma_clamp", Vector2(float(render["luma_clamp"][0]), float(render["luma_clamp"][1])))
	var gain: Array = render["tint_gain"]
	mat.set_shader_parameter("fa_tint_gain", Vector3(float(gain[0]), float(gain[1]), float(gain[2])))
	var wheat: Array = fields["wheat_gain"]
	mat.set_shader_parameter("fa_wheat_gain", Vector3(float(wheat[0]), float(wheat[1]), float(wheat[2])))
	mat.set_shader_parameter("fa_wheat_foot", float(fields["wheat_foot_shade"]))
	var sown: Array = fields["seedling_gain"]
	mat.set_shader_parameter("fa_sown_gain", Vector3(float(sown[0]), float(sown[1]), float(sown[2])))
	for key in ["flat_height", "height_var", "gap_fill", "patch_fill", "tint_var", "hue_mix", "contrast", "foot_shade", "foot_height", "up_normal", "far_luma", "mip_boost", "backlight"]:
		mat.set_shader_parameter("fa_" + key, float(render[key]))


## Touffe : trois cartes croisées à 60°, 0,6 m de large, 0,42 m de haut, pied à l'origine.
static func _clump_mesh() -> ArrayMesh:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	for k in 3:
		var a := PI * float(k) / 3.0
		var d := Vector3(cos(a), 0.0, sin(a)) * 0.3
		var n := Vector3(-sin(a), 0.0, cos(a))
		var corners := [-d, d, d + Vector3(0, 0.42, 0), -d + Vector3(0, 0.42, 0)]
		var uvs := [Vector2(0, 1), Vector2(1, 1), Vector2(1, 0), Vector2(0, 0)]
		for idx in [0, 1, 2, 0, 2, 3]:
			st.set_normal(n)
			st.set_uv(uvs[idx])
			st.add_vertex(corners[idx])
	st.index()
	return st.commit()


## DA6 : touffe en volume — quatre cartes à 45°, cintrées (colonne centrale décalée le long de la
## normale) et évasées (pied de 0,24 m, sommet de 0,64 m) ; normales arrondies (verticale +
## direction depuis l'axe de la touffe) ; normale propre de la carte dans COLOR (le shader amincit
## les cartes vues par la tranche). 0,44 m de haut, pied à l'origine.
static func _clump_mesh_da6() -> ArrayMesh:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	for k in 4:
		var a := PI * float(k) / 4.0 + 0.2
		var along := Vector3(cos(a), 0.0, sin(a))
		var n := Vector3(-sin(a), 0.0, cos(a))
		var bend := 0.05 if k % 2 == 0 else -0.05
		# Grille 3 colonnes × 2 rangées : [u, v, demi-largeur, hauteur, décalage normal].
		var grid := []
		for row in 2:
			var y := 0.0 if row == 0 else 0.44
			var half := 0.12 if row == 0 else 0.32
			for col in 3:
				var u := float(col) * 0.5
				var off := bend * (0.4 if row == 0 else 1.0) if col == 1 else 0.0
				var p := along * (u * 2.0 - 1.0) * half + n * off + Vector3(0, y, 0)
				var radial := Vector3(p.x, 0.0, p.z)
				var normal := (Vector3.UP * 0.8 + (radial.normalized() * 0.55 if radial.length() > 0.01 else n * 0.2)).normalized()
				grid.append([p, Vector2(u, 1.0 - float(row)), normal])
		for idx in [0, 1, 4, 0, 4, 3, 1, 2, 5, 1, 5, 4]:
			var g: Array = grid[idx]
			st.set_normal(g[2])
			st.set_uv(g[1])
			st.set_color(Color(n.x * 0.5 + 0.5, 0.5, n.z * 0.5 + 0.5))
			st.add_vertex(g[0])
	st.index()
	return st.commit()


## FA7 : touffe de `count` cartes croisées, hautes d'un mètre (le shader les ramène à la hauteur
## de la case tirée), larges de `width_m` au sommet et de `foot_ratio` fois moins au pied, cintrées
## de `bend_m`. Mêmes normales arrondies et normale de carte dans COLOR que DA6 ; rang de la carte
## dans UV2.x (chaque carte tire sa case de l'atlas).
static func _clump_mesh_fa(card: Dictionary) -> ArrayMesh:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var count := int(card["count"])
	var half_top := float(card["width_m"]) * 0.5
	var half_foot := half_top * float(card["foot_ratio"])
	for k in count:
		var a := PI * float(k) / float(count) + 0.2
		var along := Vector3(cos(a), 0.0, sin(a))
		var n := Vector3(-sin(a), 0.0, cos(a))
		var bend := float(card["bend_m"]) * (1.0 if k % 2 == 0 else -1.0)
		var grid := []
		for row in 2:
			var y := float(row)
			var half := half_foot if row == 0 else half_top
			for col in 3:
				var u := float(col) * 0.5
				var off := bend * (0.4 if row == 0 else 1.0) if col == 1 else 0.0
				var p := along * (u * 2.0 - 1.0) * half + n * off + Vector3(0, y, 0)
				var radial := Vector3(p.x, 0.0, p.z)
				var normal := (Vector3.UP * 0.8 + (radial.normalized() * 0.55 if radial.length() > 0.01 else n * 0.2)).normalized()
				grid.append([p, Vector2(u, 1.0 - float(row)), normal])
		for idx in [0, 1, 4, 0, 4, 3, 1, 2, 5, 1, 5, 4]:
			var g: Array = grid[idx]
			st.set_normal(g[2])
			st.set_uv(g[1])
			st.set_uv2(Vector2(float(k), 0.0))
			st.set_color(Color(n.x * 0.5 + 0.5, 0.5, n.z * 0.5 + 0.5))
			st.add_vertex(g[0])
	st.index()
	return st.commit()
