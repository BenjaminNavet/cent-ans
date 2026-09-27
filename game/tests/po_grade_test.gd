extends SceneTree

## Chantier PO (ADR 0097, bible DA § 12.6) : critère C4. Chaque contexte de la tranche résout un
## préréglage complet d'heure du jour et d'étalonnage depuis `data/fx/atmosphere.json`, sans valeur
## de soleil codée en dur :
## - campagne (PO3) : chaque saison ;
## - bataille (PO4) : météo × saison × heure (`morning`, `midday`, `evening`).
## PO3 : partie campagne active ; partie bataille désactivée jusqu'à PO4.
## Usage : godot --headless --path game --script res://tests/po_grade_test.gd

var _failures: PackedStringArray = PackedStringArray()


func _init() -> void:
	_campaign()
	print("po_grade_test: bataille désactivée (PO4)")
	if _failures.is_empty():
		print("po_grade_test: OK")
		quit(0)
	else:
		for failure in _failures:
			printerr("po_grade_test: FAIL ", failure)
		quit(1)


func _check(condition: bool, message: String) -> void:
	if not condition:
		_failures.append(message)


## PO3 : carte × saison. Soleil rasant de fin d'après-midi (18-28°) venu de l'ouest-sud-ouest,
## chaud ; brume bleutée (pas grise) dorée côté soleil ; étalonnage de saison puis de carte en S.
func _campaign() -> void:
	# Aucune valeur de soleil dans le code : sans données, le soleil reste celui de la scène.
	var bare := CampaignAtmosphere.new()
	_check(bare.sun_elevation_deg < 0.0 and bare.sun_azimuth_deg < 0.0, "campaign: sun values hard-coded in CampaignAtmosphere")
	bare.free()
	for season in AtmosphereLibrary.SEASONS:
		var preset := CampaignAtmosphere.resolve_preset(season)
		if preset.is_empty():
			_check(false, "campaign %s: no complete preset" % season)
			continue
		var elevation := float(preset["sun_elevation"])
		var azimuth := float(preset["sun_azimuth"])
		_check(elevation >= 18.0 and elevation <= 28.0, "campaign %s: sun elevation %.1f outside 18-28" % [season, elevation])
		_check(azimuth >= 225.0 and azimuth <= 270.0, "campaign %s: sun azimuth %.1f not west-south-west" % [season, azimuth])
		var sun: Color = preset["sun_color"]
		_check(sun.r >= sun.g and sun.g >= sun.b and sun.r - sun.b >= 0.1, "campaign %s: sun colour not warm" % season)
		_check(float(preset["sun_energy"]) > 0.0, "campaign %s: sun energy" % season)
		var fog: Color = preset["fog_color"]
		_check(fog.b - fog.r >= 0.03, "campaign %s: fog colour grey, not blue" % season)
		_check(float(preset["fog_sun_scatter"]) > 0.0, "campaign %s: no golden sun scatter" % season)
		_check(not (preset["sky"] as Dictionary).is_empty(), "campaign %s: sky %s unknown" % [season, preset["sky_id"]])
		var grades: Array = preset["grades"]
		_check(grades.size() >= 2, "campaign %s: season and map grades expected" % season)
		if grades.size() >= 2:
			_check(float((grades[-1] as Dictionary).get("contrast", 1.0)) > 1.0, "campaign %s: map grade without S curve" % season)
		var lut := AtmosphereLibrary.grade_lut(grades, float(preset["strength"]))
		_check(lut != null and lut.get_width() > 0, "campaign %s: grade LUT" % season)
	print("po_grade_test: campagne vérifiée (%d saisons)" % AtmosphereLibrary.SEASONS.size())
