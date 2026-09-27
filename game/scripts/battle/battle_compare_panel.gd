class_name BattleComparePanel
extends PanelContainer

## Lot CB-M4 : comparaison face à face au survol d'un ennemi, façon Total War. Ouverte seulement
## quand UNE seule troupe du joueur est sélectionnée et qu'un ennemi est survolé (sur le terrain,
## par sa bannière ou par une carte). Les chiffres et les drapeaux d'avantage net viennent tels
## quels du dictionnaire `compare` de `hover_context` (cœur, `hover.rs`) : neuf lignes, en vert
## le côté qui a l'avantage net, en rouge l'autre, rien sinon. Pas de pronostic.
## Page de vélin du kit enluminé (UI1, `BattleUiKit.page_box`).

## Ordre d'affichage et libellés des lignes du cœur (`Compare.advantages`).
const LINES := [
	["soldiers", "Effectif", ""],
	["melee", "Mêlée", ""],
	["defense", "Défense", ""],
	["charge", "Charge", ""],
	["ranged", "Tir", ""],
	["range", "Portée", " m"],
	["morale", "Moral", ""],
	["fatigue", "Fatigue", ""],
	["bonus_vs", "Bonus contre", " %"],
]
const REFRESH_S := 0.25  # rafraîchissement des chiffres tant que le survol dure (interface)
const NONE := "—"

## Dernier dictionnaire `compare` affiché (vide : panneau fermé).
var compare: Dictionary = {}
## Ennemi comparé (-1 : aucun) et nombre d'appels au cœur (tests).
var enemy := -1
var core_calls := 0
## Textes affichés, par ligne : [nôtre, sienne, avantage] (tests).
var shown: Dictionary = {}

var _title_ours: Label
var _title_theirs: Label
var _cells: Dictionary = {}  # ligne -> [Label nôtre, Label sienne]
var _key := ""
var _last_call := -1.0


func _init() -> void:
	name = "ComparePanel"
	visible = false
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_theme_stylebox_override("panel", BattleUiKit.page_box(10))
	set_anchors_preset(Control.PRESET_BOTTOM_LEFT)
	offset_left = 12
	offset_bottom = -BattleHud.BAND_HEIGHT - 16
	grow_vertical = Control.GROW_DIRECTION_BEGIN
	var grid := GridContainer.new()
	grid.columns = 3
	grid.add_theme_constant_override("h_separation", 14)
	grid.add_theme_constant_override("v_separation", 1)
	grid.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(grid)
	_title_ours = BattleUiKit.label("", 15, BattleUiKit.INK, true)
	_title_theirs = BattleUiKit.label("", 15, BattleUiKit.RUBRIC, true)
	_title_ours.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	grid.add_child(_title_ours)
	grid.add_child(BattleUiKit.label("contre", 13, BattleUiKit.INK_FADED))
	grid.add_child(_title_theirs)
	for line in LINES:
		var ours := BattleUiKit.label("", 15, BattleUiKit.INK, false, true)
		ours.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
		var caption := BattleUiKit.label(str(line[1]), 14, BattleUiKit.INK_SOFT)
		caption.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		var theirs := BattleUiKit.label("", 15, BattleUiKit.INK, false, true)
		grid.add_child(ours)
		grid.add_child(caption)
		grid.add_child(theirs)
		_cells[line[0]] = [ours, theirs]


## Vrai si le panneau doit s'ouvrir : une seule troupe sélectionnée et un ennemi survolé.
static func wanted(selected_count: int, enemy_id: int) -> bool:
	return selected_count == 1 and enemy_id >= 0


## Premier régiment ennemi présent parmi les survolés (-1 : aucun).
static func hovered_enemy(units: Array, hovered: Array, player_side: String) -> int:
	for id in hovered:
		for unit in units:
			if int(unit["id"]) == int(id):
				if str(unit["side"]) != player_side and bool(unit.get("present", true)):
					return int(id)
				break
	return -1


## Couleur d'une valeur selon le drapeau d'avantage de sa ligne (`ours`, `theirs`, `even`).
static func value_color(advantage: String, is_ours: bool) -> Color:
	if advantage == "even":
		return BattleUiKit.INK
	var good := (advantage == "ours") == is_ours
	return BattleUiKit.GOOD if good else BattleUiKit.RUBRIC


## Texte d'une valeur : entier arrondi au format français, « — » pour une portée ou un tir nul.
static func value_text(line: String, value: float, suffix: String) -> String:
	if (line == "range" or line == "ranged") and value <= 0.0:
		return NONE
	return RuleValues.number(roundf(value)) + suffix


## À chaque image : choisit l'ennemi survolé et interroge le cœur au plus toutes les REFRESH_S s
## (ou dès que la paire change). `hovered` = troupes survolées (terrain, bannière, carte).
func refresh(battle: Object, units: Array, selected: Array, hovered: Array, player_side: String, now: float) -> void:
	var foe := hovered_enemy(units, hovered, player_side)
	if battle == null or not wanted(selected.size(), foe):
		close()
		return
	var key := "%d>%d" % [int(selected[0]), foe]
	if key == _key and now - _last_call < REFRESH_S:
		return
	_key = key
	_last_call = now
	var at := Vector2.ZERO
	for unit in units:
		if int(unit["id"]) == foe:
			at = Vector2(float(unit["x"]), float(unit["z"]))
			break
	core_calls += 1
	var hover: Dictionary = battle.call("hover_context", at.x, at.y, PackedInt32Array([int(selected[0])]))
	if int(hover.get("target", -1)) != foe or not hover.has("compare"):
		close()
		return
	show_compare(hover["compare"], _name_of(units, int(selected[0])), _name_of(units, foe))
	enemy = foe


## Affiche le dictionnaire `compare` du cœur ; vide : ferme.
func show_compare(data: Dictionary, our_name: String, their_name: String) -> void:
	if data.is_empty():
		close()
		return
	compare = data
	var ours: Dictionary = data.get("ours", {})
	var theirs: Dictionary = data.get("theirs", {})
	var advantages: Dictionary = data.get("advantages", {})
	_title_ours.text = our_name
	_title_theirs.text = their_name
	shown.clear()
	for line in LINES:
		var key: String = line[0]
		var advantage := str(advantages.get(key, "even"))
		var cells: Array = _cells[key]
		var ours_text := value_text(key, float(ours.get(key, 0.0)), str(line[2]))
		var theirs_text := value_text(key, float(theirs.get(key, 0.0)), str(line[2]))
		(cells[0] as Label).text = ours_text
		(cells[1] as Label).text = theirs_text
		(cells[0] as Label).add_theme_color_override("font_color", value_color(advantage, true))
		(cells[1] as Label).add_theme_color_override("font_color", value_color(advantage, false))
		shown[key] = [ours_text, theirs_text, advantage]
	visible = true


func close() -> void:
	visible = false
	compare = {}
	enemy = -1
	_key = ""


static func _name_of(units: Array, id: int) -> String:
	for unit in units:
		if int(unit["id"]) == id:
			return str(unit.get("name", ""))
	return ""
