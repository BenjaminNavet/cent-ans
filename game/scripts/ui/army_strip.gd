class_name ArmyStrip
extends PanelContainer

## Bandeau d'ost (HUD de campagne, bas au centre ; lot F10a) : une carte par régiment de
## l'armée sélectionnée — icône de classe, nom court, barre d'effectif, moral en liseré —,
## compteur de « lances » (`8/20`), effectif total et entretien de l'armée.
##
## Aucune règle de jeu ni accès à la simulation : `set_army` reçoit le Dictionary de
## `CampaignSim.get_army(id)` tel quel (`{faction, general, general_name, location, units[],
## movement_points, supply, stance, path[]}`, chaque unité `{unit_type, name, strength,
## max_strength, morale}`) et un catalogue facultatif des types d'unité
## (`ArmyStrip.load_unit_catalog(data_dir)`) pour la classe, l'entretien et le moral de base.
##
## Interaction : clic = sélection d'un régiment (`unit_selected`) ; Maj/Ctrl-clic = ajoute ou
## retire de la sélection ; « Séparer » émet `split_requested` avec les indices choisis.
## Survol d'une carte = infobulle détaillée (effectif, moral, entretien).

## Clic sur une carte (index dans `army.units`), avec ou sans modificateur.
signal unit_selected(index: int)
## Sélection courante après chaque clic (indices croissants).
signal selection_changed(indices: PackedInt32Array)
## Bouton « Séparer » : les régiments choisis formeraient une nouvelle armée.
signal split_requested(indices: PackedInt32Array)
## Bouton « Garnison » (lot C7d) : les régiments choisis rejoindraient la garnison de la
## colonie où l'armée se trouve (ordre `garrison_units`).
signal garrison_requested(indices: PackedInt32Array)
## TW2-T3 : bouton « Mercenaires » (compagnies engageables dans la région de l'armée).
signal mercenaries_requested

const CARD_WIDTH := 84.0
const CARD_MIN_WIDTH := 52.0
## Largeur minimale pour afficher le nom sous l'icône.
const CARD_NAMED_MIN_WIDTH := 70.0
const CARD_HEIGHT := 122.0
const CARD_COMPACT_HEIGHT := 84.0
const CARD_GAP := 4
const HEADER_WIDTH := 132.0
## Moral de référence quand le catalogue ne fournit pas le moral de base du type.
const DEFAULT_BASE_MORALE := 100.0
const CLASS_LABELS := {
	"cavalry": "Cavalerie", "infantry": "Infanterie", "ranged": "Tireurs", "siege": "Siège"}

## Largeur maximale du bandeau (px) : au-delà, les cartes rétrécissent puis passent sur deux rangs.
@export var max_width: float = 880.0:
	set(value):
		if is_equal_approx(value, max_width):
			return
		max_width = value
		_refresh()
## Faux pour une armée étrangère : pas de multisélection ni de bouton « Séparer ».
@export var can_split: bool = true
## Lot C7d : vrai quand l'armée est sur une colonie que le joueur contrôle (bouton
## « Garnison » affiché, désactivé si `garrison_disabled_reason` n'est pas vide).
@export var can_garrison: bool = false
## Raison en français si le bouton « Garnison » doit rester désactivé (colonie assiégée ou
## pleine) ; vide = bouton actif dès qu'au moins un régiment est choisi.
@export var garrison_disabled_reason: String = ""
## TW2-T3 : vrai pour une armée du joueur (bouton « Mercenaires » affiché ; le panneau dit
## pourquoi rien n'est engageable, le cas échéant).
@export var can_hire_mercenaries: bool = false

var army: Dictionary = {}
var capacity: int = 20
var unit_catalog: Dictionary = {}

var _selection: Array[int] = []
var _cards: Array[RegimentCard] = []
var _count_label: Label
var _men_label: Label
var _upkeep_label: Label
var _title_label: Label
var _split_button: Button
var _garrison_button: Button
var _mercenary_button: Button
var _grid: GridContainer


func _ready() -> void:
	add_theme_stylebox_override("panel", HudStyle.panel_box(10))
	mouse_filter = Control.MOUSE_FILTER_STOP
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 10)
	add_child(row)

	var header := VBoxContainer.new()
	header.custom_minimum_size = Vector2(HEADER_WIDTH, 0)
	header.add_theme_constant_override("separation", 2)
	row.add_child(header)
	_title_label = HudStyle.label("", UiType.size(UiType.CAPTION), HudStyle.RUBRIC)
	_title_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_title_label.custom_minimum_size = Vector2(HEADER_WIDTH, 0)
	header.add_child(_title_label)
	_count_label = HudStyle.label("", UiType.size(UiType.HEADING), HudStyle.INK)
	_count_label.set_script(RichLabel)  # B1 : infobulle riche auto-liée (T : bulle du Codex)
	_count_label.tooltip_text = "Régiments sous contrat d'endenture / capacité du chef"
	_count_label.mouse_filter = Control.MOUSE_FILTER_PASS
	header.add_child(_count_label)
	_men_label = HudStyle.label("", UiType.size(UiType.CAPTION), HudStyle.INK_SOFT)
	header.add_child(_men_label)
	_upkeep_label = HudStyle.label("", UiType.size(UiType.CAPTION), HudStyle.INK_SOFT)
	_upkeep_label.set_script(RichLabel)
	_upkeep_label.mouse_filter = Control.MOUSE_FILTER_PASS
	header.add_child(_upkeep_label)
	var spacer := Control.new()
	spacer.size_flags_vertical = Control.SIZE_EXPAND_FILL
	header.add_child(spacer)
	_split_button = RichButton.new()
	_split_button.text = "Séparer"
	UiType.apply(_split_button, UiType.CAPTION)
	_split_button.tooltip_text = "Maj ou Ctrl + clic pour choisir les régiments à détacher"
	_split_button.pressed.connect(_on_split_pressed)
	header.add_child(_split_button)
	_garrison_button = RichButton.new()
	_garrison_button.text = "Garnison"
	UiType.apply(_garrison_button, UiType.CAPTION)
	_garrison_button.pressed.connect(_on_garrison_pressed)
	header.add_child(_garrison_button)
	_mercenary_button = RichButton.new()
	_mercenary_button.name = "MercenaryButton"
	_mercenary_button.text = "Mercenaires"
	UiType.apply(_mercenary_button, UiType.CAPTION)
	_mercenary_button.tooltip_text = "Compagnies à engager dans la région où se trouve l'armée"
	_mercenary_button.pressed.connect(func() -> void: mercenaries_requested.emit())
	header.add_child(_mercenary_button)

	var rule := ColorRect.new()
	rule.color = HudStyle.GOLD
	rule.custom_minimum_size = Vector2(1, 0)
	rule.mouse_filter = Control.MOUSE_FILTER_IGNORE
	row.add_child(rule)

	_grid = GridContainer.new()
	_grid.add_theme_constant_override("h_separation", CARD_GAP)
	_grid.add_theme_constant_override("v_separation", CARD_GAP)
	row.add_child(_grid)
	_refresh()


## Affiche l'armée. `capacity` = nombre maximal de régiments (lié au rang du chef, fourni par
## l'appelant). `catalog` : `{unit_type: {name, category, upkeep, morale, soldiers}}`, voir
## `load_unit_catalog`. `title` : rubrique au-dessus du compteur (« Ost de Philippe VI »).
func set_army(new_army: Dictionary, new_capacity: int = 20, catalog: Dictionary = {}, title: String = "") -> void:
	army = new_army.duplicate(true)
	capacity = maxi(new_capacity, 1)
	if not catalog.is_empty():
		unit_catalog = catalog
	if title != "":
		army["_title"] = title
	_selection.clear()
	_refresh()


## Vide le bandeau (aucune armée sélectionnée).
func clear() -> void:
	set_army({}, capacity)


## Indices sélectionnés, croissants.
func get_selection() -> PackedInt32Array:
	var sorted := _selection.duplicate()
	sorted.sort()
	return PackedInt32Array(sorted)


## Remplace la sélection (indices hors bornes ignorés) ; émet `selection_changed`.
func select(indices: Array) -> void:
	_selection.clear()
	for index in indices:
		if int(index) >= 0 and int(index) < _units().size() and not _selection.has(int(index)):
			_selection.append(int(index))
	_update_selection()


## Entretien total de l'armée (₶ par saison) : champ `upkeep` de l'armée s'il existe, sinon
## somme des `upkeep` des unités ou du catalogue.
func total_upkeep() -> int:
	if army.has("upkeep"):
		return int(army["upkeep"])
	var total := 0
	for unit in _units():
		total += unit_upkeep(unit)
	return total


## Entretien d'un régiment (₶) : `unit.upkeep` sinon catalogue, 0 si inconnu.
func unit_upkeep(unit: Dictionary) -> int:
	if unit.has("upkeep"):
		return int(unit["upkeep"])
	return int(_type_info(unit).get("upkeep", 0))


## Classe (`cavalry`, `infantry`, `ranged`, `siege`) : `unit.category` sinon catalogue.
func unit_class(unit: Dictionary) -> String:
	return str(unit.get("category", _type_info(unit).get("category", "")))


## Nom affiché : `unit.name`, sinon nom du catalogue, sinon id lisible.
func unit_name(unit: Dictionary) -> String:
	var name := str(unit.get("name", ""))
	if name == "":
		name = str(_type_info(unit).get("name", ""))
	if name == "":
		name = str(unit.get("unit_type", "?")).trim_prefix("unit_").replace("_", " ").capitalize()
	return name


## Moral rapporté au moral de base du type (0..1).
func morale_ratio(unit: Dictionary) -> float:
	var base := float(_type_info(unit).get("morale", DEFAULT_BASE_MORALE))
	if base <= 0.0:
		base = DEFAULT_BASE_MORALE
	return clampf(float(unit.get("morale", 0)) / base, 0.0, 1.0)


## Catalogue des types d'unité lu dans `<data_dir>/unit_types/*.json` :
## `{unit_type: {name, category, upkeep, morale, soldiers}}`. Vide si le dossier manque.
static func load_unit_catalog(data_dir: String) -> Dictionary:
	var catalog := {}
	var dir_path := data_dir.path_join("unit_types")
	var dir := DirAccess.open(dir_path)
	if dir == null:
		return catalog
	for file_name in dir.get_files():
		if not file_name.ends_with(".json"):
			continue
		var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(dir_path.path_join(file_name)))
		if not parsed is Dictionary:
			continue
		var entry: Dictionary = parsed
		var name_field: Variant = entry.get("name", "")
		var display := str(name_field.get("display", "")) if name_field is Dictionary else str(name_field)
		catalog[str(entry.get("id", file_name.get_basename()))] = {
			"name": display,
			"category": str(entry.get("category", "")),
			"upkeep": int(entry.get("upkeep", 0)),
			"morale": int((entry.get("stats", {}) as Dictionary).get("morale", DEFAULT_BASE_MORALE)),
			"soldiers": int(entry.get("soldiers", 0)),
		}
	return catalog


# --- Interne -------------------------------------------------------------------


func _units() -> Array:
	return army.get("units", [])


func _type_info(unit: Dictionary) -> Dictionary:
	return unit_catalog.get(str(unit.get("unit_type", "")), {})


func _refresh() -> void:
	if _grid == null:
		return
	for card in _cards:
		card.queue_free()
	_cards.clear()
	var units := _units()
	visible = not army.is_empty()
	var men := 0
	for unit in units:
		men += int(unit.get("strength", 0))
	_title_label.text = str(army.get("_title", "")).to_upper()
	_title_label.visible = _title_label.text != ""
	_count_label.text = "%d/%d" % [units.size(), capacity]
	_count_label.add_theme_color_override("font_color", HudStyle.RUBRIC if units.size() > capacity else HudStyle.INK)
	_men_label.text = "%s hommes" % HudStyle.thousands(men)
	var upkeep := total_upkeep()
	_upkeep_label.text = "Entretien %s ₶" % HudStyle.thousands(upkeep)
	_upkeep_label.tooltip_text = "Entretien de l'armée par saison : %s (livres tournois)" % Money.amount(upkeep)

	var layout := card_layout(units.size())
	_grid.columns = int(layout["columns"])
	for index in units.size():
		var card := RegimentCard.new()
		card.strip = self
		card.index = index
		card.unit = units[index]
		card.show_name = bool(layout["show_name"])
		card.custom_minimum_size = Vector2(float(layout["width"]), float(layout["height"]))
		_grid.add_child(card)
		_cards.append(card)
	_update_selection(false)


## Disposition des cartes pour `count` régiments dans `max_width` :
## `{columns, rows, width, height, show_name}`. Un rang de cartes pleines (84 px, avec nom)
## tant qu'elles tiennent ; sinon cartes compactes réparties sur le moins de rangs possible
## (52 à 84 px, sans nom sous 70 px — le nom reste dans l'infobulle).
func card_layout(count: int) -> Dictionary:
	var available := maxf(max_width - HEADER_WIDTH - 57.0, CARD_MIN_WIDTH)
	if count <= 0:
		return {"columns": 1, "rows": 0, "width": CARD_WIDTH, "height": CARD_HEIGHT, "show_name": true}
	var per_row_full := int(floor((available + CARD_GAP) / (CARD_WIDTH + CARD_GAP)))
	if count <= per_row_full:
		return {"columns": count, "rows": 1, "width": CARD_WIDTH, "height": CARD_HEIGHT, "show_name": true}
	var fit_one_row := (available + CARD_GAP) / float(count) - CARD_GAP
	if fit_one_row >= CARD_NAMED_MIN_WIDTH:
		return {"columns": count, "rows": 1, "width": minf(fit_one_row, CARD_WIDTH), "height": CARD_HEIGHT, "show_name": true}
	var rows := 1
	var width := fit_one_row
	while width < CARD_MIN_WIDTH and rows < count:
		rows += 1
		var per_row := ceili(float(count) / float(rows))
		width = (available + CARD_GAP) / float(per_row) - CARD_GAP
	var columns := ceili(float(count) / float(rows))
	width = clampf(width, CARD_MIN_WIDTH, CARD_WIDTH)
	var named := width >= CARD_NAMED_MIN_WIDTH
	return {"columns": columns, "rows": rows, "width": width,
		"height": CARD_HEIGHT if named else CARD_COMPACT_HEIGHT, "show_name": named}


func _on_card_clicked(index: int, additive: bool) -> void:
	if additive and can_split:
		if _selection.has(index):
			_selection.erase(index)
		else:
			_selection.append(index)
	else:
		_selection = [index]
	unit_selected.emit(index)
	_update_selection()


func _update_selection(emit := true) -> void:
	for card in _cards:
		card.selected = _selection.has(card.index)
		card.queue_redraw()
	var count := _selection.size()
	_split_button.visible = can_split
	_split_button.disabled = count == 0 or count >= _units().size()
	if count > 0 and count < _units().size():
		_split_button.text = "Séparer (%d)" % count
	else:
		_split_button.text = "Séparer"
	_garrison_button.visible = can_garrison
	_mercenary_button.visible = can_hire_mercenaries
	_garrison_button.disabled = count == 0 or garrison_disabled_reason != ""
	_garrison_button.text = "Garnison (%d)" % count if count > 0 else "Garnison"
	_garrison_button.tooltip_text = garrison_disabled_reason if garrison_disabled_reason != "" \
		else "Maj ou Ctrl + clic pour choisir les régiments à laisser en garnison"
	if emit:
		selection_changed.emit(get_selection())


func _on_split_pressed() -> void:
	var indices := get_selection()
	if indices.is_empty() or indices.size() >= _units().size():
		return
	split_requested.emit(indices)


func _on_garrison_pressed() -> void:
	var indices := get_selection()
	if indices.is_empty() or not can_garrison or garrison_disabled_reason != "":
		return
	garrison_requested.emit(indices)


## Texte d'infobulle d'un régiment (aussi utilisé comme repli texte et par les tests).
func tooltip_for(unit: Dictionary) -> String:
	var lines := PackedStringArray()
	lines.append(unit_name(unit))
	var klass := unit_class(unit)
	if CLASS_LABELS.has(klass):
		lines.append(str(CLASS_LABELS[klass]))
	lines.append("Effectif : %d / %d" % [int(unit.get("strength", 0)), int(unit.get("max_strength", 0))])
	var base := int(_type_info(unit).get("morale", 0))
	if base > 0:
		lines.append("Moral : %d (base %d)" % [int(unit.get("morale", 0)), base])
	else:
		lines.append("Moral : %d" % int(unit.get("morale", 0)))
	var upkeep := unit_upkeep(unit)
	lines.append("Entretien : %s ₶ par saison" % HudStyle.thousands(upkeep) if upkeep > 0 else "Entretien : —")
	return "\n".join(lines)


## Nom adapté à une carte de `width` px, à la taille `Caption` (bible DA § 12.2 : rien en
## dessous) ; les mots trop longs sont abrégés proprement (« Arbalét. »). Renvoie `{text, font_size}`.
static func fit_name(name: String, font: Font, width: float) -> Dictionary:
	var words := name.split(" ", false)
	var font_size := UiType.size(UiType.CAPTION)
	var fits := true
	for word in words:
		if font.get_string_size(word, HORIZONTAL_ALIGNMENT_LEFT, -1, font_size).x > width:
			fits = false
			break
	if fits:
		return {"text": name, "font_size": font_size}
	var parts := PackedStringArray()
	for word in words:
		parts.append(abbreviate(word, font, width, font_size))
	return {"text": " ".join(parts), "font_size": font_size}


## Abrège `word` à la plus longue racine qui tient (au moins 3 lettres) suivie d'un point,
## en coupant de préférence après une consonne (usage des abréviations de chancellerie).
static func abbreviate(word: String, font: Font, width: float, font_size: int) -> String:
	if font.get_string_size(word, HORIZONTAL_ALIGNMENT_LEFT, -1, font_size).x <= width:
		return word
	var vowels := "aeiouyàâéèêëîïôûüAEIOUY"
	var best := word.substr(0, 3) + "."
	for length in range(word.length() - 1, 2, -1):
		var candidate := word.substr(0, length) + "."
		if font.get_string_size(candidate, HORIZONTAL_ALIGNMENT_LEFT, -1, font_size).x > width:
			continue
		if not vowels.contains(word[length - 1]):
			return candidate
		best = candidate if best.length() < candidate.length() else best
	return best


## Carte de régiment : dessin au trait + étiquette de nom (retour à la ligne aux mots).
class RegimentCard:
	extends Control

	var strip: ArmyStrip
	var index: int = 0
	var unit: Dictionary = {}
	var selected := false
	var show_name := true
	var _hover := false
	var _name_label: Label

	func _ready() -> void:
		mouse_filter = Control.MOUSE_FILTER_STOP
		mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND
		tooltip_text = strip.tooltip_for(unit)
		var fitted := ArmyStrip.fit_name(strip.unit_name(unit), get_theme_default_font(), custom_minimum_size.x - 10.0)
		_name_label = HudStyle.label(str(fitted["text"]), int(fitted["font_size"]))
		_name_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		_name_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
		# Retour à la ligne aux espaces seulement : jamais de coupure au milieu d'un mot.
		_name_label.autowrap_mode = TextServer.AUTOWRAP_WORD
		_name_label.text_overrun_behavior = TextServer.OVERRUN_TRIM_WORD_ELLIPSIS
		_name_label.max_lines_visible = 3
		_name_label.add_theme_constant_override("line_spacing", -2)
		_name_label.visible = show_name
		add_child(_name_label)
		mouse_entered.connect(func() -> void:
			_hover = true
			queue_redraw())
		mouse_exited.connect(func() -> void:
			_hover = false
			queue_redraw())
		resized.connect(_layout)
		_layout()

	func _layout() -> void:
		_name_label.position = Vector2(7, 46)
		_name_label.size = Vector2(size.x - 10, 44)

	func _gui_input(event: InputEvent) -> void:
		var click := event as InputEventMouseButton
		if click != null and click.pressed and click.button_index == MOUSE_BUTTON_LEFT:
			strip._on_card_clicked(index, click.shift_pressed or click.ctrl_pressed or click.meta_pressed)
			accept_event()

	## B1 : infobulle riche (parchemin, auto-liens, T : bulle du Codex) ; le nom mène à la
	## fiche du Codex du type d'unité s'il y en a une.
	func _make_custom_tooltip(for_text: String) -> Object:
		var lines := for_text.split("\n")
		lines[0] = "[b]%s[/b]" % RichTooltip.entity_name(str(unit.get("unit_type", "")), lines[0])
		return RichTooltip.make_panel("\n".join(lines))

	func _draw() -> void:
		var rect := Rect2(Vector2.ZERO, size)
		var bg := HudStyle.PARCHMENT_LIGHT if not _hover else HudStyle.PARCHMENT_LIGHT.lightened(0.25)
		draw_rect(rect, bg)
		# Liseré de moral : bande verticale à gauche, hauteur et teinte selon le moral.
		var ratio := strip.morale_ratio(unit)
		draw_rect(Rect2(0, 0, 4, size.y), HudStyle.PARCHMENT_DARK)
		draw_rect(Rect2(0, size.y * (1.0 - ratio), 4, size.y * ratio), HudStyle.gauge_color(ratio))
		if Accessibility.colorblind():  # U12 : moral lisible sans les couleurs (▲ bon, ■ moyen, ▼ bas)
			draw_string(get_theme_default_font(), Vector2(7, 14), Accessibility.level_symbol(ratio), HORIZONTAL_ALIGNMENT_LEFT, -1, 12, HudStyle.INK)
		# Icône de classe.
		var icon_center := Vector2(size.x * 0.5 + 2, 25)
		var texture := HudStyle.icon(str(unit.get("unit_type", "")), "unit")
		if texture == null:
			texture = HudStyle.icon("class_" + strip.unit_class(unit), "class")
		if texture != null:
			HudStyle.draw_texture_fit(self, texture, icon_center, 34)
		else:
			draw_circle(icon_center, 18, HudStyle.PARCHMENT)
			draw_arc(icon_center, 18, 0, TAU, 32, HudStyle.GOLD, 1.0, true)
			HudStyle.draw_glyph(self, strip.unit_class(unit), icon_center, 22, HudStyle.INK)
		# Effectif : chiffres + barre.
		var strength := int(unit.get("strength", 0))
		var max_strength := maxi(int(unit.get("max_strength", 0)), 1)
		var font := get_theme_default_font()
		var text := "%d/%d" % [strength, max_strength]
		var text_size := font.get_string_size(text, HORIZONTAL_ALIGNMENT_LEFT, -1, 11)
		draw_string(font, Vector2((size.x - text_size.x) * 0.5 + 2, size.y - 16), text, HORIZONTAL_ALIGNMENT_LEFT, -1, 11, HudStyle.INK_SOFT)
		var bar := Rect2(9, size.y - 11, size.x - 14, 5)
		var fill := clampf(float(strength) / float(max_strength), 0.0, 1.0)
		draw_rect(bar, HudStyle.PARCHMENT_DARK)
		draw_rect(Rect2(bar.position, Vector2(bar.size.x * fill, bar.size.y)), HudStyle.gauge_color(fill))
		draw_rect(bar, HudStyle.INK_SOFT, false, 1.0)
		# Cadre : encre, ou rubrique + filet d'or si sélectionné.
		if selected:
			draw_rect(rect.grow(-1), HudStyle.RUBRIC, false, 2.0)
			draw_rect(rect.grow(-4), HudStyle.GOLD, false, 1.0)
		else:
			draw_rect(rect, HudStyle.INK_SOFT, false, 1.0)


## TW2-T5 : bouton d'un contrôleur (« Traditions ») ajouté sous « Séparer » et « Garnison ».
func add_header_button(button: Control) -> void:
	_garrison_button.get_parent().add_child(button)
