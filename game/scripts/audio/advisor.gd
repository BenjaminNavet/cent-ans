class_name Advisor
extends Node

## VO1 — le conseiller, à la Total War : Jean le Bel, chanoine de Liège et chroniqueur, intervient
## brièvement (voix sur le bus « Voix » + sous-titre) au premier tour d'une partie, à la première
## bataille, au premier assaut, au premier siège, à la première victoire ou défaite et lors des
## alertes importantes du tour (guerre, peste, famine, révolte, banqueroute…). Textes :
## `data/voice/advisor.json` ; audio `res://assets/audio/voice/advisor/<id>.ogg` (absent : sous-titre
## seul). Désactivable (réglage `voice/advisor`). Les « premières fois » ne sont dites qu'une fois
## (réglage `voice/advisor_seen`) ; une même alerte pas plus d'une fois en `alert_cooldown_turns`
## tours. La musique est atténuée pendant qu'il parle ; les répliques d'unités se taisent.
##
## Usage : `Advisor.say_trigger("first_battle")`, `Advisor.on_turn_events(events, faction, turn)`.
## Le nœud se crée à la demande sous l'autoload `AudioDirector` (il survit aux changements de
## scène). Rendu seulement.

const NODE_NAME := "Advisor"
const BUS := "Voix"
const DUCK_DB := -9.0
const READ_CHARS_PER_SECOND := 14.0
const MAX_QUEUE := 2

var silent := false
## Faux en capture et en test : les « premières fois » ne sont pas enregistrées.
var persist := true
## Dernières interventions décidées (tests) : ids, 16 au plus.
var said: Array[String] = []

var _player: AudioStreamPlayer = null
var _layer: CanvasLayer = null
var _panel: PanelContainer = null
var _text_label: Label = null
var _queue: Array = []
var _until: float = 0.0
var _time: float = 0.0
var _last_end: float = -1000.0
var _alert_turns: Dictionary = {}  # événement → tour de la dernière alerte


static func current() -> Advisor:
	var tree := Engine.get_main_loop() as SceneTree
	if tree == null:
		return null
	var host: Node = tree.root.get_node_or_null("AudioDirector")
	if host == null:
		host = tree.root
	return host.get_node_or_null(NODE_NAME) as Advisor


static func instance() -> Advisor:
	var node := current()
	if node != null:
		return node
	var tree := Engine.get_main_loop() as SceneTree
	if tree == null:
		return null
	var host: Node = tree.root.get_node_or_null("AudioDirector")
	if host == null:
		host = tree.root
	node = Advisor.new()
	node.name = NODE_NAME
	host.add_child(node)
	return node


## Intervention liée à `trigger` (`campaign_start`, `first_battle`…) ; faux si rien n'est dit.
static func say_trigger(trigger: String, faction: String = "") -> bool:
	var node := instance()
	return node != null and node.trigger(trigger, faction)


## Fin de tour : premier siège et alertes qui concernent la faction du joueur.
static func on_turn_events(events: Array, player_faction: String, turn: int) -> void:
	var node := instance()
	if node != null:
		node.turn_events(events, player_faction, turn)


func _ready() -> void:
	process_mode = Node.PROCESS_MODE_ALWAYS
	silent = DisplayServer.get_name() == "headless"
	persist = not silent
	_player = AudioStreamPlayer.new()
	_player.name = "Voice"
	_player.bus = BUS if AudioServer.get_bus_index(BUS) >= 0 else "Master"
	add_child(_player)


func _exit_tree() -> void:
	if _player != null:
		_player.stop()
		_player.stream = null


func enabled() -> bool:
	var settings := get_node_or_null("/root/Settings")
	return settings == null or bool(settings.call("get_value", "voice/advisor"))


func speaking() -> bool:
	return _time < _until


func _seen() -> PackedStringArray:
	var settings := get_node_or_null("/root/Settings")
	if settings == null:
		return PackedStringArray()
	return str(settings.call("get_value", "voice/advisor_seen")).split(",", false)


func _mark_seen(trigger_name: String) -> void:
	var settings := get_node_or_null("/root/Settings")
	if settings == null or not persist:
		return
	var seen := _seen()
	if not seen.has(trigger_name):
		seen.append(trigger_name)
		settings.call("set_value", "voice/advisor_seen", ",".join(seen))


func trigger(trigger_name: String, faction: String = "") -> bool:
	if not enabled():
		return false
	var once := trigger_name.begins_with("first_")
	if once and _seen().has(trigger_name):
		return false
	var line := VoiceLines.advisor_line(trigger_name, faction)
	if line.is_empty():
		return false
	if once:
		_mark_seen(trigger_name)
	say(line)
	return true


func turn_events(events: Array, player_faction: String, turn: int) -> void:
	if not enabled():
		return
	var cooldown := int(VoiceLines.advisor().get("alert_cooldown_turns", 8))
	for event in events:
		if not event is Dictionary or str(event.get("faction", "")) != player_faction:
			continue
		var kind := str(event.get("kind", ""))
		if kind == "siege_started" and trigger("first_siege"):
			return
		var line := VoiceLines.advisor_line("alert", player_faction, kind)
		if line.is_empty():
			continue
		if _alert_turns.has(kind) and turn - int(_alert_turns[kind]) < cooldown:
			continue
		_alert_turns[kind] = turn
		say(line)
		return  # une alerte par tour au plus


## Dit `line` ({id, text}) maintenant, ou après l'intervention en cours (file de 2 au plus).
func say(line: Dictionary) -> void:
	if speaking():
		if _queue.size() < MAX_QUEUE:
			_queue.append(line)
		return
	var id := str(line.get("id", ""))
	said.append(id)
	if said.size() > 16:
		said.pop_front()
	var stream := VoiceLines.stream("advisor/" + id)
	var text := str(line.get("text", ""))
	var data := VoiceLines.advisor()
	var spoken := stream.get_length() if stream != null else text.length() / READ_CHARS_PER_SECOND
	_until = _time + spoken + float(data.get("linger_s", 2.5))
	_show(text, data)
	if silent or stream == null:
		return
	_player.stream = stream
	_player.play()
	var director := get_node_or_null("/root/AudioDirector")
	if director != null and director.has_method("duck_music"):
		director.call("duck_music", DUCK_DB, spoken + 0.5)


## Coupe l'intervention en cours (clic sur le sous-titre).
func dismiss() -> void:
	_until = _time
	_player.stop()
	if _panel != null:
		_panel.visible = false


func _process(delta: float) -> void:
	_time += delta
	if _panel != null and _panel.visible and not speaking():
		_panel.visible = false
		_last_end = _time
	if not speaking() and not _queue.is_empty() and _time - _last_end >= float(VoiceLines.advisor().get("min_gap_s", 1.5)):
		say(_queue.pop_front())


# --- Sous-titre ------------------------------------------------------------------------


func _show(text: String, data: Dictionary) -> void:
	if _layer == null:
		_build(data)
	_text_label.text = "« %s »" % text
	_panel.visible = true


func _build(data: Dictionary) -> void:
	_layer = CanvasLayer.new()
	_layer.layer = 40
	add_child(_layer)
	_panel = PanelContainer.new()
	_panel.name = "AdvisorPanel"
	var style := StyleBoxFlat.new()
	style.bg_color = Color(0.1, 0.08, 0.05, 0.86)
	style.border_color = Color(0.72, 0.58, 0.32, 0.9)
	style.set_border_width_all(2)
	style.set_corner_radius_all(5)
	style.content_margin_left = 18
	style.content_margin_right = 18
	style.content_margin_top = 10
	style.content_margin_bottom = 12
	_panel.add_theme_stylebox_override("panel", style)
	_panel.anchor_left = 0.0
	_panel.anchor_right = 0.0
	_panel.anchor_top = 0.0
	_panel.anchor_bottom = 0.0
	_panel.offset_left = 16
	_panel.offset_top = 96
	_panel.custom_minimum_size = Vector2(440, 0)
	_panel.mouse_filter = Control.MOUSE_FILTER_STOP
	_panel.tooltip_text = "Clic : faire taire le conseiller (Réglages → Son pour le désactiver)."
	_panel.gui_input.connect(func(event: InputEvent) -> void:
		if event is InputEventMouseButton and event.pressed:
			dismiss())
	_layer.add_child(_panel)
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 4)
	_panel.add_child(box)
	var name_label := Label.new()
	name_label.text = "%s — %s" % [str(data.get("name", "Le conseiller")), str(data.get("title", ""))]
	name_label.add_theme_font_size_override("font_size", 14)
	name_label.add_theme_color_override("font_color", Color(0.93, 0.8, 0.5))
	box.add_child(name_label)
	_text_label = Label.new()
	_text_label.custom_minimum_size = Vector2(404, 0)
	_text_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_text_label.add_theme_font_size_override("font_size", 17)
	_text_label.add_theme_color_override("font_color", Color(0.97, 0.94, 0.88))
	box.add_child(_text_label)
	_panel.visible = false
