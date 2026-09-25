class_name BattleDemosMenu
extends Control

## SG2 — « Batailles de démonstration » du menu principal : liste `data/ui/battle_demos.json`
## (bataille rangée, assauts de Guyenne, de Paris, d'Avignon et de Bruges dans le plan de la
## ville). Un clic lance la scène de bataille autonome avec les options de la démo
## (`BattleScene.demo_args`) ; la fin de la bataille ramène au menu. Échap ou « Fermer » :
## signal `closed`.

signal closed
signal demo_started(id: String)

const DEMOS_FILE := "ui/battle_demos.json"
const BATTLE_SCENE := "res://scenes/battle/battle.tscn"

var demos: Array = []
var buttons: Dictionary = {}  # id -> Button (tests)


## Démos déclarées (`[]` si le fichier manque).
static func load_demos() -> Array:
	var candidates: Array[String] = []
	var tree := Engine.get_main_loop() as SceneTree
	var paths: Node = tree.root.get_node_or_null("/root/MapPaths") if tree != null else null
	if paths != null:
		candidates.append(str(paths.get("data_dir")))
	candidates.append(ProjectSettings.globalize_path("res://").path_join("../data").simplify_path())
	for dir in candidates:
		var path := dir.path_join(DEMOS_FILE)
		if FileAccess.file_exists(path):
			var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
			if parsed is Dictionary:
				return (parsed as Dictionary).get("demos", [])
	return []


## Options de la scène de bataille pour la démo `demo` (mêmes options que la ligne de commande).
static func args_for(demo: Dictionary) -> PackedStringArray:
	var args := PackedStringArray()
	if str(demo.get("kind", "field")) == "siege":
		if demo.has("landmark"):
			args.append("--siege-landmark=" + str(demo["landmark"]))
			args.append("--siege-attacker=" + str(demo.get("attacker", "fac_france")))
		elif demo.has("province"):
			args.append("--siege-province=" + str(demo["province"]))
		else:
			args.append("--siege")
		var engines: Array = demo.get("engines", [])
		if not engines.is_empty():
			args.append("--siege-engines=" + ",".join(PackedStringArray(engines)))
	return args


func _ready() -> void:
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	theme = load("res://scenes/ui/parchment_theme.tres")
	mouse_filter = Control.MOUSE_FILTER_STOP
	demos = load_demos()
	var veil := ColorRect.new()
	veil.color = Color(0.05, 0.03, 0.01, 0.45)
	veil.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	add_child(veil)
	var center := CenterContainer.new()
	center.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	add_child(center)
	var panel := PanelContainer.new()
	panel.custom_minimum_size = Vector2(680, 0)
	center.add_child(panel)
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 8)
	panel.add_child(box)
	var title := Label.new()
	title.text = "Batailles de démonstration"
	title.add_theme_font_size_override("font_size", 26)
	box.add_child(title)
	var hint := Label.new()
	hint.text = "Hors campagne : le résultat n'est pas conservé."
	hint.add_theme_font_size_override("font_size", 15)
	hint.modulate = Color(1, 1, 1, 0.7)
	box.add_child(hint)
	for demo in demos:
		var entry := demo as Dictionary
		var button := Button.new()
		button.name = "Demo_" + str(entry["id"])
		button.text = str(entry["label"])
		button.alignment = HORIZONTAL_ALIGNMENT_LEFT
		button.tooltip_text = str(entry.get("detail", ""))
		button.pressed.connect(start.bind(str(entry["id"])))
		box.add_child(button)
		var detail := Label.new()
		detail.text = str(entry.get("detail", ""))
		detail.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		detail.custom_minimum_size = Vector2(640, 0)
		detail.add_theme_font_size_override("font_size", 14)
		detail.modulate = Color(1, 1, 1, 0.75)
		box.add_child(detail)
		buttons[str(entry["id"])] = button
	var row := HBoxContainer.new()
	row.alignment = BoxContainer.ALIGNMENT_END
	box.add_child(row)
	var close_button := Button.new()
	close_button.name = "CloseButton"
	close_button.text = "Fermer"
	close_button.pressed.connect(close)
	row.add_child(close_button)
	if not buttons.is_empty():
		(buttons.values()[0] as Button).grab_focus.call_deferred()


func _unhandled_input(event: InputEvent) -> void:
	if visible and event.is_action_pressed("ui_cancel"):
		close()
		get_viewport().set_input_as_handled()


func close() -> void:
	closed.emit()
	queue_free()


## Lance la démo `id` : options transmises à la scène de bataille, puis changement de scène.
func start(id: String) -> void:
	for demo in demos:
		if str((demo as Dictionary)["id"]) == id:
			BattleScene.demo_args = args_for(demo)
			demo_started.emit(id)
			var audio := get_node_or_null("/root/AudioDirector")
			if audio != null and audio.has_method("stop_all"):
				audio.call("stop_all")
			get_tree().change_scene_to_file(BATTLE_SCENE)
			return
