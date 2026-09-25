extends SceneTree

## Test headless du lot EP4 (son de mêlée de proximité) :
##  1. banque élargie : nouveaux événements présents avec le nombre de variantes attendu ;
##  2. fronts de mêlée : un front proche de la caméra reçoit un émetteur dédié (nappe + chocs
##     individuels), un front loin n'en reçoit pas (repli sur la nappe globale) ;
##  3. la couche « proche » choisit des chocs individuels sans répéter le même type deux fois de
##     suite sur un front ;
##  4. le nombre de voix actives ne dépasse jamais `max_voices`, même avec plusieurs fronts denses ;
##  5. charge de cavalerie et sifflement de volée au-dessus de la caméra sont bien déclenchés.
## Usage : godot --headless --path game --script res://tests/ep4_audio_test.gd

var _failures := 0


func _init() -> void:
	await process_frame
	await _run()
	print("ep4_audio_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("ep4_audio_test: " + message)
	return condition


func _run() -> void:
	_check_bank()
	await _check_layers_by_distance()
	await _check_voice_budget()
	await _check_events()


func _check_bank() -> void:
	var bank := SoundBank.load_default()
	for event_name in ["armor_hit", "effort_cry", "body_fall", "cavalry_charge_impact", "arrow_flyby"]:
		_check(bank.has_event(event_name), "event %s missing" % event_name)
	var counts := {
		"sword_clash": 12,
		"shield_bash": 8,
		"armor_hit": 6,
		"effort_cry": 10,
		"death_groan": 10,
		"body_fall": 6,
		"horse_neigh": 6,
	}
	for event_name in counts:
		var files: Array = bank.event(event_name).get("files", [])
		_check(files.size() >= int(counts[event_name]), "%s should have >= %d clips (%d)" % [event_name, counts[event_name], files.size()])
	for bed_name in ["melee_bed_1", "melee_bed_2", "melee_bed_3"]:
		_check(bank.bed_stream(bed_name) != null, "bed %s missing" % bed_name)
	_check(int(bank.fronts.get("max_emitters", 0)) >= 1, "fronts.max_emitters should be set")
	_check((bank.fronts.get("beds", []) as Array).size() >= 3, "fronts should cycle through the 3 massive melee beds")


func _unit(id: int, side: String, x: float, z: float, soldiers: int, target: int, extra: Dictionary = {}) -> Dictionary:
	var unit := {"id": id, "side": side, "type": "unit_men_at_arms", "render": "infantry", "state": "melee", "present": true, "soldiers": soldiers, "ammo": 0, "x": x, "y": 0.0, "z": z, "facing": 0.0, "depth": 8.0, "is_general": false, "target": target}
	unit.merge(extra, true)
	return unit


func _events(audio: BattleAudio) -> Array:
	return audio.history.map(func(entry: Dictionary) -> String: return str(entry["event"]))


## Un front proche de la caméra doit obtenir un émetteur (nappe active ou chocs) ; un front loin
## ne doit pas en garder un (il retombe sur la nappe globale de `_update_beds`).
func _check_layers_by_distance() -> void:
	var camera := Camera3D.new()
	root.add_child(camera)
	camera.global_position = Vector3(0, 5, 0)
	var bank := SoundBank.load_default()
	var audio := BattleAudio.new()
	root.add_child(audio)
	audio.setup("clear", camera, bank)
	var near_a := _unit(1, "attacker", 0.0, 0.0, 200, 2)
	var near_b := _unit(2, "defender", 4.0, 0.0, 200, 1)
	var far_a := _unit(3, "attacker", 900.0, 0.0, 200, 4)
	var far_b := _unit(4, "defender", 904.0, 0.0, 200, 3)
	for step in 20:
		audio.update([near_a, near_b, far_a, far_b], Vector3(450, 0, 0), 30.0, 0.2, 0.1, 1.0 + step * 0.2)
	_check(audio._front_emitters.has("1:2"), "near front should have a dedicated emitter")
	_check(not audio._front_emitters.has("3:4"), "far front should not keep a dedicated emitter")
	var near_names := _events(audio)
	_check(near_names.size() > 0, "near front should have produced discrete events: %s" % [near_names])
	var near_only := ["sword_clash", "shield_bash", "armor_hit", "effort_cry", "body_fall", "death_groan"]
	var saw_near_event := false
	for n in near_names:
		if near_only.has(n):
			saw_near_event = true
	_check(saw_near_event, "near layer should pick individual clashes: %s" % [near_names])
	# Pas de répétition immédiate du même type sur un front.
	var repeats := 0
	for i in range(1, near_names.size()):
		if near_names[i] == near_names[i - 1] and near_only.has(near_names[i]):
			repeats += 1
	_check(repeats == 0, "the same clash type should not repeat back to back: %s" % [near_names])
	audio.queue_free()
	camera.queue_free()
	await process_frame


## Beaucoup de fronts denses ne doivent jamais dépasser le budget de voix du pool.
func _check_voice_budget() -> void:
	var camera := Camera3D.new()
	root.add_child(camera)
	camera.global_position = Vector3(0, 5, 0)
	var bank := SoundBank.load_default()
	bank.voices["max_voices"] = 10
	var audio := BattleAudio.new()
	root.add_child(audio)
	audio.setup("clear", camera, bank)
	var units: Array = []
	for pair in 8:
		var base := float(pair) * 30.0
		units.append(_unit(pair * 2 + 1, "attacker", base, 0.0, 300, pair * 2 + 2))
		units.append(_unit(pair * 2 + 2, "defender", base + 4.0, 0.0, 300, pair * 2 + 1))
	for step in 40:
		audio.update(units, Vector3.ZERO, 30.0, 0.15, 0.1, 1.0 + step * 0.15)
		_check(audio.busy_voices() <= audio.voice_count(), "voice pool should never exceed max_voices (%d/%d)" % [audio.busy_voices(), audio.voice_count()])
	audio.queue_free()
	camera.queue_free()
	await process_frame


## Charge de cavalerie (grondement + impact) et sifflement de volée au-dessus de la caméra.
func _check_events() -> void:
	var camera := Camera3D.new()
	root.add_child(camera)
	camera.global_position = Vector3(0, 5, 0)
	var bank := SoundBank.load_default()
	var audio := BattleAudio.new()
	root.add_child(audio)
	audio.setup("clear", camera, bank)
	var horse := {"id": 1, "side": "attacker", "type": "unit_knights", "render": "cavalry", "state": "marching", "present": true, "soldiers": 80, "ammo": 0, "x": 0.0, "y": 0.0, "z": -100.0, "facing": 0.0, "depth": 8.0, "is_general": false, "target": -1}
	audio.update([horse], Vector3.ZERO, 30.0, 0.1, 0.1, 1.0)
	horse["state"] = "charging"
	audio.update([horse], Vector3.ZERO, 30.0, 0.1, 0.1, 1.1)
	_check(_events(audio).has("cavalry_charge_impact"), "cavalry charge should trigger the swell + impact event: %s" % [_events(audio)])
	# Volée d'archers qui passe juste au-dessus de la caméra (postée entre tireur et cible).
	var archer := {"id": 2, "side": "attacker", "type": "unit_longbowmen", "render": "infantry", "state": "shooting", "present": true, "soldiers": 100, "ammo": 20, "x": -30.0, "y": 0.0, "z": 0.0, "facing": 0.0, "depth": 8.0, "is_general": false, "target": 3}
	var target := {"id": 3, "side": "defender", "type": "unit_men_at_arms", "render": "infantry", "state": "idle", "present": true, "soldiers": 100, "ammo": 0, "x": 30.0, "y": 0.0, "z": 0.0, "facing": 0.0, "depth": 8.0, "is_general": false, "target": -1}
	audio.update([archer, target], Vector3.ZERO, 30.0, 0.1, 0.1, 2.0)
	archer["ammo"] = 19
	audio.update([archer, target], Vector3.ZERO, 30.0, 0.1, 0.1, 2.1)
	for step in 20:
		audio.update([archer, target], Vector3.ZERO, 30.0, 0.1, 0.1, 2.2 + step * 0.1)
	_check(_events(audio).has("arrow_flyby"), "an overhead volley should whistle above the camera: %s" % [_events(audio)])
	audio.queue_free()
	camera.queue_free()
	await process_frame
