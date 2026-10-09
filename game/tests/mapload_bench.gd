extends SceneTree

## Banc de chargement des données vectorielles de la carte (lot SC DT4), sans rendu.
## Chronomètre le chargement GDScript historique (JSON.parse_string) des provinces (deux fois :
## MapData et sélecteur de faction), rivières, côte, routes et rivières rendues.
## Usage : godot --headless --path game --script res://tests/mapload_bench.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")


func _init() -> void:
	await process_frame
	var dir: String = MAP_PATHS.default_data_dir().path_join("map")
	var t0 := Time.get_ticks_msec()
	var timings := {}
	for name in ["provinces.geojson", "provinces.geojson", "rivers.geojson", "coastline.geojson",
			"roads.geojson", "rivers_render.json"]:
		var t := Time.get_ticks_msec()
		var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(dir.path_join(name)))
		var count := 0
		if parsed is Dictionary:
			count = (parsed.get("features", parsed.get("rivers", [])) as Array).size()
		timings[name + "#" + str(timings.size())] = [Time.get_ticks_msec() - t, count]
	print("mapload_bench legacy: total %d ms %s" % [Time.get_ticks_msec() - t0, timings])
	quit(0)
