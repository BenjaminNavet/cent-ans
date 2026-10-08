class_name BattleDemosMenu
extends PanelContainer

## SG2 — « Batailles de démonstration » du menu principal : liste `data/ui/battle_demos.json`
## (bataille rangée, assauts de Guyenne, de Paris, d'Avignon et de Bruges dans le plan de la
## ville). Un clic lance la scène de bataille autonome avec les options de la démo
## (`BattleScene.demo_args`) ; la fin de la bataille ramène au menu. Échap ou « Fermer » :
## signal `closed`.
## Lot P2e (ADR 0097, bible DA § 12.1) : la fenêtre rejoint la zone `MODAL` de `UiLayout`
## (voile assombri et blocage des entrées fournis par la zone, plus de veil ni de
## `CenterContainer` propres) ; tailles de texte par `UiType`, ouverture et fermeture par
## `UiMotion`.

signal closed
signal demo_started(id: String)

const DEMOS_FILE := "ui/battle_demos.json"
const BATTLE_SCENE := "res://scenes/battle/battle.tscn"
## EP8 : phases proposées pour l'heure de la bataille (règles du cœur, libellés des données).
const TIME_OF_DAY_FILE := "rules/battle_time_of_day.json"

## EP8 : heure choisie (clé de phase, "" : tirée par la campagne), gardée d'une démo à l'autre.
static var chosen_hour: String = ""

var demos: Array = []
var buttons: Dictionary = {}  # id -> Button (tests)
var hour_option: OptionButton = null
var hour_keys: Array[String] = [""]


## Démos déclarées (`[]` si le fichier manque).
static func load_demos() -> Array:
	if DataFile.exists(DEMOS_FILE):
		var parsed: Variant = DataFile.read_json(DEMOS_FILE)
		if parsed is Dictionary:
			return (parsed as Dictionary).get("demos", [])
	return []


## EP8 : phases sélectionnables `[{key, label}]` (`[]` si le fichier manque).
static func load_day_phases() -> Array:
	if DataFile.exists(TIME_OF_DAY_FILE):
		var parsed: Variant = DataFile.read_json(TIME_OF_DAY_FILE)
		if parsed is Dictionary:
			var out: Array = []
			for phase in (parsed as Dictionary).get("phases", []):
				if bool((phase as Dictionary).get("selectable", true)):
					out.append({"key": str(phase["key"]), "label": str(phase["label"])})
			return out
	return []


## Options de la scène de bataille pour la démo `demo` (mêmes options que la ligne de commande).
static func args_for(demo: Dictionary) -> PackedStringArray:
	var args := PackedStringArray()
	if chosen_hour != "":
		args.append("--hour=" + chosen_hour)
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
	theme = load("res://scenes/ui/parchment_theme.tres")
	mouse_filter = Control.MOUSE_FILTER_STOP
	custom_minimum_size = Vector2(680, 0)
	demos = load_demos()
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 8)
	add_child(box)
	var title := Label.new()
	title.text = "Batailles de démonstration"
	UiType.apply(title, UiType.TITLE)
	box.add_child(title)
	var hint := Label.new()
	hint.text = "Hors campagne : le résultat n'est pas conservé."
	UiType.apply(hint, UiType.CAPTION)
	hint.modulate = Color(1, 1, 1, 0.7)
	box.add_child(hint)
	# EP8 : heure de la bataille (lumière ; aube et crépuscule raccourcissent la portée des tireurs).
	var hour_row := HBoxContainer.new()
	box.add_child(hour_row)
	var hour_label := Label.new()
	hour_label.text = "Heure de la bataille"
	RichTooltip.attach_plain(hour_label, "battle_demo_hour")
	hour_label.mouse_filter = Control.MOUSE_FILTER_STOP
	hour_row.add_child(hour_label)
	hour_option = OptionButton.new()
	hour_option.name = "HourOption"
	hour_option.add_item("Au hasard (comme en campagne)", 0)
	for phase in load_day_phases():
		hour_keys.append(str(phase["key"]))
		hour_option.add_item(str(phase["label"]), hour_keys.size() - 1)
	hour_option.selected = maxi(hour_keys.find(chosen_hour), 0)
	hour_option.item_selected.connect(func(index: int) -> void: chosen_hour = hour_keys[index])
	hour_row.add_child(hour_option)
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
		UiType.apply(detail, UiType.CAPTION)
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
	UiZones.put(UiZones.Zone.MODAL, self)
	UiMotion.fade_in(self)


func _unhandled_input(event: InputEvent) -> void:
	if visible and event.is_action_pressed("ui_cancel"):
		close()
		get_viewport().set_input_as_handled()


func close() -> void:
	closed.emit()
	UiMotion.fade_out(self, UiMotion.DURATION, true)


## Lance la démo `id` : options transmises à la scène de bataille, puis changement de scène.
func start(id: String) -> void:
	for demo in demos:
		if str((demo as Dictionary)["id"]) == id:
			BattleScene.demo_args = args_for(demo)
			demo_started.emit(id)
			var audio := get_node_or_null("/root/AudioDirector")
			if audio != null and audio.has_method("stop_all"):
				audio.call("stop_all")
			SceneFader.go(BATTLE_SCENE)
			return
