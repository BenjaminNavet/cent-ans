class_name BattleFormationMenu
extends PanelContainer
## RJ-a (ADR 0174) : menu des formations de régiment, ouvert par le bouton « Formation » de la
## barre des ordres. Une ligne par formation de `data/rules/unit_formations.json` (catalogue
## `BattleSim.unit_formations()`, ordre des données) : nom historique et libellé court ; grisée
## si aucun régiment choisi ne peut la prendre (`formations` de `get_units`, verdict du cœur),
## dorée si c'est la formation de toute la sélection. Infobulle riche (`ib:formation:<clé>`) :
## contexte historique et effets chiffrés lus dans les données. Aucune règle ici : le choix part
## en ordre `formation` par `chosen`, le cœur décide et fait marcher les hommes (reformation).

signal chosen(key: String)

const INK := Color(0.22, 0.14, 0.07)
const ROW_SIZE := Vector2(190, 26)

var catalog: Array = []
## Multiplicateurs pendant la reformation (`formation_reform_rules`), cités dans l'infobulle.
var reform_rules: Dictionary = {}
var _rows: Dictionary = {}  # clé -> RichButton
var _state: Dictionary = {}


func setup(formations: Array, reform: Dictionary) -> void:
	catalog = formations
	reform_rules = reform
	name = "FormationMenu"
	visible = false
	mouse_filter = Control.MOUSE_FILTER_STOP
	add_theme_stylebox_override("panel", BattleUiKit.page_box(6))
	var column := VBoxContainer.new()
	column.add_theme_constant_override("separation", 2)
	add_child(column)
	var title := Label.new()
	title.text = "Formation"
	title.add_theme_color_override("font_color", INK)
	title.add_theme_font_size_override("font_size", UiType.size(UiType.CAPTION))
	column.add_child(title)
	for entry in catalog:
		var key := str(entry["key"])
		var row := RichButton.new()
		row.name = "Formation_%s" % key
		row.custom_minimum_size = ROW_SIZE
		row.focus_mode = Control.FOCUS_NONE
		row.alignment = HORIZONTAL_ALIGNMENT_LEFT
		row.text = str(entry["name"])
		row.add_theme_font_size_override("font_size", UiType.size(UiType.CAPTION))
		RichTooltip.set_tooltip(row, "formation", key, tooltip_live(entry, reform_rules))
		row.draw.connect(_draw_row.bind(row, key))
		row.pressed.connect(func() -> void: chosen.emit(key))
		column.add_child(row)
		_rows[key] = row


## Données de l'infobulle d'une formation : l'entrée du catalogue et les règles de reformation.
static func tooltip_live(entry: Dictionary, reform: Dictionary) -> Dictionary:
	var live := entry.duplicate(true)
	live["reform"] = reform
	return live


## {clé: {able, current}} pour les régiments `selected` (fonction pure, testée) : `able` si l'un
## d'eux peut la prendre, `current` si tous ceux qui le peuvent l'ont déjà.
static func menu_state(formations: Array, units: Array, selected: Array) -> Dictionary:
	var out := {}
	for entry in formations:
		var key := str(entry["key"])
		var able := 0
		var current := 0
		for unit in units:
			if not selected.has(int(unit["id"])) or not bool(unit.get("present", true)):
				continue
			if not Array(unit.get("formations", [])).has(key):
				continue
			able += 1
			if str(unit.get("formation", "")) == key:
				current += 1
		out[key] = {"able": able > 0, "current": able > 0 and current == able}
	return out


func refresh(units: Array, selected: Array) -> void:
	var state := menu_state(catalog, units, selected)
	if state == _state:
		return
	_state = state
	for key in _rows:
		var row: Button = _rows[key]
		row.disabled = not bool(state.get(key, {}).get("able", false))
		row.queue_redraw()


## Ouvre le menu au-dessus du contrôle `anchor` (le bouton « Formation »), ou le ferme.
func toggle_at(anchor: Control) -> void:
	visible = not visible
	if visible and anchor != null:
		reset_size()
		var top_left := anchor.get_global_rect().position - Vector2(0, size.y + 6)
		var view := get_viewport_rect().size
		top_left.x = clampf(top_left.x, 4.0, view.x - size.x - 4.0)
		top_left.y = maxf(top_left.y, 4.0)
		global_position = top_left


func _draw_row(row: Button, key: String) -> void:
	if bool(_state.get(key, {}).get("current", false)):
		row.draw_rect(Rect2(Vector2(2, 2), row.size - Vector2(4, 4)), Color(HudStyle.GOLD, 0.45))


## Lignes d'effet signées d'une formation (`modifiers` du catalogue) : +1 favorable, -1 défavorable.
static func effect_lines(entry: Dictionary) -> Array:
	var m: Dictionary = entry.get("modifiers", {})
	var lines: Array = []
	# [champ, libellé, plus haut = mieux]
	const FACTORS := [
		["speed", "Allure", true],
		["charge", "Charge", true],
		["shooting", "Pertes infligées au tir", true],
		["push_drive", "Poussée en mêlée", true],
		["push_resistance", "Tenue sous la poussée", true],
		["melee_taken", "Dégâts subis en mêlée", false],
		["missile_taken", "Pertes sous les traits", false],
		["morale_loss", "Moral perdu avec les pertes", false],
		["horse_blows", "Coups des cavaliers contre elle", false],
	]
	for factor in FACTORS:
		var value := float(m.get(factor[0], 1.0))
		if is_equal_approx(value, 1.0):
			continue
		var percent := roundi((value - 1.0) * 100.0)
		var better := (value > 1.0) == bool(factor[2])
		lines.append({"text": "%s %s%d %%" % [factor[1], "+" if percent > 0 else "", percent], "sign": 1 if better else -1})
	var ranks := float(m.get("fighting_ranks", 2.0))
	if not is_equal_approx(ranks, 2.0):
		lines.append({"text": "%d hommes combattent par file du front (2 d'ordinaire)" % roundi(ranks), "sign": 1 if ranks > 2.0 else -1})
	if bool(entry.get("all_round", false)):
		lines.append({"text": "Ni flanc ni dos : tout coup porte de face", "sign": 1})
	if bool(entry.get("braced", false)):
		lines.append({"text": "Hérissée : la charge ne renverse personne ni n'ébranle le moral", "sign": 1})
	return lines


## Libellé des troupes permises (« gens de pied, tireurs »).
static func troops_label(entry: Dictionary) -> String:
	const NAMES := {"infantry": "gens de pied", "ranged": "tireurs", "cavalry": "cavaliers", "siege": "engins"}
	var names := PackedStringArray()
	for category in entry.get("categories", []):
		names.append(str(NAMES.get(str(category), category)))
	var text := ", ".join(names)
	if bool(entry.get("foot_only", false)):
		text += " (à pied)"
	elif bool(entry.get("mounted_only", false)):
		text += " (montés)"
	return text


## IB : spec en sections de l'infobulle d'une formation (`RichTooltip.spec_for`, type
## « formation ») : nom historique, troupes permises, effets chiffrés, contexte, reformation.
static func tooltip_spec(key: String, live: Dictionary) -> Dictionary:
	var spec := {
		"id": key, "kind": "formation", "headline_kind": "formation",
		"title": str(live.get("name", key)), "subtitle": "Formation · " + troops_label(live),
		"icon": "", "icon_category": "", "headline": [], "stats": [], "effects": [],
		"traits": {"strengths": [], "weaknesses": [], "abilities": []}, "requires": [],
		"warnings": [], "flavour": "", "footer": {}, "detail": [],
	}
	# Le contexte historique d'abord, visible dès le survol (le bloc `flavour` n'apparaît que
	# dans la version verrouillée).
	var description := str(live.get("description", ""))
	if description != "":
		spec["effects"].append({"key": "", "text": "[i]%s[/i]" % description, "sign": 0})
	for line in effect_lines(live):
		spec["effects"].append({"key": "", "text": str(line["text"]), "sign": int(line["sign"])})
	if effect_lines(live).is_empty():
		spec["effects"].append({"key": "", "text": "Aucun effet particulier : l'ordre de référence", "sign": 0})
	var reform: Dictionary = live.get("reform", {})
	var seconds := float(live.get("reform_s", 0.0))
	if seconds > 0.0:
		var text := "Reformation : environ %d s pour cent hommes (plus long pour un grand régiment, plus court pour des vétérans)" % roundi(seconds)
		if not reform.is_empty():
			text += " ; pendant ce temps, allure ×%s et dégâts subis en mêlée ×%s" % [_factor(float(reform.get("speed", 1.0))), _factor(float(reform.get("melee_taken", 1.0)))]
		spec["effects"].append({"key": "", "text": text, "sign": 0})
	return spec


static func _factor(value: float) -> String:
	return String.num(value, 2).replace(".", ",")
