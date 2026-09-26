extends SceneTree

## Banc PB1 : coût d'une tuile de végétation (`VegetationTileJob`) par étape, fil principal, sur
## quelques tuiles représentatives (forêt d'Orléans, bocage normand, Île-de-France).
## Usage : godot --headless --path game --script res://tests/pb1_veg_job.gd
## Sortie : lignes `PB1_VEG` (ms par étape, instances) et `PB1_VEG_TOTAL`.

const POINTS := [Vector2(2144, 2054), Vector2(1900, 2098), Vector2(2213, 1924), Vector2(1700, 1500)]


func _init() -> void:
	_run.call_deferred()


func _run() -> void:
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	(map.get_node("Vegetation") as Vegetation).enabled = false  # pas de tâches concurrentes
	root.add_child(map)
	var t_wait := Time.get_ticks_msec()
	while not map.get("load_ok"):
		if Time.get_ticks_msec() - t_wait > 60000:
			quit(1)
			return
		await process_frame
	var vegetation: Vegetation = map.get_node("Vegetation")
	while vegetation.mask == null:
		await process_frame
	var terrain: TerrainBuilder = map.get("terrain")
	var total := 0.0
	var native_total := 0.0
	var native: Object = Vegetation._make_native(vegetation.map_data)
	if native != null:
		var floor_grid := MapData.relief_floor_grid()
		native.call("set_floor", floor_grid["data"], floor_grid["side"].x, floor_grid["side"].y, floor_grid["cell"])
		native.call("set_relief_fields", floor_grid["base"], floor_grid["squash"])
	for p: Vector2 in POINTS:
		var index := terrain.chunk_index_at(p.x, p.y)
		var job := VegetationTileJob.new()
		job.mask = vegetation.mask
		job.tile_index = index
		job.origin_px = Vector2i((index % TerrainBuilder.CHUNKS) * terrain.chunk_px, (index / TerrainBuilder.CHUNKS) * terrain.chunk_px)
		job.size_px = terrain.chunk_px
		job.spacing = vegetation.spacing
		job.tree_scale = vegetation.tree_scale
		job.ground_grid = terrain.surface_grid(index)
		job.exclusions = vegetation._exclusions_for(Rect2(Vector2(job.origin_px), Vector2(job.size_px, job.size_px)))
		var t0 := Time.get_ticks_usec()
		var noise := VegetationMask.make_noise(1337)
		var grove_noise := VegetationMask.make_noise(4242)
		grove_noise.frequency = 1.0 / 9.0
		grove_noise.fractal_octaves = 2
		job._sample_coarse(noise, grove_noise)
		var t1 := Time.get_ticks_usec()
		var raw: Array = []
		for slot in VegetationTileJob.PARTS * VegetationTileJob.KIND_COUNT:
			raw.append([])
		# `_scatter` appelle `_scatter_hedges` à la fin : on les sépare en comptant les haies à part.
		var rng := RandomNumberGenerator.new()
		job._scatter(raw)
		var t2 := Time.get_ticks_usec()
		var n := 0
		for slot in raw.size():
			n += (raw[slot] as Array).size()
		for slot in raw.size():
			job._pack(raw[slot])
		var t3 := Time.get_ticks_usec()
		var ms := (t3 - t0) / 1000.0
		total += ms
		print("PB1_VEG tile %d: coarse %.1f ms, scatter+hedges %.1f ms, pack %.1f ms, total %.1f ms, %d instances" % [
			index, (t1 - t0) / 1000.0, (t2 - t1) / 1000.0, (t3 - t2) / 1000.0, ms, n])
		# Lot PB2 : même tuile semée par le pool natif (grille grossière déjà calculée).
		if native != null:
			var t4 := Time.get_ticks_usec()
			native.call("request", index, job.native_params())
			var results: Array = []
			while results.is_empty():
				OS.delay_usec(100)
				results = native.call("poll", 1)
			var t5 := Time.get_ticks_usec()
			var per_kind := PackedInt32Array([0, 0, 0, 0])
			var gd_kind := PackedInt32Array([0, 0, 0, 0])
			var counts: PackedInt32Array = results[0]["counts"]
			for slot in counts.size():
				per_kind[slot % VegetationTileJob.KIND_COUNT] += counts[slot]
				gd_kind[slot % VegetationTileJob.KIND_COUNT] += (raw[slot] as Array).size()
			native_total += (t5 - t4) / 1000.0 + (t1 - t0) / 1000.0
			# Pose : pied natif = surface affichée de référence (GDScript) − 8 % de la hauteur.
			var worst := 0.0
			for buffer: PackedFloat32Array in results[0]["buffers"]:
				for k in range(0, buffer.size(), VegetationTileJob.FLOATS_PER_INSTANCE):
					var height := Vector3(buffer[k + 1], buffer[k + 5], buffer[k + 9]).length()
					var ground := job._display_ground(buffer[k + 3], buffer[k + 11], vegetation.map_data.height_world_at(buffer[k + 3], buffer[k + 11]))
					worst = maxf(worst, absf(buffer[k + 7] - (ground - VegetationTileJob.GROUND_SINK * height)))
			print("PB1_VEG_NATIVE tile %d: worst ground error %.5f (relief gain %.2f)" % [index, worst, MapData.relief_gain()])
			print("PB1_VEG_NATIVE tile %d: native %.1f ms (worker %.1f ms), per kind oak/beech/conifer/hedge %s vs GDScript %s" % [
				index, (t5 - t4) / 1000.0, float(results[0]["ms"]), per_kind, gd_kind])
	print("PB1_VEG_TOTAL %.1f ms (native with coarse: %.1f ms)" % [total, native_total])
	quit(0)
