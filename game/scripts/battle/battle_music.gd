class_name BattleMusicDirector
extends Node

## B3 / T4 : musique dynamique de bataille par intensité — calme / approche avant contact →
## engagement (mêlée ou tir nourri) → moment critique (un camp proche de la déroute) →
## victoire / défaite. Aucune piste dédiée n'existe pour ces états : on réutilise et transforme
## `music/war.ogg` (volume, filtre passe-bas pour l'assourdir à l'approche) plutôt que d'en
## générer, avec deux couches d'ambiance en boucle réutilisant des effets déjà présents
## (`sfx/sword_clash.ogg` pour la clameur / le fer, `sfx/march_drum.ogg` pour la percussion du
## moment critique) et un stinger de victoire (`fanfare`) ou de défaite (`choir`).
##
## Hystérésis : une montée d'intensité (approche → engagement → critique) est immédiate, une
## descente exige que le nouvel état soit stable pendant `HYSTERESIS_SECONDS` pour éviter les
## bascules incessantes autour d'un seuil. L'intensité vient de `BattleSim.get_units()`
## (état des régiments, moral, effectifs présents) — `compute_state` est une fonction pure,
## testable sans nœud ni audio.
##
## Silencieux en tête (headless / smoke test) : les flux sont chargés et les états suivis
## normalement, mais rien n'est joué (même convention que `AudioDirector.silent`).

signal state_changed(state: String)

const BUS_NAME := "BatailleMusique"
const PARENT_BUS := "Musique"
const FADE_SECONDS := 2.0
const HYSTERESIS_SECONDS := 2.5

const URGENCY := {"approach": 0, "engagement": 1, "critical": 2, "victory": 3, "defeat": 3}
const ENGAGED_STATES := {"melee": true, "shooting": true, "charging": true}
const CRITICAL_MORALE := 0.32
const CRITICAL_ROUT_FRACTION := 0.3

const MUSIC_PATH := "res://assets/audio/music/war.ogg"
const AMBIENCE_PATH := "res://assets/audio/sfx/sword_clash.ogg"
const PERCUSSION_PATH := "res://assets/audio/sfx/march_drum.ogg"
const VICTORY_STINGER := "res://assets/audio/sfx/fanfare.ogg"
const DEFEAT_STINGER := "res://assets/audio/sfx/choir.ogg"

## Réglages par état : [volume_db piste de base, coupure du filtre passe-bas (Hz),
## volume_db ambiance, volume_db percussion, hauteur de la piste de base].
const PRESETS := {
	"approach": [-11.0, 900.0, -80.0, -80.0, 1.0],
	"engagement": [-3.0, 20000.0, -14.0, -80.0, 1.0],
	"critical": [0.0, 20000.0, -8.0, -6.0, 1.05],
	"victory": [-40.0, 20000.0, -80.0, -80.0, 1.0],
	"defeat": [-40.0, 500.0, -80.0, -80.0, 0.94],
}

var current_state: String = "approach"
var silent: bool = false

var _scene: Node = null  # BattleScene
var _pending_state: String = "approach"
var _pending_elapsed: float = 0.0
var _filter: AudioEffectLowPassFilter = null
var _base: AudioStreamPlayer
var _ambience: AudioStreamPlayer
var _percussion: AudioStreamPlayer
var _stinger: AudioStreamPlayer
var _tween: Tween


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


## `scene` : `BattleScene` (attend `battle`, `resolved`, `player_side`). À appeler une fois la
## bataille prête ; `update(delta)` ensuite à chaque image.
func setup(scene: Node) -> void:
	_scene = scene
	silent = DisplayServer.get_name() == "headless"
	_ensure_bus()
	_base = _make_player("Base", MUSIC_PATH, true)
	_ambience = _make_player("Ambience", AMBIENCE_PATH, true)
	_percussion = _make_player("Percussion", PERCUSSION_PATH, true)
	_stinger = _make_player("Stinger", "", false)
	_apply_state("approach", true)


func update(delta: float) -> void:
	if _scene == null:
		return
	var battle: Object = _scene.get("battle")
	if battle == null:
		return
	var units: Array = battle.call("get_units")
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
	if _pending_elapsed >= HYSTERESIS_SECONDS:
		_commit(wanted)


func _commit(state: String) -> void:
	current_state = state
	_pending_state = state
	_pending_elapsed = 0.0
	_apply_state(state)
	state_changed.emit(state)


func _apply_state(state: String, instant: bool = false) -> void:
	var preset: Array = PRESETS.get(state, PRESETS["approach"])
	var base_db: float = preset[0]
	var cutoff: float = preset[1]
	var ambience_db: float = preset[2]
	var percussion_db: float = preset[3]
	var pitch: float = preset[4]
	if state == "victory":
		_play_stinger(VICTORY_STINGER)
	elif state == "defeat":
		_play_stinger(DEFEAT_STINGER)
	if silent:
		return
	_start_if_needed(_base)
	# AU1 : quand l'audio spatialisé de bataille est actif, ses nappes de mêlée remplacent cette
	# couche 2D (sinon le fer serait entendu deux fois).
	if BattleAudio.active != null:
		ambience_db = -80.0
	if ambience_db > -79.0:
		_start_if_needed(_ambience)
	if percussion_db > -79.0:
		_start_if_needed(_percussion)
	var time := 0.05 if instant else FADE_SECONDS
	if _tween != null and _tween.is_valid():
		_tween.kill()
	_tween = create_tween()
	_tween.set_parallel(true)
	_tween.tween_property(_base, "volume_db", base_db, time)
	_tween.tween_property(_base, "pitch_scale", pitch, time)
	if _filter != null:
		_tween.tween_property(_filter, "cutoff_hz", cutoff, time)
	_tween.tween_property(_ambience, "volume_db", ambience_db, time)
	_tween.tween_property(_percussion, "volume_db", percussion_db, time)


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


static func _load(path: String, loop: bool) -> AudioStream:
	if not ResourceLoader.exists(path):
		return null
	var stream := load(path) as AudioStream
	if stream is AudioStreamOggVorbis:
		(stream as AudioStreamOggVorbis).loop = loop
	return stream


func _exit_tree() -> void:
	for player in [_base, _ambience, _percussion, _stinger]:
		if player != null:
			player.stop()
			player.stream = null
