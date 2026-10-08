extends SceneTree

## Test headless du lot VO1 (voix) :
##  1. données (`VoiceLines`) : langue d'un régiment (faction, noblesse anglaise, unités
##     régionales), répliques par catégorie avec repli, voix d'un général stable, nom des
##     fichiers de discours ;
##  2. `BattleVoices` sur une scène factice : sélection, ordre, charge, déroute, chute du
##     général, victoire ; délais de recharge et écart global (pas de rafale), priorités ;
##     silence pendant le discours, réglage `voice/barks` ;
##  3. `Advisor` : premières fois dites une seule fois, alertes filtrées par faction et
##     espacées, réglage `voice/advisor`, file d'attente, sous-titre ;
##  4. discours (`BattleSpeech`) : calendrier des phrases synchronisé sur les voix ;
##  5. fichiers générés : ceux qui existent se chargent.
## Usage : godot --headless --path game --script res://tests/vo1_voice_test.gd

var _failures := 0


class FakeBattle:
	extends RefCounted
	var finished := false
	var winner := "attacker"

	func is_finished() -> bool:
		return finished

	func get_outcome() -> Dictionary:
		return {"winner": winner} if finished else {}


class FakeScene:
	extends Node
	var setup := {"attacker": {"faction": "fac_england"}, "defender": {"faction": "fac_france"}}
	var units: Array = []
	var selected: Array[int] = []
	var player_side := "attacker"
	var battle: FakeBattle = FakeBattle.new()
	var paused := false
	var speech: Node = null
	var battle_seed := 11


func _init() -> void:
	await process_frame
	await _run()
	print("vo1_voice_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("vo1_voice_test: " + message)
	return condition


func _run() -> void:
	var settings := root.get_node_or_null("Settings")
	if settings != null:
		settings.call("use_test_file")
	_check_data()
	await _check_barks(settings)
	await _check_advisor(settings)
	_check_speech_schedule()
	_check_files()


func _check_data() -> void:
	_check(VoiceLines.language_for("unit_longbowmen", "fac_england") == "en", "English archers speak English")
	_check(VoiceLines.language_for("unit_knights", "fac_england") == "an", "English knights speak Anglo-Norman")
	_check(VoiceLines.language_for("unit_longbowmen", "fac_england", true) == "an", "English general speaks Anglo-Norman")
	_check(VoiceLines.language_for("unit_knights", "fac_france") == "fr", "French knights speak French")
	_check(VoiceLines.language_for("unit_gascon_crossbowmen", "fac_england") == "oc", "Gascons speak Gascon")
	_check(VoiceLines.language_for("unit_welsh_spearmen", "fac_england") == "cy", "Welsh speak Welsh")
	_check(VoiceLines.language_for("unit_flemish_pikemen", "fac_flanders") == "nl", "Flemings speak Flemish")
	_check(VoiceLines.language_for("unit_urban_militia", "fac_castile") == "fr", "unknown faction falls back on French")
	var ranged := VoiceLines.bark_candidates("en", "attack", "ranged")
	var cavalry := VoiceLines.bark_candidates("en", "attack", "cavalry")
	_check(ranged.size() > cavalry.size(), "archers get extra attack lines")
	for line in cavalry:
		_check(str(line.get("category", "")) != "ranged", "cavalry never gets archer lines")
	var welsh_victory := VoiceLines.bark_candidates("cy", "victory", "infantry")
	_check(not welsh_victory.is_empty() and str(welsh_victory[0]["id"]).begins_with("en_"), "Welsh victory falls back on English")
	var v1 := VoiceLines.speech_voice("fac_france", "chr_philippe_vi")
	_check(v1 != "" and v1 == VoiceLines.speech_voice("fac_france", "chr_philippe_vi"), "general keeps his voice")
	var voices := {}
	for id in ["a", "b", "c", "d", "e", "f"]:
		voices[VoiceLines.speech_voice("fac_england", id)] = true
	_check(voices.size() >= 2, "several voices for English generals")
	_check(VoiceLines.speech_path("onyx", "abc") == "speech/onyx/a9993e364706", "speech file naming (sha1)")
	_check(not VoiceLines.advisor_line("campaign_start", "fac_france").is_empty(), "advisor start line")
	_check(str(VoiceLines.advisor_line("campaign_start", "fac_castile").get("id", "")) == "adv_start_default", "advisor default start")
	_check(str(VoiceLines.advisor_line("alert", "fac_france", "plague").get("id", "")) == "adv_plague", "advisor plague alert")


func _unit(id: int, side: String, type: String, category: String, state: String = "idle") -> Dictionary:
	return {"id": id, "side": side, "type": type, "category": category, "state": state, "present": true,
		"soldiers": 100, "is_general": false, "x": 100.0 + id, "z": 100.0, "y": 0.0, "facing": 0.0}


func _check_barks(settings: Node) -> void:
	var scene := FakeScene.new()
	root.add_child(scene)
	scene.units = [
		_unit(1, "attacker", "unit_longbowmen", "ranged"),
		_unit(2, "attacker", "unit_knights", "cavalry"),
		_unit(3, "defender", "unit_knights", "cavalry"),
	]
	scene.units[1]["is_general"] = true
	var voices := BattleVoices.new()
	scene.add_child(voices)
	voices.setup(scene)
	voices.update(0.1)  # premier passage : état de référence
	_check(voices.history.is_empty(), "no bark on the first frame")
	scene.selected = [1]
	voices.update(0.1)
	# La sélection a une probabilité < 1 : on force le tirage par un appel direct si besoin.
	var selected_once := voices.history.size()
	voices._time += 3.0
	voices.on_order({"type": "move", "units": [1]}, {"ok": true})
	var after_order := voices.history.size()
	_check(after_order <= selected_once + 1, "one bark per order at most")
	voices._time += 3.0
	voices.on_order({"type": "attack", "units": [1]}, {"ok": false})
	_check(voices.history.size() == after_order, "refused orders stay silent")
	# Pas de rafale : deux ordres coup sur coup, une seule réplique.
	voices._time += 5.0
	var before := voices.history.size()
	_check(voices.play("attack", scene.units[0], true), "attack bark plays")
	_check(not voices.play("move", scene.units[0], true), "lower priority does not interrupt")
	_check(voices.history.size() == before + 1, "no burst")
	_check(str(voices.history.back()["id"]).begins_with("en_"), "longbowmen answer in English")
	# Une réplique plus prioritaire (déroute) interrompt la courante.
	_check(voices.play("rout", scene.units[2], true), "rout interrupts a lower priority bark")
	_check(str(voices.history.back()["id"]).begins_with("fr_"), "French knights rout in French")
	# Recharge de situation : pas de seconde déroute tout de suite.
	voices._time += 1.5
	_check(not voices.play("rout", scene.units[2], true), "rout cooldown")
	# Transitions d'état : charge anglaise (noblesse → anglo-normand ou repli).
	voices._time += 10.0
	scene.units[1]["state"] = "charging"
	voices.update(0.1)
	var cry_seen := false
	for entry in voices.history:
		cry_seen = cry_seen or (str(entry["situation"]) == "war_cry" and int(entry["unit"]) == 2)
	_check(cry_seen, "first charge of a side: its war cry")
	scene.units[1]["state"] = "idle"
	voices.update(0.1)
	voices._time += 10.0
	scene.units[1]["state"] = "charging"
	voices.update(0.1)
	var charge_seen := false
	for entry in voices.history:
		charge_seen = charge_seen or (str(entry["situation"]) == "charge" and int(entry["unit"]) == 2)
	# chance 0,7 : la charge peut être tirée silencieuse ; on force pour vérifier la langue.
	voices._time += 10.0
	if not charge_seen:
		voices.play("charge", scene.units[1], true)
	_check(str(voices.history.back()["id"]).begins_with("an_"), "English knights charge in Anglo-Norman")
	# Chute du général : un allié proche le crie.
	voices._time += 10.0
	scene.units[1]["present"] = false
	voices.update(0.1)
	_check(str(voices.history.back()["situation"]) == "general_down", "general down bark")
	_check(int(voices.history.back()["unit"]) == 1, "the nearest ally shouts it")
	# Victoire.
	voices._time += 10.0
	scene.battle.finished = true
	voices.update(0.1)
	_check(str(voices.history.back()["situation"]) == "victory", "victory bark")
	# Silence pendant le discours.
	var speech := BattleSpeech.new()
	speech.active = true
	scene.speech = speech
	voices._time += 20.0
	_check(not voices.play("attack", scene.units[0], true), "silent during the speech")
	scene.speech = null
	speech.free()
	# Réglage.
	if settings != null:
		settings.call("set_value", "voice/barks", false, false)
		var muted := BattleVoices.new()
		scene.add_child(muted)
		muted.setup(scene)
		_check(not muted.play("attack", scene.units[0], true), "voice/barks off")
		settings.call("set_value", "voice/barks", true, false)
	scene.queue_free()
	await process_frame


func _check_advisor(settings: Node) -> void:
	if settings != null:
		settings.call("set_value", "voice/advisor_seen", "", false)
	var advisor := Advisor.instance()
	_check(advisor != null, "advisor instance")
	advisor.persist = true  # le fichier de test est isolé
	_check(Advisor.say_trigger("first_battle"), "first battle advice")
	_check(advisor.speaking(), "advisor speaking")
	_check(advisor._panel != null and advisor._panel.visible, "advisor subtitle shown")
	_check(not Advisor.say_trigger("first_battle"), "first battle said only once")
	# File d'attente : une seconde intervention attend la fin de la première.
	_check(Advisor.say_trigger("first_assault"), "first assault queued")
	_check(advisor.said.back() == "adv_first_battle", "queued, not said yet")
	advisor._time = advisor._until + 10.0
	advisor._process(0.0)
	advisor._time += 2.0
	advisor._process(0.0)
	_check(advisor.said.back() == "adv_first_assault", "queue drained")
	advisor.dismiss()
	# Alertes : faction du joueur seulement, une par tour, espacées.
	var events := [
		{"kind": "plague", "faction": "fac_england"},
		{"kind": "plague", "faction": "fac_france"},
		{"kind": "famine", "faction": "fac_france"},
	]
	var count := advisor.said.size()
	Advisor.on_turn_events(events, "fac_france", 5)
	_check(advisor.said.size() == count + 1 and advisor.said.back() == "adv_plague", "one alert, player's faction")
	advisor.dismiss()
	Advisor.on_turn_events([{"kind": "plague", "faction": "fac_france"}], "fac_france", 6)
	_check(advisor.said.size() == count + 1, "same alert not repeated within the cooldown")
	Advisor.on_turn_events([{"kind": "plague", "faction": "fac_france"}], "fac_france", 20)
	_check(advisor.said.size() == count + 2, "alert again after the cooldown")
	advisor.dismiss()
	Advisor.on_turn_events([{"kind": "siege_started", "faction": "fac_france"}], "fac_france", 21)
	_check(advisor.said.back() == "adv_first_siege", "first siege")
	# Q4 : la bulle ne capte pas la souris (sauf « × ») et attend la fermeture d'une modale.
	_check(advisor._panel.mouse_filter == Control.MOUSE_FILTER_IGNORE, "advisor bubble ignores the mouse")
	_check(advisor._close_button != null and advisor._close_button.mouse_filter == Control.MOUSE_FILTER_STOP, "dismiss button")
	_check(advisor._layer.layer < 20, "advisor layer below the modal layers")
	advisor.dismiss()
	var modal := Control.new()
	PanelStack.mark_blocking(modal)
	get_root().add_child(modal)
	advisor._time += 5.0
	advisor._process(0.0)
	advisor.say({"id": "q4_probe", "text": "Essai."})
	_check(advisor.said.back() != "q4_probe" and not advisor._panel.visible, "advisor waits while a modal is open")
	modal.hide()
	advisor._time += 5.0
	advisor._process(0.3)
	_check(advisor.said.back() == "q4_probe", "advisor speaks once the modal is closed")
	modal.free()
	advisor.dismiss()
	if settings != null:
		settings.call("set_value", "voice/advisor", false, false)
		_check(not Advisor.say_trigger("first_victory"), "voice/advisor off")
		settings.call("set_value", "voice/advisor", true, false)
		_check(str(settings.call("get_value", "voice/advisor_seen")).contains("first_battle"), "firsts persisted")
	await process_frame


func _check_speech_schedule() -> void:
	var speech := BattleSpeech.new()
	root.add_child(speech)
	speech.lines = ["Première phrase.", "Seconde phrase."]
	speech.cry = "Montjoie ! Saint-Denis !"
	speech.speaker = "Philippe"
	speech._schedule_voice({"faction": "fac_france", "general_id": "chr_x"})
	_check(speech.voice != "", "speech voice chosen")
	_check(speech._starts.size() == 2 and is_equal_approx(speech._starts[1], speech._durations[0]), "sentences scheduled back to back")
	_check(speech._line_index(speech._starts[1] + 0.01) == 1, "subtitle follows the schedule")
	_check(speech.total_seconds() >= speech._speaking_seconds(), "cry after the sentences")
	speech.queue_free()


func _check_files() -> void:
	var manifest_path := ProjectSettings.globalize_path("res://assets/audio/voice/manifest.json")
	if not FileAccess.file_exists(manifest_path):
		print("vo1_voice_test: no generated voice yet (subtitles only)")
		return
	var manifest: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(manifest_path))
	for relative in manifest:
		_check(VoiceLines.stream(str(relative).trim_suffix(".ogg")) != null, "voice file loads: %s" % relative)
