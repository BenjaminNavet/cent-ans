class_name BattleHorizon
extends Node3D

## Horizon des batailles (lot EP2, ADR 0032). Rendu seulement, aucune collision :
## - relief réel autour du lieu de la bataille (tuile de 26 km cuite hors ligne par province,
##   `cent-ans geo horizon`), raccordé au relief généré du champ par `blend()` (appelé depuis
##   `BattleTerrain.world_height`) : 0 près du champ, 1 au-delà de `blend_end_m` ;
## - anneau d'horizon (au-delà de l'anneau lointain, jusqu'à `ring_radius_m`) : maillage au pas de
##   400 m, shader `battle_horizon_land` (champs, forêts réelles, roche, neige), brume qui
##   s'allège en altitude ; mer lointaine là où le relief réel est sous le niveau de la mer ;
## - panorama peint (bande cylindrique qui suit la caméra) recalé colonne par colonne sur la ligne
##   d'horizon réelle (profil cuit de 12 à 150 km), horizon marin dans les secteurs de mer ;
## - silhouettes lointaines (clochers, château, fumées) posées sur l'anneau lointain.
## Données : `data/fx/horizon.json`. Options : `--no-horizon` (tout coupé, mesures A/B),
## `--horizon-province=<id>` (tuile d'une autre province, captures), `--panorama=<id>`.

const DATA_PATH := "fx/horizon.json"
const PANORAMA_META := "res://assets/horizon/panoramas/panoramas.json"
const MAP_PATHS_SCRIPT := preload("res://scripts/map/map_paths.gd")
const LAND_SHADER := preload("res://shaders/battle_horizon_land.gdshader")
const PANORAMA_SHADER := preload("res://shaders/battle_panorama.gdshader")
const SMOKE_SHADER := preload("res://shaders/battle_horizon_smoke.gdshader")
const SEA_SHADER := preload("res://shaders/battle_sea.gdshader")
const SEA_CLASS := 255
const FOREST_MAX := 200.0

static var _data: Dictionary = {}

## Relief réel chargé et actif (sinon repli sur le seul relief généré).
var active: bool = false
var province: String = ""
## EP7 : tuile d'un site historique (`hist_crecy`, cuite par `cent-ans geo battle-site` déjà
## tournée dans le repère du champ) à la place de celle de la province ; relief réel dès le bord.
var site_key: String = ""
const SITE_BLEND_M := Vector2(120.0, 1400.0)
var panorama_id: String = ""
var field_size: Vector2 = Vector2(1200, 800)
var centre: Vector2 = Vector2(600, 400)
## Altitude du monde de bataille = altitude réelle + `offset_y`.
var offset_y: float = 0.0
var ref_m: float = 0.0
var rotation_rad: float = 0.0  # relèvement ajouté aux directions réelles (alignement de la côte)
var ring_tris: int = 0
var silhouette_count: int = 0

var _n: int = 0
var _step: float = 100.0
var _half: float = 13000.0
var _heights := PackedFloat32Array()
var _classes := PackedByteArray()
var _skyline := PackedFloat32Array()  # angle (degrés) par azimut réel
var _skyline_dist := PackedFloat32Array()  # m
var _sea_share := PackedFloat32Array()
var _cos: float = 1.0
var _sin: float = 0.0
var _blend_start: float = 700.0
var _blend_end: float = 3000.0
var _river_keep: float = 450.0
var _cfg: Dictionary = {}
var _land_material: ShaderMaterial
var _panorama_material: ShaderMaterial
var _smoke_material: ShaderMaterial
var _panorama: MeshInstance3D
var _terrain_key: String = "plains"
var _season: String = "summer"


static func data() -> Dictionary:
	if _data.is_empty():
		var path := _data_dir().path_join(DATA_PATH)
		if FileAccess.file_exists(path):
			var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
			if parsed is Dictionary:
				_data = parsed
		if _data.is_empty():
			push_warning("BattleHorizon: %s missing or invalid" % path)
	return _data


static func _data_dir() -> String:
	var tree := Engine.get_main_loop() as SceneTree
	if tree != null and tree.root != null:
		var map_paths := tree.root.get_node_or_null("MapPaths")
		if map_paths != null:
			return str(map_paths.get("data_dir"))
	return MAP_PATHS_SCRIPT.default_data_dir()


static func disabled_by_flag() -> bool:
	return OS.get_cmdline_user_args().has("--no-horizon")


## Charge la tuile de `p_province` et aligne sa côte sur le flanc côtier du champ (`flank` :
## "west", "east" ou ""). `mean_height` : hauteur moyenne du champ. Faux si pas de tuile.
func setup(p_province: String, p_field_size: Vector2, mean_height: float, flank: String, terrain_key: String, season: String) -> bool:
	active = false
	province = p_province
	field_size = p_field_size
	centre = field_size * 0.5
	_terrain_key = terrain_key
	_season = season
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--horizon-province="):
			province = arg.trim_prefix("--horizon-province=")
	_cfg = data()
	if _cfg.is_empty() or disabled_by_flag() or province == "":
		return false
	var relief: Dictionary = _cfg.get("relief", {})
	_blend_start = float(relief.get("blend_start_m", 700.0))
	_blend_end = float(relief.get("blend_end_m", 3000.0))
	_river_keep = float(relief.get("river_keep_m", 450.0))
	var index_path := str(relief.get("index", ""))
	if not FileAccess.file_exists(index_path):
		return false
	var index: Variant = JSON.parse_string(FileAccess.get_file_as_string(index_path))
	if not (index is Dictionary):
		return false
	var tiles: Dictionary = (index as Dictionary).get("provinces", {})
	var entry: Dictionary = tiles.get(province, {})
	var sites: Dictionary = (index as Dictionary).get("sites", {})  # EP7 : tuiles des cartes historiques
	if site_key != "" and sites.has(site_key):
		entry = sites[site_key]
		_blend_start = SITE_BLEND_M.x
		_blend_end = SITE_BLEND_M.y
		flank = ""
	if entry.is_empty():
		return false
	if not _load_tile(index_path.get_base_dir().path_join(str(entry["file"]))):
		return false
	# Côte : la mer réelle est tournée vers le flanc côtier de la simulation (B5).
	rotation_rad = 0.0
	var bearing: Variant = entry.get("coast_bearing_deg")
	if flank != "" and bearing != null:
		var flank_bearing := 270.0 if flank == "west" else 90.0
		rotation_rad = deg_to_rad(flank_bearing - float(bearing))
	_cos = cos(rotation_rad)
	_sin = sin(rotation_rad)
	# Altitude : le champ est posé sur l'altitude réelle moyenne de son emprise ; une bataille
	# côtière garde la mer réelle au niveau de la mer du champ (`BattleVillage.SEA_LEVEL`).
	offset_y = BattleVillage.SEA_LEVEL if flank != "" else mean_height - ref_m
	panorama_id = _choose_panorama(entry)
	active = true
	print("BattleHorizon: %s, ref %.0f m, offset %.1f, rotation %.0f°, panorama %s" % [province, ref_m, offset_y, rad_to_deg(rotation_rad), panorama_id])
	return true


func _load_tile(path: String) -> bool:
	if not FileAccess.file_exists(path):
		return false
	var bytes := FileAccess.get_file_as_bytes(path)
	if bytes.size() < 20 or bytes.slice(0, 4).get_string_from_ascii() != "HZR1":
		push_warning("BattleHorizon: bad tile %s" % path)
		return false
	_n = bytes.decode_u16(4)
	var p := bytes.decode_u16(6)
	_step = bytes.decode_float(8)
	ref_m = bytes.decode_float(12)
	var raw := bytes.slice(20).decompress(bytes.decode_u32(16), FileAccess.COMPRESSION_DEFLATE)
	if raw.size() != _n * _n * 3 + p * 5:
		push_warning("BattleHorizon: truncated tile %s" % path)
		return false
	_half = (_n - 1) * _step * 0.5
	_heights.resize(_n * _n)
	for i in _n * _n:
		_heights[i] = float(raw.decode_s16(i * 2)) * 0.25
	var offset := _n * _n * 2
	_classes = raw.slice(offset, offset + _n * _n)
	offset += _n * _n
	_skyline.resize(p)
	_skyline_dist.resize(p)
	_sea_share.resize(p)
	for k in p:
		_skyline[k] = float(raw.decode_s16(offset + k * 2)) * 0.01
		_skyline_dist[k] = float(raw.decode_u16(offset + p * 2 + k * 2)) * 100.0
		_sea_share[k] = float(raw[offset + p * 4 + k]) / 255.0
	return true


## Premier panorama dont la règle s'applique (`rules` de `horizon.json`).
func _choose_panorama(entry: Dictionary) -> String:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--panorama="):
			return arg.trim_prefix("--panorama=")
	var skyline_max := float(entry.get("skyline_max_deg", 0.0))
	for rule in _cfg.get("rules", []):
		var r: Dictionary = rule
		if r.has("provinces") and not (r["provinces"] as Array).has(province):
			continue
		if r.has("season") and not (r["season"] as Array).has(_season):
			continue
		if r.has("terrain") and not (r["terrain"] as Array).has(_terrain_key):
			continue
		if r.has("coastal") and bool(r["coastal"]) != bool(entry.get("coastal", false)):
			continue
		if r.has("skyline_max_deg") and skyline_max > float(r["skyline_max_deg"]):
			continue
		if r.has("skyline_min_deg") and skyline_max < float(r["skyline_min_deg"]):
			continue
		return str(r["panorama"])
	return ""


# --- Relief -------------------------------------------------------------------------------


## Coordonnées réelles (est, nord ; m, relatives au centre de la tuile) d'un point du monde.
func _real_offset(x: float, z: float) -> Vector2:
	var e := x - centre.x
	var n := centre.y - z
	return Vector2(e * _cos - n * _sin, n * _cos + e * _sin)


func _sample(x: float, z: float) -> Vector2:
	var r := _real_offset(x, z)
	return Vector2((r.x + _half) / _step, (_half - r.y) / _step)


## Altitude réelle (monde de bataille) en (x, z), bilinéaire, bornée à la tuile.
func real_height(x: float, z: float) -> float:
	var c := _sample(x, z)
	var fx := clampf(c.x, 0.0, float(_n - 1))
	var fz := clampf(c.y, 0.0, float(_n - 1))
	var ix := mini(int(fx), _n - 2)
	var iz := mini(int(fz), _n - 2)
	var tx := fx - ix
	var tz := fz - iz
	var i := iz * _n + ix
	var top := lerpf(_heights[i], _heights[i + 1], tx)
	var bottom := lerpf(_heights[i + _n], _heights[i + _n + 1], tx)
	return lerpf(top, bottom, tz) + offset_y


## Classe de la tuile (0-200 part de forêt, 255 mer) au plus proche.
func class_at(x: float, z: float) -> int:
	var c := _sample(x, z)
	var ix := clampi(int(round(c.x)), 0, _n - 1)
	var iz := clampi(int(round(c.y)), 0, _n - 1)
	return _classes[iz * _n + ix]


func is_sea(x: float, z: float) -> bool:
	return active and class_at(x, z) == SEA_CLASS


## Part de forêt réelle (0-1) en (x, z).
func forest_at(x: float, z: float) -> float:
	var k := class_at(x, z)
	return 0.0 if k == SEA_CLASS else float(k) / FOREST_MAX


## Distance (m) au rectangle du champ.
func field_distance(x: float, z: float) -> float:
	return Vector2(x - clampf(x, 0.0, field_size.x), z - clampf(z, 0.0, field_size.y)).length()


## Poids du relief réel (0 près du champ et le long de la rivière, 1 au loin).
func weight(x: float, z: float, river_distance: float = INF) -> float:
	if not active:
		return 0.0
	var w := smoothstep(_blend_start, _blend_end, field_distance(x, z))
	if river_distance < INF:
		w *= smoothstep(40.0, _river_keep, river_distance)
	return w


## Relief de bataille raccordé : `generated` près du champ, relief réel au loin.
func blend(x: float, z: float, generated: float, river_distance: float = INF) -> float:
	var w := weight(x, z, river_distance)
	if w <= 0.0:
		return generated
	return lerpf(generated, real_height(x, z), w)


## Limite des neiges de la saison en hauteur du monde de bataille.
func snow_line_world() -> float:
	var snow: Dictionary = (_cfg.get("relief", {}) as Dictionary).get("snow_line_m", {})
	return float(snow.get(_season, 2500.0)) + offset_y


# --- Rendu --------------------------------------------------------------------------------


## Anneau d'horizon, mer lointaine, panorama et silhouettes. `terrain` fournit la hauteur
## (`world_height`), le bruit macro et les vagues ; `far_rect` = anneau lointain existant.
func build_visuals(terrain: BattleTerrain, far_rect: Rect2, weather: String, sea_rect: Rect2) -> void:
	for child in get_children():
		child.queue_free()
	if not active:
		return
	_build_ring(terrain, far_rect, weather)
	_build_far_sea(terrain, far_rect, sea_rect, weather)
	_build_panorama(weather)
	_build_silhouettes(terrain, far_rect, weather)
	print("BattleHorizon: ring %d tris, %d silhouettes" % [ring_tris, silhouette_count])


func _build_ring(terrain: BattleTerrain, far_rect: Rect2, _weather: String) -> void:
	var relief: Dictionary = _cfg.get("relief", {})
	var step := float(relief.get("ring_step_m", 400.0))
	var radius := float(relief.get("ring_radius_m", 13000.0))
	# Grille alignée sur l'anneau lointain (mêmes sommets sur le raccord).
	var cells_x := int(ceil((radius - (centre.x - far_rect.position.x)) / step))
	var cells_z := int(ceil((radius - (centre.y - far_rect.position.y)) / step))
	var origin := far_rect.position - Vector2(cells_x, cells_z) * step
	var nx := int(ceil((centre.x + radius - origin.x) / step)) + 1
	var nz := int(ceil((centre.y + radius - origin.y) / step)) + 1
	var hole := far_rect.grow(-3.0 * step)
	var inner := far_rect.grow(1.0)
	var vertices := PackedVector3Array()
	var normals := PackedVector3Array()
	var heights := PackedFloat32Array()
	vertices.resize(nx * nz)
	normals.resize(nx * nz)
	heights.resize(nx * nz)
	for iz in nz:
		for ix in nx:
			var x := origin.x + ix * step
			var z := origin.y + iz * step
			var h := terrain.world_height(x, z)
			if inner.has_point(Vector2(x, z)):
				h -= 4.0  # sous l'anneau lointain, qui recouvre le raccord
			# Bord extérieur : le relief plonge (pas de falaise au bord de la tuile) ; le panorama,
			# recalé sur la crête réelle au-delà de 12 km, prend le relais.
			var edge := smoothstep(radius - 2.5 * step, radius + step, Vector2(x, z).distance_to(centre))
			if edge > 0.0:
				h = lerpf(h, minf(h, offset_y + ref_m) - 400.0, edge)
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
			var x0 := origin.x + ix * step
			var z0 := origin.y + iz * step
			if hole.encloses(Rect2(x0, z0, step, step)):
				continue
			if Vector2(x0 + step * 0.5, z0 + step * 0.5).distance_to(centre) > radius + step:
				continue
			var a := iz * nx + ix
			indices.append_array([a, a + 1, a + nx, a + 1, a + nx + 1, a + nx])
	ring_tris = indices.size() / 3
	var mesh := BattleTerrain._commit(vertices, normals, indices)
	_land_material = ShaderMaterial.new()
	_land_material.shader = LAND_SHADER
	_land_material.set_shader_parameter("class_map", _class_texture())
	_land_material.set_shader_parameter("tile_xform", Vector4(centre.x, centre.y, _cos, _sin))
	_land_material.set_shader_parameter("tile_half", _half)
	_land_material.set_shader_parameter("offset_y", offset_y)
	_land_material.set_shader_parameter("macro_noise", terrain.macro_noise)
	var snow: Dictionary = relief.get("snow_line_m", {})
	_land_material.set_shader_parameter("snow_line", float(snow.get(_season, 2500.0)))
	var colors: Dictionary = relief.get("colors", {})
	var field := _rgb(colors.get("field"), Color(0.34, 0.37, 0.21))
	var forest := _rgb(colors.get("forest"), Color(0.1, 0.16, 0.08))
	if _season == "winter":
		field = _rgb(colors.get("winter_field"), field)
	elif _season == "autumn":
		forest = forest.lerp(_rgb(colors.get("autumn_forest"), forest), 0.5)
	_land_material.set_shader_parameter("field_color", field)
	_land_material.set_shader_parameter("meadow_color", _rgb(colors.get("meadow"), field))
	_land_material.set_shader_parameter("forest_color", forest)
	_land_material.set_shader_parameter("rock_color", _rgb(colors.get("rock"), Color(0.42, 0.4, 0.37)))
	_land_material.set_shader_parameter("snow_color", _rgb(colors.get("snow"), Color(0.92, 0.94, 0.97)))
	_land_material.set_shader_parameter("snowy", 1.0 if terrain.snowy() else 0.0)
	var instance := MeshInstance3D.new()
	instance.name = "HorizonRing"
	instance.mesh = mesh
	instance.material_override = _land_material
	instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(instance)


func _class_texture() -> ImageTexture:
	var image := Image.create_from_data(_n, _n, false, Image.FORMAT_R8, _classes)
	return ImageTexture.create_from_image(image)


## Mer réelle au-delà du rivage : grille de 400 m là où le relief réel est sous la mer, hors
## de la mer du champ (`sea_rect`, B5) pour ne pas superposer deux plans d'eau.
func _build_far_sea(terrain: BattleTerrain, far_rect: Rect2, sea_rect: Rect2, weather: String) -> void:
	var step := 400.0
	var radius := float((_cfg.get("relief", {}) as Dictionary).get("ring_radius_m", 13000.0)) + 1600.0
	var level := offset_y - 0.6
	var vertices := PackedVector3Array()
	var normals := PackedVector3Array()
	var indices := PackedInt32Array()
	var x0 := centre.x - radius
	var z0 := centre.y - radius
	var cells := int(2.0 * radius / step)
	for iz in cells:
		for ix in cells:
			var x := x0 + ix * step
			var z := z0 + iz * step
			var cell := Rect2(x, z, step, step)
			if sea_rect.has_area() and sea_rect.encloses(cell):
				continue
			if cell.get_center().distance_to(centre) > radius:
				continue
			var wet := false
			for corner in [Vector2(x, z), Vector2(x + step, z), Vector2(x, z + step), Vector2(x + step, z + step)]:
				if terrain.world_height(corner.x, corner.y) < level + 0.2 or is_sea(corner.x, corner.y):
					wet = true
					break
			if not wet:
				continue
			var base := vertices.size()
			vertices.append_array([Vector3(x, level, z), Vector3(x + step, level, z), Vector3(x, level, z + step), Vector3(x + step, level, z + step)])
			normals.append_array([Vector3.UP, Vector3.UP, Vector3.UP, Vector3.UP])
			indices.append_array([base, base + 1, base + 2, base + 1, base + 3, base + 2])
	if indices.is_empty():
		return
	var material := ShaderMaterial.new()
	material.shader = SEA_SHADER
	material.set_shader_parameter("shore_x", -1.0e7)
	material.set_shader_parameter("flank", 1.0)
	material.set_shader_parameter("macro_noise", terrain.macro_noise)
	material.set_shader_parameter("wave_normal", terrain.water_waves())
	material.set_shader_parameter("sky_color", terrain.sky_reflection())
	if weather == "rain":
		material.set_shader_parameter("ripple", 1.0)
	var instance := MeshInstance3D.new()
	instance.name = "FarSea"
	instance.mesh = BattleTerrain._commit(vertices, normals, indices)
	instance.material_override = material
	instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(instance)


## Bande peinte : cylindre (rayon `panorama.radius_m`) qui suit la caméra dans le shader.
func _build_panorama(weather: String) -> void:
	var pano_cfg: Dictionary = _cfg.get("panorama", {})
	var entry: Dictionary = (_cfg.get("panoramas", {}) as Dictionary).get(panorama_id, {})
	var meta := _panorama_meta(panorama_id)
	var texture: Texture2D = null
	if not entry.is_empty() and ResourceLoader.exists(str(entry.get("path", ""))):
		texture = load(str(entry["path"]))
	var radius := float(pano_cfg.get("radius_m", 14600.0))
	var bottom := float(pano_cfg.get("bottom_deg", -1.2))
	var top := float(pano_cfg.get("max_angle_deg", 7.5)) * 1.6 + 1.0
	var ref_y := offset_y + maxf(ref_m, 0.0) + 25.0
	var segments := 128
	var vertices := PackedVector3Array()
	var indices := PackedInt32Array()
	# Bas enfoui (sous l'horizon vu d'une caméra haute), haut au-dessus de la plus haute crête.
	var ys := [ref_y - radius * 0.2, ref_y + radius * tan(deg_to_rad(bottom)), ref_y + radius * tan(deg_to_rad(top))]
	for k in segments + 1:
		var a := TAU * float(k) / float(segments)
		for y in ys:
			vertices.append(Vector3(sin(a) * radius, y, -cos(a) * radius))
	for k in segments:
		for r in ys.size() - 1:
			var i := k * ys.size() + r
			var j := i + ys.size()
			indices.append_array([i, j, i + 1, i + 1, j, j + 1])
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_INDEX] = indices
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	_panorama_material = ShaderMaterial.new()
	_panorama_material.shader = PANORAMA_SHADER
	_panorama_material.set_shader_parameter("skyline_real", _skyline_texture())
	_panorama_material.set_shader_parameter("rotation_turns", rotation_rad / TAU)
	_panorama_material.set_shader_parameter("ref_y", ref_y)
	_panorama_material.set_shader_parameter("radius", radius)
	_panorama_material.set_shader_parameter("bottom_deg", bottom)
	_panorama_material.set_shader_parameter("min_angle_deg", float(pano_cfg.get("min_angle_deg", 0.55)))
	_panorama_material.set_shader_parameter("max_angle_deg", float(pano_cfg.get("max_angle_deg", 7.5)))
	_panorama_material.set_shader_parameter("exaggeration", float(pano_cfg.get("exaggeration", 1.35)))
	_panorama_material.set_shader_parameter("sea_threshold", float(pano_cfg.get("sea_share", 0.55)))
	_panorama_material.set_shader_parameter("haze", float((pano_cfg.get("haze", {}) as Dictionary).get(weather, 0.6)))
	_panorama_material.set_shader_parameter("exposure", float((pano_cfg.get("exposure", {}) as Dictionary).get(weather, 1.0)))
	if texture != null and not meta.is_empty():
		_panorama_material.set_shader_parameter("painting", texture)
		_panorama_material.set_shader_parameter("has_painting", 1.0)
		_panorama_material.set_shader_parameter("repeats", float(entry.get("repeats", 4)))
		_panorama_material.set_shader_parameter("paint_crest", _mean(meta.get("skyline", [0.2])))
		_panorama_material.set_shader_parameter("paint_skyline", _row_texture(meta.get("skyline", [0.2])))
		_panorama_material.set_shader_parameter("paint_detail", float(pano_cfg.get("paint_detail", 0.3)))
		var rng := RandomNumberGenerator.new()
		rng.seed = hash(province)
		_panorama_material.set_shader_parameter("paint_offset", rng.randf())
	_panorama = MeshInstance3D.new()
	_panorama.name = "Panorama"
	_panorama.mesh = mesh
	_panorama.material_override = _panorama_material
	_panorama.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	_panorama.custom_aabb = AABB(Vector3(-60000, -20000, -60000), Vector3(120000, 40000, 120000))
	_panorama.gi_mode = GeometryInstance3D.GI_MODE_DISABLED
	add_child(_panorama)


static func _panorama_meta(id: String) -> Dictionary:
	if not FileAccess.file_exists(PANORAMA_META):
		return {}
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(PANORAMA_META))
	if not (parsed is Dictionary):
		return {}
	return ((parsed as Dictionary).get("panoramas", {}) as Dictionary).get(id, {})


static func _row_texture(values: Array) -> ImageTexture:
	var image := Image.create(maxi(values.size(), 1), 1, false, Image.FORMAT_RF)
	for k in values.size():
		image.set_pixel(k, 0, Color(float(values[k]), 0.0, 0.0))
	return ImageTexture.create_from_image(image)


static func _mean(values: Array) -> float:
	var total := 0.0
	for v in values:
		total += float(v)
	return total / maxf(float(values.size()), 1.0)


## Profil réel en texture 1D (R = angle en degrés, G = distance en km, B = part de mer).
func _skyline_texture() -> ImageTexture:
	var p := _skyline.size()
	var image := Image.create(p, 1, false, Image.FORMAT_RGBAF)
	for k in p:
		image.set_pixel(k, 0, Color(_skyline[k], _skyline_dist[k] / 1000.0, _sea_share[k], 1.0))
	return ImageTexture.create_from_image(image)


# --- Silhouettes --------------------------------------------------------------------------


## Clochers de villages, château sur une hauteur, fumées : maillages simples sans ombre,
## fondus par le brouillard de distance, posés sur l'anneau lointain (jamais dans la mer).
func _build_silhouettes(terrain: BattleTerrain, far_rect: Rect2, weather: String) -> void:
	var cfg: Dictionary = _cfg.get("silhouettes", {})
	if cfg.is_empty():
		return
	var rng := RandomNumberGenerator.new()
	rng.seed = hash(province + "silhouettes")
	var dmin := float(cfg.get("min_distance_m", 2600.0))
	var dmax := float(cfg.get("max_distance_m", 8500.0))
	var scale := float(cfg.get("scale", 1.3))
	var stone := _rgb(cfg.get("stone"), Color(0.36, 0.34, 0.31))
	var roof := _rgb(cfg.get("roof"), Color(0.3, 0.22, 0.18))
	if terrain.snowy():
		roof = Color(0.85, 0.87, 0.9)
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var villages := _range_count(rng, cfg.get("villages", [2, 5]))
	var sites: Array[Vector3] = []
	for _v in villages:
		var p := _pick_site(terrain, rng, dmin, dmax, far_rect, false)
		if p == Vector3.INF:
			continue
		sites.append(p)
		_add_church(st, p, rng.randf() * TAU, scale, stone, roof)
		for _h in rng.randi_range(4, 8):
			var off := Vector2.from_angle(rng.randf() * TAU) * rng.randf_range(25.0, 90.0)
			var q := Vector3(p.x + off.x, 0.0, p.z + off.y)
			q.y = terrain.world_height(q.x, q.z)
			_add_house(st, q, rng.randf() * TAU, scale * rng.randf_range(0.8, 1.2), stone.lightened(0.15), roof)
		silhouette_count += 1
	for _c in _range_count(rng, cfg.get("castles", [0, 1])):
		var p := _pick_site(terrain, rng, dmin, dmax, far_rect, true)
		if p == Vector3.INF:
			continue
		_add_castle(st, p, rng.randf() * TAU, scale, stone)
		sites.append(p)
		silhouette_count += 1
	st.generate_normals()
	var mesh := st.commit()
	if mesh.get_surface_count() > 0:
		var mat := StandardMaterial3D.new()
		mat.vertex_color_use_as_albedo = true
		mat.roughness = 0.95
		var instance := MeshInstance3D.new()
		instance.name = "Silhouettes"
		instance.mesh = mesh
		instance.material_override = mat
		instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		add_child(instance)
	# Fumées de villages (pas sous la pluie ni la neige, qui les rabattent).
	if weather in ["rain", "snow"] or sites.is_empty():
		return
	_smoke_material = ShaderMaterial.new()
	_smoke_material.shader = SMOKE_SHADER
	_smoke_material.set_shader_parameter("smoke_color", _rgb(cfg.get("smoke"), Color(0.62, 0.62, 0.64)))
	for s in _range_count(rng, cfg.get("smokes", [1, 4])):
		var p: Vector3 = sites[rng.randi() % sites.size()]
		var quad := QuadMesh.new()
		quad.size = Vector2(70.0, 420.0)
		quad.center_offset = Vector3(0, 210.0, 0)
		var smoke := MeshInstance3D.new()
		smoke.name = "Smoke%d" % s
		smoke.mesh = quad
		smoke.material_override = _smoke_material
		smoke.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		smoke.position = p + Vector3(rng.randf_range(-40, 40), 5.0, rng.randf_range(-40, 40))
		smoke.set_instance_shader_parameter("seed", rng.randf() * 10.0)
		add_child(smoke)


static func _range_count(rng: RandomNumberGenerator, bounds: Variant) -> int:
	if bounds is Array and (bounds as Array).size() >= 2:
		return rng.randi_range(int(bounds[0]), int(bounds[1]))
	return 0


## Site au sol entre `dmin` et `dmax` du champ ; `high` : le plus haut de plusieurs essais.
func _pick_site(terrain: BattleTerrain, rng: RandomNumberGenerator, dmin: float, dmax: float, far_rect: Rect2, high: bool) -> Vector3:
	var best := Vector3.INF
	for _attempt in (40 if high else 12):
		var a := rng.randf() * TAU
		var d := rng.randf_range(dmin, dmax)
		var p := centre + Vector2(sin(a), -cos(a)) * (d + field_size.length() * 0.5)
		if not far_rect.has_point(p) or is_sea(p.x, p.y) or terrain._in_sea(p.x, p.y):
			continue
		var h := terrain.world_height(p.x, p.y)
		var slope := absf(terrain.world_height(p.x + 30.0, p.y) - terrain.world_height(p.x - 30.0, p.y)) / 60.0
		if slope > 0.25 or h < offset_y + 1.0:
			continue
		if best == Vector3.INF or (high and h > best.y):
			best = Vector3(p.x, h, p.y)
			if not high:
				break
	return best


static func _box(st: SurfaceTool, base: Vector3, yaw: float, size: Vector3, color: Color) -> void:
	var b := Basis(Vector3.UP, yaw)
	var hx := size.x * 0.5
	var hz := size.z * 0.5
	var c := [Vector3(-hx, 0, -hz), Vector3(hx, 0, -hz), Vector3(hx, 0, hz), Vector3(-hx, 0, hz)]
	var sink := Vector3(0, -3.0, 0)
	st.set_color(color)
	for k in 4:
		var a: Vector3 = base + b * c[k] + sink
		var d: Vector3 = base + b * c[(k + 1) % 4] + sink
		var a2: Vector3 = base + b * c[k] + Vector3(0, size.y, 0)
		var d2: Vector3 = base + b * c[(k + 1) % 4] + Vector3(0, size.y, 0)
		st.add_vertex(a)
		st.add_vertex(a2)
		st.add_vertex(d)
		st.add_vertex(d)
		st.add_vertex(a2)
		st.add_vertex(d2)
	var top := [base + b * c[0] + Vector3(0, size.y, 0), base + b * c[1] + Vector3(0, size.y, 0), base + b * c[2] + Vector3(0, size.y, 0), base + b * c[3] + Vector3(0, size.y, 0)]
	st.add_vertex(top[0])
	st.add_vertex(top[2])
	st.add_vertex(top[1])
	st.add_vertex(top[0])
	st.add_vertex(top[3])
	st.add_vertex(top[2])


## Toit à deux pans (prisme) ou flèche (pyramide si `ridge` = 0).
static func _roof(st: SurfaceTool, base: Vector3, yaw: float, size: Vector3, ridge: float, color: Color) -> void:
	var b := Basis(Vector3.UP, yaw)
	var hx := size.x * 0.5
	var hz := size.z * 0.5
	var r := ridge * 0.5
	var p0: Vector3 = base + b * Vector3(-hx, 0, -hz)
	var p1: Vector3 = base + b * Vector3(hx, 0, -hz)
	var p2: Vector3 = base + b * Vector3(hx, 0, hz)
	var p3: Vector3 = base + b * Vector3(-hx, 0, hz)
	var t0: Vector3 = base + b * Vector3(-r, size.y, 0)
	var t1: Vector3 = base + b * Vector3(r, size.y, 0)
	st.set_color(color)
	for tri in [[p0, t0, p1], [p1, t0, t1], [p2, t1, p3], [p3, t1, t0], [p1, t1, p2], [p3, t0, p0]]:
		st.add_vertex(tri[0])
		st.add_vertex(tri[1])
		st.add_vertex(tri[2])


static func _add_church(st: SurfaceTool, p: Vector3, yaw: float, s: float, stone: Color, roof: Color) -> void:
	_box(st, p, yaw, Vector3(24, 11, 10) * s, stone)
	_roof(st, p + Vector3(0, 11 * s, 0), yaw, Vector3(24, 7, 10) * s, 24 * s, roof)
	var tower := p + Basis(Vector3.UP, yaw) * Vector3(-14 * s, 0, 0)
	_box(st, tower, yaw, Vector3(6, 24, 6) * s, stone)
	_roof(st, tower + Vector3(0, 24 * s, 0), yaw, Vector3(6, 16, 6) * s, 0.0, roof.darkened(0.2))


static func _add_house(st: SurfaceTool, p: Vector3, yaw: float, s: float, wall: Color, roof: Color) -> void:
	_box(st, p, yaw, Vector3(10, 5, 6) * s, wall)
	_roof(st, p + Vector3(0, 5 * s, 0), yaw, Vector3(10, 4.5, 6) * s, 10 * s, roof)


static func _add_castle(st: SurfaceTool, p: Vector3, yaw: float, s: float, stone: Color) -> void:
	var b := Basis(Vector3.UP, yaw)
	_box(st, p, yaw, Vector3(14, 32, 14) * s, stone.darkened(0.08))
	for k in 4:
		var corner: Vector3 = p + b * (Vector3(1 if k % 2 == 0 else -1, 0, 1 if k < 2 else -1) * 34.0 * s)
		_box(st, corner, yaw, Vector3(9, 20, 9) * s, stone)
	for k in 4:
		var side: Vector3 = b * Vector3(0, 0, 34.0 * s).rotated(Vector3.UP, k * PI * 0.5)
		var along := yaw + k * PI * 0.5
		_box(st, p + side, along, Vector3(66, 12, 3) * s, stone)


# --- Atmosphère ---------------------------------------------------------------------------


## Après `BattleAtmosphere.apply` : densité du brouillard propre à l'horizon (visibilité
## lointaine) et teintes du ciel transmises à l'anneau, au panorama et aux fumées.
func apply_atmosphere(env: Environment, sun: DirectionalLight3D, weather: String) -> void:
	if not active or env == null:
		return
	var haze: Dictionary = _cfg.get("haze", {})
	var densities: Dictionary = haze.get("fog_density", {})
	if densities.has(weather):
		env.fog_density = float(densities[weather])
	var fog_color := env.fog_light_color * env.fog_light_energy
	var light := Color.WHITE
	if sun != null:
		var energy := clampf(sun.light_energy / 1.75, 0.45, 1.1)
		light = Color.WHITE.lerp(sun.light_color, 0.5) * energy
	if _land_material != null:
		_land_material.set_shader_parameter("fog_color", fog_color)
		_land_material.set_shader_parameter("fog_density", env.fog_density)
		_land_material.set_shader_parameter("full_density_m", float(haze.get("full_density_m", 6000.0)))
		_land_material.set_shader_parameter("altitude_scale", float(haze.get("altitude_scale_m", 1400.0)))
		_land_material.set_shader_parameter("ref_y", offset_y + maxf(ref_m, 0.0))
		_land_material.set_shader_parameter("haze_scale", float(haze.get("ring_scale", 0.6)))
		_land_material.set_shader_parameter("haze_tint", float(haze.get("ring_tint", 0.9)))
	if _panorama_material != null:
		_panorama_material.set_shader_parameter("fog_color", fog_color)
		_panorama_material.set_shader_parameter("fog_density", env.fog_density)
		_panorama_material.set_shader_parameter("light_tint", light)
	if _smoke_material != null:
		_smoke_material.set_shader_parameter("light_tint", light)


static func _rgb(values: Variant, fallback: Color) -> Color:
	if values is Array and (values as Array).size() >= 3:
		return Color(float(values[0]), float(values[1]), float(values[2]))
	return fallback
