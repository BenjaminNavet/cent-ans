extends TestCase

## B4 : les icônes d'unité rapetissent au-delà d'un seuil de distance caméra ; B3 : réglages
## d'ouverture présents dans `camera_feel.json`.
##
## Usage : godot --headless --path game --script res://tests/b4_marker_scale_test.gd


func _init() -> void:
	var start := CameraFeel.get_value("battle", "marker_shrink_start_m")
	var end := CameraFeel.get_value("battle", "marker_shrink_end_m")
	var min_scale := CameraFeel.get_value("battle", "marker_min_scale")
	check(CameraFeel.loaded_from_data(), "camera_feel.json loaded")
	check(end > start and min_scale < 1.0, "shrink settings are sane")
	check(BattleUnitMarkers.scale_for_distance(start - 10.0, start, end, min_scale) == 1.0, "full size below the threshold")
	check(is_equal_approx(BattleUnitMarkers.scale_for_distance(end, start, end, min_scale), min_scale), "min scale at the far end")
	check(is_equal_approx(BattleUnitMarkers.scale_for_distance(end * 3.0, start, end, min_scale), min_scale), "min scale beyond")
	var mid := BattleUnitMarkers.scale_for_distance((start + end) * 0.5, start, end, min_scale)
	check(mid < 1.0 and mid > min_scale, "intermediate scale in between")
	var opening := CameraFeel.get_value("battle", "opening_distance_m")
	check(opening > 100.0 and opening < 220.0, "opening distance %f" % opening)
	finish()
