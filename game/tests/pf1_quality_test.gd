extends SceneTree

## PF1 : préréglages de qualité complets et ordonnés, blocs LOD du relief fin, particules
## réduites, reprise des fichiers du joueur (dossier utilisateur du jeu exporté).
## Usage : godot --headless --path game --script res://tests/pf1_quality_test.gd
## Code de sortie 0 si tout passe, 1 sinon.

var _failures := 0


func _init() -> void:
	await process_frame
	_test_presets()
	_test_fine_blocks()
	await _test_particles()
	_test_user_dir()
	print("pf1_quality_test: %s" % ("OK" if _failures == 0 else "%d échec(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _test_presets() -> void:
	for level: String in RenderQuality.PRESETS:
		var p: Dictionary = RenderQuality.PRESETS[level]
		for key: String in RenderQuality.SCENE_KEYS:
			_check(p.has(key), "%s has %s" % [level, key])
	var low: Dictionary = RenderQuality.PRESETS["low"]
	var medium: Dictionary = RenderQuality.PRESETS["medium"]
	var high: Dictionary = RenderQuality.PRESETS["high"]
	var ultra: Dictionary = RenderQuality.PRESETS["ultra"]
	# Plus le niveau est bas, moins il y a de géométrie et d'effets.
	for key in ["veg_density", "veg_detail", "terrain_near", "battle_lod", "grass", "particles", "map_shadow_range"]:
		_check(float(low[key]) < float(medium[key]) and float(medium[key]) <= float(high[key]) and float(high[key]) <= float(ultra[key]), "%s ordered low < medium <= high <= ultra" % key)
	_check(float(low["fine_lod_bias"]) > float(medium["fine_lod_bias"]) and float(medium["fine_lod_bias"]) > float(high["fine_lod_bias"]), "fine LOD bias decreases with quality")
	_check(not bool(low["fine_relief"]) and bool(high["fine_relief"]), "fine relief off in low only")
	_check(int(low["msaa"]) == Viewport.MSAA_DISABLED and int(high["msaa"]) == Viewport.MSAA_2X, "MSAA off in low, kept in high")
	_check(not bool(low["ssao"]) and bool(medium["ssao"]), "SSAO off in low")
	_check(float(low["veg_shadow_distance"]) == 0.0, "no tree shadows in low")
	_check(int(low["map_shadow_splits"]) == 2 and int(high["map_shadow_splits"]) == 4, "map shadow cascades")


func _test_fine_blocks() -> void:
	# Grille plate : erreur nulle ; bosse au centre : erreur > 0 pour le pas 2.
	var bside := 9
	var flat := PackedFloat32Array()
	flat.resize(bside * bside)
	_check(FineTerrainJob.lod_error(flat, bside, 2) == 0.0, "flat grid has no LOD error")
	var bump := flat.duplicate()
	bump[4 * bside + 3] = 1.0
	_check(is_equal_approx(FineTerrainJob.lod_error(bump, bside, 2), 1.0), "odd vertex bump is the stride-2 error")
	_check(FineTerrainJob.lod_error(flat, bside, 16) == INF, "stride larger than the block is refused")
	# Plan incliné : exactement représenté à tout pas.
	var slope := flat.duplicate()
	for j in bside:
		for i in bside:
			slope[j * bside + i] = 0.3 * i + 0.1 * j
	_check(FineTerrainJob.lod_error(slope, bside, 4) < 0.0001, "plane has no LOD error")
	for stride: int in [1, 2, 4, 8]:
		var cells := 8 / stride
		var indices := FineTerrainJob.block_indices(8, stride)
		_check(indices.size() == cells * cells * 6 + 4 * cells * 12, "block indices size at stride %d" % stride)
		var top := 0
		for index in indices:
			top = maxi(top, index)
		_check(top < bside * bside + 4 * bside, "block indices in range at stride %d" % stride)
	# Tâche complète sur une petite tuile synthétique : blocs, sommets, clés croissantes.
	var job := FineTerrainJob.new()
	job.tile_side = 64
	job.chunk_px = 32
	job.step = 1
	job.block_quads = 16
	job.edge_step = 4
	job.map_size = Vector2i(64, 64)
	job.map_bpp = 2
	job.map_bytes = PackedByteArray()
	job.map_bytes.resize(64 * 64 * 2)
	job.tile_bytes = PackedByteArray()
	job.tile_bytes.resize(64 * 64 * 2)
	for k in 64 * 64:
		var v := int(30000 + 2000 * sin(k * 0.37))
		job.tile_bytes[k * 2] = v & 0xff
		job.tile_bytes[k * 2 + 1] = (v >> 8) & 0xff
	job.run()
	_check(job.ok, "fine job ran")
	_check(job.blocks_per_side == 4 and job.block_vertices.size() == 16, "4 x 4 blocks (got %d)" % job.block_vertices.size())
	if not job.block_errors.is_empty():
		var keys: PackedFloat32Array = job.block_errors[0]
		_check(keys.size() == FineTerrainJob.LOD_STRIDES.size(), "one LOD key per stride")
		_check(keys[0] > 0.0 and keys[0] < keys[1] and keys[1] < keys[2], "LOD keys strictly increasing: %s" % keys)
		_check(job.block_vertices[0].size() == 17 * 17 + 4 * 17, "block vertices with skirt")


func _test_particles() -> void:
	var saved := RenderQuality.particle_ratio
	RenderQuality.particle_ratio = 0.35
	var particles := GPUParticles3D.new()
	particles.amount = 1000
	root.add_child(particles)
	RenderQuality.scale_particles(particles)
	_check(particles.amount == 350, "particles scaled to 35 %% (got %d)" % particles.amount)
	RenderQuality.particle_ratio = 1.0
	RenderQuality.scale_particles(particles)
	_check(particles.amount == 1000, "original amount restored (got %d)" % particles.amount)
	RenderQuality.particle_ratio = saved
	particles.queue_free()
	await process_frame


func _test_user_dir() -> void:
	for name in ["smoke.json", "codex_test.json", "settings_smoke.cfg", "rl1_journey_1.json"]:
		_check(UserDirMigration.is_test_file(name), "%s is a test file" % name)
	for name in ["auto_1.json", "Sauvegarde rapide.json", "auto_1.png", "settings.cfg"]:
		_check(not UserDirMigration.is_test_file(name), "%s is a player file" % name)
	# Éditeur et tests : dossier partagé historique, aucune copie.
	_check(not OS.has_feature("template"), "test runs on the editor binary")
	_check(not UserDirMigration.isolated(), "editor keeps the shared user dir")
	_check(UserDirMigration.migrate_legacy() == 0, "no migration outside the exported game")


func _check(condition: bool, message: String) -> void:
	if not condition:
		_failures += 1
		push_error("FAIL: " + message)
		print("FAIL: " + message)
