extends SceneTree

## Banc et garde-fou du chargeur des données vectorielles de la carte (lot SC DT4, ADR 0206).
## Chronomètre `MapDataLoader` (provinces, sélecteur depuis le cache, rivières, côte, routes,
## rivières rendues) et vérifie les effectifs. Avant le lot, le même chargement en GDScript prenait
## 490-530 ms (parseurs supprimés ; comparaison octet à octet faite avant suppression, ADR 0206).
## Usage : godot --headless --path game --script res://tests/mapload_bench.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")

var _failures := 0


func _init() -> void:
	await process_frame
	var dir: String = MAP_PATHS.default_data_dir().path_join("map")
	MapDataLoader.clear_cache()
	var t := Time.get_ticks_msec()
	var provinces := MapDataLoader.provinces(dir)
	var t_provinces := Time.get_ticks_msec() - t
	t = Time.get_ticks_msec()
	var picker := MapDataLoader.province_polygons(dir.path_join("provinces.geojson"))
	var t_picker := Time.get_ticks_msec() - t
	t = Time.get_ticks_msec()
	var rivers := MapDataLoader.rivers(dir)
	var coast := MapDataLoader.coastline(dir)
	var roads := MapDataLoader.roads(dir)
	var rendered := MapDataLoader.rendered_rivers(dir)
	var t_lines := Time.get_ticks_msec() - t
	_check(provinces.size() == 443, "443 provinces, got %d" % provinces.size())
	_check(picker.size() == provinces.size(), "picker size")
	_check(not (provinces[0]["rings"] as Array).is_empty(), "province rings")
	_check(rivers.size() == 1883, "1883 rivers, got %d" % rivers.size())
	_check(coast.size() == 497, "497 coast lines, got %d" % coast.size())
	_check(roads.size() == 13198, "13198 roads, got %d" % roads.size())
	_check(rendered.get("rivers", []).size() == 1888, "1888 rendered rivers")
	_check(float(rendered.get("bank_px", 0.0)) > 0.0, "bank_px")
	print("mapload_bench: provinces %d ms, picker (cache) %d ms, rivers+coast+roads+rendered %d ms" % [t_provinces, t_picker, t_lines])
	print("mapload_bench: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(ok: bool, message: String) -> void:
	if not ok:
		_failures += 1
		printerr("FAIL: " + message)
