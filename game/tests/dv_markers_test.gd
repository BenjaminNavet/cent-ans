extends TestCase

## Test headless des lieux de la carte de campagne (lot DA3, ADR 0066 ; refondu au lot DV2,
## ADR 0124) : rangs, tailles de l'écu par rang, densité des noms par rang et distance caméra,
## atlas des écus (`HeraldryAtlas`). Plus de marqueur peint.
## Usage : godot --headless --path game --script res://tests/dv_markers_test.gd


func _init() -> void:
	await process_frame
	var markers := SettlementMarkers.load_default()
	check(markers.is_valid(), "catalogue loaded")
	check(not markers.catalog.has("atlas") and not markers.catalog.has("kinds"), "no painted marker atlas (DV2)")
	# Rangs par règles de données.
	var paris := {"id": "set_paris", "kind": "city", "weight": 50, "fortification_level": 3}
	var city := {"id": "set_nowhere", "kind": "city", "weight": 30, "fortification_level": 2}
	var fortress := {"id": "set_x", "kind": "castle", "weight": 10, "fortification_level": 4}
	var tower := {"id": "set_y", "kind": "castle", "weight": 10, "fortification_level": 1}
	check(markers.rank_of(paris) == 4, "Paris is a capital (rank 4)")
	check(markers.rank_of(city) == 2, "plain city rank 2")
	check(markers.rank_of(fortress) == 3, "fortification 4 castle rank 3")
	check(markers.rank_of(tower) == 1, "weak castle rank 1")
	check(markers.size_px("city", 4) > markers.size_px("city", 2), "size grows with rank")
	check(markers.size_px("village", 1) < markers.size_px("town", 1), "village size factor")
	# Écu : petit (fraction de la taille de rang), juste au-dessus du nom.
	var factor := markers.shield_size_factor()
	check(factor > 0.2 and factor < 0.8, "shield is a fraction of the rank size (%.2f)" % factor)
	check(markers.shield_gap_px() >= 0.0, "shield gap above the name")
	# Densité par rang : à 1100, seules capitales, grandes cités et forteresses.
	var far := 1100.0
	check(markers.visible_until("city", 4) > far, "capital name at 1100")
	check(markers.visible_until("castle", 3) > far, "fortress name at 1100")
	check(markers.visible_until("city", 2) < far, "plain city name hidden at 1100")
	check(markers.visible_until("village", 1) > 0.0 and markers.visible_until("village", 1) < 500.0, "village only when close")
	# Atlas des écus (HeraldryAtlas).
	var heraldry := HeraldryAtlas.new()
	heraldry.build(["fac_france", "fac_england", "fac_unknown"])
	check(heraldry.shield_of("fac_france") == 0 and heraldry.shield_of("fac_england") == 1, "shield index")
	check(heraldry.shield_of("fac_unknown") == -1, "faction without arms has no shield")
	check(heraldry.has("fac_unknown"), "faction without arms is remembered (no rebuild)")
	check(heraldry.texture != null, "shield atlas built")
	check(heraldry.grid() == Vector2(HeraldryAtlas.COLUMNS, 1), "atlas grid")
	finish()


