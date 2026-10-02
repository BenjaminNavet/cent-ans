class_name BattleMusicDirector
extends Node

## B3 / T4 / DA4 : musique dynamique de bataille par intensité — calme / approche avant contact →
## engagement (mêlée ou tir nourri) → moment critique (un camp proche de la déroute) →
## victoire / défaite. La piste de base (tirée au hasard dans la liste « war » de
## `data/audio/music.json` via `AudioDirector.playlist`, `war.ogg` à défaut) est transformée
## (volume, filtre passe-bas pour l'assourdir à l'approche) et superposée à des couches
## d'instruments d'époque en boucle — tambour, trompette droite, bourdon de cornemuse, chalemie,
## plus la clameur de mêlée héritée de B3 — dont le mélange par état vient de
## `data/audio/battle_layers.json` (schéma `data/schemas/battle_layers.schema.json`, DA4) :
## aucun paramètre n'est codé en dur ici, seule la mécanique (fondu, hystérésis) l'est.
##
## Hystérésis : une montée d'intensité (approach → engagement → critical) est immédiate, une
## descente exige que le nouvel état soit stable pendant `hysteresis_seconds` (config) pour
## éviter les bascules incessantes autour d'un seuil. L'intensité vient de
## `BattleSim.get_units()` (état des régiments, moral, effectifs présents) — `compute_state` est
## une fonction pure, testable sans nœud ni audio.
##
## Silencieux en tête (headless / smoke test) : les flux sont chargés et les états suivis
## normalement, mais rien n'est joué (même convention que `AudioDirector.silent`).

signal state_changed(state: String)

const BUS_NAME := "BatailleMusique"
const PARENT_BUS := "Musique"
const CONFIG_PATH := "audio/battle_layers.json"
const MUSIC_PATH := "res://assets/audio/music/war.ogg"

const URGENCY := {"approach": 0, "engagement": 1, "critical": 2, "victory": 3, "defeat": 3}
const ENGAGED_STATES := {"melee": true, "shooting": true, "charging": true}
const CRITICAL_MORALE := 0.32
const CRITICAL_ROUT_FRACTION := 0.3
const DEFAULT_STATE_PRESET := {"base_db": -40.0, "base_cutoff_hz": 20000.0, "base_pitch": 1.0, "layers": {}}

var current_state: String = "approach"
var silent: bool = false

var _scene: Node = null  # BattleScene
var _pending_state: String = "approach"
var _pending_elapsed: float = 0.0
var _filter: AudioEffectLowPassFilter = null
var _base: AudioStreamPlayer
var _layer_players: Dictionary = {}  # nom de couche -> AudioStreamPlayer
var _stinger: AudioStreamPlayer
var _tween: Tween
## Paramétrage chargé de `data/audio/battle_layers.json` (DA4).
var _config: Dictionary = {}
var _fade_seconds: float = 2.0
var _hysteresis_seconds: float = 2.5
var _mute_when_battle_audio_active: Array = []


## Détermine l'état d'intensité à partir de `BattleSim.get_units()` (fonction pure, sans nœud :
## utilisée telle quelle par les tests). `resolved` / `victory` viennent de `is_finished()` /
## `get_outcome().winner == player_side`.
static func compute_state(units: Array, resolved: bool, victory: bool) -> String:
	if resolved:
		return "victory" if victory else "defeat"
	var engaged := false
	var sides := {"attacker": {"morale": 0.0, "present": 0, "routing": 0, "total": 0}, "defender": {"morale": 0.0, "present": 0, "routing": 0, "total": 0}}
	for entry in units:
		if typeof(entry) != TYPE_DICTIONARY:
			continue
		var unit: Dictionary = entry
		var side := str(unit.get("side", ""))
		if not sides.has(side):
			continue
		var state := str(unit.get("state", ""))
		if ENGAGED_STATES.has(state):
			engaged = true
		var stats: Dictionary = sides[side]
		stats["total"] += 1
		if state == "routing":
			stats["routing"] += 1
		if bool(unit.get("present", false)):
			stats["morale"] += float(unit.get("morale", 1.0))
			stats["present"] += 1
	if not engaged:
		return "approach"
	for side in sides:
		var stats: Dictionary = sides[side]
		if int(stats["present"]) == 0 and int(stats["total"]) == 0:
			continue
		var avg_morale: float = float(stats["morale"]) / maxf(float(stats["present"]), 1.0)
		var rout_fraction: float = float(stats["routing"]) / maxf(float(stats["total"]), 1.0)
		if int(stats["present"]) > 0 and (avg_morale < CRITICAL_MORALE or rout_fraction > CRITICAL_ROUT_FRACTION):
			return "critical"
	return "engagement"


## Lit `data/audio/battle_layers.json` (vide si absent : les couches restent silencieuses, la
## piste de base seule joue, cf. `_apply_state`).
static func _load_config() -> Dictionary:
	var path := SoundBank.data_path(CONFIG_PATH)
	if not FileAccess.file_exists(path):
		return {}
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	return parsed if parsed is Dictionary else {}


## `scene` : `BattleScene` (attend `battle`, `resolved`, `player_side`). À appeler une fois la
## bataille prête ; `update(delta)` ensuite à chaque image.
func setup(scene: Node) -> void:
	_scene = scene
	silent = DisplayServer.get_name() == "headless"
	_config = _load_config()
	_fade_seconds = float(_config.get("fade_seconds", 2.0))
	_hysteresis_seconds = float(_config.get("hysteresis_seconds", 2.5))
	_mute_when_battle_audio_active = _config.get("mute_when_battle_audio_active", [])
	_ensure_bus()
	_base = _make_player("Base", _pick_base_track(), true)
	var layers: Dictionary = _config.get("layers", {})
	for layer_name in layers:
		var path := "res://" + str(layers[layer_name])
		_layer_players[str(layer_name)] = _make_player(str(layer_name).capitalize(), path, true)
	_stinger = _make_player("Stinger", "", false)
	_apply_state("approach", true)


## `p_units` : régiments déjà lus par la scène pour cette image (sinon relus ici).
func update(delta: float, p_units: Variant = null) -> void:
	if _scene == null:
		return
	var battle: Object = _scene.get("battle")
	if battle == null:
		return
	var units: Array = p_units if p_units is Array else battle.call("get_units")
	var resolved: bool = bool(battle.call("is_finished"))
	var victory := false
	if resolved:
		var outcome: Dictionary = battle.call("get_outcome")
		victory = str(outcome.get("winner", "")) == str(_scene.get("player_side"))
	_advance(compute_state(units, resolved, victory), delta)


func _advance(wanted: String, delta: float) -> void:
	if wanted == current_state:
		_pending_state = wanted
		_pending_elapsed = 0.0
		return
	if int(URGENCY[wanted]) > int(URGENCY[current_state]):
		_commit(wanted)
		return
	if wanted != _pending_state:
		_pending_state = wanted
		_pending_elapsed = 0.0
		return
	_pending_elapsed += delta
	if _pending_elapsed >= _hysteresis_seconds:
		_commit(wanted)


func _commit(state: String) -> void:
	current_state = state
	_pending_state = state
	_pending_elapsed = 0.0
	_apply_state(state)
	state_changed.emit(state)


func _apply_state(state: String, instant: bool = false) -> void:
	var states: Dictionary = _config.get("states", {})
	var preset: Dictionary = states.get(state, DEFAULT_STATE_PRESET)
	var base_db: float = float(preset.get("base_db", -40.0))
	var cutoff: float = float(preset.get("base_cutoff_hz", 20000.0))
	var pitch: float = float(preset.get("base_pitch", 1.0))
	var layer_volumes: Dictionary = preset.get("layers", {})
	var stingers: Dictionary = _config.get("stingers", {})
	if stingers.has(state):
		_play_stinger("res://" + str(stingers[state]))
	if silent:
		return
	_start_if_needed(_base)
	var time := 0.05 if instant else _fade_seconds
	if _tween != null and _tween.is_valid():
		_tween.kill()
	_tween = create_tween()
	_tween.set_parallel(true)
	_tween.tween_property(_base, "volume_db", base_db, time)
	_tween.tween_property(_base, "pitch_scale", pitch, time)
	if _filter != null:
		_tween.tween_property(_filter, "cutoff_hz", cutoff, time)
	for layer_name in _layer_players:
		var player: AudioStreamPlayer = _layer_players[layer_name]
		var volume_db: float = float(layer_volumes.get(layer_name, -80.0))
		# AU1 : quand l'audio spatialisé de bataille est actif, ses nappes de mêlée remplacent les
		# couches listées dans `mute_when_battle_audio_active` (sinon le fer serait entendu deux
		# fois) — par défaut `melee_din` (`sfx/sword_clash.ogg`).
		if BattleAudio.active != null and _mute_when_battle_audio_active.has(layer_name):
			volume_db = -80.0
		if volume_db > -79.0:
			_start_if_needed(player)
		_tween.tween_property(player, "volume_db", volume_db, time)


func _play_stinger(path: String) -> void:
	if silent or _stinger == null:
		return
	var stream := _load(path, false)
	if stream == null:
		return
	_stinger.stream = stream
	_stinger.volume_db = 0.0
	_stinger.play()


func _start_if_needed(player: AudioStreamPlayer) -> void:
	if player == null or player.stream == null or player.playing or silent:
		return
	player.play()


func _ensure_bus() -> void:
	if AudioServer.get_bus_index(BUS_NAME) == -1:
		AudioServer.add_bus()
		var index := AudioServer.bus_count - 1
		AudioServer.set_bus_name(index, BUS_NAME)
		var parent := PARENT_BUS if AudioServer.get_bus_index(PARENT_BUS) != -1 else "Master"
		AudioServer.set_bus_send(index, parent)
	var index := AudioServer.get_bus_index(BUS_NAME)
	if AudioServer.get_bus_effect_count(index) == 0:
		_filter = AudioEffectLowPassFilter.new()
		_filter.cutoff_hz = 20000.0
		AudioServer.add_bus_effect(index, _filter)
	else:
		_filter = AudioServer.get_bus_effect(index, 0)


func _make_player(player_name: String, path: String, loop: bool) -> AudioStreamPlayer:
	var player := AudioStreamPlayer.new()
	player.name = player_name
	player.bus = BUS_NAME
	add_child(player)
	if path != "":
		player.stream = _load(path, loop)
	return player


## Piste de base : un morceau de bataille au hasard (différent d'une bataille à l'autre). Liste
## `battle` (ADR 0166 : la carte en guerre a sa propre liste, plus calme), à défaut `war`.
func _pick_base_track() -> String:
	var director: Node = get_node_or_null("/root/AudioDirector")
	var tracks: Array = []
	if director != null and director.has_method("playlist"):
		tracks = director.call("playlist", "battle")
		if tracks.is_empty():
			tracks = director.call("playlist", "war")
	var existing := tracks.filter(func(path: String) -> bool: return ResourceLoader.exists(path))
	return MUSIC_PATH if existing.is_empty() else str(existing.pick_random())


static func _load(path: String, loop: bool) -> AudioStream:
	if not ResourceLoader.exists(path):
		return null
	# Copie : `loop` ne doit pas déteindre sur la ressource partagée avec l'AudioDirector.
	var stream := (load(path) as AudioStream).duplicate() as AudioStream
	stream.set("loop", loop)
	return stream


func _exit_tree() -> void:
	var players: Array = [_base, _stinger]
	players.append_array(_layer_players.values())
	for player in players:
		if player != null:
			player.stop()
			player.stream = null
