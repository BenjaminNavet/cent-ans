extends TestCase

## Lot ZG7a (ADR 0036) : perf et finitions de la vue rapprochée.
## godot --headless --path game --script res://tests/zg7a_test.gd
##  1. Aperçu de chemin : plancher de largeur et soulèvement historiques en vue stratégique, ruban
##     fin et presque posé de près ; reconstruction quand la distance change assez.
##  2. Ponts fins et ponts-portes : tablier à sa largeur réelle (`BridgeMeshes.fine_deck_scale`),
##     jamais élargi ; tableaux préparés dans un fil identiques au maillage du fil principal.
##  3. Villes : matériaux du HLOD par instance (`lod_mode`), caméra et portée publiées.


func _init() -> void:
	_test_path_preview()
	_test_bridges()
	_test_town_lod()
	if failures == 0:
		print("zg7a_test: OK")
	else:
		finish()


func _test_path_preview() -> void:
	var preview := PathPreview.new()
	# Vue stratégique : comme avant ZG7a (plancher 0,8, soulèvement 0,6, 0,005 × distance).
	check(is_equal_approx(preview.width_at(300.0), 1.5), "strategic width unchanged (%f)" % preview.width_at(300.0))
	check(absf(preview.width_at(160.0) - 0.8) < 0.01, "strategic floor kept at 160 (%f)" % preview.width_at(160.0))
	check(is_equal_approx(preview.lift_at(200.0), preview.lift), "strategic lift unchanged")
	# Vallée / site : ruban fin (≈ 5 px), soulèvement de quelques mètres.
	check(preview.width_at(5.0) < 0.05, "valley ribbon thin (%f units)" % preview.width_at(5.0))
	check(preview.width_at(0.5) <= PathPreview.NEAR_MIN_WIDTH + 1e-6, "site ribbon at its floor")
	check(preview.lift_at(1.0) < 0.01, "site ribbon nearly on the ground (%f)" % preview.lift_at(1.0))
	var last := 0.0
	for d: float in [0.3, 1.0, 5.0, 20.0, 60.0, 150.0, 400.0]:
		var w := preview.width_at(d)
		check(w >= last, "width grows with distance (%s)" % d)
		last = w
	preview.free()


func _test_bridges() -> void:
	var mpu := 719.0
	for structure: String in ["gate", "stone", "wood"]:
		for river_m: float in [30.0, 130.0, 350.0]:
			var mesh_width := river_m / mpu / RiverCrossings.FINE_SCALE
			var k := BridgeMeshes.fine_deck_scale(structure, mesh_width, mpu, RiverCrossings.FINE_SCALE)
			var deck_m := BridgeMeshes.mesh_deck_width(structure, mesh_width) * k * mpu
			check(k <= RiverCrossings.FINE_SCALE + 1e-6, "%s deck never widened (%s m)" % [structure, river_m])
			check(deck_m <= float(BridgeMeshes.FINE_DECK_M[structure]) + 0.01, "%s deck %.1f m over a %s m river" % [structure, deck_m, river_m])
			# Avant ZG7a : 0,15 + 0,05 × portée à l'échelle FINE_SCALE (30-45 m sur la Loire).
			var before_m := BridgeMeshes.mesh_deck_width(structure, mesh_width) * RiverCrossings.FINE_SCALE * mpu
			if river_m >= 130.0:
				check(deck_m < before_m * 0.5, "%s deck narrower than before (%.1f vs %.1f m)" % [structure, deck_m, before_m])
	# Tableaux préparés dans un fil = maillage construit sur le fil principal.
	var surfaces := BridgeMeshes.build_arrays("gate", 2.5, 7)
	var mesh := BridgeMeshes.build("gate", 2.5, 7)
	var non_empty := 0
	for s in surfaces:
		if not (s as Array).is_empty():
			non_empty += 1
	check(non_empty == mesh.get_surface_count(), "prepared surfaces match the built mesh")
	if non_empty > 0 and mesh.get_surface_count() > 0:
		var prepared: PackedVector3Array = surfaces[0][Mesh.ARRAY_VERTEX]
		check(prepared.size() == mesh.surface_get_array_len(0), "same vertex count")
	var key := BridgeMeshes.cache_key("gate", 2.5, 7)
	check(BridgeMeshes.cached(key) == mesh, "cache key shared by build and build_from")


func _test_town_lod() -> void:
	TownBuilder.clear_cache()
	var detail := TownBuilder.material(0, false, 0.0, 719.0, 1)
	var blocks := TownBuilder.material(0, true, 0.0, 719.0, 2)
	var plain := TownBuilder.material(1, false, 0.9, 719.0)
	check(int(detail.get_shader_parameter("lod_mode")) == 1 and int(blocks.get_shader_parameter("lod_mode")) == 2, "per-instance HLOD modes")
	check(int(plain.get_shader_parameter("lod_mode")) == 0, "draped meshes always drawn")
	TownBuilder.set_lod_view(Vector3(10.0, 2.0, 20.0), 1.6)
	check((detail.get_shader_parameter("lod_camera") as Vector3).is_equal_approx(Vector3(10.0, 2.0, 20.0)), "camera published to the detail material")
	check(is_equal_approx(float(blocks.get_shader_parameter("lod_range")), 1.6), "range published to the block material")
	# Un matériau créé après la publication reçoit la vue courante.
	var late := TownBuilder.material(0, false, 0.0, 500.0, 1)
	check((late.get_shader_parameter("lod_camera") as Vector3).is_equal_approx(Vector3(10.0, 2.0, 20.0)), "late material gets the current view")
	TownBuilder.clear_cache()
