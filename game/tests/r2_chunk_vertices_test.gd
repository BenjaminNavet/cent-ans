extends SceneTree

## Lot OMR-R2 : `TerrainBuilder._chunk_vertices` (colonnes et lignes précalculées, grilles du fond
## de relief en local) rend exactement les sommets et hauteurs de la version de référence
## (`_chunk_vertices_reference`, appel de `MapData.display_height_with` par sommet), au pas
## lointain et au pas proche, sur la vraie carte ; temps des deux versions.
## Usage : godot --headless --path game --script res://tests/r2_chunk_vertices_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")


func _init() -> void:
	_run.call_deferred()


func _run() -> void:
	var map_data := MapData.load_from_dir(MAP_PATHS.default_data_dir().path_join("map"))
	if map_data.load_error != "":
		print("R2 sommets: carte illisible (%s)" % map_data.load_error)
		quit(1)
		return
	var terrain := TerrainBuilder.new()
	root.add_child(terrain)
	terrain.build(map_data)
	var ok := true
	var count := terrain.chunks_x * terrain.chunks_y
	var fast_us := 0
	var reference_us := 0
	var checked := 0
	for step: int in [terrain.far_step, terrain.near_step]:
		for index in range(0, count, 7):
			var cx := index % terrain.chunks_x
			var cy := index / terrain.chunks_x
			var heights_fast := PackedFloat32Array()
			var heights_ref := PackedFloat32Array()
			var t0 := Time.get_ticks_usec()
			var fast := terrain._chunk_vertices(cx, cy, step, heights_fast)
			var t1 := Time.get_ticks_usec()
			var reference := terrain._chunk_vertices_reference(cx, cy, step, heights_ref)
			reference_us += Time.get_ticks_usec() - t1
			fast_us += t1 - t0
			checked += 1
			if fast != reference or heights_fast != heights_ref:
				print("R2 sommets: tuile (%d, %d) pas %d différente" % [cx, cy, step])
				ok = false
				break
	print("R2 sommets: %d tuiles, %.0f ms contre %.0f ms (référence)" % [checked, fast_us / 1000.0, reference_us / 1000.0])
	if fast_us >= reference_us:
		print("R2 sommets: pas plus rapide que la référence")
		ok = false
	terrain.queue_free()
	print("R2 sommets: %s" % ("OK" if ok else "ÉCHEC"))
	quit(0 if ok else 1)
