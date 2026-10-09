extends Node

## Autoload `AudioDirector` : musique d'ambiance et effets de la campagne.
## - Bus « Musique » et « Interface » ; volumes réglés par `Settings` (`audio/bus_*`).
## - Musique par contexte (`campaign[_<région>]`, `war`, `court`, `menu` ; `battle` pour la
##   bataille), fondu enchaîné entre deux lecteurs. Listes `primary` puis `fallback` dans
##   `data/audio/music.json`, rotation mélangée persistée (`user://music_rotation.cfg`) ;
##   `culture_regions` associe la culture de la faction jouée à une région musicale.
## - Effets : clic de bouton, page tournée, cloche de fin de tour puis effet de l'événement le
##   plus marquant du tour (`event_sfx` de `data/audio/sound_bank.json`).
## - Fichiers `res://assets/audio/{sfx,music}/<nom>.ogg|.wav` ; absent = silence, sans erreur.
##
## Accès depuis les autres scripts : `get_node_or_null("/root/AudioDirector")` (les scripts
## chargés par le smoke test sont compilés avant l'enregistrement des autoloads).

const ROTATION_PATH := "user://music_rotation.cfg"
const TIERS := ["primary", "fallback"]
const MUSIC_BUS := "Musique"
## Les effets d'interface (clic, page, cloche de tour) passent par le bus « Interface ».
const SFX_BUS := "Interface"
const DUCK_ATTACK := 0.25
const DUCK_RELEASE := 1.5
const SFX_DIR := "res://assets/audio/sfx/"
const MUSIC_DIR := "res://assets/audio/music/"
const PLAYLISTS_PATH := "audio/music.json"
const SFX_VOICES := 6
const FADE_SECONDS := 1.5
## Ambiances de la carte de campagne (créées par `attach_campaign`).
var campaign_ambience: CampaignAmbience = null
var current_context: String = ""
## Headless (smoke test, serveur) : flux chargés et contextes suivis, mais rien n'est joué
## (le pilote audio factice ne libère pas les lectures OGG à la sortie).
var silent: bool = false

var _streams: Dictionary = {}  # chemin → AudioStream ou null
var _music_players: Array[AudioStreamPlayer] = []
var _music_tween: Tween = null  # fondu enchaîné en cours (tué au changement suivant)
var _active_music := 0
var _sfx_players: Array[AudioStreamPlayer] = []
var _next_voice := 0
var _campaign: Node = null
var _base_context := "campaign"
var _court_open := false
var _duck_tween: Tween = null
var _duck_until: float = 0.0
## Listes de lecture par contexte : `{context: {"primary": [chemins res://], "fallback": [...]}}`.
var _playlists: Dictionary = {}
## Culture (`data/factions/<id>.json`, champ `culture`) → région musicale
## (`campaign_<région>`), lue dans `data/audio/music.json` (clé `culture_regions`).
var _culture_regions: Dictionary = {}
var _faction_cultures: Dictionary = {}  # cache faction_id -> culture id
## Rotation mélangée : `{"<contexte>/<liste>": [morceaux restant à jouer]}`. Une liste
## n'est rebattue qu'une fois épuisée ; l'état est écrit dans `rotation_path` à chaque tirage pour
## que deux sessions successives n'ouvrent pas sur le même morceau ("" = pas de persistance).
var rotation_path := ROTATION_PATH
var _rotation: Dictionary = {}
var _current_track := ""  # dernier morceau tiré, jamais rejoué aussitôt (même dans un autre contexte)
## Liste de campagne régionale mêlée à `war` tant que le joueur est en guerre ("" = aucune).
var _war_blend := ""
var _music_rng := RandomNumberGenerator.new()


func _ready() -> void:
	process_mode = Node.PROCESS_MODE_ALWAYS
	silent = DisplayServer.get_name() == "headless"
	AudioBuses.ensure_layout()
	for index in 2:
		var player := AudioStreamPlayer.new()
		player.name = "Music%d" % index
		player.bus = MUSIC_BUS
		player.finished.connect(_on_music_finished.bind(player))
		add_child(player)
		_music_players.append(player)
	for index in SFX_VOICES:
		var voice := AudioStreamPlayer.new()
		voice.name = "Sfx%d" % index
		voice.bus = SFX_BUS
		add_child(voice)
		_sfx_players.append(voice)
	load_playlists(SoundBank.data_path(PLAYLISTS_PATH))
	if silent:
		rotation_path = ""  # tests sans affichage : ne pas toucher à la rotation du joueur
	load_rotation()
	get_tree().node_added.connect(_on_node_added)


## Arrête tout et libère les flux (appelé à la sortie ; évite des fuites signalées par Godot).
func stop_all() -> void:
	if campaign_ambience != null:
		campaign_ambience.silence()
	for player in _music_players + _sfx_players:
		player.stop()
		player.stream = null
	_streams.clear()
	current_context = ""


func _exit_tree() -> void:
	stop_all()


# --- Réglages --------------------------------------------------------------------


# --- Ducking -----------------------------------------------------------------


## Atténue la musique de `db` (≤ 0) pendant `hold` secondes (moments forts : cri de guerre, mort
## d'un général, effondrement de muraille), puis la rétablit en `DUCK_RELEASE` s. Un ducking plus
## fort ou plus long en cours n'est pas raccourci.
func duck_music(db: float, hold: float = 3.0) -> void:
	var now := Time.get_ticks_msec() / 1000.0
	var depth := minf(db, 0.0)
	var ducking := _duck_tween != null and _duck_tween.is_valid()
	if ducking:
		if depth >= AudioBuses.music_duck_db() and now + hold <= _duck_until:
			return
		depth = minf(depth, AudioBuses.music_duck_db())
		_duck_tween.kill()
	else:
		_duck_until = 0.0
	_duck_until = maxf(_duck_until, now + hold)
	_duck_tween = create_tween()
	_duck_tween.tween_method(AudioBuses.set_music_duck_db, AudioBuses.music_duck_db(), depth, DUCK_ATTACK)
	_duck_tween.tween_interval(maxf(_duck_until - now - DUCK_ATTACK, 0.0))
	_duck_tween.tween_method(AudioBuses.set_music_duck_db, depth, 0.0, DUCK_RELEASE)


func music_duck_db() -> float:
	return AudioBuses.music_duck_db()


# --- Flux ------------------------------------------------------------------------


func _load_stream(directory: String, clip: String, loop: bool) -> AudioStream:
	for extension in [".ogg", ".wav"]:
		var path: String = directory + clip + extension
		if _streams.has(path):
			if _streams[path] != null:
				return _streams[path]
			continue
		var stream: AudioStream = null
		if ResourceLoader.exists(path):
			stream = load(path) as AudioStream
		elif FileAccess.file_exists(ProjectSettings.globalize_path(path)):
			var absolute := ProjectSettings.globalize_path(path)
			stream = AudioStreamOggVorbis.load_from_file(absolute) if extension == ".ogg" else AudioStreamWAV.load_from_file(absolute)
		if stream != null:
			if stream is AudioStreamOggVorbis:
				(stream as AudioStreamOggVorbis).loop = loop
			elif stream is AudioStreamWAV and loop:
				var wav := stream as AudioStreamWAV
				wav.loop_mode = AudioStreamWAV.LOOP_FORWARD
				wav.loop_end = int(wav.get_length() * wav.mix_rate)
		_streams[path] = stream
		if stream != null:
			return stream
	return null


func has_sfx(clip: String) -> bool:
	return _load_stream(SFX_DIR, clip, false) != null


func has_music(context: String) -> bool:
	return _pick_track(context, false) != null


## Charge les listes de lecture (`{"playlists": {contexte: {"primary": [...], "fallback": [...]}},
## "culture_regions": {culture: région}}`, chemins relatifs à res://).
func load_playlists(path: String) -> bool:
	_playlists.clear()
	_culture_regions.clear()
	if not FileAccess.file_exists(path):
		return false
	var parsed: Variant = DataFile.parse_file(path)
	if not parsed is Dictionary:
		push_warning("AudioDirector: %s is not a JSON object" % path)
		return false
	var playlists: Dictionary = (parsed as Dictionary).get("playlists", {})
	for context in playlists:
		var entry: Variant = playlists[context]
		var primary: Array[String] = []
		var fallback: Array[String] = []
		if entry is Dictionary:
			for track in (entry as Dictionary).get("primary", []):
				primary.append("res://" + str(track))
			for track in (entry as Dictionary).get("fallback", []):
				fallback.append("res://" + str(track))
		elif entry is Array:
			# Format historique (liste simple), toute la liste en primary.
			for track in entry:
				primary.append("res://" + str(track))
		_playlists[str(context)] = {"primary": primary, "fallback": fallback}
	for culture in (parsed as Dictionary).get("culture_regions", {}):
		_culture_regions[str(culture)] = str((parsed as Dictionary)["culture_regions"][culture])
	return true


## Morceaux du contexte (primary puis fallback ; vide si aucune liste de lecture).
func playlist(context: String) -> Array:
	var entry: Dictionary = _playlists.get(context, {})
	var combined: Array = (entry.get("primary", []) as Array).duplicate()
	combined.append_array(entry.get("fallback", []) as Array)
	return combined


## Morceaux d'une liste (`primary` ou `fallback`) du contexte. En guerre, `war` est complétée par
## la même liste de la région de la faction jouée (`_war_blend`), sans doublon.
func tier_tracks(context: String, tier: String) -> Array:
	var tracks: Array = ((_playlists.get(context, {}) as Dictionary).get(tier, []) as Array).duplicate()
	if context == "war" and _war_blend != "":
		for path in (_playlists.get(_war_blend, {}) as Dictionary).get(tier, []):
			if not tracks.has(path):
				tracks.append(path)
	return tracks


## Tire le prochain morceau du contexte dans sa rotation mélangée et l'en retire : `primary`
## d'abord, `fallback` si `primary` ne fournit aucun fichier présent. "" si rien n'est jouable.
func next_track(context: String) -> String:
	for tier in TIERS:
		var tracks := tier_tracks(context, tier)
		var key := "%s/%s" % [context, tier]
		var bag: Array = []
		for path in _rotation.get(key, []):
			if tracks.has(path) and not bag.has(path):
				bag.append(path)
		# Sac épuisé (ou réduit au morceau en cours) : on rebat toute la liste.
		if bag.is_empty() or (tracks.size() > 1 and bag == [_current_track]):
			bag = tracks.duplicate()
			_shuffle(bag)
		for path in bag.duplicate():
			if path == _current_track and bag.size() > 1:
				continue
			bag.erase(path)
			if _load_music(path) != null:
				_rotation[key] = bag
				_current_track = path
				save_rotation()
				return path
		_rotation[key] = bag
	return ""


## Mélange de Fisher-Yates sur le générateur de la musique (graine aléatoire à chaque session).
func _shuffle(tracks: Array) -> void:
	for index in range(tracks.size() - 1, 0, -1):
		var other := _music_rng.randi_range(0, index)
		var swapped: Variant = tracks[index]
		tracks[index] = tracks[other]
		tracks[other] = swapped


func load_rotation() -> void:
	_rotation.clear()
	var config := ConfigFile.new()
	if rotation_path == "" or config.load(rotation_path) != OK:
		return
	for key in config.get_section_keys("rotation") if config.has_section("rotation") else PackedStringArray():
		var bag: Variant = config.get_value("rotation", key, [])
		if bag is Array:
			_rotation[key] = bag
	_current_track = str(config.get_value("state", "last_track", ""))


func save_rotation() -> void:
	if rotation_path == "":
		return
	var config := ConfigFile.new()
	for key in _rotation:
		config.set_value("rotation", key, _rotation[key])
	config.set_value("state", "last_track", _current_track)
	config.save(rotation_path)


## Flux du prochain morceau du contexte (`next_track`), à défaut `music/<contexte>.ogg` en boucle.
## `advance = false` : simple test de présence, la rotation n'avance pas.
func _pick_track(context: String, advance: bool = true) -> AudioStream:
	if advance:
		var path := next_track(context)
		if path != "":
			return _load_music(path)
	else:
		for tier in TIERS:
			for path in tier_tracks(context, tier):
				if _load_music(path) != null:
					return _load_music(path)
	return _load_stream(MUSIC_DIR, context, true)


## Région musicale (france/england/burgundy/iberia/italy) de la culture d'une faction, "" si
## inconnue ou absente de `culture_regions` (`data/audio/music.json`).
func culture_region(faction_id: String) -> String:
	if faction_id == "":
		return ""
	if not _faction_cultures.has(faction_id):
		var data := SoundBank.data_path("factions/" + faction_id + ".json")
		var parsed: Variant = DataFile.parse_file(data) if FileAccess.file_exists(data) else null
		_faction_cultures[faction_id] = str((parsed as Dictionary).get("culture", "")) if parsed is Dictionary else ""
	return str(_culture_regions.get(_faction_cultures[faction_id], ""))


## Contexte de campagne (paix) pour une faction : `campaign_<région>` si sa culture a une région
## musicale connue et que la liste de lecture existe, sinon `campaign`.
func campaign_context(faction_id: String) -> String:
	var region := culture_region(faction_id)
	var context := "campaign_%s" % region if region != "" else "campaign"
	return context if _playlists.has(context) else "campaign"


## Morceau d'une liste de lecture, sans boucle (la fin enchaîne sur le suivant).
func _load_music(path: String) -> AudioStream:
	if _streams.has(path):
		return _streams[path]
	var stream: AudioStream = load(path) as AudioStream if ResourceLoader.exists(path) else null
	if stream != null:
		# Copie : la bataille peut boucler la même ressource de son côté.
		stream = stream.duplicate()
		stream.set("loop", false)
	_streams[path] = stream
	return stream


func _on_music_finished(player: AudioStreamPlayer) -> void:
	if player != _music_players[_active_music] or current_context == "":
		return
	var stream := _pick_track(current_context)
	if stream != null and not silent:
		player.stream = stream
		player.volume_db = 0.0
		player.play()


func play_sfx(clip: String) -> bool:
	var stream := _load_stream(SFX_DIR, clip, false)
	if stream == null or _sfx_players.is_empty():
		return false
	if silent:
		return true
	var voice := _sfx_players[_next_voice]
	_next_voice = (_next_voice + 1) % _sfx_players.size()
	voice.stream = stream
	voice.play()
	return true


## Lance la musique du contexte (`campaign`, `war`, `court`) avec un fondu enchaîné.
func play_music(context: String) -> void:
	if context == current_context:
		return
	var stream := _pick_track(context)
	current_context = context
	if stream == null or _music_players.size() < 2 or silent:
		return
	var old_player := _music_players[_active_music]
	_active_music = 1 - _active_music
	var new_player := _music_players[_active_music]
	# Fondu précédent interrompu : sinon son arrêt différé couperait le lecteur réutilisé ici.
	if _music_tween != null and _music_tween.is_valid():
		_music_tween.kill()
	new_player.stream = stream
	new_player.volume_db = -40.0
	new_player.play()
	var tween := create_tween()
	_music_tween = tween
	tween.set_parallel(true)
	tween.tween_property(new_player, "volume_db", 0.0, FADE_SECONDS)
	if old_player.playing:
		tween.tween_property(old_player, "volume_db", -40.0, FADE_SECONDS)
		tween.chain().tween_callback(old_player.stop)


# --- Branchements ----------------------------------------------------------------


func _on_node_added(node: Node) -> void:
	if node is BaseButton and not node.has_meta("m10_silent"):
		var button := node as BaseButton
		if not button.pressed.is_connected(_on_button_pressed):
			button.pressed.connect(_on_button_pressed)


func _on_button_pressed() -> void:
	play_sfx("ui_click")


## Écran de démarrage : musique de menu.
func enter_menu() -> void:
	_campaign = null
	if campaign_ambience != null:
		campaign_ambience.setup(null)
	_court_open = false
	_base_context = "campaign"
	play_music("menu")


## Carte de campagne : page tournée à l'ouverture des panneaux, contexte guerre/cour.
func attach_campaign(campaign: Node) -> void:
	_campaign = campaign
	_court_open = false
	var ui: Node = campaign.get("ui")
	if ui != null:
		for child in ui.get_children():
			if child is Control and str(child.name).ends_with("Panel"):
				var panel := child as Control
				if not panel.has_meta("m10_audio"):
					panel.set_meta("m10_audio", true)
					panel.visibility_changed.connect(_on_panel_visibility.bind(panel))
	# Ambiances de carte (enfant du directeur : il survit à la mise en veille de la carte).
	if campaign_ambience == null:
		campaign_ambience = CampaignAmbience.new()
		campaign_ambience.name = "CampaignAmbience"
		add_child(campaign_ambience)
	campaign_ambience.setup(campaign)
	refresh_context()


func _on_panel_visibility(panel: Control) -> void:
	if panel.visible:
		play_sfx("page_turn")
	if str(panel.name) == "CourtPanel":
		_court_open = panel.visible
		_update_music()


func player_at_war() -> bool:
	if _campaign == null:
		return false
	var sim: Object = _campaign.get("sim")
	var faction := str(_campaign.get("player_faction"))
	if sim == null or not sim.has_method("get_diplomacy"):
		return false
	for entry in sim.call("get_diplomacy", faction):
		if entry is Dictionary and str(entry.get("status", "")) == "war":
			return true
	return false


func refresh_context() -> void:
	var faction := str(_campaign.get("player_faction")) if _campaign != null else ""
	var peace_context := campaign_context(faction)
	# La guerre dure presque toute la partie, sa liste s'enrichit des airs de la région.
	_war_blend = peace_context
	_base_context = "war" if player_at_war() else peace_context
	_update_music()


func _update_music() -> void:
	play_music("court" if _court_open else _base_context)


## Fin de tour : cloche, puis l'effet de l'événement le plus marquant, puis contexte musical.
func on_turn_events(events: Array) -> void:
	play_sfx("turn_bell")
	var clip := event_sfx(events)
	if clip != "" and is_inside_tree():
		get_tree().create_timer(0.7).timeout.connect(play_sfx.bind(clip))
	refresh_context()


static func event_sfx(events: Array) -> String:
	var kinds := {}
	for event in events:
		if event is Dictionary:
			kinds[str(event.get("kind", ""))] = true
	for pair in DataFile.load_cached(SoundBank.BANK_PATH)["event_sfx"]["order"]:
		if kinds.has(pair["kind"]):
			return str(pair["clip"])
	return ""
