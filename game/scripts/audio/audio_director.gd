extends Node

## M10 assets — autoload `AudioDirector` : musique d'ambiance et effets sonores.
##
## - Bus « Musique » et « Effets » (créés au démarrage s'ils manquent), volumes linéaires
##   0..1 persistés dans `user://settings.cfg` (section `audio`).
## - Musique selon le contexte (`campaign` — ou `campaign_<région>` selon la culture de la
##   faction jouée : france/england/burgundy/iberia/italy, DA4 —, `war` si le joueur est en
##   guerre, `court` quand la cour est ouverte, `menu` à l'écran-titre), fondu enchaîné entre
##   deux lecteurs. Chaque contexte a deux listes de lecture (`data/audio/music.json`) : `primary`
##   (musique d'époque libre de droits, tirée en priorité) et `fallback` (repli, notamment les
##   pistes Kevin MacLeod, utilisées seulement si `primary` ne fournit aucun fichier présent),
##   jouées en ordre aléatoire sans répéter le morceau précédent ; à défaut de tout,
##   `music/<contexte>.ogg` en boucle. `culture_regions` associe la culture de la faction jouée
##   (`data/factions/<id>.json`, champ `culture`) à une région musicale.
## - Effets : clic sur tout bouton (via `SceneTree.node_added`), page tournée à l'ouverture
##   des panneaux de la carte, cloche de fin de tour puis l'effet de l'événement le plus
##   marquant du tour (bataille, naissance, mort, guerre, paix, religion).
## - Fichiers `res://assets/audio/{sfx,music}/<nom>.ogg|.wav` ; absent = silence, sans erreur.
##
## Accès depuis les autres scripts : `get_node_or_null("/root/AudioDirector")` (les scripts
## chargés par le smoke test sont compilés avant l'enregistrement des autoloads).

const SETTINGS_PATH := "user://settings.cfg"
const MUSIC_BUS := "Musique"
## AU1 : les effets d'interface (clic, page, cloche de tour) passent par le bus « Interface ».
const SFX_BUS := "Interface"
## AU1 : volumes par défaut des bus réglables (voir `AudioBuses.PLAYER_BUSES`).
const DEFAULT_BUS_VOLUMES := {"Master": 1.0, "Musique": 0.6, "Ambiance": 0.8, "Bataille": 0.9, "Interface": 0.8, "Voix": 0.9}
const DUCK_ATTACK := 0.25
const DUCK_RELEASE := 1.5
const SFX_DIR := "res://assets/audio/sfx/"
const MUSIC_DIR := "res://assets/audio/music/"
const PLAYLISTS_PATH := "audio/music.json"
const SFX_VOICES := 6
const FADE_SECONDS := 1.5
## Effet par type d'événement du journal, par priorité décroissante.
const EVENT_SFX := [
	["battle", "sword_clash"],
	["siege_started", "war_horn"],
	["war_declared", "war_horn"],
	["province_captured", "fanfare"],
	["peace_signed", "fanfare"],
	["death", "choir"],
	["succession", "choir"],
	["excommunication", "choir"],
	["schism", "choir"],
	["heresy", "choir"],
	["birth", "fanfare"],
	["marriage", "fanfare"],
	["building_completed", "page_turn"],
]

var music_volume: float = 0.6
var sfx_volume: float = 0.8
## AU1 : volume linéaire 0..1 par bus (Master, Musique, Ambiance, Bataille, Interface, Voix) ;
## `music_volume` / `sfx_volume` restent synchronisés avec « Musique » / « Interface ».
var bus_volumes: Dictionary = DEFAULT_BUS_VOLUMES.duplicate()
## AU1 : ambiances de la carte de campagne (créées par `attach_campaign`).
var campaign_ambience: CampaignAmbience = null
var current_context: String = ""
## Headless (smoke test, serveur) : flux chargés et contextes suivis, mais rien n'est joué
## (le pilote audio factice ne libère pas les lectures OGG à la sortie).
var silent: bool = false

var _streams: Dictionary = {}  # chemin → AudioStream ou null
var _music_players: Array[AudioStreamPlayer] = []
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
## DA4 : culture (`data/factions/<id>.json`, champ `culture`) → région musicale
## (`campaign_<région>`), lue dans `data/audio/music.json` (clé `culture_regions`).
var _culture_regions: Dictionary = {}
var _faction_cultures: Dictionary = {}  # cache faction_id -> culture id
var _last_track: Dictionary = {}
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
	load_settings()
	load_playlists(SoundBank.data_path(PLAYLISTS_PATH))
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


## Volume du joueur (linéaire 0..1) d'un bus réglable ; persisté par défaut.
func set_bus_volume(bus_name: String, linear: float, persist: bool = true) -> void:
	if not bus_volumes.has(bus_name):
		return
	var value := clampf(linear, 0.0, 1.0)
	bus_volumes[bus_name] = value
	if bus_name == MUSIC_BUS:
		music_volume = value
	elif bus_name == SFX_BUS:
		sfx_volume = value
	AudioBuses.set_linear_volume(bus_name, value)
	if persist:
		save_settings()


func bus_volume(bus_name: String) -> float:
	return float(bus_volumes.get(bus_name, 0.0))


func set_music_volume(linear: float, persist: bool = true) -> void:
	set_bus_volume(MUSIC_BUS, linear, persist)


func set_sfx_volume(linear: float, persist: bool = true) -> void:
	set_bus_volume(SFX_BUS, linear, persist)


func load_settings(path: String = SETTINGS_PATH) -> void:
	var config := ConfigFile.new()
	if config.load(path) == OK:
		for bus_name in bus_volumes:
			bus_volumes[bus_name] = float(config.get_value("audio", "bus_" + str(bus_name), bus_volumes[bus_name]))
		# Clés historiques (M10) : elles priment pour la musique et l'interface.
		bus_volumes[MUSIC_BUS] = float(config.get_value("audio", "music_volume", music_volume))
		bus_volumes[SFX_BUS] = float(config.get_value("audio", "sfx_volume", sfx_volume))
	else:
		bus_volumes[MUSIC_BUS] = music_volume
		bus_volumes[SFX_BUS] = sfx_volume
	for bus_name in bus_volumes:
		set_bus_volume(bus_name, float(bus_volumes[bus_name]), false)


func save_settings(path: String = SETTINGS_PATH) -> Error:
	var config := ConfigFile.new()
	config.load(path)  # conserve les autres sections éventuelles
	config.set_value("audio", "music_volume", music_volume)
	config.set_value("audio", "sfx_volume", sfx_volume)
	for bus_name in bus_volumes:
		config.set_value("audio", "bus_" + str(bus_name), bus_volumes[bus_name])
	return config.save(path)


# --- Ducking (AU1) -----------------------------------------------------------------


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
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
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
			# Compat : ancien format (liste simple), toute la liste en primary.
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


## Choisit un morceau du contexte au hasard parmi `primary`, différent du précédent si possible ;
## repli sur `fallback` si `primary` ne fournit aucun fichier présent, puis sur
## `music/<contexte>.ogg` en boucle. `remember` : note le choix pour éviter la répétition.
func _pick_track(context: String, remember: bool = true) -> AudioStream:
	var entry: Dictionary = _playlists.get(context, {})
	for tier in [entry.get("primary", []), entry.get("fallback", [])]:
		var candidates: Array = (tier as Array).duplicate()
		if candidates.size() > 1:
			candidates.erase(_last_track.get(context, ""))
		while not candidates.is_empty():
			var index := _music_rng.randi_range(0, candidates.size() - 1)
			var path: String = candidates[index]
			var stream := _load_music(path)
			if stream != null:
				if remember:
					_last_track[context] = path
				return stream
			candidates.remove_at(index)
	return _load_stream(MUSIC_DIR, context, true)


## Région musicale (france/england/burgundy/iberia/italy) de la culture d'une faction, "" si
## inconnue ou absente de `culture_regions` (`data/audio/music.json`).
func culture_region(faction_id: String) -> String:
	if faction_id == "":
		return ""
	if not _faction_cultures.has(faction_id):
		var data := SoundBank.data_path("factions/" + faction_id + ".json")
		var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(data)) if FileAccess.file_exists(data) else null
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
	new_player.stream = stream
	new_player.volume_db = -40.0
	new_player.play()
	var tween := create_tween()
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


## Écran de démarrage : musique de menu (DA4).
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
	# AU1 : ambiances de carte (enfant du directeur : il survit à la mise en veille de la carte).
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
	if player_at_war():
		_base_context = "war"
	else:
		var faction := str(_campaign.get("player_faction")) if _campaign != null else ""
		_base_context = campaign_context(faction)
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
	for pair in EVENT_SFX:
		if kinds.has(pair[0]):
			return pair[1]
	return ""


# --- Interface de réglage --------------------------------------------------------


## AU1 : un curseur par bus réglable (Général, Musique, Ambiance, Bataille, Interface, Voix),
## 0..100 %, persisté à chaque changement.
func make_volume_controls() -> Control:
	var grid := GridContainer.new()
	grid.name = "SoundControls"
	grid.columns = 2
	grid.add_theme_constant_override("h_separation", 12)
	for spec in AudioBuses.PLAYER_BUSES:
		var bus_name: String = spec[0]
		var label := Label.new()
		label.text = str(spec[1])
		grid.add_child(label)
		var slider := HSlider.new()
		slider.name = "%sSlider" % bus_name
		slider.min_value = 0.0
		slider.max_value = 1.0
		slider.step = 0.05
		slider.value = bus_volume(bus_name)
		slider.custom_minimum_size = Vector2(220, 0)
		slider.size_flags_vertical = Control.SIZE_SHRINK_CENTER
		slider.value_changed.connect(func(value: float) -> void: set_bus_volume(bus_name, value))
		grid.add_child(slider)
	return grid
