extends SceneTree

## Chantier PO (ADR 0097, bible DA § 12.6) : critère C4. Chaque contexte de la tranche résout un
## préréglage complet d'heure du jour et d'étalonnage depuis `data/fx/atmosphere.json`, sans valeur
## de soleil codée en dur :
## - campagne (PO3) : chaque saison ;
## - bataille (PO4) : météo × saison × heure (`morning`, `midday`, `evening`).
## PO4 : partie « bataille » active ; PO3 active la partie « campagne ».
## Usage : godot --headless --path game --script res://tests/po_grade_test.gd

const WEATHERS: Array[String] = ["clear", "fog", "rain", "snow"]
const SEASONS: Array[String] = ["spring", "summer", "autumn", "winter"]
const TIMES: Array[String] = ["morning", "midday", "evening"]
const PHASES: Array[String] = ["dawn", "morning", "midday", "afternoon", "dusk", "night"]


func _init() -> void:
	var failures: Array[String] = []
	failures.append_array(_check_campaign())
	failures.append_array(_check_battle())
	for failure in failures:
		print("po_grade_test FAIL: " + failure)
	print("po_grade_test: %s" % ("OK" if failures.is_empty() else "%d échec(s)" % failures.size()))
	quit(0 if failures.is_empty() else 1)


## Partie « campagne » (PO3).
func _check_campaign() -> Array[String]:
	print("po_grade_test: campagne désactivée (PO3)")
	return []


## Partie « bataille » (PO4) : météo × saison × heure résout ciel, étalonnage et soleil ; l'heure
## suit la phase du cœur, sinon la graine (déterministe, `midday` exclu par temps couvert).
func _check_battle() -> Array[String]:
	var failures: Array[String] = []
	var presets: Dictionary = AtmosphereLibrary.data().get("time_of_day", {})
	for time_key in TIMES:
		var preset: Dictionary = presets.get(time_key, {})
		for field in ["sun_elevation", "sun_azimuth", "sun_color", "sun_energy", "grade", "phases"]:
			if not preset.has(field):
				failures.append("time_of_day.%s.%s manquant" % [time_key, field])
	for weather in WEATHERS:
		for season in SEASONS:
			for time_key in TIMES:
				var ctx := "%s/%s/%s" % [weather, season, time_key]
				var look := BattleAtmosphere.resolve_look(weather, season, time_key)
				if look.is_empty() or (look.get("sky", {}) as Dictionary).is_empty():
					failures.append(ctx + " : pas de ciel")
					continue
				var grades: Array = look.get("grades", [])
				if grades.size() < 3 or (grades[grades.size() - 1] as Dictionary).is_empty():
					failures.append(ctx + " : étalonnage de l'heure absent")
				if AtmosphereLibrary.grade_lut(grades, 1.0) == null:
					failures.append(ctx + " : LUT absente")
				var sun := BattleAtmosphere.sun_for(weather, time_key, look)
				var elevation := float(sun["elevation"])
				if elevation <= 0.0 or elevation > 60.0 or float(sun["energy"]) <= 0.0:
					failures.append(ctx + " : soleil invalide %s" % str(sun))
	# Soleil bas le matin et le soir (ombres longues), plus haut à midi, par temps clair.
	var low_morning := float(BattleAtmosphere.sun_for("clear", "morning")["elevation"])
	var low_evening := float(BattleAtmosphere.sun_for("clear", "evening")["elevation"])
	var high := float(BattleAtmosphere.sun_for("clear", "midday")["elevation"])
	if not (low_morning < high and low_evening < high and low_evening <= 20.0):
		failures.append("soleil : matin %.0f°, midi %.0f°, soir %.0f°" % [low_morning, high, low_evening])
	# L'heure suit la phase du cœur.
	for phase in PHASES:
		var key := BattleAtmosphere.time_key_for({"key": phase}, 1, "clear")
		if not key in TIMES or not phase in ((presets.get(key, {}) as Dictionary).get("phases", []) as Array):
			failures.append("phase %s → %s" % [phase, key])
	# Sans heure du cœur : tirage déterministe, midi exclu par temps couvert, les trois heures
	# présentes par temps clair.
	var seen := {}
	for seed in 64:
		var key := BattleAtmosphere.time_key_for({}, seed, "clear")
		if key != BattleAtmosphere.time_key_for({}, seed, "clear"):
			failures.append("tirage non déterministe (graine %d)" % seed)
		seen[key] = true
		for weather in ["fog", "rain", "snow"]:
			if BattleAtmosphere.time_key_for({}, seed, weather) == "midday":
				failures.append("midi tiré par temps couvert (%s, graine %d)" % [weather, seed])
	if seen.size() != TIMES.size():
		failures.append("tirage par temps clair : %s" % str(seen.keys()))
	print("po_grade_test: bataille %d contextes" % (WEATHERS.size() * SEASONS.size() * TIMES.size()))
	return failures
