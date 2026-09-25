extends SceneTree

## PF1 : préréglages de qualité complets et ordonnés, quadtree de relief (ZG2), particules
## réduites, reprise des fichiers du joueur (dossier utilisateur du jeu exporté).
## Usage : godot --headless --path game --script res://tests/pf1_quality_test.gd
## Code de sortie 0 si tout passe, 1 sinon.

var _failures := 0


func _init() -> void:
	await process_frame
	_test_presets()
	_test_quadtree_quality()
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
	_check(float(low["relief_vertex_px"]) > float(medium["relief_vertex_px"]) and float(medium["relief_vertex_px"]) > float(high["relief_vertex_px"]) and float(high["relief_vertex_px"]) > float(ultra["relief_vertex_px"]), "relief vertex spacing decreases with quality")
	_check(int(low["relief_items"]) < int(medium["relief_items"]) and int(medium["relief_items"]) < int(high["relief_items"]), "relief node budget grows with quality")
	_check(int(low["relief_pages"]) < int(high["relief_pages"]) and int(high["relief_pages"]) <= 256, "relief pages within the ADR 0036 VRAM cap")
	_check(not bool(low["fine_relief"]) and bool(high["fine_relief"]), "fine relief off in low only")
	_check(int(low["msaa"]) == Viewport.MSAA_DISABLED and int(high["msaa"]) == Viewport.MSAA_2X, "MSAA off in low, kept in high")
	_check(not bool(low["ssao"]) and bool(medium["ssao"]), "SSAO off in low")
	_check(float(low["veg_shadow_distance"]) == 0.0, "no tree shadows in low")
	_check(int(low["map_shadow_splits"]) == 2 and int(high["map_shadow_splits"]) == 4, "map shadow cascades")


## Quadtree de relief (ZG2) : les valeurs du niveau lui sont passées par `TerrainBuilder`.
func _test_quadtree_quality() -> void:
	var terrain := TerrainBuilder.new()
	var quadtree := ReliefQuadtree.new()
	terrain.quadtree = quadtree
	terrain.apply_render_quality(RenderQuality.PRESETS["low"])
	_check(is_equal_approx(quadtree.max_vertex_px, 12.0) and quadtree.max_items == 350 and quadtree.extra_depth == 2, "low preset reaches the quadtree")
	_check(not terrain.quality_fine, "low preset turns the fallback fine relief off")
	for level: String in ["low", "medium", "high"]:
		_check(int(RenderQuality.PRESETS[level]["relief_shadow_cascades"]) == 1, "%s: relief casts shadows in the first cascade only" % level)
	_check(int(RenderQuality.PRESETS["ultra"]["relief_shadow_cascades"]) >= 2, "ultra: relief shadows further")
	terrain.apply_render_quality(RenderQuality.PRESETS["high"])
	_check(is_equal_approx(quadtree.max_vertex_px, 6.0) and quadtree.max_items == 700, "high preset reaches the quadtree")
	terrain.quadtree = null
	quadtree.free()
	terrain.free()


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
