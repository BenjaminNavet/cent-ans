class_name BattleVoices
extends Node

## VO1 — répliques des régiments en bataille : sélection, ordre de marche,
## ordre d'attaque (2D, bus « Voix »), charge, déroute, chute du général (3D, spatialisées par
## `BattleAudio.play_at` sur le bus « Voix »), victoire. Langue selon l'unité et sa faction
## (`VoiceLines.language_for` : français, anglais, anglo-normand pour la noblesse anglaise,
## gascon, flamand, gallois, écossais), corpus `data/voice/barks.json`.
##
## Première charge d'un camp (VX) : le cri de guerre de sa faction, repris en chœur
## (`speech/<voix>/<sha1>.ogg`, même fichier que la fin du discours), à la place de la réplique.
##
## Pas de rafales : une réplique à la fois (une plus prioritaire interrompt la courante),
## écart global minimal entre deux répliques, délai de recharge et probabilité par situation.
## Muet pendant le discours du général et quand le conseiller parle ; désactivable
## (réglage `voice/barks`). Rendu seulement : lit l'état de la scène, ne change aucune règle.
##
## Branchements (`battle_scene.gd`) : `setup(scene)`, `update(delta)` à chaque image,
## `on_order(command, result)` après un ordre du joueur.

const BUS := "Voix"
const EVENT_PREFIX := "vo_"
## Portée des répliques spatialisées (m) et taille de source.
const SPATIAL_MAX_DISTANCE := 240.0
const SPATIAL_UNIT_SIZE := 22.0
## Le cri d'une armée porte plus loin qu'une réplique (VX).
const CRY_MAX_DISTANCE := 700.0
const CRY_UNIT_SIZE := 60.0
const CRY_PRIORITY := 8

var scene: Node = null
var enabled: bool = true
var silent: bool = false
## Dernières répliques décidées (tests) : [{id, situation, unit, time}], 24 au plus.
var history: Array = []

var _player: AudioStreamPlayer = null
var _current_priority: int = -1
var _current_until: float = 0.0
var _last_any: float = -1000.0
var _last_situation: Dictionary = {}  # situation → instant
var _last_line: Dictionary = {}  # langue/situation → dernier id (pas deux fois de suite)
var _time: float = 0.0
var _track: Dictionary = {}  # id → {state, present, soldiers}
var _selection: Array = []
var _victory_done: bool = false
var _side_cried: Dictionary = {}  # camp → vrai une fois son cri de guerre lancé
var _rng := RandomNumberGenerator.new()


func setup(p_scene: Node) -> void:
	name = "BattleVoices"
	scene = p_scene
	silent = DisplayServer.get_name() == "headless"
	_rng.seed = int(p_scene.get("battle_seed")) if p_scene.get("battle_seed") != null else 7
	var settings := get_node_or_null("/root/Settings")
	if settings != null:
		enabled = bool(settings.call("get_value", "voice/barks"))
	_player = AudioStreamPlayer.new()
	_player.name = "Bark2D"
	_player.bus = BUS if AudioServer.get_bus_index(BUS) >= 0 else "Master"
	add_child(_player)


func _exit_tree() -> void:
	if _player != null:
		_player.stop()
		_player.stream = null


func _side_faction(side: String) -> String:
	var setup: Dictionary = scene.get("setup")
	return str((setup.get(side, {}) as Dictionary).get("faction", ""))


## Vrai quand une autre voix a la parole (discours du général, conseiller).
func _muted() -> bool:
	var speech: Variant = scene.get("speech")
	if speech != null and is_instance_valid(speech) and bool(speech.get("active")):
		return true
	var enemy_speech: Variant = scene.get("enemy_speech")
	if enemy_speech != null and is_instance_valid(enemy_speech) and bool(enemy_speech.get("active")):
		return true
	var advisor := Advisor.current()
	return advisor != null and advisor.speaking()


# --- Détection -----------------------------------------------------------------------


func update(delta: float) -> void:
	_time += delta
	var units: Array = scene.get("units")
	var player_side := str(scene.get("player_side"))
	var battle: Object = scene.get("battle")
	var running := not bool(scene.get("paused")) and battle != null and not bool(battle.call("is_finished"))
	_detect_selection(units, player_side)
	var by_id := {}
	for unit in units:
		by_id[int(unit["id"])] = unit
	for unit in units:
		var id := int(unit["id"])
		var state := str(unit["state"])
		var present := bool(unit["present"])
		var soldiers := int(unit.get("soldiers", 0))
		var prev: Dictionary = _track.get(id, {})
		_track[id] = {"state": state, "present": present, "soldiers": soldiers}
		if prev.is_empty() or not running:
			continue
		if bool(unit.get("is_general", false)) and bool(prev["present"]) and (not present or soldiers <= 0):
			var witness := _nearest_ally(units, unit)
			if not witness.is_empty():
				play("general_down", witness)
			continue
		if not present:
			continue
		var prev_state := str(prev["state"])
		if state == "charging" and prev_state != "charging":
			if not war_cry(unit):
				play("charge", unit)
		elif state == "routing" and prev_state != "routing":
			play("rout", unit)
	if battle != null and bool(battle.call("is_finished")) and not _victory_done:
		_victory_done = true
		var outcome: Dictionary = battle.call("get_outcome")
		if str(outcome.get("winner", "")) == player_side:
			var speaker := _first_present(units, player_side, [])
			if not speaker.is_empty():
				play("victory", speaker, true)


func _detect_selection(units: Array, player_side: String) -> void:
	var selected: Array = scene.get("selected")
	var current: Array = selected.duplicate()
	if current == _selection:
		return
	var added: Array = current.filter(func(id: int) -> bool: return not _selection.has(id))
	_selection = current
	if added.is_empty():
		return
	var unit := _first_present(units, player_side, added)
	if not unit.is_empty():
		play("select", unit)


## Après un ordre du joueur : réplique de marche ou d'attaque du premier régiment concerné.
func on_order(command: Dictionary, result: Dictionary) -> void:
	if not bool(result.get("ok", false)):
		return
	var kind := str(command.get("type", ""))
	var situation_name := "attack" if kind == "attack" else ("move" if kind == "move" else "")
	if situation_name == "":
		return
	var ids: Array = command.get("units", [])
	var unit := _first_present(scene.get("units"), str(scene.get("player_side")), ids)
	if not unit.is_empty():
		play(situation_name, unit)


func _first_present(units: Array, side: String, ids: Array) -> Dictionary:
	for unit in units:
		if str(unit["side"]) != side or not bool(unit["present"]) or str(unit.get("category", "")) == "siege":
			continue
		if ids.is_empty() or ids.has(int(unit["id"])):
			return unit
	return {}


func _nearest_ally(units: Array, fallen: Dictionary) -> Dictionary:
	var best := {}
	var best_d := INF
	var origin := BattleAudio._pos(fallen)
	for unit in units:
		if unit == fallen or str(unit["side"]) != str(fallen["side"]) or not bool(unit["present"]):
			continue
		var d := origin.distance_squared_to(BattleAudio._pos(unit))
		if d < best_d:
			best_d = d
			best = unit
	return best


# --- Lecture -------------------------------------------------------------------------


## Joue une réplique de `situation_name` par `unit` ; `force` ignore la probabilité. Faux si
## rien n'est joué (désactivé, muet, recharge, moins prioritaire que la réplique en cours).
func play(situation_name: String, unit: Dictionary, force: bool = false) -> bool:
	if not enabled or _muted():
		return false
	var spec := VoiceLines.situation(situation_name)
	if spec.is_empty():
		return false
	var priority := int(spec.get("priority", 1))
	var speaking := _time < _current_until
	if speaking and priority <= _current_priority:
		return false
	if not speaking and _time - _last_any < float(VoiceLines.barks().get("global_gap_s", 1.0)):
		return false
	if _time - float(_last_situation.get(situation_name, -1000.0)) < float(spec.get("cooldown_s", 0.0)):
		return false
	var spatial := bool(spec.get("spatial", false))
	var position := BattleAudio._pos(unit) + Vector3(0, 1.7, 0)
	if spatial and _too_far(position):
		return false
	if not force and _rng.randf() > float(spec.get("chance", 1.0)):
		return false
	var faction := _side_faction(str(unit["side"]))
	var language := VoiceLines.language_for(str(unit.get("type", "")), faction, bool(unit.get("is_general", false)))
	var candidates := VoiceLines.bark_candidates(language, situation_name, str(unit.get("category", "")))
	if candidates.is_empty():
		return false
	var key := language + "/" + situation_name
	var index := _rng.randi_range(0, candidates.size() - 1)
	if candidates.size() > 1 and str(candidates[index]["id"]) == str(_last_line.get(key, "")):
		index = (index + 1) % candidates.size()
	var line: Dictionary = candidates[index]
	_last_line[key] = str(line["id"])
	var relative := "barks/" + str(line["id"])
	var stream := VoiceLines.stream(relative)
	var length := stream.get_length() if stream != null else 1.2
	_last_situation[situation_name] = _time
	_last_any = _time
	_current_priority = priority
	_current_until = _time + length
	history.append({"id": str(line["id"]), "situation": situation_name, "unit": int(unit["id"]), "time": _time})
	if history.size() > 24:
		history.pop_front()
	if silent or stream == null:
		return true
	if spatial and _battle_audio() != null:
		_play_spatial(relative, line, position)
		return true
	_player.stream = stream
	_player.play()
	return true


## Cri de guerre du camp de `unit` à sa première charge (une fois par camp et par bataille).
## Faux si le camp a déjà crié ou si rien n'est joué (désactivé, muet, cri non généré).
func war_cry(unit: Dictionary) -> bool:
	var side := str(unit["side"])
	if _side_cried.has(side):
		return false
	_side_cried[side] = true
	if not enabled or _muted():
		return false
	var faction := _side_faction(side)
	var cry := VoiceLines.war_cry(faction)
	var relative := VoiceLines.speech_path(VoiceLines.speech_voice(faction, side), cry)
	var stream := VoiceLines.stream(relative)
	if cry == "" or stream == null:
		return false
	_last_any = _time
	_current_priority = CRY_PRIORITY
	_current_until = _time + stream.get_length()
	history.append({"id": relative, "situation": "war_cry", "unit": int(unit["id"]), "time": _time})
	if history.size() > 24:
		history.pop_front()
	if silent:
		return true
	var line := {"id": "cry_" + relative.get_file()}
	if _battle_audio() == null or not _play_spatial(relative, line, BattleAudio._pos(unit) + Vector3(0, 1.7, 0), CRY_UNIT_SIZE, CRY_MAX_DISTANCE):
		_player.stream = stream
		_player.play()
	return true


func _battle_audio() -> BattleAudio:
	var audio := BattleAudio.active
	return audio if audio != null and is_instance_valid(audio) and audio.bank != null else null


## Vrai si `position` est hors de portée d'écoute (répliques spatialisées).
func _too_far(position: Vector3) -> bool:
	var audio := _battle_audio()
	return audio != null and audio._listener_position().distance_to(position) > SPATIAL_MAX_DISTANCE


## Réplique en 3D par le pool d'AU1 : l'événement `vo_<id>` est déclaré à la volée dans la
## banque de la bataille (bus « Voix », une instance, priorité haute).
func _play_spatial(relative: String, line: Dictionary, position: Vector3, unit_size: float = SPATIAL_UNIT_SIZE, max_distance: float = SPATIAL_MAX_DISTANCE) -> bool:
	var audio := _battle_audio()
	if audio == null:
		return false
	var event_name := EVENT_PREFIX + str(line["id"])
	if not audio.bank.has_event(event_name):
		var entry: Dictionary = SoundBank.EVENT_DEFAULTS.duplicate(true)
		entry.merge({
			"files": ["voice/" + relative], "bus": BUS, "priority": 9, "volume_db": 0.0,
			"pitch": [0.98, 1.02], "max_instances": 1, "cooldown_s": 0.0,
			"unit_size_m": unit_size, "max_distance_m": max_distance,
		}, true)
		audio.bank.events[event_name] = entry
	return BattleAudio.play_at(event_name, position)
