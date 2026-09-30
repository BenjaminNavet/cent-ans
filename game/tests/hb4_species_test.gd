extends SceneTree

## Test headless du lot HB4 (essences et répartition par biome, ADR 0143) :
##  1. catalogue `data/art/tree_species.json` chargé (≥ 16 essences, lignes 0-2 chêne/hêtre/sapin) ;
##  2. atlas GA3 complet : une ligne de 8 azimuts par essence, chaque cellule couverte, matériau
##     des imposteurs en mode « ligne par instance » ;
##  3. semis GDScript d'une tuile témoin (forêt d'Orléans et Loire, villages synthétiques) pour
##     chaque biome (biomes.png synthétique 1 × 1) : ≥ 2 essences par biome ;
##  4. densité hors forêt (champs) bien sous celle du semis V4, steppe presque nue ;
##  5. instances ≤ +30 % et emplacements non vides (≈ appels de dessin) ≤ +30 % par rapport au
##     semis V4, en GDScript et en natif (`VegetationScatter.set_species`).
## Usage : godot --headless --path game --script res://tests/hb4_species_test.gd

const WITNESS_ORIGIN := Vector2i(2048, 3200)
const WITNESS_SIZE := 256
const VILLAGES: Array[Vector3] = [Vector3(2100, 3260, 3.0), Vector3(2230, 3400, 3.5), Vector3(2150, 3420, 2.5)]
const GROWTH_CAP := 1.3
## Distance au lit (px) en deçà de laquelle un arbre hors forêt compte comme ripisylve.
const RIPARIAN_BAND := 2.5

var _failures := 0


func _init() -> void:
	await process_frame
	var species := TreeSpecies.shared()
	_test_catalogue(species)
	_test_atlas(species)
	if species.ok:
		_test_scatter(species)
	if _failures > 0:
		push_error("hb4_species_test: %d failure(s)" % _failures)
		quit(1)
		return
	print("hb4_species_test: OK")
	quit(0)


func _check(condition: bool, label: String) -> bool:
	if not condition:
		_failures += 1
		push_error("FAIL: " + label)
	return condition


func _test_catalogue(species: TreeSpecies) -> void:
	if not _check(species.ok, "catalogue loads (%s)" % TreeSpecies.data_path()):
		return
	_check(species.count >= 16, "%d species (>= 16)" % species.count)
	_check(Array(species.ids.slice(0, 3)) == ["oak", "beech", "fir"], "rows 0-2 = oak, beech, fir")
	for s in species.count:
		_check(species.kind[s] >= 0 and species.kind[s] <= 2, "%s: fallback mesh" % species.ids[s])
		_check(species.height[2 * s] > 0.0 and species.height[2 * s] <= species.height[2 * s + 1], "%s: height range" % species.ids[s])
	print("hb4: %d species: %s" % [species.count, ", ".join(species.ids)])


func _test_atlas(species: TreeSpecies) -> void:
	var texture := load(Ga3Vegetation.IMPOSTOR_ALBEDO) as Texture2D
	if not _check(texture != null, "GA3 albedo atlas loads"):
		return
	var cell := texture.get_width() / VegetationMeshes.IMPOSTOR_VIEWS
	_check(texture.get_height() == species.count * cell, "atlas: one row per species (%d x %d, %d species)" % [texture.get_width(), texture.get_height(), species.count])
	var normal := load(Ga3Vegetation.IMPOSTOR_NORMAL) as Texture2D
	_check(normal != null and normal.get_size() == texture.get_size(), "normal atlas matches")
	var image := Image.load_from_file(ProjectSettings.globalize_path(Ga3Vegetation.IMPOSTOR_ALBEDO))
	if _check(image != null, "atlas image"):
		image.convert(Image.FORMAT_RGBA8)
		var bytes := image.get_data()
		var width := image.get_width()
		for row in mini(species.count, image.get_height() / cell):
			for view in VegetationMeshes.IMPOSTOR_VIEWS:
				var covered := 0
				var total := 0
				for y in range(row * cell, (row + 1) * cell, 4):
					for x in range(view * cell, (view + 1) * cell, 4):
						total += 1
						if bytes[(y * width + x) * 4 + 3] > 127:
							covered += 1
				var cov := float(covered) / total
				_check(cov > 0.03 and cov < 0.7, "cell %s/%d covered (%.3f)" % [species.ids[row], view, cov])
	var material := Vegetation._make_impostor_material()
	if _check(material != null, "impostor material"):
		_check(int(material.get_shader_parameter("rows")) == species.count, "material rows = species (%d)" % int(material.get_shader_parameter("rows")))
		_check(bool(material.get_shader_parameter("species_rows")), "material reads the row per instance")


func _test_scatter(species: TreeSpecies) -> void:
	var map_dir := (load("res://scripts/map/map_paths.gd") as GDScript).call("default_data_dir").path_join("map") as String
	var data := MapData.load_from_dir(map_dir)
	if not _check(data.load_error == "", "map loads (%s)" % data.load_error):
		return
	var mask := VegetationMask.new()
	mask.setup(data)
	print("hb4: real biomes.png %s" % ("present" if mask.has_biomes() else "absent (default biome %d)" % species.d("default_biome", 2.0)))
	var exclusions := PackedVector3Array(VILLAGES)
	mask.set_biome_image(_biome_image(2))
	var legacy := _scatter(mask, null, exclusions)
	var legacy_stats := _stats(legacy, null)
	print("hb4: V4 legacy: %s" % legacy_stats)
	var per_biome := {}
	for b in range(1, TreeSpecies.BIOME_COUNT):
		mask.set_biome_image(_biome_image(b))
		var job := _scatter(mask, species, exclusions)
		var stats := _stats(job, species)
		per_biome[b] = stats
		print("hb4: biome %d: %s" % [b, stats])
		_check((stats["species"] as Dictionary).size() >= 2, "biome %d: >= 2 species (%s)" % [b, stats["species"]])
		_check(int(stats["trees"]) <= legacy_stats["trees"] * GROWTH_CAP, "biome %d: trees %d <= +30%% of %d" % [b, stats["trees"], legacy_stats["trees"]])
		_check(int(stats["slots"]) <= ceili(legacy_stats["slots"] * GROWTH_CAP), "biome %d: non-empty slots %d <= +30%% of %d" % [b, stats["slots"], legacy_stats["slots"]])
		_check(int(stats["unencoded"]) == 0, "biome %d: every tree carries its species" % b)
	var temperate: Dictionary = per_biome[2]
	_check(float(temperate["open_density"]) < 0.6 * float(legacy_stats["open_density"]), "fewer trees in open fields (%.4f vs V4 %.4f per px2)" % [temperate["open_density"], legacy_stats["open_density"]])
	_check(float(temperate["open_density"]) < 0.03, "open-field density %.4f < 0.03 per px2" % temperate["open_density"])
	_check(int(per_biome[4]["trees"]) * 4 < int(temperate["trees"]), "steppe nearly bare (%d vs %d)" % [per_biome[4]["trees"], temperate["trees"]])
	for b in [3, 7]:
		for id in ["oak", "beech", "maple", "birch", "apple"]:
			_check(not (per_biome[b]["species"] as Dictionary).has(id), "biome %d: no %s" % [b, id])
	_check((temperate["species"] as Dictionary).has("apple"), "orchards around the villages (continental)")
	_check((per_biome[3]["species"] as Dictionary).has("olive"), "olive groves (Mediterranean)")
	_check((temperate["species"] as Dictionary).has("poplar") or (temperate["species"] as Dictionary).has("willow"), "riparian trees along the Loire")
	_test_native(data, mask, species, exclusions)


func _test_native(data: MapData, mask: VegetationMask, species: TreeSpecies, exclusions: PackedVector3Array) -> void:
	var native: Object = Vegetation._make_native(data)
	if native == null or not native.has_method("set_species"):
		print("hb4: native scatter unavailable, skipped")
		return
	mask.set_biome_image(_biome_image(2))
	var totals := {}
	for mode in ["legacy", "species"]:
		var table: Dictionary = species.table() if mode == "species" else {}
		_check(bool(native.call("set_species", table)) == (mode == "species"), "native set_species (%s)" % mode)
		var job := VegetationTileJob.new()
		job.mask = mask
		job.tile_index = 4242
		job.origin_px = WITNESS_ORIGIN
		job.size_px = WITNESS_SIZE
		job.spacing = 1.35
		job.exclusions = exclusions
		job.coarse_only = true
		job.run()
		_check(bool(native.call("request", 1, job.native_params())), "native request (%s)" % mode)
		var result: Dictionary = {}
		for attempt in 400:
			var polled: Array = native.call("poll", 4)
			if not polled.is_empty():
				result = polled[0]
				break
			OS.delay_msec(10)
		if not _check(not result.is_empty(), "native result (%s)" % mode):
			return
		job.apply_native(result)
		totals[mode] = _stats(job, species if mode == "species" else null)
		print("hb4: native %s: %s" % [mode, totals[mode]])
	var s: Dictionary = totals["species"]
	var l: Dictionary = totals["legacy"]
	_check(int(s["trees"]) <= l["trees"] * GROWTH_CAP, "native: trees %d <= +30%% of %d" % [s["trees"], l["trees"]])
	_check(int(s["slots"]) <= ceili(l["slots"] * GROWTH_CAP), "native: slots %d <= +30%% of %d" % [s["slots"], l["slots"]])
	_check((s["species"] as Dictionary).size() >= 4, "native: several species (%s)" % s["species"])
	_check(int(s["unencoded"]) == 0, "native: every tree carries its species")


func _biome_image(b: int) -> Image:
	return Image.create_from_data(1, 1, false, Image.FORMAT_L8, PackedByteArray([b]))


func _scatter(mask: VegetationMask, species: TreeSpecies, exclusions: PackedVector3Array) -> VegetationTileJob:
	var job := VegetationTileJob.new()
	job.mask = mask
	job.tile_index = 4242
	job.origin_px = WITNESS_ORIGIN
	job.size_px = WITNESS_SIZE
	job.spacing = 1.35
	job.exclusions = exclusions
	job.species = species
	job.run()
	return job


## Arbres (haies exclues), emplacements non vides, essences, densité hors forêt et hors ripisylve
## (arbres par px² de cellules grossières sans forêt), arbres de ripisylve hors forêt.
func _stats(job: VegetationTileJob, species: TreeSpecies) -> Dictionary:
	var trees := 0
	var slots := 0
	var unencoded := 0
	var open_trees := 0
	var riparian_trees := 0
	var names := {}
	for slot in job.buffers.size():
		var buffer: PackedFloat32Array = job.buffers[slot]
		var count := buffer.size() / VegetationTileJob.FLOATS_PER_INSTANCE
		if count > 0:
			slots += 1
		if slot % VegetationTileJob.KIND_COUNT == VegetationTileJob.Kind.HEDGE:
			continue
		trees += count
		for i in count:
			var k := i * VegetationTileJob.FLOATS_PER_INSTANCE
			var row := floori(buffer[k + 12] / TreeSpecies.CUSTOM_STRIDE) - 1
			if species != null:
				if row < 0 or row >= species.count:
					unencoded += 1
				else:
					names[species.ids[row]] = int(names.get(species.ids[row], 0)) + 1
			var gx := (buffer[k + 3] - job.origin_px.x) / job.coarse_step
			var gy := (buffer[k + 11] - job.origin_px.y) / job.coarse_step
			# Champs : hors forêt et hors bande de ripisylve (comptée à part).
			if job._lerp_grid(job._forest, gx, gy) < 0.02:
				if job.mask.map_data.river_sd_at(buffer[k + 3], buffer[k + 11]) > RIPARIAN_BAND:
					open_trees += 1
				else:
					riparian_trees += 1
	var open_cells := 0
	for value in job._forest:
		if value < 0.02:
			open_cells += 1
	var open_area := float(open_cells) / maxf(job._forest.size(), 1.0) * WITNESS_SIZE * WITNESS_SIZE
	return {"trees": trees, "slots": slots, "species": names, "unencoded": unencoded, "open_density": open_trees / maxf(open_area, 1.0), "riparian": riparian_trees}
