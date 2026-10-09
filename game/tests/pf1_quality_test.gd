extends TestCase

## PF1 : préréglages de qualité complets et ordonnés, quadtree de relief (ZG2), particules
## réduites, reprise des fichiers du joueur (dossier utilisateur du jeu exporté).
## Usage : godot --headless --path game --script res://tests/pf1_quality_test.gd
## Code de sortie 0 si tout passe, 1 sinon.


func _init() -> void:
	await process_frame
	_test_presets()
	_test_quadtree_quality()
	await _test_particles()
	_test_particles_freed_deferred()
	_test_user_dir()
	finish()


func _test_presets() -> void:
	var low: Dictionary = RenderQuality.presets()["low"]
	var medium: Dictionary = RenderQuality.presets()["medium"]
	var high: Dictionary = RenderQuality.presets()["high"]
	var ultra: Dictionary = RenderQuality.presets()["ultra"]
	# Plus le niveau est bas, moins il y a de géométrie et d'effets.
	for key in ["veg_density", "veg_detail", "terrain_near", "battle_lod", "grass", "particles", "map_shadow_range"]:
		check(float(low[key]) < float(medium[key]) and float(medium[key]) <= float(high[key]) and float(high[key]) <= float(ultra[key]), "%s ordered low < medium <= high <= ultra" % key)
	check(float(low["relief_vertex_px"]) > float(medium["relief_vertex_px"]) and float(medium["relief_vertex_px"]) > float(high["relief_vertex_px"]) and float(high["relief_vertex_px"]) > float(ultra["relief_vertex_px"]), "relief vertex spacing decreases with quality")
	check(int(low["relief_items"]) < int(medium["relief_items"]) and int(medium["relief_items"]) < int(high["relief_items"]), "relief node budget grows with quality")
	check(int(low["relief_pages"]) < int(high["relief_pages"]) and int(high["relief_pages"]) <= 256, "relief pages within the ADR 0036 VRAM cap")
	check(not bool(low["fine_relief"]) and bool(high["fine_relief"]), "fine relief off in low only")
	check(int(low["msaa"]) == Viewport.MSAA_DISABLED and int(high["msaa"]) == Viewport.MSAA_2X, "MSAA off in low, kept in high")
	# PF (ADR 0169) : pas de MSAA sur la carte en Haute (shader du terrain), gardé en bataille.
	check(RenderQuality.msaa_for(high, "campaign") == Viewport.MSAA_DISABLED and RenderQuality.msaa_for(high, "battle") == Viewport.MSAA_2X, "map MSAA off in high, battle MSAA kept")
	check(not bool(low["ssao"]) and bool(medium["ssao"]), "SSAO off in low")
	check(float(low["veg_shadow_distance"]) == 0.0, "no tree shadows in low")
	check(int(low["map_shadow_splits"]) == 2 and int(high["map_shadow_splits"]) == 4, "map shadow cascades")


## Quadtree de relief (ZG2) : les valeurs du niveau lui sont passées par `TerrainBuilder`.
func _test_quadtree_quality() -> void:
	var terrain := TerrainBuilder.new()
	var quadtree := ReliefQuadtree.new()
	terrain.quadtree = quadtree
	terrain.apply_render_quality(RenderQuality.presets()["low"])
	check(is_equal_approx(quadtree.max_vertex_px, 12.0) and quadtree.max_items == 350 and quadtree.extra_depth == 2, "low preset reaches the quadtree")
	check(not terrain.quality_fine, "low preset turns the fallback fine relief off")
	for level: String in ["low", "medium", "high"]:
		check(int(RenderQuality.presets()[level]["relief_shadow_cascades"]) == 1, "%s: relief casts shadows in the first cascade only" % level)
	check(int(RenderQuality.presets()["ultra"]["relief_shadow_cascades"]) >= 2, "ultra: relief shadows further")
	terrain.apply_render_quality(RenderQuality.presets()["high"])
	check(is_equal_approx(quadtree.max_vertex_px, 7.5) and quadtree.max_items == 700, "high preset reaches the quadtree")
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
	check(particles.amount == 350, "particles scaled to 35 %% (got %d)" % particles.amount)
	RenderQuality.particle_ratio = 1.0
	RenderQuality.scale_particles(particles)
	check(particles.amount == 1000, "original amount restored (got %d)" % particles.amount)
	RenderQuality.particle_ratio = saved
	particles.queue_free()
	await process_frame


## PF1 fix : un nœud de particules libéré avant l'exécution de l'appel différé
## (`RenderQuality._on_node_added`) ne doit pas faire échouer `scale_particles` — l'ID d'instance
## différé se résout à `null` sans toucher un objet invalide (auparavant : erreur de la file de
## messages « Cannot convert argument 1 from Object to Object »).
func _test_particles_freed_deferred() -> void:
	var particles := GPUParticles3D.new()
	var id := particles.get_instance_id()
	particles.free()
	RenderQuality._scale_particles_by_id(id)
	check(instance_from_id(id) == null, "freed particle instance id resolves to null, scale_particles skipped safely")


func _test_user_dir() -> void:
	for name in ["smoke.json", "codex_test.json", "settings_smoke.cfg", "rl1_journey_1.json"]:
		check(UserDirMigration.is_test_file(name), "%s is a test file" % name)
	for name in ["auto_1.json", "Sauvegarde rapide.json", "auto_1.png", "settings.cfg"]:
		check(not UserDirMigration.is_test_file(name), "%s is a player file" % name)
	# Éditeur et tests : dossier partagé historique, aucune copie.
	check(not OS.has_feature("template"), "test runs on the editor binary")
	check(not UserDirMigration.isolated(), "editor keeps the shared user dir")
	check(UserDirMigration.migrate_legacy() == 0, "no migration outside the exported game")
