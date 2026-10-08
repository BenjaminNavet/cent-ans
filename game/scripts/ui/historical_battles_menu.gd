class_name HistoricalBattlesMenu
extends ListMenu

## EP7 — « Batailles historiques » du menu principal : Crécy (1346), Poitiers (1356), Azincourt
## (1415), lues dans `data/battle_maps/` par le cœur (`BattleSim.list_historical`). Pour chaque
## bataille : date, lieu, résumé, les deux armées ; on mène l'un des deux camps ou on regarde
## l'IA contre l'IA. La scène de bataille reçoit `--historical=<id>` et `--historical-side=` ;
## la fin de la bataille ramène au menu (hors campagne, rien n'est conservé). Échap ou
## « Fermer » : signal `closed`.
## NT4 : le « Didacticiel de bataille » (bataille-prologue guidée, `BattlePrologue`) ouvre la liste.

signal battle_started(id: String, side: String)

var battles: Array = []
var prologue_button: Button = null  # NT4 : « Commencer le didacticiel »


## Batailles historiques déclarées (`[]` sans l'extension ou sans données).
static func load_battles() -> Array:
	if not ClassDB.class_exists("BattleSim"):
		return []
	var sim: Object = ClassDB.instantiate("BattleSim")
	if not sim.has_method("list_historical"):
		return []
	return sim.call("list_historical", DataFile.data_dir())


## Dossier `data/` (utilisé par les tests).
static func data_dir() -> String:
	return DataFile.data_dir()


## Options de la scène de bataille pour la bataille `id` menée par `side` ("" : IA contre IA).
static func args_for(id: String, side: String) -> PackedStringArray:
	var args := PackedStringArray(["--historical=" + id])
	if side != "":
		args.append("--historical-side=" + side)
	return args


func _menu_title() -> String:
	return "Batailles historiques"


func _menu_hint() -> String:
	return "Le champ réel, les armées de ce jour-là, la météo et l'heure : menez l'un des camps, ou regardez. Hors campagne : le résultat n'est pas conservé."


func _menu_width() -> float:
	return 780.0


func _scroll_height() -> float:
	return 560.0


func _load_entries() -> void:
	battles = load_battles()


func _build_entries(box: VBoxContainer) -> void:
	_add_prologue(box)
	if battles.is_empty():
		UiBuild.label("Aucune bataille historique n'a été trouvée.", 0, null, false, 0.0, box)
	for entry in battles:
		_add_battle(box, entry as Dictionary)


## NT4 : « Didacticiel de bataille » en tête de liste (bataille-prologue guidée), pour ne pas
## ajouter de ligne à la colonne du menu principal (tenue à 1280×720 par NT2).
func _add_prologue(box: VBoxContainer) -> void:
	var prologue := BattlePrologue.load_data()
	if prologue.is_empty():
		return
	var sep := HSeparator.new()
	box.add_child(sep)
	var name := UiBuild.label(str(prologue.get("menu_label", "Didacticiel de bataille")))
	UiType.apply(name, UiType.HEADING)
	box.add_child(name)
	var detail := UiBuild.label(str(prologue.get("menu_detail", "")), 0, null, true, 720)
	UiType.apply(detail, UiType.BODY)
	box.add_child(detail)
	prologue_button = UiBuild.button("Commencer le didacticiel", start_prologue)
	prologue_button.name = "BattlePrologue"
	prologue_button.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN
	box.add_child(prologue_button)


## NT4 : lance la bataille-prologue guidée.
func start_prologue() -> void:
	if BattlePrologue.launch(get_tree()):
		battle_started.emit("battle_prologue", str((BattlePrologue.load_data().get("battle", {}) as Dictionary).get("player_side", "")))


func _add_battle(box: VBoxContainer, entry: Dictionary) -> void:
	var id := str(entry.get("id", ""))
	var sep := HSeparator.new()
	box.add_child(sep)
	var name := UiBuild.label("%s — %s" % [str(entry.get("name", id)), str(entry.get("date_fr", entry.get("date", "")))])
	UiType.apply(name, UiType.HEADING)
	box.add_child(name)
	var place := UiBuild.label("%s (%s) · %s" % [str(entry.get("place", "")), str(entry.get("province_name", "")), str(entry.get("weather_label", ""))], 0, null, true, 720)
	UiType.apply(place, UiType.CAPTION)
	place.modulate = Color(1, 1, 1, 0.8)
	box.add_child(place)
	var summary := UiBuild.label(str(entry.get("summary", "")), 0, null, true, 720)
	UiType.apply(summary, UiType.BODY)
	box.add_child(summary)
	var armies := UiBuild.label("%s\n%s" % [_army_line(entry, "attacker"), _army_line(entry, "defender")], 0, null, true, 720)
	UiType.apply(armies, UiType.CAPTION)
	armies.modulate = Color(1, 1, 1, 0.8)
	box.add_child(armies)
	var row := UiBuild.hbox(10, box)
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
		button.pressed.connect(func() -> void: BattlePrologueInvite.gate(self, start.bind(id, side)))  # NT4
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


## Lance la bataille `id`, `side` menée par le joueur ("" : IA contre IA).
func start(id: String, side: String) -> void:
	battle_started.emit(id, side)
	_launch_battle(args_for(id, side))
