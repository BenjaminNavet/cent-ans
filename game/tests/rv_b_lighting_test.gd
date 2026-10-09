extends TestCase

## Lot RV-B : éclairage de jeu de la carte (`CampaignLighting`, `data/fx/campaign_lighting.json`).
## Phase du tour bornée et déterministe, soleil d'après-midi jamais à l'est, ambiance froide,
## perspective aérienne plus légère de dessus, interpolation sans saut.
## Usage : godot --headless --path game --script res://tests/rv_b_lighting_test.gd


func _init() -> void:
	var cfg := CampaignLighting.data()
	check(not cfg.is_empty(), "campaign_lighting.json missing")
	for turn in 200:
		var p := CampaignLighting.turn_phase(turn)
		check(p >= -1.0 and p <= 1.0, "turn %d: phase %.2f out of [-1, 1]" % [turn, p])
		check(is_equal_approx(p, CampaignLighting.turn_phase(turn)), "turn %d: phase not deterministic" % turn)
	for season in AtmosphereLibrary.SEASONS:
		var preset := CampaignAtmosphere.resolve_preset(season)
		if preset.is_empty():
			check(false, "%s: no preset" % season)
			continue
		for turn in 40:
			var sun := CampaignLighting.sun_target(preset, turn, cfg)
			var az := float(sun["azimuth"])
			check(az >= 200.0 and az <= 290.0, "%s turn %d: afternoon sun expected, azimuth %.0f" % [season, turn, az])
			var bounds: Array = cfg["elevation_bounds_deg"]
			check(float(sun["elevation"]) >= float(bounds[0]) and float(sun["elevation"]) <= float(bounds[1]), "%s turn %d: elevation out of bounds" % [season, turn])
			var color: Color = sun["color"]
			check(color.r >= color.g and color.g >= color.b, "%s turn %d: sun not warm" % [season, turn])
		var amb := CampaignLighting.ambient(season, cfg)
		var amb_color: Color = amb["color"]
		check(amb_color.b > amb_color.r + 0.15, "%s: shade light should be cool sky blue" % season)
	var low := CampaignLighting.aerial(30.0, cfg)
	var top := CampaignLighting.aerial(70.0, cfg)
	check(float(top["density"]) < float(low["density"]), "haze should be lighter seen from above")
	check(float(top["aerial_perspective"]) < float(low["aerial_perspective"]), "aerial perspective lighter from above")
	check(float(top["begin_factor"]) > float(low["begin_factor"]), "haze should start farther seen from above")
	# Interpolation : écart décroissant, azimut par le plus court chemin (350 -> 10 passe par 0).
	var a := {"elevation": 20.0, "azimuth": 350.0, "color": Color(1, 0.8, 0.6), "energy": 1.0}
	var b := {"elevation": 24.0, "azimuth": 10.0, "color": Color(1, 0.9, 0.8), "energy": 1.2}
	var mid := CampaignLighting.blend_sun(a, b, 0.5)
	check(is_equal_approx(wrapf(float(mid["azimuth"]), 0.0, 360.0), 0.0), "azimuth blend should take the short way")
	var gap := CampaignLighting.sun_gap(a, b)
	check(CampaignLighting.sun_gap(mid, b) < gap, "blend should reduce the gap")
	finish()
