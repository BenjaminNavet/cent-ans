class_name UiSounds
extends Node

## UB1 / U13 — sons d'interface distincts, joués en 2D sur le bus « Interface » d'AU1 depuis la
## banque `data/audio/sound_bank.json` (événements `ui_*`, fichiers `assets/audio/ui/`) :
## ordre donné, ordre refusé, alerte, lettre reçue, recrutement, construction, sélection
## d'armée, clic sur une carte d'unité. Respecte priorité, recharge et limite d'instances de la
## banque. Muet en headless (comme `AudioDirector.silent`).
##
## Usage : `UiSounds.play("ui_order")` (ou `UiSounds.play_order_result(result)` après un ordre).
## Le nœud se crée à la demande sous l'autoload `AudioDirector`.

const NODE_NAME := "UiSounds"
const BUS := "Interface"
const VOICES := 4

## Nom court → événement de la banque (pour les appelants qui pensent « action »).
const EVENTS := {
	"order": "ui_order", "refused": "ui_order_refused", "alert": "ui_alert", "letter": "ui_letter",
	"recruit": "ui_recruit", "build": "ui_build", "army": "ui_army_select", "card": "ui_card",
}

var bank: SoundBank = null
var silent := false
var played: Array[String] = []  # derniers événements joués (tests), 16 au plus
var _players: Array[AudioStreamPlayer] = []
var _last_time: Dictionary = {}  # événement → instant (s) de la dernière lecture
var _next := 0


static func instance() -> UiSounds:
	var tree := Engine.get_main_loop() as SceneTree
	if tree == null:
		return null
	var host: Node = tree.root.get_node_or_null("AudioDirector")
	if host == null:
		host = tree.root
	var node := host.get_node_or_null(NODE_NAME) as UiSounds
	if node == null:
		node = UiSounds.new()
		node.name = NODE_NAME
		host.add_child(node)
	return node


## Joue l'événement `name` (`ui_order`… ou son nom court `order`…). Faux s'il est inconnu, en
## recharge, ou si l'audio est muet.
static func play(name: String) -> bool:
	var node := instance()
	return node != null and node.play_event(str(EVENTS.get(name, name)))


## Après un ordre : « ordre donné » s'il est accepté, « ordre refusé » sinon.
static func play_order_result(result: Dictionary) -> void:
	play("ui_order" if bool(result.get("ok", false)) else "ui_order_refused")


func _ready() -> void:
	process_mode = Node.PROCESS_MODE_ALWAYS
	silent = DisplayServer.get_name() == "headless"
	bank = SoundBank.load_default()
	for index in VOICES:
		var player := AudioStreamPlayer.new()
		player.name = "Voice%d" % index
		player.bus = BUS if AudioServer.get_bus_index(BUS) >= 0 else "Master"
		add_child(player)
		_players.append(player)


func _exit_tree() -> void:
	for player in _players:
		player.stop()
		player.stream = null


func play_event(event_name: String) -> bool:
	if bank == null or not bank.has_event(event_name):
		return false
	var entry := bank.event(event_name)
	var now := Time.get_ticks_msec() / 1000.0
	if now - float(_last_time.get(event_name, -1000.0)) < float(entry.get("cooldown_s", 0.0)):
		return false
	_last_time[event_name] = now
	played.push_back(event_name)
	if played.size() > 16:
		played.pop_front()
	if silent:
		return true
	var stream := bank.pick_stream(event_name)
	if stream == null:
		return false
	var player := _pick_player(int(entry.get("priority", 3)), event_name, int(entry.get("max_instances", 2)))
	if player == null:
		return false
	player.stream = stream
	player.volume_db = float(entry.get("volume_db", 0.0))
	player.pitch_scale = bank.random_pitch(event_name)
	player.set_meta("event", event_name)
	player.set_meta("priority", int(entry.get("priority", 3)))
	player.play()
	return true


## Voix libre ; sinon la plus ancienne du même événement au-delà de sa limite ; sinon une voix de
## priorité inférieure ou égale.
func _pick_player(priority: int, event_name: String, max_instances: int) -> AudioStreamPlayer:
	var same: Array[AudioStreamPlayer] = []
	for player in _players:
		if player.playing and str(player.get_meta("event", "")) == event_name:
			same.append(player)
	if same.size() >= max_instances:
		return same[0]
	for player in _players:
		if not player.playing:
			return player
	for i in _players.size():
		var player := _players[(_next + i) % _players.size()]
		if int(player.get_meta("priority", 0)) <= priority:
			_next = (_next + i + 1) % _players.size()
			return player
	return null
