class_name SoundBank
extends RefCounted

## AU1 — banque sonore décrite par `data/audio/sound_bank.json` (schéma
## `data/schemas/sound_bank.schema.json`) : événements ponctuels (variantes, priorité, limites de
## voix, hauteur aléatoire, portée), nappes 3D en boucle (`beds`) et ambiances 2D (`ambience`).
## Fichiers sous `res://assets/audio/<chemin>.ogg` ; absent = silence, sans erreur.

const BANK_PATH := "audio/sound_bank.json"
const AUDIO_ROOT := "res://assets/audio/"

## Valeurs par défaut d'un événement (complétées par le fichier de données).
const EVENT_DEFAULTS := {
	"files": [],
	"layers": [],
	"bus": "Bataille",
	"priority": 3,
	"volume_db": 0.0,
	"pitch": [0.94, 1.06],
	"max_instances": 4,
	"cooldown_s": 0.0,
	"unit_size_m": 12.0,
	"max_distance_m": 320.0,
}

var voices: Dictionary = {"max_voices": 28, "near_distance_m": 70.0}
## EP4 : paramètres des émetteurs par front de mêlée (`data/audio/sound_bank.json`, clé `fronts`).
var fronts: Dictionary = {"max_emitters": 6, "near_m": 40.0, "mid_m": 200.0, "engaged_full": 250.0, "event_period_s": [0.5, 1.6], "beds": ["melee_bed_1", "melee_bed_2"]}
var events: Dictionary = {}
var beds: Dictionary = {}
var ambience: Dictionary = {}

var _streams: Dictionary = {}  # chemin → AudioStream ou null
var _rng := RandomNumberGenerator.new()


static func load_default() -> SoundBank:
	var bank := SoundBank.new()
	bank.load_file(_data_dir().path_join(BANK_PATH))
	return bank


static func _data_dir() -> String:
	var loop := Engine.get_main_loop() as SceneTree
	var paths: Node = loop.root.get_node_or_null("/root/MapPaths") if loop != null else null
	if paths != null:
		return str(paths.get("data_dir"))
	return ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()


func load_file(path: String) -> bool:
	_rng.seed = 1337
	if not FileAccess.file_exists(path):
		push_warning("SoundBank: %s missing, audio bank empty" % path)
		return false
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	if not parsed is Dictionary:
		push_warning("SoundBank: %s is not a JSON object" % path)
		return false
	var data: Dictionary = parsed
	voices.merge(data.get("voices", {}), true)
	fronts.merge(data.get("fronts", {}), true)
	for event_name in data.get("events", {}):
		var entry: Dictionary = EVENT_DEFAULTS.duplicate(true)
		entry.merge(data["events"][event_name], true)
		events[event_name] = entry
	beds = data.get("beds", {})
	ambience = data.get("ambience", {})
	return true


func has_event(event_name: String) -> bool:
	return events.has(event_name)


func event(event_name: String) -> Dictionary:
	return events.get(event_name, {})


## Une variante tirée au hasard (jamais deux fois de suite la même si possible).
func pick_stream(event_name: String) -> AudioStream:
	var entry: Dictionary = events.get(event_name, {})
	var files: Array = entry.get("files", [])
	if files.is_empty():
		return null
	var index := _rng.randi_range(0, files.size() - 1)
	var last := int(entry.get("_last", -1))
	if files.size() > 1 and index == last:
		index = (index + 1) % files.size()
	entry["_last"] = index
	return stream(str(files[index]), false)


func random_pitch(event_name: String) -> float:
	var span: Array = events.get(event_name, EVENT_DEFAULTS).get("pitch", [1.0, 1.0])
	return _rng.randf_range(float(span[0]), float(span[1]))


func bed_stream(bed_name: String) -> AudioStream:
	var entry: Dictionary = beds.get(bed_name, {})
	return stream(str(entry.get("file", "")), true) if not entry.is_empty() else null


func ambience_stream(layer: String) -> AudioStream:
	var entry: Dictionary = ambience.get(layer, {})
	return stream(str(entry.get("file", "")), true) if not entry.is_empty() else null


## Flux `res://assets/audio/<relative>.ogg` (ou `.wav`), mis en cache ; `loop` force la boucle.
func stream(relative: String, loop: bool) -> AudioStream:
	if relative == "":
		return null
	var key := relative + ("#loop" if loop else "")
	if _streams.has(key):
		return _streams[key]
	var result: AudioStream = null
	for extension in [".ogg", ".wav"]:
		var path: String = AUDIO_ROOT + relative + extension
		if ResourceLoader.exists(path):
			result = load(path) as AudioStream
			break
	if result != null and loop:
		result = result.duplicate() as AudioStream
		if result is AudioStreamOggVorbis:
			(result as AudioStreamOggVorbis).loop = true
		elif result is AudioStreamWAV:
			var wav := result as AudioStreamWAV
			wav.loop_mode = AudioStreamWAV.LOOP_FORWARD
			wav.loop_end = int(wav.get_length() * wav.mix_rate)
	_streams[key] = result
	return result


## Tous les fichiers référencés (tests : vérifier qu'ils existent).
func all_files() -> Array:
	var result: Array = []
	for entry in events.values():
		result.append_array(entry.get("files", []))
	for entry in beds.values():
		result.append(entry.get("file", ""))
	for entry in ambience.values():
		result.append(entry.get("file", ""))
	return result


func clear_cache() -> void:
	_streams.clear()
