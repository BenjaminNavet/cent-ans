class_name BattleSpeech
extends Node

## Discours du général avant la bataille (lot BV3), rendu seulement.
## Texte composé depuis `data/speeches/battle_speeches.json` selon la faction, le général, le
## rapport de forces, le terrain et la météo (choix déterministes : graine de la bataille), puis
## le cri de guerre de la faction (`data/battle_orders/order_war_cry.json`). Affiché en
## sous-titre (couche à part : le HUD de bataille n'est pas modifié) pendant un travelling de
## caméra le long des lignes du joueur ; le cri de guerre final fait reculer la caméra et joue le
## son `war_cry`. Échap, Entrée, espace ou un clic : passer. La caméra est rendue telle qu'elle
## était. Joué à l'ouverture du déploiement (simulation figée) ; sans déploiement, la bataille
## est mise en pause le temps du discours.
##
## VO1 : chaque phrase et le cri sont dits par une voix synthétique propre au général (voix de sa
## faction, `data/voice/speech_voices.json`, fichiers `assets/audio/voice/speech/`), sur le bus
## « Voix », musique atténuée ; la durée d'une phrase s'allonge à celle de sa voix (sous-titre
## synchronisé). Phrase sans fichier (nom du général) : sous-titre seul, durée fixe.

signal finished

const DATA_FILE := "speeches/battle_speeches.json"
const WAR_CRY_FILE := "battle_orders/order_war_cry.json"
const CAMERA_DISTANCE := 34.0
const CRY_DISTANCE := 95.0

## U22 : décalage (px) du bandeau sous le haut de l'écran, sous la barre de rapport de forces.
const SUBTITLE_TOP := 76.0

var active: bool = false
## Vrai si le joueur a passé le discours (le discours adverse n'est alors pas joué).
var skipped: bool = false
var lines: Array = []
var cry: String = ""
var speaker: String = ""

var _scene: Node = null
var _t: float = 0.0
var _line_s: float = 4.6
var _cry_s: float = 3.0
var _path: PackedVector3Array = PackedVector3Array()
var _yaw: float = 0.0
var _saved: Dictionary = {}
var _paused_before: bool = false
var _layer: CanvasLayer = null
var _speaker_label: Label = null
var _line_label: Label = null
var _cry_label: Label = null
var _band: Control = null
var _cried: bool = false
## VO1 : voix du général, début et durée de chaque phrase.
var voice: String = ""
var _voice_player: AudioStreamPlayer = null
var _starts: PackedFloat32Array = PackedFloat32Array()
var _durations: PackedFloat32Array = PackedFloat32Array()
var _spoken_index: int = -1
var _cry_stream: AudioStream = null


## Discours de `side` : {lines: [phrases], cry, speaker}. `ratio` = nos hommes / les leurs.
static func compose(setup: Dictionary, side: String, ratio: float, terrain_key: String, weather_key: String, seed_value: int) -> Dictionary:
	var data := BattleStandards.read_data(DATA_FILE)
	if data.is_empty():
		return {}
	var side_setup: Dictionary = setup.get(side, {})
	var faction := str(side_setup.get("faction", ""))
	var general: Variant = side_setup.get("general", null)
	var general_name := str((general as Dictionary).get("name", "")) if general is Dictionary else ""
	var pick := func(list: Array, salt: String) -> String:
		return str(list[absi(hash("%d/%s/%s" % [seed_value, side, salt])) % list.size()]) if not list.is_empty() else ""
	var by_key := func(section: String, key: String) -> Array:
		var table: Dictionary = data.get(section, {})
		return table.get(key, table.get("default", []))
	var out: Array = []
	out.append(pick.call(by_key.call("openings", faction), "open"))
	if general_name != "":
		out.append(str(pick.call(data.get("general_lines", []), "general")).replace("{general}", general_name))
	var thresholds: Dictionary = data.get("odds_thresholds", {})
	var odds := "even"
	if ratio >= float(thresholds.get("strong", 1.3)):
		odds = "strong"
	elif ratio <= float(thresholds.get("weak", 0.77)):
		odds = "weak"
	out.append(pick.call((data.get("odds", {}) as Dictionary).get(odds, []), "odds"))
	out.append(pick.call(by_key.call("terrain", terrain_key), "terrain"))
	var weather_lines: Array = (data.get("weather", {}) as Dictionary).get(weather_key, [])
	if not weather_lines.is_empty():
		out.append(pick.call(weather_lines, "weather"))
	out.append(pick.call(by_key.call("closings", faction), "close"))
	var cries: Dictionary = BattleStandards.read_data(WAR_CRY_FILE).get("labels_by_faction", {})
	var result_lines: Array = []
	for line in out:
		if str(line) != "":
			result_lines.append(str(line))
	return {
		"lines": result_lines,
		"cry": str(cries.get(faction, data.get("default_cry", "En avant !"))),
		"speaker": general_name if general_name != "" else str(data.get("speaker_without_general", "le capitaine")),
		"line_seconds": float(data.get("line_seconds", 4.6)),
		"cry_seconds": float(data.get("cry_seconds", 3.0)),
		"faction": faction,
		"general_id": str((general as Dictionary).get("character", general_name)) if general is Dictionary else "",
	}


## Lance le discours ; `false` s'il n'y a rien à dire (données absentes, aucun régiment).
func start(scene: Node, speech: Dictionary, units: Array, side: String) -> bool:
	if speech.is_empty() or (speech.get("lines", []) as Array).is_empty():
		return false
	_scene = scene
	lines = speech["lines"]
	cry = str(speech["cry"])
	speaker = str(speech["speaker"])
	_line_s = float(speech["line_seconds"])
	_cry_s = float(speech["cry_seconds"])
	if not _build_path(units, side):
		return false
	_schedule_voice(speech)
	var rig: BattleCamera = scene.camera_rig
	_saved = {"target": rig.target, "distance": rig.distance, "yaw": rig.yaw}
	_paused_before = bool(scene.paused)
	var deployment: Node = scene.get("deployment")
	if deployment == null or not bool(deployment.get("active")):
		scene.paused = true
	_build_subtitle()
	active = true
	_t = 0.0
	_update(0.0)
	return true


func total_seconds() -> float:
	return _speaking_seconds() + _cry_s


func _speaking_seconds() -> float:
	if _durations.size() == lines.size() and not lines.is_empty():
		return _starts[lines.size() - 1] + _durations[lines.size() - 1]
	return _line_s * float(lines.size())


## VO1 : voix du général et calendrier des phrases (durée = max(durée écrite, voix + pause)).
func _schedule_voice(speech: Dictionary) -> void:
	var faction := str(speech.get("faction", ""))
	voice = VoiceLines.speech_voice(faction, str(speech.get("general_id", speaker)))
	var padding := float(VoiceLines.speech_casting().get("line_padding_s", 0.6))
	_starts = PackedFloat32Array()
	_durations = PackedFloat32Array()
	var t := 0.0
	for line in lines:
		var stream := VoiceLines.speech_stream(voice, str(line))
		var length := _line_s if stream == null else maxf(_line_s, stream.get_length() + padding)
		_starts.append(t)
		_durations.append(length)
		t += length
	_cry_stream = VoiceLines.speech_stream(voice, cry)
	if _cry_stream != null:
		_cry_s = maxf(_cry_s, _cry_stream.get_length() + float(VoiceLines.speech_casting().get("cry_padding_s", 0.3)))
	_voice_player = AudioStreamPlayer.new()
	_voice_player.name = "GeneralVoice"
	_voice_player.bus = "Voix" if AudioServer.get_bus_index("Voix") >= 0 else "Master"
	add_child(_voice_player)
	var director := get_node_or_null("/root/AudioDirector")
	if director != null and director.has_method("duck_music"):
		director.call("duck_music", -8.0, total_seconds())


func _line_index(t: float) -> int:
	var index := 0
	for i in _starts.size():
		if t >= _starts[i]:
			index = i
	return mini(index, lines.size() - 1)


func _say(stream: AudioStream) -> void:
	if _voice_player == null or stream == null or DisplayServer.get_name() == "headless":
		return
	_voice_player.stop()
	_voice_player.stream = stream
	_voice_player.play()


## Chemin de la caméra : centres des régiments du joueur, rangés le long du front.
func _build_path(units: Array, side: String) -> bool:
	var points: Array = []
	var facing := Vector2.ZERO
	for unit in units:
		if str(unit["side"]) != side or not bool(unit["present"]) or str(unit.get("render", "")) == "siege":
			continue
		points.append(Vector3(float(unit["x"]), float(unit.get("y", 0.0)), float(unit["z"])))
		var f := float(unit.get("facing", 0.0))
		facing += Vector2(sin(f), cos(f))
	if points.is_empty():
		return false
	_yaw = atan2(facing.x, facing.y)
	var lateral := Vector3(cos(_yaw), 0, -sin(_yaw))
	points.sort_custom(func(a: Vector3, b: Vector3) -> bool: return a.dot(lateral) < b.dot(lateral))
	_path = PackedVector3Array(points)
	return true


func _point_at(u: float) -> Vector3:
	if _path.size() == 1:
		return _path[0]
	var x := clampf(u, 0.0, 1.0) * float(_path.size() - 1)
	var i := mini(int(x), _path.size() - 2)
	return _path[i].lerp(_path[i + 1], x - float(i))


func _process(delta: float) -> void:
	if not active:
		return
	_t += delta
	_update(delta)
	if _t >= total_seconds():
		stop()


func _update(_delta: float) -> void:
	var speaking := _speaking_seconds()
	var rig: BattleCamera = _scene.camera_rig
	if _t < speaking:
		var u := smoothstep(0.0, 1.0, _t / speaking)
		# Travelling le long des lignes, de trois quarts, la caméra tournant d'un flanc à l'autre.
		rig.look_at_point(_point_at(u), CAMERA_DISTANCE, _yaw + lerpf(0.75, -0.75, u))
		var index := _line_index(_t)
		if index != _spoken_index:
			_spoken_index = index
			_say(VoiceLines.speech_stream(voice, str(lines[index])))
		var text := str(lines[index])
		if index == 0:
			text = "« " + text
		if index == lines.size() - 1:
			text += " »"
		_line_label.text = text
		var local := clampf((_t - _starts[index]) / _durations[index], 0.0, 1.0) if index < _durations.size() else fmod(_t, _line_s) / _line_s
		_line_label.modulate.a = smoothstep(0.0, 0.08, local) * (1.0 - smoothstep(0.92, 1.0, local))
		_cry_label.visible = false
	else:
		# Cri de guerre : la caméra recule sur l'armée entière.
		var k := smoothstep(0.0, 1.0, (_t - speaking) / _cry_s)
		rig.look_at_point(_point_at(0.5), lerpf(CAMERA_DISTANCE, CRY_DISTANCE, k), _yaw)
		_band.visible = false
		_cry_label.visible = true
		_cry_label.text = cry
		_cry_label.modulate.a = smoothstep(0.0, 0.15, k)
		if not _cried:
			_cried = true
			BattleAudio.play_at("war_cry", _point_at(0.5))
			_say(_cry_stream)


func stop() -> void:
	if not active:
		return
	active = false
	if _voice_player != null:
		_voice_player.stop()
		_voice_player.stream = null
	var rig: BattleCamera = _scene.camera_rig
	rig.look_at_point(_saved["target"], float(_saved["distance"]), float(_saved["yaw"]))
	_scene.paused = _paused_before
	if _layer != null:
		_layer.queue_free()
		_layer = null
	finished.emit()


func _input(event: InputEvent) -> void:
	if not active:
		return
	var skip := false
	if event is InputEventKey and event.pressed and not event.echo:
		skip = (event as InputEventKey).keycode in [KEY_ESCAPE, KEY_ENTER, KEY_KP_ENTER, KEY_SPACE]
	elif event is InputEventMouseButton and event.pressed:
		skip = (event as InputEventMouseButton).button_index == MOUSE_BUTTON_LEFT
	if skip:
		get_viewport().set_input_as_handled()
		skipped = true
		stop()


## Sous-titre : nom de l'orateur et phrase, bandeau sombre au-dessus des panneaux du HUD ; cri de
## guerre en grand au centre.
func _build_subtitle() -> void:
	_layer = CanvasLayer.new()
	_layer.layer = 20
	add_child(_layer)
	var root := Control.new()
	root.set_anchors_preset(Control.PRESET_FULL_RECT)
	root.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_layer.add_child(root)
	var band := PanelContainer.new()
	var style := StyleBoxFlat.new()
	style.bg_color = Color(0.06, 0.05, 0.04, 0.72)
	style.content_margin_left = 28
	style.content_margin_right = 28
	style.content_margin_top = 10
	style.content_margin_bottom = 12
	style.corner_radius_top_left = 4
	style.corner_radius_top_right = 4
	style.corner_radius_bottom_left = 4
	style.corner_radius_bottom_right = 4
	band.add_theme_stylebox_override("panel", style)
	band.anchor_left = 0.2
	band.anchor_right = 0.8
	# U22 : en haut au centre, sous la barre de rapport de forces (haute de ~56 px), loin des
	# panneaux de formations et des cartes d'unités qui occupent le milieu et le bas de l'écran.
	band.anchor_top = 0.0
	band.anchor_bottom = 0.0
	band.offset_top = SUBTITLE_TOP
	band.grow_vertical = Control.GROW_DIRECTION_END
	band.mouse_filter = Control.MOUSE_FILTER_IGNORE
	root.add_child(band)
	_band = band
	PanelStack.mark_blocking(band)  # Q4 : pas de bulle du conseiller sur le discours
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 4)
	band.add_child(box)
	_speaker_label = BattleUiKit.label(speaker.to_upper(), 15, Color(0.93, 0.8, 0.5), true)
	_speaker_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	box.add_child(_speaker_label)
	_line_label = BattleUiKit.label("", 23, Color(0.97, 0.95, 0.9))
	_line_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_line_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	box.add_child(_line_label)
	var hint := BattleUiKit.label("Échap ou clic : passer", 12, Color(0.8, 0.76, 0.66, 0.8))
	hint.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	box.add_child(hint)
	_cry_label = BattleUiKit.label("", 54, Color(0.98, 0.9, 0.62), true)
	_cry_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_cry_label.anchor_left = 0.0
	_cry_label.anchor_right = 1.0
	_cry_label.anchor_top = 0.36
	_cry_label.anchor_bottom = 0.36
	_cry_label.add_theme_color_override("font_shadow_color", Color(0, 0, 0, 0.8))
	_cry_label.add_theme_constant_override("shadow_offset_x", 3)
	_cry_label.add_theme_constant_override("shadow_offset_y", 3)
	_cry_label.visible = false
	root.add_child(_cry_label)

