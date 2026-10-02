extends SceneTree

## Lot TB5 (mer et côtes) : captures et mesures des côtes et des mers de la carte de campagne.
## Usage : godot --path game --resolution 1600x900 --script res://tests/tb5_shot.gd --
##   [--out=<dossier>] [--only=<nom>,…] [--season=<saison>] [--stats] [--foam] [--bench] [--distances=90,40]
## Sans `--stats` : écrit `tb5-<nom>-<distance>.png` (960 px de large) par lieu et par distance.
## Avec `--stats` : aucune image écrite ; pour chaque lieu, rend la vue sans puis avec le lot
## (`coast_strength`, `basin_on`, `swash_amount` à 0 puis aux valeurs des données) et affiche la
## moyenne RVB de l'image entière et celle des seuls pixels que le lot change (part de l'image,
## couleur avant → après, puis part et couleur des pixels très changés) : la falaise de craie doit sortir claire, la Méditerranée plus claire
## que la Manche, sans lire d'image. Affiche aussi la part de l'image qui bouge en
## `MOTION_SECONDS` secondes, sans puis avec le lot (le ressac doit faire bouger le rivage).
## Avec `--foam` : aucune image écrite ; pour chaque lieu (mers ouvertes surtout), densité d'écume
## (part des pixels nettement plus clairs que la médiane de l'image) et régularité du motif (plus
## haut pic secondaire de l'autocorrélation du masque clair le long des lignes : proche de 0 pour
## des crêtes irrégulières, élevé pour une grille), plus l'empreinte d'un pixel
## écran en pixels carte.
## `--stats` affiche aussi la largeur moyenne (pixels écran) de la bande côtière du terrain seule.
## Avec `--bench` (lancer avec `--disable-vsync`) : coût du lot en ms par image, mesuré dans le
## même processus en alternant sans / avec (`BENCH_ROUNDS` passes de `BENCH_FRAMES` images,
## médiane) : insensible à la charge de la machine entre deux lancements.

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const WIDTH := 960
const SETTLE_FRAMES := 150
const CHANGE_THRESHOLD := 10.0 / 255.0
## Écart moyen par canal au-delà duquel un pixel est « très changé » (cœur de la bande côtière).
const STRONG_THRESHOLD := 45.0 / 255.0
## Durée entre les deux images de la mesure du mouvement (une demi-période du ressac environ).
const MOTION_SECONDS := 3.0
## Écume : pixel « clair » si sa luminance dépasse `FOAM_RATIO` × médiane + `FOAM_OFFSET`.
const FOAM_RATIO := 1.5
const FOAM_OFFSET := 10.0 / 255.0
const AUTOCORRELATION_LAGS := 80
const AUTOCORRELATION_ROWS := 64
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
## Réglages coupés pour le bench : le lot entier est sauté dans les deux shaders.
const TERRAIN_BENCH_OFF := {"coast_enabled": false, "swash_amount": 0.0}
const SEA_BENCH_OFF := {"basin_on": false, "swash_amount": 0.0, "coast_enabled": false}
const BENCH_ROUNDS := 5
const BENCH_FRAMES := 120


func _init() -> void:
	var out_dir := "user://tb5"
	var only := PackedStringArray()
	var distances: Array[float] = [90.0, 40.0]
	var stats := false
	var bench := false
	var foam := false
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
		elif arg == "--bench":
			bench = true
		elif arg == "--foam":
			foam = true
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
	var terrain_bench_on := _values(terrain_material, TERRAIN_BENCH_OFF)
	var sea_bench_on := _values(sea_material, SEA_BENCH_OFF)
	for place: String in PLACES:
		if not only.is_empty() and not only.has(place):
			continue
		var at: Vector2 = PLACES[place]
		var ground := Vector3(at.x, data.surface_world_at(at.x, at.y), at.y)
		for distance in distances:
			rig.look_at_point(ground, maxf(distance, rig.min_distance_at(ground)))
			rig.snap()
			var label := "tb5-%s-%d" % [place, int(distance)]
			if bench:
				await _settle(map, SETTLE_FRAMES)
				var off_ms: Array[float] = []
				var on_ms: Array[float] = []
				for round_index in BENCH_ROUNDS:
					_apply(terrain_material, TERRAIN_BENCH_OFF)
					_apply(sea_material, SEA_BENCH_OFF)
					off_ms.append(await _frame_ms(map))
					_apply(terrain_material, terrain_bench_on)
					_apply(sea_material, sea_bench_on)
					on_ms.append(await _frame_ms(map))
				off_ms.sort()
				on_ms.sort()
				print("TB5 bench %s: %.2f ms/frame without the lot, %.2f with it (median of %d rounds of %d frames)" % [label, off_ms[BENCH_ROUNDS / 2], on_ms[BENCH_ROUNDS / 2], BENCH_ROUNDS, BENCH_FRAMES])
				continue
			if foam:
				await _settle(map, SETTLE_FRAMES)
				_foam_stats(label, _grab(), _footprint(ground.y))
				continue
			if not stats:
				await _settle(map, SETTLE_FRAMES)
				_shot(out_dir, label)
				continue
			_apply(terrain_material, TERRAIN_OFF)
			_apply(sea_material, SEA_OFF)
			await _settle(map, SETTLE_FRAMES)
			var before := _grab()
			var motion_off := await _motion(map)
			if motion_off <= 0.0:
				push_warning("TB5 shot: %s did not move in %.1f s, the window is probably occluded or throttled: the motion figures are not reliable" % [label, MOTION_SECONDS])
			_apply(terrain_material, terrain_on)
			await _settle(map, 8)
			_band_width(label, before, _grab(), _footprint(ground.y))
			_apply(sea_material, sea_on)
			await _settle(map, 8)
			_stats(label, before, _grab())
			print("TB5 motion %s over %.1f s: %.2f %% of the image without the lot, %.2f %% with it" % [label, MOTION_SECONDS, motion_off, await _motion(map)])
	quit(0)


## Image de la vue. Une fenêtre recouverte n'est plus dessinée sous macOS (l'image lue serait
## figée) : le dessin est forcé avant la lecture.
func _grab() -> Image:
	RenderingServer.force_draw(false)
	return root.get_viewport().get_texture().get_image()


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


## Empreinte d'un pixel écran au centre de la vue, en pixels carte (plan horizontal à `ground_y`).
func _footprint(ground_y: float) -> float:
	var camera := root.get_viewport().get_camera_3d()
	if camera == null:
		return 0.0
	var center := Vector2(root.get_viewport().get_visible_rect().size) * 0.5
	var plane := Plane(Vector3.UP, ground_y)
	var a: Variant = plane.intersects_ray(camera.project_ray_origin(center), camera.project_ray_normal(center))
	var b: Variant = plane.intersects_ray(camera.project_ray_origin(center + Vector2(1, 0)), camera.project_ray_normal(center + Vector2(1, 0)))
	if a == null or b == null:
		return 0.0
	return (a as Vector3).distance_to(b as Vector3)


## Largeur moyenne (pixels écran) de la bande côtière : pixels très changés par le terrain seul,
## divisés par la longueur du rivage à l'écran (nombre de lignes ou de colonnes touchées).
func _band_width(label: String, before: Image, after: Image, footprint: float) -> void:
	var rows := {}
	var cols := {}
	var strong := 0
	var light := Vector3.ZERO
	for y in after.get_height():
		for x in after.get_width():
			var a := before.get_pixel(x, y)
			var b := after.get_pixel(x, y)
			if absf(a.r - b.r) + absf(a.g - b.g) + absf(a.b - b.b) > STRONG_THRESHOLD * 3.0:
				strong += 1
				rows[y] = true
				cols[x] = true
				light += Vector3(b.r, b.g, b.b)
	var length := maxi(rows.size(), cols.size())
	if length == 0:
		print("TB5 band %s: no coast band in view (footprint %.4f map px per screen px)" % [label, footprint])
		return
	var mean := light / strong * 255.0
	var width := float(strong) / length
	print("TB5 band %s: %.1f screen px wide (%.2f map px) over %d px of shore, mean %.0f %.0f %.0f, footprint %.4f" % [label, width, width * footprint, length, mean.x, mean.y, mean.z, footprint])


## Densité et régularité de l'écume sur les deux tiers bas de l'image.
func _foam_stats(label: String, image: Image, footprint: float) -> void:
	var width := image.get_width()
	var top := image.get_height() / 3
	var height := image.get_height() - top
	var lum := PackedFloat32Array()
	lum.resize(width * height)
	var histogram := PackedInt32Array()
	histogram.resize(256)
	var sum := Vector3.ZERO
	for y in height:
		for x in width:
			var c := image.get_pixel(x, y + top)
			var l := c.r * 0.2126 + c.g * 0.7152 + c.b * 0.0722
			lum[y * width + x] = l
			histogram[clampi(int(l * 255.0), 0, 255)] += 1
			sum += Vector3(c.r, c.g, c.b)
	var half := width * height / 2
	var median := 0
	var seen := 0
	for bin in 256:
		seen += histogram[bin]
		if seen >= half:
			median = bin
			break
	var limit := median / 255.0 * FOAM_RATIO + FOAM_OFFSET
	var mask := PackedByteArray()
	mask.resize(width * height)
	var bright := 0
	for index in width * height:
		if lum[index] > limit:
			mask[index] = 1
			bright += 1
	var density := float(bright) / (width * height)
	var mean := sum / (width * height) * 255.0
	var line := "TB5 foam %s: density %.2f %% (pixels above %.0f, median %d), image mean %.0f %.0f %.0f, footprint %.4f" % [label, density * 100.0, limit * 255.0, median, mean.x, mean.y, mean.z, footprint]
	if density > 0.0005 and density < 0.5:
		var peak := _autocorrelation_peak(mask, width, height)
		line += " ; regularity peak %.2f at %d px, oscillation %.3f (rows band)" % [peak.x, int(peak.y), peak.z]
	print(line)


## Régularité du motif clair le long des lignes, sur une bande de `AUTOCORRELATION_ROWS` lignes au
## milieu du masque (échelle à peu près constante dans la bande) : autocorrélation normalisée par
## décalage ; x = plus haut pic après la première retombée sous 0,05 (0 = aucun retour, motif
## irrégulier ; une grille revient à chaque pas), y = son décalage (px), z = écart-type de
## l'autocorrélation après la retombée (une grille oscille, un semis au hasard reste à plat).
func _autocorrelation_peak(mask: PackedByteArray, width: int, height: int) -> Vector3:
	var first_row := maxi((height - AUTOCORRELATION_ROWS) / 2, 0)
	var last_row := mini(first_row + AUTOCORRELATION_ROWS, height)
	var bright := 0
	for row in range(first_row, last_row):
		for x in width:
			bright += mask[row * width + x]
	var density := float(bright) / ((last_row - first_row) * width)
	if density <= 0.0 or density >= 1.0:
		return Vector3.ZERO
	var variance := density - density * density
	var best := Vector2.ZERO
	var dipped := false
	var tail_sum := 0.0
	var tail_sq := 0.0
	var tail := 0
	for lag in range(1, AUTOCORRELATION_LAGS + 1):
		var hits := 0
		for row in range(first_row, last_row):
			var base := row * width
			for x in width - lag:
				if mask[base + x] == 1 and mask[base + x + lag] == 1:
					hits += 1
		var r := (float(hits) / ((last_row - first_row) * (width - lag)) - density * density) / variance
		if r < 0.05:
			dipped = true
		if dipped:
			tail_sum += r
			tail_sq += r * r
			tail += 1
			if r > best.x:
				best = Vector2(r, lag)
	var spread := sqrt(maxf(tail_sq / tail - (tail_sum / tail) * (tail_sum / tail), 0.0)) if tail > 0 else 0.0
	return Vector3(best.x, best.y, spread)


## Durée moyenne d'une image (ms) sur `BENCH_FRAMES` images, après quelques images d'élan.
func _frame_ms(map: Node3D) -> float:
	await _settle(map, 10)
	var t0 := Time.get_ticks_usec()
	await _settle(map, BENCH_FRAMES)
	return (Time.get_ticks_usec() - t0) / (1000.0 * BENCH_FRAMES)


## Part de l'image (%) qui change en `MOTION_SECONDS` secondes (vagues, écume, ressac).
func _motion(map: Node3D) -> float:
	var first := _grab()
	var until := Time.get_ticks_msec() + int(MOTION_SECONDS * 1000.0)
	while Time.get_ticks_msec() < until:
		await _settle(map, 1)
	var second := _grab()
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
	var image := _grab()
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
