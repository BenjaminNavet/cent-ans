extends TestCase

## Test headless du lot RX audio (ADR 0247) :
##  1. `music.json` : un gain par morceau présent, cible de sonie, fondu enchaîné, fallback en
##     rotation (un morceau fallback après `fallback_every` primary) ;
##  2. aucune pièce commune à menu / guerre / bataille ; aucun morceau de fond de moins de 60 s ;
##  3. `should_crossfade` (début du fondu de fin de morceau) ;
##  4. `faction_language` : les 177 factions des cultures couvertes ont une langue de réplique ;
##  5. variantes de la banque (`file_gain_db`) et curseur « Musique de bataille ».
## Usage : godot --headless --path game --script res://tests/rx_audio_test.gd

const DirectorScript := preload("res://scripts/audio/audio_director.gd")
const MIN_BACKGROUND_S := 60.0
## Cultures sans langue de réplique proche (à enregistrer, voir ADR 0247).
const UNVOICED_CULTURES := [
	"cul_alan", "cul_arabic", "cul_turkish", "cul_maghrebi", "cul_bashkir", "cul_russian", "cul_czech",
	"cul_bosnian", "cul_bulgarian", "cul_greek", "cul_armenian", "cul_circassian", "cul_albanian",
	"cul_ruthenian", "cul_georgian", "cul_kipchak", "cul_andalusi", "cul_hungarian", "cul_lithuanian",
	"cul_mordvin", "cul_novgorodian", "cul_polish", "cul_permian", "cul_serbian", "cul_croatian",
]


func _init() -> void:
	await process_frame
	_check_music_data()
	_check_exclusive_pieces()
	_check_rotation()
	_check_crossfade()
	_check_barks()
	_check_bank_variants()
	_check_battle_music_bus()
	finish()


func _music() -> Dictionary:
	return DataFile.parse_file(SoundBank.data_path("audio/music.json"))


func _all_tracks(music: Dictionary) -> Array:
	var result: Array = []
	for playlist in (music["playlists"] as Dictionary).values():
		for tier in ["primary", "fallback"]:
			for track in (playlist as Dictionary).get(tier, []):
				if not result.has(track):
					result.append(track)
	return result


func _check_music_data() -> void:
	var music := _music()
	var gains: Dictionary = music.get("track_gain_db", {})
	check(music.has("loudness_target_lufs"), "loudness_target_lufs missing")
	check(float(music.get("crossfade_seconds", 0.0)) > 0.0, "crossfade_seconds should be set")
	check(int(music.get("fallback_every", 0)) > 0, "fallback_every should be set")
	var missing: Array = []
	var excessive: Array = []
	for track in _all_tracks(music):
		if not FileAccess.file_exists(ProjectSettings.globalize_path("res://" + str(track))) and not ResourceLoader.exists("res://" + str(track)):
			continue  # absent du dépôt : silence sans erreur
		if not gains.has(track):
			missing.append(track)
		elif absf(float(gains[track])) > 12.0:
			excessive.append(track)
	check(missing.is_empty(), "tracks without gain_db: %s" % [missing])
	check(excessive.is_empty(), "gain_db beyond 12 dB: %s" % [excessive])
	var director: Node = DirectorScript.new()
	director.load_playlists(SoundBank.data_path("audio/music.json"))
	var any_track: String = "res://" + str(gains.keys()[0])
	check(is_equal_approx(director.track_gain_db(any_track), float(gains[gains.keys()[0]])), "track_gain_db should read music.json")
	check(director.track_gain_db("res://nowhere.ogg") == 0.0, "unknown track has 0 dB")
	check(director.crossfade_seconds() == float(music["crossfade_seconds"]), "crossfade_seconds read")
	director.free()


func _check_exclusive_pieces() -> void:
	var playlists: Dictionary = _music()["playlists"]
	var owner_of := {}
	for context in ["menu", "war", "battle"]:
		for tier in ["primary", "fallback"]:
			for track in (playlists[context] as Dictionary).get(tier, []):
				check(not owner_of.has(track) or owner_of[track] == context, "%s serves both %s and %s" % [track, owner_of.get(track, ""), context])
				owner_of[track] = context
	# Un morceau court ne boucle plus et n'est pas un fond : tous durent au moins une minute.
	var short: Array = []
	for context in ["war", "battle", "campaign"]:
		for tier in ["primary", "fallback"]:
			for track in (playlists[context] as Dictionary).get(tier, []):
				var path := "res://" + str(track)
				if ResourceLoader.exists(path) and not str(track).begins_with("assets/audio/music/"):
					var stream := load(path) as AudioStream
					if stream != null and stream.get_length() < MIN_BACKGROUND_S:
						short.append("%s (%.0f s)" % [track, stream.get_length()])
	check(short.is_empty(), "background tracks under %d s: %s" % [int(MIN_BACKGROUND_S), short])
	check((playlists["battle"] as Dictionary)["primary"].size() >= 4, "battle needs at least 4 tracks to avoid repeats")


func _check_rotation() -> void:
	var director: Node = DirectorScript.new()
	director.rotation_path = ""
	director.load_playlists(SoundBank.data_path("audio/music.json"))
	director.set("_fallback_every", 3)
	var primary: Array = director.tier_tracks("campaign", "primary")
	var fallback: Array = director.tier_tracks("campaign", "fallback")
	var heard: Array = []
	for _i in 12:
		heard.append(director.next_track("campaign"))
	var from_fallback := 0
	for index in heard.size():
		if fallback.has(heard[index]):
			from_fallback += 1
			check(index >= 3, "fallback should come after 3 primary tracks (draw %d)" % index)
		if index > 0:
			check(heard[index] != heard[index - 1], "no immediate repeat")
	check(from_fallback >= 2, "fallback tracks should join the rotation (%d in 12)" % from_fallback)
	check(primary.size() >= 8, "campaign primary list intact")
	director.set("_fallback_every", 0)
	for _i in 12:
		check(not fallback.has(director.next_track("campaign")), "fallback_every = 0 keeps fallback as a last resort")
	director.free()


func _check_crossfade() -> void:
	check(DirectorScript.should_crossfade(97.5, 100.0, 3.0), "crossfade starts in the last 3 s")
	check(not DirectorScript.should_crossfade(50.0, 100.0, 3.0), "no crossfade mid-track")
	check(not DirectorScript.should_crossfade(8.0, 9.0, 3.0), "tracks shorter than 4 fades are not crossfaded")
	check(not DirectorScript.should_crossfade(99.0, 100.0, 0.0), "fade of 0 disables it")


func _check_barks() -> void:
	var table := VoiceLines.barks()
	var by_faction: Dictionary = table["faction_language"]
	var languages: Dictionary = table["languages"]
	var uncovered: Array = []
	for path in DirAccess.get_files_at(ProjectSettings.globalize_path(SoundBank.data_path("factions"))):
		if not path.ends_with(".json"):
			continue
		var faction: Dictionary = DataFile.parse_file(SoundBank.data_path("factions/" + path))
		var faction_id := str(faction.get("id", ""))
		if by_faction.has(faction_id):
			check(languages.has(by_faction[faction_id]), "%s: unknown language %s" % [faction_id, by_faction[faction_id]])
		elif not UNVOICED_CULTURES.has(str(faction.get("culture", ""))):
			uncovered.append(faction_id)
	check(uncovered.is_empty(), "factions without bark language: %s" % [uncovered])
	check(by_faction.size() >= 100, "faction_language should cover the western cultures (%d)" % by_faction.size())
	check(VoiceLines.language_for("unit_knights", "fac_castile") == "oc", "Castile speaks the nearest Romance language")
	check(VoiceLines.language_for("unit_knights", "fac_france") == "fr", "France keeps French")


func _check_bank_variants() -> void:
	var bank := SoundBank.load_default()
	var graded := 0
	for event_name in bank.events:
		var gains: Dictionary = bank.event(event_name).get("file_gain_db", {})
		for file in gains:
			graded += 1
			check((bank.event(event_name)["files"] as Array).has(file), "%s: gain for unknown variant %s" % [event_name, file])
	check(graded >= 5, "variant gains should exist (%d)" % graded)
	bank.pick_stream("sword_clash")
	var gain := bank.last_gain_db("sword_clash")
	check(gain >= -12.0 and gain <= 12.0, "last_gain_db bounded")
	check(bank.last_gain_db("unknown_event") == 0.0, "unknown event has no gain")


func _check_battle_music_bus() -> void:
	AudioBuses.ensure_layout()
	var names: Array = []
	for spec in AudioBuses.PLAYER_BUSES:
		names.append(spec[0])
	check(names.has(AudioBuses.BATTLE_MUSIC), "battle music slider missing")
	var bus := AudioServer.get_bus_index(AudioBuses.BATTLE_MUSIC)
	check(bus != -1 and AudioServer.get_bus_send(bus) == AudioBuses.MUSIC, "BatailleMusique should send to Musique")
	check(Settings.DEFAULTS.has("audio/bus_BatailleMusique"), "default volume for the battle music bus")
