extends TestCase

## Test headless du lot AU1 (audio) :
##  1. disposition des bus (limiteur sur Master, BatailleLointain → Bataille, ducking) ;
##  2. banque `data/audio/sound_bank.json` : tous les fichiers se chargent ;
##  3. pool de voix de `BattleAudio` : API statique, limite d'instances, recharge, vol de voix
##     par priorité ;
##  4. événements déduits des régiments (charge, cri de guerre, contact, volée et impact différé,
##     déroute, mort de général + ducking), nappes de mêlée, bus lointain selon le zoom ;
##  5. ambiances de carte (`CampaignAmbience.layer_levels`, météo saisonnière) ;
##  6. volumes par bus persistés, curseurs de réglage.
## Usage : godot --headless --path game --script res://tests/au1_audio_test.gd


func _init() -> void:
	await process_frame
	await _run()
	finish()


func _run() -> void:
	_check_buses()
	_check_bank()
	await _check_battle_audio()
	_check_campaign_ambience()
	_check_volumes()


func _check_buses() -> void:
	AudioBuses.ensure_layout()
	for spec in AudioBuses.PLAYER_BUSES:
		check(AudioServer.get_bus_index(spec[0]) != -1, "bus %s missing" % spec[0])
	var far := AudioServer.get_bus_index(AudioBuses.BATTLE_FAR)
	check(far != -1 and AudioServer.get_bus_send(far) == AudioBuses.BATTLE, "BatailleLointain should send to Bataille")
	var master := AudioServer.get_bus_index("Master")
	var limited := false
	for i in AudioServer.get_bus_effect_count(master):
		limited = limited or AudioServer.get_bus_effect(master, i) is AudioEffectHardLimiter
	check(limited, "Master should end with a hard limiter")
	AudioBuses.ensure_layout()  # idempotent
	check(AudioServer.get_bus_effect_count(far) == 2, "far bus effects duplicated")
	AudioBuses.set_battle_distance(0.0)
	var reverb := AudioServer.get_bus_effect(far, AudioBuses.FAR_REVERB_EFFECT) as AudioEffectReverb
	var near_wet := reverb.wet
	AudioBuses.set_battle_distance(1.0)
	check(reverb.wet > near_wet, "reverb should grow with camera height")


func _check_bank() -> void:
	var bank := SoundBank.load_default()
	check(bank.events.size() >= 20, "sound bank should define at least 20 events (%d)" % bank.events.size())
	var missing: Array = []
	for relative in bank.all_files():
		if bank.stream(str(relative), false) == null:
			missing.append(relative)
	check(missing.is_empty(), "sound bank files missing: %s" % [missing])
	for event_name in ["sword_clash", "arrow_whistle", "arrow_impact", "charge_cry", "war_cry", "death_groan", "horse_neigh", "horn", "drum", "bell_toll", "ram_hit", "trebuchet_release", "stone_impact", "wall_collapse", "bombard", "thunder", "general_death", "contact"]:
		check(bank.has_event(event_name), "event %s missing" % event_name)
	for bed_name in ["melee_bed_1", "clamor_bed", "march_bed", "cavalry_bed", "fire_bed"]:
		check(bank.bed_stream(bed_name) != null, "bed %s missing" % bed_name)
	for layer in CampaignAmbience.LAYERS:
		check(bank.ambience_stream(layer) != null, "ambience %s missing" % layer)


func _unit(id: int, side: String, state: String, x: float, extra: Dictionary = {}) -> Dictionary:
	var unit := {"id": id, "side": side, "type": "unit_men_at_arms", "render": "infantry", "state": state, "present": true, "soldiers": 100, "ammo": 0, "x": x, "y": 0.0, "z": 0.0, "facing": 0.0, "depth": 8.0, "is_general": false, "target": -1}
	unit.merge(extra, true)
	return unit


func _events(audio: BattleAudio) -> Array:
	return audio.history.map(func(entry: Dictionary) -> String: return str(entry["event"]))


func _check_battle_audio() -> void:
	var camera := Camera3D.new()
	root.add_child(camera)
	camera.global_position = Vector3(0, 30, 40)
	var bank := SoundBank.load_default()
	bank.voices["max_voices"] = 4
	var audio := BattleAudio.new()
	root.add_child(audio)
	audio.setup("rain", camera, bank)
	check(BattleAudio.active == audio, "BattleAudio.active not set")
	check(audio.voice_count() == 4, "pool size should follow the bank")
	# API statique, événement inconnu, portée.
	check(BattleAudio.play_at("arrow_impact", Vector3.ZERO), "play_at(arrow_impact) failed")
	check(not BattleAudio.play_at("does_not_exist", Vector3.ZERO), "unknown event should be refused")
	check(not BattleAudio.play_at("death_groan", Vector3(5000, 0, 0)), "far event should be culled")
	# Recharge : même instant → refusé.
	check(not BattleAudio.play_at("arrow_impact", Vector3.ZERO), "cooldown should block an immediate replay")
	for i in 3:
		audio._time += 0.1
		BattleAudio.play_at("arrow_impact", Vector3.ZERO)
	check(audio.busy_voices() == 4, "pool should be full (%d)" % audio.busy_voices())
	audio._time += 0.01
	check(not BattleAudio.play_at("death_groan", Vector3.ZERO), "low priority sound should not steal a voice")
	check(BattleAudio.play_at("war_cry", Vector3.ZERO), "high priority sound should steal a voice")
	check(audio.busy_voices() == 4, "stealing should not grow the pool")
	audio.queue_free()
	await process_frame
	check(BattleAudio.active == null, "BattleAudio.active should clear on exit")

	# Événements déduits des régiments.
	bank = SoundBank.load_default()
	audio = BattleAudio.new()
	root.add_child(audio)
	audio.setup("clear", camera, bank)
	var director: Node = root.get_node_or_null("/root/AudioDirector")
	var a := _unit(1, "attacker", "marching", 0.0)
	var d := _unit(2, "defender", "idle", 20.0, {"is_general": true})
	var archers := _unit(3, "defender", "shooting", 60.0, {"type": "unit_longbowmen", "ammo": 20, "target": 1})
	audio.update([a, d, archers], Vector3.ZERO, 30.0, 0.1, 0.1, 1.0)
	a["state"] = "charging"
	audio.update([a, d, archers], Vector3.ZERO, 30.0, 0.1, 0.1, 1.1)
	var names := _events(audio)
	check(names.has("war_cry") and names.has("charge_cry"), "charge should play war_cry + charge_cry: %s" % [names])
	a["state"] = "melee"
	d["state"] = "melee"
	archers["ammo"] = 19
	audio.update([a, d, archers], Vector3.ZERO, 30.0, 0.1, 0.1, 1.2)
	names = _events(audio)
	check(names.has("shield_bash") and names.has("sword_clash"), "contact should clash: %s" % [names])
	check(names.has("bow_release"), "volley should play bow_release: %s" % [names])
	for step in 30:
		audio.update([a, d, archers], Vector3.ZERO, 30.0, 0.1, 0.1, 1.3 + step * 0.1)
	names = _events(audio)
	check(names.has("arrow_whistle") and names.has("arrow_impact"), "volley should whistle then hit: %s" % [names])
	check(float(audio.bed_levels.get("melee_bed_1", 0.0)) > 0.2, "melee bed should rise (%.2f)" % float(audio.bed_levels.get("melee_bed_1", 0.0)))
	a["state"] = "routing"
	d["present"] = false
	audio.update([a, d, archers], Vector3.ZERO, 30.0, 0.1, 0.1, 5.0)
	names = _events(audio)
	check(names.has("rout_cry"), "rout should cry: %s" % [names])
	check(names.has("horn"), "general death should sound the horn: %s" % [names])
	if director != null:
		for i in 3:
			await process_frame
		check(float(director.call("music_duck_db")) < -1.0, "general death should duck the music (%.1f dB)" % float(director.call("music_duck_db")))
	# Zoom : caméra haute → bus lointain plus filtré.
	audio.update([], Vector3.ZERO, 400.0, 0.0, 0.1, 5.1)
	check(audio.far01 > 0.9, "high camera should be far (%.2f)" % audio.far01)
	# Siège : coup sur la porte, pan de mur qui tombe.
	var siege := {"center": Vector2(0, 0), "pieces": [{"index": 0, "kind": "gate", "hp": 100.0, "intact": true, "a": Vector2(-5, 0), "b": Vector2(5, 0)}, {"index": 1, "kind": "wall", "hp": 50.0, "intact": true, "a": Vector2(5, 0), "b": Vector2(30, 0)}], "houses": []}
	audio.update_siege(siege, 200.0)
	siege["pieces"][0]["hp"] = 90.0
	siege["pieces"][1]["hp"] = 0.0
	siege["pieces"][1]["intact"] = false
	audio.update_siege(siege, 200.5)
	names = _events(audio)
	check(names.has("ram_hit") and names.has("wall_collapse"), "siege should ram and collapse: %s" % [names])
	audio.queue_free()
	camera.queue_free()
	await process_frame


func _check_campaign_ambience() -> void:
	var sea := CampaignAmbience.layer_levels({"sea": 1.0, "distance": 60.0, "season": "summer", "weather": "clear"})
	check(float(sea["sea"]) > 0.8 and float(sea["forest"]) == 0.0, "open sea should sound like the sea: %s" % [sea])
	var forest := CampaignAmbience.layer_levels({"forest": 0.8, "fields": 0.3, "distance": 60.0, "season": "summer", "weather": "clear"})
	check(float(forest["forest"]) > 0.5 and float(forest["crickets"]) > 0.1, "summer forest: %s" % [forest])
	var winter := CampaignAmbience.layer_levels({"fields": 0.8, "distance": 60.0, "season": "winter", "weather": "snow"})
	check(float(winter["crickets"]) == 0.0 and float(winter["wind_strong"]) > 0.5 and float(winter["rain"]) == 0.0, "winter: %s" % [winter])
	var rain := CampaignAmbience.layer_levels({"fields": 0.8, "distance": 60.0, "season": "autumn", "weather": "rain"})
	check(float(rain["rain"]) > 0.5, "rain layer: %s" % [rain])
	var town := CampaignAmbience.layer_levels({"fields": 0.5, "town": 1.0, "distance": 60.0, "season": "spring", "weather": "clear"})
	check(float(town["town"]) > 0.8, "town layer: %s" % [town])
	var high := CampaignAmbience.layer_levels({"fields": 1.0, "forest": 0.5, "distance": 1400.0, "season": "spring", "weather": "clear"})
	var low := CampaignAmbience.layer_levels({"fields": 1.0, "forest": 0.5, "distance": 60.0, "season": "spring", "weather": "clear"})
	check(float(high["wind"]) > float(low["wind"]) and float(high["countryside"]) < 0.05 and float(low["countryside"]) > 0.5, "zoom should trade local layers for altitude wind: %s / %s" % [low, high])
	var first := CampaignAmbience.seasonal_weather("autumn", "Automne 1340")
	check(first == CampaignAmbience.seasonal_weather("autumn", "Automne 1340"), "seasonal weather must be deterministic")
	check(CampaignAmbience.seasonal_weather("winter", "") == "clear", "no date → clear")
	var rainy := 0
	for year in range(1337, 1437):
		if CampaignAmbience.seasonal_weather("autumn", "Automne %d" % year) == "rain":
			rainy += 1
	check(rainy > 25 and rainy < 75, "autumn should rain about half the time (%d/100)" % rainy)
	# Vraie carte : un point en mer et le centre d'une province.
	var map_dir := (load("res://scripts/map/map_paths.gd") as GDScript).call("default_data_dir").path_join("map") as String
	var map_data := MapData.load_from_dir(map_dir)
	if not check(map_data.load_error == "", "map load failed: %s" % map_data.load_error):
		return
	var sea_point := Vector2(-1, -1)
	for y in range(8, map_data.size.y - 8, 16):
		for x in range(8, map_data.size.x - 8, 16):
			if sea_point.x < 0 and not map_data.is_land_px(x, y) and not map_data.is_land_px(x + 6, y + 6) and not map_data.is_land_px(x - 6, y - 6) and not map_data.is_land_px(x + 6, y - 6) and not map_data.is_land_px(x - 6, y + 6):
				sea_point = Vector2(x, y)
	var towns := PackedVector2Array()
	var land_point := Vector2.ZERO
	for province in map_data.provinces.values():
		towns.append(province["capital_px"])
		land_point = province["centroid"]
	var at_sea := CampaignAmbience.sample_environment(map_data, sea_point, 15.0, towns)
	check(float(at_sea["sea"]) > 0.9, "sea sample should be sea: %s at %s" % [at_sea, sea_point])
	var inland := CampaignAmbience.sample_environment(map_data, land_point, 15.0, towns)
	check(float(inland["sea"]) < 0.5 and float(inland["fields"]) + float(inland["forest"]) > 0.2, "province centre should be land: %s" % [inland])


func _check_volumes() -> void:
	var settings: Node = root.get_node_or_null("/root/Settings")
	if not check(settings != null, "Settings autoload missing"):
		return
	settings.call("use_test_file")
	var original: float = settings.call("bus_volume", "Ambiance")
	settings.call("set_bus_volume", "Ambiance", 0.3)
	check(is_equal_approx(float(settings.call("bus_volume", "Ambiance")), 0.3), "Ambiance volume not stored")
	var index := AudioServer.get_bus_index("Ambiance")
	check(absf(AudioServer.get_bus_volume_db(index) - linear_to_db(0.3)) < 0.01, "Ambiance bus volume not applied")
	settings.call("set_bus_volume", "Ambiance", original)
