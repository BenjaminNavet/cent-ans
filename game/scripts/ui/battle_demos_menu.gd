class_name BattleDemosMenu
extends ListMenu

## SG2 — « Batailles de démonstration » du menu principal : liste `data/ui/battle_demos.json`
## (bataille rangée, assauts de Guyenne, de Paris, d'Avignon et de Bruges dans le plan de la
## ville). Un clic lance la scène de bataille autonome avec les options de la démo
## (`BattleScene.demo_args`) ; la fin de la bataille ramène au menu. Échap ou « Fermer » :
## signal `closed`.

signal demo_started(id: String)

const DEMOS_FILE := "ui/battle_demos.json"
## EP8 : phases proposées pour l'heure de la bataille (règles du cœur, libellés des données).
const TIME_OF_DAY_FILE := "rules/battle_time_of_day.json"

## EP8 : heure choisie (clé de phase, "" : tirée par la campagne), gardée d'une démo à l'autre.
static var chosen_hour: String = ""

var demos: Array = []
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


func _menu_title() -> String:
	return "Batailles de démonstration"


func _menu_hint() -> String:
	return "Hors campagne : le résultat n'est pas conservé."


func _load_entries() -> void:
	demos = load_demos()


func _build_entries(box: VBoxContainer) -> void:
	# Heure de la bataille (lumière ; aube et crépuscule raccourcissent la portée des tireurs).
	var hour_row := UiBuild.hbox(8, box)
	var hour_label := UiBuild.label("Heure de la bataille", 0, null, false, 0.0, hour_row)
	RichTooltip.attach_plain(hour_label, "battle_demo_hour")
	hour_label.mouse_filter = Control.MOUSE_FILTER_STOP
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
		var button := UiBuild.button(str(entry["label"]), start.bind(str(entry["id"])), box)
		button.name = "Demo_" + str(entry["id"])
		button.alignment = HORIZONTAL_ALIGNMENT_LEFT
		button.tooltip_text = str(entry.get("detail", ""))
		var detail := UiBuild.label(str(entry.get("detail", "")), 0, null, true, 640, box)
		UiType.apply(detail, UiType.CAPTION)
		detail.modulate = Color(1, 1, 1, 0.75)
		buttons[str(entry["id"])] = button


## Lance la démo `id` : options transmises à la scène de bataille, puis changement de scène.
func start(id: String) -> void:
	for demo in demos:
		if str((demo as Dictionary)["id"]) == id:
			demo_started.emit(id)
			_launch_battle(args_for(demo))
			return
