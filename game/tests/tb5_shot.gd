extends SceneTree

## Lot TB5 (mer et côtes) : captures et mesures des côtes et des mers de la carte de campagne.
## Usage : godot --path game --resolution 1600x900 --script res://tests/tb5_shot.gd --
##   [--out=<dossier>] [--only=<nom>,…] [--season=<saison>] [--stats] [--distances=90,40]
## Sans `--stats` : écrit `tb5-<nom>-<distance>.png` (960 px de large) par lieu et par distance.
## Avec `--stats` : aucune image écrite ; pour chaque lieu, rend la vue sans puis avec le lot
## (`coast_strength`, `basin_on`, `swash_amount` à 0 puis aux valeurs des données) et affiche la
## moyenne RVB de l'image entière et celle des seuls pixels que le lot change (part de l'image,
## couleur avant → après, puis part et couleur des pixels très changés) : la falaise de craie doit sortir claire, la Méditerranée plus claire
## que la Manche, sans lire d'image. Affiche aussi la part de l'image qui bouge en
## `MOTION_SECONDS` secondes, sans puis avec le lot (le ressac doit faire bouger le rivage).

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const WIDTH := 960
const SETTLE_FRAMES := 150
const CHANGE_THRESHOLD := 10.0 / 255.0
## Écart moyen par canal au-delà duquel un pixel est « très changé » (cœur de la bande côtière).
const STRONG_THRESHOLD := 45.0 / 255.0
## Durée entre les deux images de la mesure du mouvement (une demi-période du ressac environ).
const MOTION_SECONDS := 3.0
## Lieux (pixels carte) : côtes puis mers ouvertes.
const PLACES := {
	"douvres": Vector2(2153.0, 2840.0),
	"etretat": Vector2(2012.6, 3047.5),
	"raz": Vector2(1476.5, 3218.0),
	"landes": Vector2(1740.0, 3867.2),
	"manche": Vector2(2030.0, 2950.0),
	"mer-du-nord": Vector2(2400.0, 2500.0),
	"atlantique": Vector2(1000.0, 2800.0),
	"mediterranee": Vector2(2500.0, 4200.0),
	"mediterranee-large": Vector2(2400.0, 4500.0),
}
## Réglages coupés pour la vue « sans le lot » (matériau du terrain, matériau de la mer).
const TERRAIN_OFF := {"coast_strength": 0.0, "swash_amount": 0.0}
const SEA_OFF := {"basin_on": false, "swash_amount": 0.0}


func _init() -> void:
	var out_dir := "user://tb5"
	var only := PackedStringArray()
	var distances: Array[float] = [90.0, 40.0]
	var stats := false
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out_dir = arg.trim_prefix("--out=")
		elif arg.begins_with("--only="):
			only = arg.trim_prefix("--only=").split(",")
		elif arg.begins_with("--distances="):
			distances.clear()
			for d in arg.trim_prefix("--distances=").split(","):
				distances.append(float(d))
		elif arg == "--stats":
			stats = true
	DirAccess.make_dir_recursive_absolute(out_dir)
	await process_frame
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "tutorial/enabled", false, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 5:
		await process_frame
	if not map.get("load_ok"):
		push_error("TB5 shot: campaign map failed to load")
		quit(1)
		return
	var rig: CampaignCamera = map.camera_rig
	var data: MapData = map.get("map_data")
	for layer in root.find_children("*", "CanvasLayer", true, false):
		(layer as CanvasLayer).visible = false
	var terrain: TerrainBuilder = map.get("terrain")
	var sea := map.get("sea") as Sea
	var terrain_material: ShaderMaterial = terrain.material if terrain != null else null
	var sea_material: ShaderMaterial = sea.material_override as ShaderMaterial if sea != null else null
	var terrain_on := _values(terrain_material, TERRAIN_OFF)
	var sea_on := _values(sea_material, SEA_OFF)
	for place: String in PLACES:
		if not only.is_empty() and not only.has(place):
			continue
		var at: Vector2 = PLACES[place]
		var ground := Vector3(at.x, data.surface_world_at(at.x, at.y), at.y)
		for distance in distances:
			rig.look_at_point(ground, maxf(distance, rig.min_distance_at(ground)))
			rig.snap()
			var label := "tb5-%s-%d" % [place, int(distance)]
			if not stats:
				await _settle(map, SETTLE_FRAMES)
				_shot(out_dir, label)
				continue
			_apply(terrain_material, TERRAIN_OFF)
			_apply(sea_material, SEA_OFF)
			await _settle(map, SETTLE_FRAMES)
			var before := root.get_viewport().get_texture().get_image()
			var motion_off := await _motion(map)
			_apply(terrain_material, terrain_on)
			_apply(sea_material, sea_on)
			await _settle(map, 8)
			_stats(label, before, root.get_viewport().get_texture().get_image())
			print("TB5 motion %s over %.1f s: %.2f %% of the image without the lot, %.2f %% with it" % [label, MOTION_SECONDS, motion_off, await _motion(map)])
	quit(0)


## Valeurs actuelles (celles des données) des réglages de `off` sur `material`.
func _values(material: ShaderMaterial, off: Dictionary) -> Dictionary:
	var values := {}
	if material != null:
		for key: String in off:
			values[key] = material.get_shader_parameter(key)
	return values


func _apply(material: ShaderMaterial, values: Dictionary) -> void:
	if material != null:
		for key: String in values:
			material.set_shader_parameter(key, values[key])


## Laisse la vue se stabiliser, nuées masquées (leur ombre fait varier les moyennes).
func _settle(map: Node3D, frames: int) -> void:
	for i in frames:
		var clouds := map.find_child("Clouds", true, false) as Node3D
		if clouds != null:
			clouds.visible = false
		await process_frame


## Part de l'image (%) qui change en `MOTION_SECONDS` secondes (vagues, écume, ressac).
func _motion(map: Node3D) -> float:
	var first := root.get_viewport().get_texture().get_image()
	var until := Time.get_ticks_msec() + int(MOTION_SECONDS * 1000.0)
	while Time.get_ticks_msec() < until:
		await _settle(map, 1)
	var second := root.get_viewport().get_texture().get_image()
	var count := 0
	var moved := 0
	for y in range(0, second.get_height(), 3):
		for x in range(0, second.get_width(), 3):
			var a := first.get_pixel(x, y)
			var b := second.get_pixel(x, y)
			count += 1
			if absf(a.r - b.r) + absf(a.g - b.g) + absf(a.b - b.b) > CHANGE_THRESHOLD * 3.0:
				moved += 1
	return 100.0 * moved / count


func _shot(out_dir: String, label: String) -> void:
	var image := root.get_viewport().get_texture().get_image()
	if image.get_width() > WIDTH:
		image.resize(WIDTH, roundi(image.get_height() * float(WIDTH) / image.get_width()), Image.INTERPOLATE_LANCZOS)
	var path := out_dir.path_join("%s.png" % label)
	image.save_png(path)
	print("TB5 shot %s" % path)


## Moyenne RVB (0-255) de l'image sans et avec le lot, puis des seuls pixels changés.
func _stats(label: String, before: Image, after: Image) -> void:
	var sum_before := Vector3.ZERO
	var sum_after := Vector3.ZERO
	var changed_before := Vector3.ZERO
	var changed_after := Vector3.ZERO
	var strong_after := Vector3.ZERO
	var count := 0
	var changed := 0
	var strong := 0
	for y in range(0, after.get_height(), 3):
		for x in range(0, after.get_width(), 3):
			var a := before.get_pixel(x, y)
			var b := after.get_pixel(x, y)
			var va := Vector3(a.r, a.g, a.b)
			var vb := Vector3(b.r, b.g, b.b)
			sum_before += va
			sum_after += vb
			count += 1
			if (vb - va).abs().dot(Vector3.ONE) > CHANGE_THRESHOLD * 3.0:
				changed_before += va
				changed_after += vb
				changed += 1
			if (vb - va).abs().dot(Vector3.ONE) > STRONG_THRESHOLD * 3.0:
				strong_after += vb
				strong += 1
	var mean_before := sum_before / count * 255.0
	var mean_after := sum_after / count * 255.0
	var line := "TB5 stats %s image %.0f %.0f %.0f -> %.0f %.0f %.0f" % [label, mean_before.x, mean_before.y, mean_before.z, mean_after.x, mean_after.y, mean_after.z]
	if changed > 0:
		var cb := changed_before / changed * 255.0
		var ca := changed_after / changed * 255.0
		line += " ; changed %.1f %% : %.0f %.0f %.0f -> %.0f %.0f %.0f" % [100.0 * changed / count, cb.x, cb.y, cb.z, ca.x, ca.y, ca.z]
	else:
		line += " ; changed 0 %"
	if strong > 0:
		var sa := strong_after / strong * 255.0
		line += " ; strongly changed %.2f %% -> %.0f %.0f %.0f" % [100.0 * strong / count, sa.x, sa.y, sa.z]
	print(line)
