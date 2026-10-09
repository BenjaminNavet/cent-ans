extends TestCase

## EP8 : mise en scène des batailles (heure du jour, nuages, fumées, oiseaux, plan cinématique).
## Usage : godot --headless --path game --script res://tests/ep8_staging_test.gd
## Code de sortie 0 si tout passe, 1 sinon.


func _init() -> void:
	await process_frame
	var cfg := BattleStaging.load_config()
	check(not cfg.is_empty(), "data/fx/battle_staging.json loads")
	# Heure du jour : midi neutre (rendu d'avant EP8), aube rasante et chaude, nuit sombre.
	var tod := BattleTimeOfDay.new((cfg["time_of_day"] as Dictionary)["keyframes"])
	var noon := tod.sample(12.0)
	check(is_equal_approx(float(noon["elevation_mul"]), 1.0) and is_equal_approx(float(noon["energy_mul"]), 1.0), "midday is neutral")
	check((noon["sun_tint"] as Color).is_equal_approx(Color.WHITE), "midday sun untinted")
	var dawn := tod.sample(5.5)
	check(float(dawn["elevation_mul"]) < 0.3, "dawn: low sun (long shadows)")
	check((dawn["sun_tint"] as Color).b < (dawn["sun_tint"] as Color).r, "dawn: warm light")
	check(not bool(dawn["hdri"]), "dawn: procedural sky")
	var night := tod.sample(23.0)
	check(float(night["energy_mul"]) < 0.3, "night: dark")
	var wrap := tod.sample(23.99)
	check(absf(float(wrap["energy_mul"]) - float(tod.sample(0.01)["energy_mul"])) < 0.05, "keyframes wrap around midnight")
	check(BattleTimeOfDay.clock_label({"label": "Aube", "hour": 5.75}) == "Aube, 5 h 40", "clock label (%s)" % BattleTimeOfDay.clock_label({"label": "Aube", "hour": 5.75}))
	# Application sur un environnement : à midi rien ne change, à l'aube le soleil est rasant.
	var env := Environment.new()
	env.fog_light_color = Color(0.7, 0.76, 0.84)
	env.ambient_light_energy = 1.0
	var sky_mat := ShaderMaterial.new()
	sky_mat.shader = BattleTimeOfDay.SKY_SHADER
	env.sky = Sky.new()
	env.sky.sky_material = sky_mat
	var sun := DirectionalLight3D.new()
	sun.rotation = Vector3(deg_to_rad(-34.0), deg_to_rad(-142.0), 0.0)
	sun.light_energy = 1.75
	get_root().add_child(sun)
	tod.capture(env, sun, "clear")
	tod.apply(12.0)
	check(is_equal_approx(sun.light_energy, 1.75), "noon keeps the sun energy")
	check(absf(rad_to_deg(sun.rotation.x) + 34.0) < 0.01, "noon keeps the sun elevation")
	tod.apply(5.5)
	check(-rad_to_deg(sun.rotation.x) < 6.0, "dawn sun near the horizon (%.1f°)" % -rad_to_deg(sun.rotation.x))
	check(sun.light_energy < 1.75, "dawn sun weaker")
	sun.queue_free()
	# Fumées : budget par qualité, la plus faible cède la place.
	var smoke := BattleSmoke.new()
	get_root().add_child(smoke)
	smoke.setup(cfg["smoke"], {"dir": Vector2(1, 0), "strength": 0.5})
	smoke.max_sources = 2
	var a := smoke.add_smoke_source(Vector3(0, 0, 0), 0.5)
	var b := smoke.add_smoke_source(Vector3(10, 0, 0), 0.8)
	var c := smoke.add_smoke_source(Vector3(20, 0, 0), 0.3)
	check(a > 0 and b > 0 and c == -1, "weaker source refused when full")
	var d := smoke.add_smoke_source(Vector3(30, 0, 0), 1.0, "column")
	check(d > 0 and smoke.sources.size() == 2 and not smoke.sources.has(a), "stronger source replaces the weakest")
	smoke.remove_smoke_source(b)
	check(smoke.sources.size() == 1, "source removed")
	smoke.queue_free()
	# Oiseaux : perchés, puis envolés par une charge à proximité ; corbeaux à la fin.
	var birds := BattleBirds.new()
	get_root().add_child(birds)
	var terrain := {"width": 1800.0, "depth": 1200.0, "forests": [{"x": 900.0, "z": 400.0, "radius": 60.0}], "obstacles": []}
	birds.setup(cfg["birds"], terrain, func(_x: float, _z: float) -> float: return 0.0, 7)
	check(birds.flocks.size() >= 2, "a flock and the crows")
	var charge := [{"present": true, "state": "charging", "x": 900.0, "z": 520.0, "soldiers": 200}]
	birds.update(charge, 0.1, false)
	check(birds.launched >= 1 and str(birds.flocks[0]["state"]) == "flee", "a charge flushes the flock")
	for _i in 60:
		birds.update(charge, 0.1, false)
	check(str(birds.flocks[0]["state"]) == "circle", "the flock circles over the field")
	var melee := [{"present": true, "state": "melee", "x": 900.0, "z": 600.0, "soldiers": 200}]
	birds.update(melee, 0.1, true)
	check(birds.crows_out, "crows after the battle")
	birds.queue_free()
	finish()


