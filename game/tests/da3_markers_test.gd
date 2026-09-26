extends SceneTree

## Test headless du lot DA3 (ADR 0066) : catalogue des marqueurs de lieux (rangs, pictogrammes,
## dé-encombrement par distance, atlas d'écus).
## Usage : godot --headless --path game --script res://tests/da3_markers_test.gd

var _failures := 0


func _init() -> void:
	await process_frame
	var markers := SettlementMarkers.load_default()
	_check(markers.is_valid(), "catalogue loaded")
	_check(markers.atlas != null, "atlas texture loaded")
	# Rangs par règles de données.
	var paris := {"id": "set_paris", "kind": "city", "weight": 50, "fortification_level": 3}
	var city := {"id": "set_nowhere", "kind": "city", "weight": 30, "fortification_level": 2}
	var fortress := {"id": "set_x", "kind": "castle", "weight": 10, "fortification_level": 4}
	var tower := {"id": "set_y", "kind": "castle", "weight": 10, "fortification_level": 1}
	var village := {"id": "set_z", "kind": "village", "weight": 6, "fortification_level": 0}
	_check(markers.rank_of(paris) == 4, "Paris is a capital (rank 4)")
	_check(markers.rank_of(city) == 2, "plain city rank 2")
	_check(markers.rank_of(fortress) == 3, "fortification 4 castle rank 3")
	_check(markers.rank_of(tower) == 1, "weak castle rank 1")
	_check(markers.pictogram_for("city", 4) == "city_capital", "capital pictogram")
	_check(markers.pictogram_for("castle", 1) == "tower", "tower pictogram")
	_check(markers.pictogram_for("village", 3) == "village", "missing rank falls back to lower rank")
	_check(markers.size_px("city", 4) > markers.size_px("city", 2), "size grows with rank")
	# Dé-encombrement : au palier Europe, seules capitales, grandes cités et forteresses.
	var europe := 1400.0
	_check(markers.visible_until("city", 4) > europe, "capital visible at Europe tier")
	_check(markers.visible_until("castle", 3) > europe, "fortress visible at Europe tier")
	_check(markers.visible_until("city", 2) < europe, "plain city hidden at Europe tier")
	_check(markers.visible_until("village", 1) > 0.0 and markers.visible_until("village", 1) < 500.0, "village only when close")
	# Atlas d'écus.
	markers.build_shield_atlas(["fac_france", "fac_england", "fac_unknown"])
	_check(markers.shield_of("fac_france") == 0 and markers.shield_of("fac_england") == 1, "shield index")
	_check(markers.shield_of("fac_unknown") == -1, "faction without arms has no shield")
	_check(markers.shield_atlas != null, "shield atlas built")
	if _failures == 0:
		print("da3 OK")
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("da3: " + message)
	return condition
