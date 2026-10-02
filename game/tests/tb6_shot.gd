extends SceneTree

## Lot TB6 : captures et mesures de la lumière de la carte de campagne (ombres de nuages, lumière
## dorée, brume du matin). Écrit un fichier par vue, sans le lire ; les chiffres sortent en texte.
## Usage : godot --path game --resolution 1600x900 --script res://tests/tb6_shot.gd --
##   [--out=<dossier>] [--prefix=<p>] [--at=<x>,<z>] [--distances=1100,400,90]
##   [--season=summer|autumn|winter|spring] [--map-weather=clear|fog|rain|snow|storm]
##   [--select] (sélectionne la province visée : la brume doit l'épargner)
##   [--no-shots] (mesures seulement)
##   [--ab] (pour chaque vue, écart de luminance du sol avec / sans chaque calque : nuées, météo
##   du sol, ombres de nuages, ombres du soleil, brume ; même image, même instant)
## Par vue : `TB6 view …` (météo au point visé, opacité des nuées, soleil, brouillard) et
## `TB6 stats …` (luminance du sol émergé : moyenne, écart-type, part des blocs très sombres).

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const PARIS := Vector2(2213.2, 3203.9)
const WIDTH := 960
const SETTLE_FRAMES := 150
const AB_FRAMES := 12
const BLOCK := 16


func _init() -> void:
	var out_dir := "user://tb6"
	var prefix := ""
	var at := PARIS
	var distances: Array[float] = [1100.0, 400.0, 90.0]
	var shots := true
	var ab := false
	var select := false
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out_dir = arg.trim_prefix("--out=")
		elif arg.begins_with("--prefix="):
			prefix = arg.trim_prefix("--prefix=")
		elif arg.begins_with("--at="):
			var xz := arg.trim_prefix("--at=").split(",")
			at = Vector2(float(xz[0]), float(xz[1]))
		elif arg.begins_with("--distances="):
			distances.clear()
			for d in arg.trim_prefix("--distances=").split(","):
				distances.append(float(d))
		elif arg == "--no-shots":
			shots = false
		elif arg == "--ab":
			ab = true
		elif arg == "--select":
			select = true
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
		push_error("TB6 shot: campaign map failed to load")
		quit(1)
		return
	var rig: CampaignCamera = map.camera_rig
	var data: MapData = map.get("map_data")
	for layer in root.find_children("*", "CanvasLayer", true, false):
		(layer as CanvasLayer).visible = false
	var ground := Vector3(at.x, data.surface_world_at(at.x, at.y), at.y)
	if select:
		var index := data.province_index_at(at.x, at.y)
		map.set("selected_index", index)
		var terrain: TerrainBuilder = map.get("terrain")
		if terrain != null:
			terrain.set_highlight(0, index)
	for distance in distances:
		rig.look_at_point(ground, maxf(distance, rig.min_distance_at(ground)))
		rig.snap()
		for i in SETTLE_FRAMES:
			await process_frame
		var name := "%stb6-%d" % [prefix, int(distance)]
		_report(map, name, at)
		var image := root.get_viewport().get_texture().get_image()
		var land := _land_blocks(map, data, image)
		_stats(name, _block_luminance(image), land)
		if shots:
			_shot(out_dir, name, image)
		if ab:
			await _ab(map, name, land)
	quit(0)


func _shot(out_dir: String, name: String, image: Image) -> void:
	if image.get_width() > WIDTH:
		image.resize(WIDTH, roundi(image.get_height() * float(WIDTH) / image.get_width()), Image.INTERPOLATE_LANCZOS)
	var path := out_dir.path_join("%s.png" % name)
	image.save_png(path)
	print("TB6 shot %s" % path)


## État appliqué à la vue : météo du cœur au point visé, nuées, ombres, soleil, brouillard.
func _report(map: Node3D, name: String, at: Vector2) -> void:
	var view: CampaignWeatherView = map.get("weather_view")
	var counts := {}
	for id in view.weather:
		var kind := str((view.weather[id] as Dictionary).get("kind", "clear"))
		counts[kind] = int(counts.get(kind, 0)) + 1
	var terrain: TerrainBuilder = map.get("terrain")
	var material: ShaderMaterial = terrain.material
	var sun := map.get_node_or_null("Sun") as DirectionalLight3D
	var env := (map.get_node_or_null("WorldEnvironment") as WorldEnvironment).environment
	var elevation := rad_to_deg(asin(clampf(sun.global_basis.z.y, -1.0, 1.0)))
	print("TB6 view %s weather_at_focus %s provinces %s cloud_alpha %.2f cloud_shadow %.2f mist_height %s" % [
		name, view.weather_at(at), counts, view.cloud_alpha_at(map.camera_rig.distance),
		float(material.get_shader_parameter("cloud_shadow_amount")), material.get_shader_parameter("weather_mist_height")])
	print("TB6 light %s sun_elevation %.1f sun_color %s sun_energy %.2f shadows %s shadow_range %.0f fog %s density %.4f ssil %s sdfgi %s volumetric %s" % [
		name, elevation, sun.light_color.to_html(false), sun.light_energy, sun.shadow_enabled,
		sun.directional_shadow_max_distance, env.fog_enabled, env.fog_density, env.ssil_enabled,
		env.sdfgi_enabled, env.volumetric_fog_enabled])


## Luminance moyenne par bloc de `BLOCK` pixels (tableau ligne par ligne).
func _block_luminance(image: Image) -> PackedFloat32Array:
	var bw := image.get_width() / BLOCK
	var bh := image.get_height() / BLOCK
	var out := PackedFloat32Array()
	out.resize(bw * bh)
	for by in bh:
		for bx in bw:
			var sum := 0.0
			for sy in range(0, BLOCK, 4):
				for sx in range(0, BLOCK, 4):
					sum += image.get_pixel(bx * BLOCK + sx, by * BLOCK + sy).get_luminance()
			out[by * bw + bx] = sum / 16.0
	return out


## Blocs dont le centre tombe sur la terre émergée (rayon caméra → plan du sol) : la mer, animée
## et très sombre, fausserait les écarts.
func _land_blocks(map: Node3D, data: MapData, image: Image) -> PackedByteArray:
	var camera := map.get_viewport().get_camera_3d()
	var bw := image.get_width() / BLOCK
	var bh := image.get_height() / BLOCK
	var view_size := Vector2(map.get_viewport().get_visible_rect().size)
	var scale := view_size / Vector2(image.get_width(), image.get_height())
	var out := PackedByteArray()
	out.resize(bw * bh)
	for by in bh:
		for bx in bw:
			var screen := Vector2((bx + 0.5) * BLOCK, (by + 0.5) * BLOCK) * scale
			var origin := camera.project_ray_origin(screen)
			var dir := camera.project_ray_normal(screen)
			if dir.y >= -0.01:
				continue
			var hit := origin + dir * (-(origin.y - 1.0) / dir.y)
			if hit.x < 0.0 or hit.z < 0.0 or hit.x >= data.size.x or hit.z >= data.size.y:
				continue
			out[by * bw + bx] = 1 if data.is_land_px(int(hit.x), int(hit.z)) else 0
	return out


## Moyenne et écart-type de la luminance du sol émergé ; part des blocs sous 60 % de la médiane
## (taches sombres) ; plus fort saut entre deux blocs voisins (bord net), en part de la médiane.
func _stats(name: String, lum: PackedFloat32Array, land: PackedByteArray) -> void:
	var values: Array[float] = []
	for i in lum.size():
		if land[i] == 1:
			values.append(lum[i])
	if values.is_empty():
		print("TB6 stats %s no land" % name)
		return
	var sum := 0.0
	var sq := 0.0
	for v in values:
		sum += v
		sq += v * v
	var mean := sum / values.size()
	var sd := sqrt(maxf(sq / values.size() - mean * mean, 0.0))
	values.sort()
	var median := values[values.size() / 2]
	var dark := 0
	for v in values:
		if v < median * 0.6:
			dark += 1
	print("TB6 stats %s land_blocks %d mean %.1f sd %.1f median %.1f dark_blocks %.1f%%" % [
		name, values.size(), mean * 255.0, sd * 255.0, median * 255.0, 100.0 * dark / values.size()])


## Écart avec / sans chaque calque, sur la même vue : rapport de luminance par bloc de sol
## (avec / sans). Un calque sobre reste au-dessus de 0,85 au 1er centile.
func _ab(map: Node3D, name: String, land: PackedByteArray) -> void:
	var view: CampaignWeatherView = map.get("weather_view")
	var terrain: TerrainBuilder = map.get("terrain")
	var material: ShaderMaterial = terrain.material
	var sun := map.get_node_or_null("Sun") as DirectionalLight3D
	var env := (map.get_node_or_null("WorldEnvironment") as WorldEnvironment).environment
	var with := _block_luminance(root.get_viewport().get_texture().get_image())
	var saved := {
		"medium": view.cloud_medium_alpha, "max": view.cloud_max_alpha,
		"weather": material.get_shader_parameter("weather_enabled"),
		"shadow": material.get_shader_parameter("cloud_shadow_amount"),
		"sun": sun.shadow_enabled,
		"mist": material.get_shader_parameter("weather_mist_max"),
		"fow": material.get_shader_parameter("fog_enabled"),
		"ssao": env.ssao_enabled, "ssil": env.ssil_enabled, "fog": env.fog_enabled,
	}
	for layer: String in ["clouds", "ground_weather", "cloud_shadows", "sun_shadows", "mist", "fog_of_war", "ssao", "ssil", "depth_fog"]:
		match layer:
			"clouds":
				view.cloud_medium_alpha = 0.0
				view.cloud_max_alpha = 0.0
			"ground_weather":
				material.set_shader_parameter("weather_enabled", false)
			"cloud_shadows":
				material.set_shader_parameter("cloud_shadow_amount", 0.0)
			"sun_shadows":
				sun.shadow_enabled = false
			"mist":
				material.set_shader_parameter("weather_mist_max", 0.0)
			"fog_of_war":
				material.set_shader_parameter("fog_enabled", false)
			"ssao":
				env.ssao_enabled = false
			"ssil":
				env.ssil_enabled = false
			"depth_fog":
				env.fog_enabled = false
		for i in AB_FRAMES:
			if layer == "cloud_shadows":  # la vue météo repose la valeur quand elle change
				material.set_shader_parameter("cloud_shadow_amount", 0.0)
			await process_frame
		var without := _block_luminance(root.get_viewport().get_texture().get_image())
		view.cloud_medium_alpha = saved["medium"]
		view.cloud_max_alpha = saved["max"]
		material.set_shader_parameter("weather_enabled", saved["weather"])
		material.set_shader_parameter("cloud_shadow_amount", saved["shadow"])
		material.set_shader_parameter("weather_mist_max", saved["mist"])
		sun.shadow_enabled = saved["sun"]
		material.set_shader_parameter("fog_enabled", saved["fow"])
		env.ssao_enabled = saved["ssao"]
		env.ssil_enabled = saved["ssil"]
		env.fog_enabled = saved["fog"]
		var ratios: Array[float] = []
		for i in with.size():
			if land[i] == 1 and without[i] > 0.02:
				ratios.append(with[i] / without[i])
		if ratios.is_empty():
			continue
		ratios.sort()
		var darker := 0
		for r in ratios:
			if r < 0.85:
				darker += 1
		print("TB6 ab %s %s ratio p1 %.2f p10 %.2f p50 %.2f p99 %.2f darker_15pct %.1f%%" % [
			name, layer, ratios[ratios.size() / 100], ratios[ratios.size() / 10], ratios[ratios.size() / 2],
			ratios[ratios.size() * 99 / 100], 100.0 * darker / ratios.size()])
		for i in AB_FRAMES:
			await process_frame
