class_name VoiceLines
extends RefCounted

## Accès aux données de voix : répliques d'unités (`data/voice/barks.json`), conseiller
## (`data/voice/advisor.json`) et voix des discours (`data/voice/speech_voices.json`).
## Les fichiers audio, synthétisés par `tools/cent_ans_tools/voice_tts.py`, sont sous
## `res://assets/audio/voice/` : `barks/<id>.ogg`, `advisor/<id>.ogg`,
## `speech/<voix>/<sha1 du texte, 12 caractères>.ogg`. Fichier absent = silence (le sous-titre
## reste affiché), sans erreur. Rendu seulement.

const BARKS_FILE := "voice/barks.json"
const ADVISOR_FILE := "voice/advisor.json"
const SPEECH_FILE := "voice/speech_voices.json"
const VOICE_ROOT := "res://assets/audio/voice/"
const SPEECH_HASH_LEN := 12

static var _cache: Dictionary = {}  # fichier de données → Dictionary
static var _streams: Dictionary = {}  # chemin → AudioStream ou null


static func data(relative: String) -> Dictionary:
	if _cache.has(relative):
		return _cache[relative]
	var path := SoundBank.data_path(relative)
	var parsed: Variant = null
	if FileAccess.file_exists(path):
		parsed = DataFile.parse_file(path)
	var result: Dictionary = parsed if parsed is Dictionary else {}
	if result.is_empty():
		push_warning("VoiceLines: %s missing or invalid" % path)
	_cache[relative] = result
	return result


static func barks() -> Dictionary:
	return data(BARKS_FILE)


static func advisor() -> Dictionary:
	return data(ADVISOR_FILE)


static func speech_casting() -> Dictionary:
	return data(SPEECH_FILE)


## Flux `res://assets/audio/voice/<relative>.ogg`, ou null s'il n'a pas (encore) été généré.
static func stream(relative: String) -> AudioStream:
	if _streams.has(relative):
		return _streams[relative]
	var path := VOICE_ROOT + relative + ".ogg"
	var result: AudioStream = null
	if ResourceLoader.exists(path):
		result = load(path) as AudioStream
	_streams[relative] = result
	return result


# --- Répliques ---------------------------------------------------------------------


## Langue d'un régiment : unité régionale (gascons, flamands, gallois, écossais), sinon
## noblesse (chevaliers anglais : anglo-normand), sinon langue de la faction.
static func language_for(unit_type: String, faction: String, is_general: bool = false) -> String:
	var table := barks()
	var by_unit: Dictionary = table.get("unit_language", {})
	if by_unit.has(unit_type):
		return str(by_unit[unit_type])
	var nobles: Array = table.get("noble_units", [])
	var noble_language: Dictionary = table.get("noble_language", {})
	if (is_general or nobles.has(unit_type)) and noble_language.has(faction):
		return str(noble_language[faction])
	var by_faction: Dictionary = table.get("faction_language", {})
	return str(by_faction.get(faction, by_faction.get("default", "fr")))


## Répliques possibles pour `language` / `situation` / catégorie d'unité, avec repli sur la
## langue principale quand une langue régionale n'a rien pour cette situation.
static func bark_candidates(language: String, situation: String, category: String) -> Array:
	var table := barks()
	var lines: Dictionary = table.get("lines", {})
	var fallback: Dictionary = table.get("fallback_language", {})
	var current := language
	for _hop in 3:
		var result: Array = []
		for line in (lines.get(current, {}) as Dictionary).get(situation, []):
			var only := str((line as Dictionary).get("category", ""))
			if only == "" or only == category:
				result.append(line)
		if not result.is_empty():
			return result
		if not fallback.has(current):
			break
		current = str(fallback[current])
	return []


static func situation(situation_name: String) -> Dictionary:
	return (barks().get("situations", {}) as Dictionary).get(situation_name, {})


# --- Discours ----------------------------------------------------------------------


## Voix du général : une des voix de sa faction (distribution `default` sinon), choisie par
## son identifiant (le même général garde la même voix).
static func speech_voice(faction: String, general_id: String) -> String:
	var casting: Dictionary = speech_casting().get("casting", {})
	var spec: Dictionary = casting.get(faction, casting.get("default", {}))
	var voices: Array = spec.get("voices", [])
	if voices.is_empty():
		return ""
	return str(voices[absi(hash(general_id)) % voices.size()])


## VX : cri de guerre de `faction` (`order_war_cry.json`, sinon le cri par défaut des discours).
static func war_cry(faction: String) -> String:
	var cries: Dictionary = data("battle_orders/order_war_cry.json").get("labels_by_faction", {})
	return str(cries.get(faction, data("speeches/battle_speeches.json").get("default_cry", "")))


static func speech_path(voice: String, text: String) -> String:
	return "speech/%s/%s" % [voice, text.sha1_text().substr(0, SPEECH_HASH_LEN)]


static func speech_stream(voice: String, text: String) -> AudioStream:
	if voice == "" or text == "":
		return null
	return stream(speech_path(voice, text))


# --- Conseiller --------------------------------------------------------------------


## Réplique du conseiller pour `trigger` (et la faction / l'événement), ou {}.
static func advisor_line(trigger: String, faction: String = "", event_kind: String = "") -> Dictionary:
	var generic := {}
	for line in advisor().get("lines", []):
		var entry: Dictionary = line
		if str(entry.get("trigger", "")) != trigger:
			continue
		if trigger == "alert" and str(entry.get("event", "")) != event_kind:
			continue
		var only := str(entry.get("faction", ""))
		if only == faction and only != "":
			return entry
		if only == "" and generic.is_empty():
			generic = entry
	return generic
