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
##   [--bench] (durée d'image en boucle serrée, sans plafond du compositeur : SSIL coupé / actif,
##   SDFGI, brume du matin, brouillard volumétrique ; configurations alternées sur plusieurs tours
##   pour lisser la charge de la machine ; lancer avec `--disable-vsync`, et `--map-weather=fog`
##   pour mesurer la brume)
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
	var bench := false
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
		elif arg == "--bench":
			bench = true
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
		if bench:
			await _bench(map, name)
			continue
		_report(map, name, at)
		var image := _grab()
		var land := _land_blocks(map, data, image)
		_stats(name, _block_luminance(image), land)
		_color_stats(name, image, land)
		if shots:
			_shot(out_dir, name, image)
		if ab:
			await _ab(map, name, land)
	quit(0)


## Image du viewport après un rendu forcé : une fenêtre occultée par une autre n'est plus
## redessinée par la boucle principale, et la capture resterait figée.
func _grab() -> Image:
	RenderingServer.force_draw(false)
	return root.get_viewport().get_texture().get_image()


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
	print("TB6 view %s weather_at_focus %s provinces %s cloud_alpha %.2f cloud_shadow %.2f valley_mist %s" % [
		name, view.weather_at(at), counts, view.cloud_alpha_at(map.camera_rig.distance),
		float(material.get_shader_parameter("cloud_shadow_amount")), material.get_shader_parameter("weather_valley_mist")])
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
	var selected := int(map.get("selected_index"))
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
			if data.is_land_px(int(hit.x), int(hit.z)):
				# 2 : sol de la province sélectionnée (la brume doit l'épargner).
				out[by * bw + bx] = 2 if selected > 0 and data.province_index_at(hit.x, hit.z) == selected else 1
	return out


## Moyenne et écart-type de la luminance du sol émergé ; part des blocs sous 60 % de la médiane
## (taches sombres) ; plus fort saut entre deux blocs voisins (bord net), en part de la médiane.
func _stats(name: String, lum: PackedFloat32Array, land: PackedByteArray) -> void:
	var values: Array[float] = []
	for i in lum.size():
		if land[i] >= 1:
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
	var very_dark := 0
	for v in values:
		if v < median * 0.6:
			dark += 1
		if v < median * 0.4:
			very_dark += 1
	print("TB6 stats %s land_blocks %d mean %.1f sd %.1f median %.1f p5 %.1f dark_blocks %.1f%% below_40pct_of_median %.1f%%" % [
		name, values.size(), mean * 255.0, sd * 255.0, median * 255.0, values[values.size() / 20] * 255.0,
		100.0 * dark / values.size(), 100.0 * very_dark / values.size()])


## Teinte et saturation (HSV, couleurs affichées) du sol émergé : de la couleur moyenne, et
## saturation moyenne par bloc. Sert à comparer les saisons sans lire l'image.
func _color_stats(name: String, image: Image, land: PackedByteArray) -> void:
	var bw := image.get_width() / BLOCK
	var sum := Vector3.ZERO
	var saturation := 0.0
	var count := 0
	for i in land.size():
		if land[i] == 0:
			continue
		var c := image.get_pixel((i % bw) * BLOCK + BLOCK / 2, (i / bw) * BLOCK + BLOCK / 2)
		sum += Vector3(c.r, c.g, c.b)
		saturation += c.s
		count += 1
	if count == 0:
		return
	var mean := Color(sum.x / count, sum.y / count, sum.z / count)
	print("TB6 color %s mean_rgb %d %d %d hue %.0f deg saturation_of_mean %.3f mean_block_saturation %.3f" % [
		name, roundi(mean.r * 255.0), roundi(mean.g * 255.0), roundi(mean.b * 255.0), mean.h * 360.0, mean.s, saturation / count])


## Écart avec / sans chaque calque, sur la même vue : rapport de luminance par bloc de sol
## (avec / sans). Un calque sobre reste au-dessus de 0,85 au 1er centile.
func _ab(map: Node3D, name: String, land: PackedByteArray) -> void:
	var view: CampaignWeatherView = map.get("weather_view")
	var terrain: TerrainBuilder = map.get("terrain")
	var material: ShaderMaterial = terrain.material
	var sun := map.get_node_or_null("Sun") as DirectionalLight3D
	var env := (map.get_node_or_null("WorldEnvironment") as WorldEnvironment).environment
	var trees: Array[Node3D] = []
	for node in map.find_children("*", "Node3D", true, false):
		if node is Vegetation and (node as Node3D).visible:
			trees.append(node)
	print("TB6 ab %s tree nodes %d, forest floor %s" % [name, trees.size(), material.get_shader_parameter("sg_dark_floor")])
	var saved := {
		"medium": view.cloud_medium_alpha, "max": view.cloud_max_alpha,
		"weather": material.get_shader_parameter("weather_enabled"),
		"shadow": material.get_shader_parameter("cloud_shadow_amount"),
		"sun": sun.shadow_enabled,
		"mist": material.get_shader_parameter("weather_mist_max"),
		"valley": material.get_shader_parameter("weather_valley_mist"),
		"fow": material.get_shader_parameter("fog_enabled"),
		"floor": material.get_shader_parameter("sg_dark_floor"),
		"sg": material.get_shader_parameter("sg_strength"), "hb": material.get_shader_parameter("hb_strength"),
		"ssao": env.ssao_enabled, "ssil": env.ssil_enabled, "fog": env.fog_enabled,
	}
	for layer: String in ["clouds", "ground_weather", "cloud_shadows", "sun_shadows", "mist", "valley_mist", "colormap", "forest_floor", "biome_ground", "trees", "fog_of_war", "ssao", "ssil", "depth_fog"]:
		# Image de référence reprise avant chaque calque : la dérive (vent, figurants) ne compte pas.
		var with := _block_luminance(_grab())
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
				material.set_shader_parameter("weather_valley_mist", 0.0)
			"valley_mist":
				material.set_shader_parameter("weather_valley_mist", 0.0)
			"fog_of_war":
				material.set_shader_parameter("fog_enabled", false)
			"colormap":
				material.set_shader_parameter("sg_strength", 0.0)
			"forest_floor":
				material.set_shader_parameter("sg_dark_floor", 0.0)
			"biome_ground":
				material.set_shader_parameter("hb_strength", 0.0)
			"trees":
				for node: Node3D in trees:
					node.visible = false
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
		var without := _block_luminance(_grab())
		view.cloud_medium_alpha = saved["medium"]
		view.cloud_max_alpha = saved["max"]
		material.set_shader_parameter("weather_enabled", saved["weather"])
		material.set_shader_parameter("cloud_shadow_amount", saved["shadow"])
		material.set_shader_parameter("weather_mist_max", saved["mist"])
		material.set_shader_parameter("weather_valley_mist", saved["valley"])
		sun.shadow_enabled = saved["sun"]
		material.set_shader_parameter("sg_strength", saved["sg"])
		material.set_shader_parameter("sg_dark_floor", saved["floor"])
		material.set_shader_parameter("hb_strength", saved["hb"])
		for node: Node3D in trees:
			node.visible = true
		material.set_shader_parameter("fog_enabled", saved["fow"] if saved["fow"] != null else true)
		env.ssao_enabled = saved["ssao"]
		env.ssil_enabled = saved["ssil"]
		env.fog_enabled = saved["fog"]
		# Part des blocs de sol très sombres (< 40 % de la médiane de la vue complète) sans ce calque.
		var with_sorted: Array[float] = []
		for i in with.size():
			if land[i] >= 1:
				with_sorted.append(with[i])
		with_sorted.sort()
		var threshold := with_sorted[with_sorted.size() / 2] * 0.4
		var dark_with := 0
		var dark_without := 0
		for i in with.size():
			if land[i] >= 1:
				dark_with += 1 if with[i] < threshold else 0
				dark_without += 1 if without[i] < threshold else 0
		print("TB6 ab %s %s very dark blocks %.1f%% with, %.1f%% without" % [name, layer, 100.0 * dark_with / with_sorted.size(), 100.0 * dark_without / with_sorted.size()])
		var ratios: Array[float] = []
		var selected_sum := 0.0
		var selected_count := 0
		var selected_changed := 0
		for i in with.size():
			if land[i] >= 1 and without[i] > 0.02:
				ratios.append(with[i] / without[i])
				if land[i] == 2:
					selected_sum += with[i] / without[i]
					selected_count += 1
					if absf(with[i] / without[i] - 1.0) > 0.1:
						selected_changed += 1
		if ratios.is_empty():
			continue
		ratios.sort()
		var darker := 0
		var lighter := 0
		for r in ratios:
			if r < 0.85:
				darker += 1
			elif r > 1.1:
				lighter += 1
		print("TB6 ab %s %s ratio p1 %.2f p10 %.2f p50 %.2f p90 %.2f p99 %.2f darker_15pct %.1f%% lighter_10pct %.1f%%" % [
			name, layer, ratios[ratios.size() / 100], ratios[ratios.size() / 10], ratios[ratios.size() / 2],
			ratios[ratios.size() * 9 / 10], ratios[ratios.size() * 99 / 100], 100.0 * darker / ratios.size(), 100.0 * lighter / ratios.size()])
		if selected_count > 0:
			print("TB6 ab %s %s selected province: %d blocks, mean ratio %.3f, changed_10pct %.1f%%" % [
				name, layer, selected_count, selected_sum / selected_count, 100.0 * selected_changed / selected_count])
		for i in AB_FRAMES:
			await process_frame


## Coût par configuration : durée d'une image rendue en boucle serrée (`force_draw`, sans vsync ni
## plafond d'images : le compositeur ne borne pas la mesure comme il borne `process_frame`).
## Configurations alternées sur `BENCH_ROUNDS` tours ; médiane des tours (la machine est chargée).
const BENCH_ROUNDS := 7
const BENCH_DRAWS := 90


func _bench(map: Node3D, name: String) -> void:
	var env := (map.get_node_or_null("WorldEnvironment") as WorldEnvironment).environment
	var terrain: TerrainBuilder = map.get("terrain")
	var material: ShaderMaterial = terrain.material
	DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
	Engine.max_fps = 0
	var saved := {"ssil": env.ssil_enabled, "sdfgi": env.sdfgi_enabled, "mist": material.get_shader_parameter("weather_valley_mist")}
	var configs := ["ssil_off", "ssil_on", "ssil_sdfgi", "mist_off", "volumetric"]
	var wall := {}
	for config: String in configs:
		wall[config] = []
	for round in BENCH_ROUNDS:
		for config: String in configs:
			env.ssil_enabled = config != "ssil_off"
			env.sdfgi_enabled = config == "ssil_sdfgi"
			env.volumetric_fog_enabled = config == "volumetric"
			if config == "mist_off":
				material.set_shader_parameter("weather_valley_mist", 0.0)
			for i in 20:
				await process_frame
			for i in 10:
				RenderingServer.force_draw(false)
			var t0 := Time.get_ticks_usec()
			for i in BENCH_DRAWS:
				RenderingServer.force_draw(false)
			(wall[config] as Array).append((Time.get_ticks_usec() - t0) / (1000.0 * BENCH_DRAWS))
			material.set_shader_parameter("weather_valley_mist", saved["mist"])
	env.ssil_enabled = saved["ssil"]
	env.sdfgi_enabled = saved["sdfgi"]
	env.volumetric_fog_enabled = false
	var best := {}
	for config: String in configs:
		var series: Array = (wall[config] as Array).duplicate()
		series.sort()
		best[config] = series[0]
		print("TB6 bench %s %s median %.2f ms/frame, best %.2f (rounds %s)" % [name, config, series[series.size() / 2], series[0], _fmt(wall[config])])
	# Coût d'un effet : écart apparié dans chaque tour (médiane des tours) et écart des meilleurs
	# tours ; sur une machine chargée, le second est le plus fiable.
	print("TB6 bench %s cost ssil %.2f / %.2f ms, sdfgi %.2f / %.2f ms, mist %.2f / %.2f ms, volumetric fog %.2f / %.2f ms (paired median / best rounds, %d rounds) ; preset %s, 3D %s" % [
		name, _paired(wall["ssil_on"], wall["ssil_off"]), best["ssil_on"] - best["ssil_off"],
		_paired(wall["ssil_sdfgi"], wall["ssil_on"]), best["ssil_sdfgi"] - best["ssil_on"],
		_paired(wall["ssil_on"], wall["mist_off"]), best["ssil_on"] - best["mist_off"],
		_paired(wall["volumetric"], wall["ssil_on"]), best["volumetric"] - best["ssil_on"],
		BENCH_ROUNDS, RenderQuality.current(), Vector2(root.size) * root.scaling_3d_scale])


func _paired(with: Array, without: Array) -> float:
	var deltas: Array[float] = []
	for i in with.size():
		deltas.append(float(with[i]) - float(without[i]))
	deltas.sort()
	return deltas[deltas.size() / 2]


func _fmt(values: Array) -> String:
	var parts := PackedStringArray()
	for v in values:
		parts.append("%.2f" % v)
	return "/".join(parts)
