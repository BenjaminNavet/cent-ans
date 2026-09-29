class_name HistoricalBattlesMenu
extends PanelContainer

## EP7 — « Batailles historiques » du menu principal : Crécy (1346), Poitiers (1356), Azincourt
## (1415), lues dans `data/battle_maps/` par le cœur (`BattleSim.list_historical`). Pour chaque
## bataille : date, lieu, résumé, les deux armées ; on mène l'un des deux camps ou on regarde
## l'IA contre l'IA. La scène de bataille reçoit `--historical=<id>` et `--historical-side=` ;
## la fin de la bataille ramène au menu (hors campagne, rien n'est conservé). Échap ou
## « Fermer » : signal `closed`.
## Lot P2e (ADR 0097, bible DA § 12.1) : zone `MODAL` de `UiLayout` (plus de veil ni de
## `CenterContainer` propres) ; tailles de texte par `UiType`, ouverture et fermeture par
## `UiMotion`.
## NT4 : le « Didacticiel de bataille » (bataille-prologue guidée, `BattlePrologue`) ouvre la liste.

signal closed
signal battle_started(id: String, side: String)

const BATTLE_SCENE := "res://scenes/battle/battle.tscn"

var battles: Array = []
var buttons: Dictionary = {}  # "<id>:<side>" -> Button (tests)
var prologue_button: Button = null  # NT4 : « Commencer le didacticiel »


## Batailles historiques déclarées (`[]` sans l'extension ou sans données).
static func load_battles() -> Array:
	if not ClassDB.class_exists("BattleSim"):
		return []
	var sim: Object = ClassDB.instantiate("BattleSim")
	if not sim.has_method("list_historical"):
		return []
	return sim.call("list_historical", data_dir())


static func data_dir() -> String:
	var tree := Engine.get_main_loop() as SceneTree
	var paths: Node = tree.root.get_node_or_null("/root/MapPaths") if tree != null else null
	if paths != null:
		return str(paths.get("data_dir"))
	return ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()


## Options de la scène de bataille pour la bataille `id` menée par `side` ("" : IA contre IA).
static func args_for(id: String, side: String) -> PackedStringArray:
	var args := PackedStringArray(["--historical=" + id])
	if side != "":
		args.append("--historical-side=" + side)
	return args


func _ready() -> void:
	theme = load("res://scenes/ui/parchment_theme.tres")
	mouse_filter = Control.MOUSE_FILTER_STOP
	custom_minimum_size = Vector2(780, 0)
	battles = load_battles()
	var scroll := ScrollContainer.new()
	scroll.custom_minimum_size = Vector2(760, 560)
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	add_child(scroll)
	var box := VBoxContainer.new()
	box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	box.add_theme_constant_override("separation", 8)
	scroll.add_child(box)
	var title := Label.new()
	title.text = "Batailles historiques"
	UiType.apply(title, UiType.TITLE)
	box.add_child(title)
	var hint := Label.new()
	hint.text = "Le champ réel, les armées de ce jour-là, la météo et l'heure : menez l'un des camps, ou regardez. Hors campagne : le résultat n'est pas conservé."
	hint.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	hint.custom_minimum_size = Vector2(720, 0)
	UiType.apply(hint, UiType.CAPTION)
	hint.modulate = Color(1, 1, 1, 0.75)
	box.add_child(hint)
	_add_prologue(box)
	if battles.is_empty():
		var none := Label.new()
		none.text = "Aucune bataille historique n'a été trouvée."
		box.add_child(none)
	for entry in battles:
		_add_battle(box, entry as Dictionary)
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


## NT4 : « Didacticiel de bataille » en tête de liste (bataille-prologue guidée), pour ne pas
## ajouter de ligne à la colonne du menu principal (tenue à 1280×720 par NT2).
func _add_prologue(box: VBoxContainer) -> void:
	var prologue := BattlePrologue.load_data()
	if prologue.is_empty():
		return
	var sep := HSeparator.new()
	box.add_child(sep)
	var name := Label.new()
	name.text = str(prologue.get("menu_label", "Didacticiel de bataille"))
	UiType.apply(name, UiType.HEADING)
	box.add_child(name)
	var detail := Label.new()
	detail.text = str(prologue.get("menu_detail", ""))
	detail.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	detail.custom_minimum_size = Vector2(720, 0)
	UiType.apply(detail, UiType.BODY)
	box.add_child(detail)
	prologue_button = Button.new()
	prologue_button.name = "BattlePrologue"
	prologue_button.text = "Commencer le didacticiel"
	prologue_button.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN
	prologue_button.pressed.connect(start_prologue)
	box.add_child(prologue_button)


## NT4 : lance la bataille-prologue guidée.
func start_prologue() -> void:
	if BattlePrologue.launch(get_tree()):
		battle_started.emit("battle_prologue", str((BattlePrologue.load_data().get("battle", {}) as Dictionary).get("player_side", "")))


func _add_battle(box: VBoxContainer, entry: Dictionary) -> void:
	var id := str(entry.get("id", ""))
	var sep := HSeparator.new()
	box.add_child(sep)
	var name := Label.new()
	name.text = "%s — %s" % [str(entry.get("name", id)), str(entry.get("date_fr", entry.get("date", "")))]
	UiType.apply(name, UiType.HEADING)
	box.add_child(name)
	var place := Label.new()
	place.text = "%s (%s) · %s" % [str(entry.get("place", "")), str(entry.get("province_name", "")), str(entry.get("weather_label", ""))]
	place.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	place.custom_minimum_size = Vector2(720, 0)
	UiType.apply(place, UiType.CAPTION)
	place.modulate = Color(1, 1, 1, 0.8)
	box.add_child(place)
	var summary := Label.new()
	summary.text = str(entry.get("summary", ""))
	summary.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	summary.custom_minimum_size = Vector2(720, 0)
	UiType.apply(summary, UiType.BODY)
	box.add_child(summary)
	var armies := Label.new()
	armies.text = "%s\n%s" % [_army_line(entry, "attacker"), _army_line(entry, "defender")]
	armies.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	armies.custom_minimum_size = Vector2(720, 0)
	UiType.apply(armies, UiType.CAPTION)
	armies.modulate = Color(1, 1, 1, 0.8)
	box.add_child(armies)
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 10)
	box.add_child(row)
	for side in ["defender", "attacker", ""]:
		var button := Button.new()
		button.name = "Battle_%s_%s" % [id, side if side != "" else "watch"]
		if side == "":
			button.text = "Regarder"
			RichTooltip.attach_plain(button, "historical_battle_ai_both")
		else:
			var army: Dictionary = entry.get(side, {})
			button.text = "Mener %s" % _the_army(str(army.get("faction_name", side)))
			button.tooltip_text = str(army.get("army", ""))
		button.pressed.connect(start.bind(id, side))
		row.add_child(button)
		buttons["%s:%s" % [id, side]] = button


static func _army_line(entry: Dictionary, side: String) -> String:
	var army: Dictionary = entry.get(side, {})
	var role := "attaque" if side == "attacker" else "défend"
	var general := str(army.get("general", ""))
	var line := "%s (%s) — %s, environ %d hommes" % [str(army.get("army", army.get("faction_name", ""))), role, general, int(army.get("soldiers", 0))]
	var waves: Array = entry.get(side + "_waves", [])
	if waves.size() > 1:
		line += " en %d batailles successives" % waves.size()
	return line


## « les Anglais », « les Français ».
static func _the_army(faction_name: String) -> String:
	match faction_name:
		"Angleterre":
			return "les Anglais"
		"France":
			return "les Français"
		_:
			return "l'armée de %s" % faction_name


func _unhandled_input(event: InputEvent) -> void:
	if visible and event.is_action_pressed("ui_cancel"):
		close()
		get_viewport().set_input_as_handled()


func close() -> void:
	closed.emit()
	UiMotion.fade_out(self, UiMotion.DURATION, true)


## Lance la bataille `id`, `side` menée par le joueur ("" : IA contre IA).
func start(id: String, side: String) -> void:
	BattleScene.demo_args = args_for(id, side)
	battle_started.emit(id, side)
	var audio := get_node_or_null("/root/AudioDirector")
	if audio != null and audio.has_method("stop_all"):
		audio.call("stop_all")
	SceneFader.go(BATTLE_SCENE)
