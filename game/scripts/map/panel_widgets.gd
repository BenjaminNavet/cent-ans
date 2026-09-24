class_name PanelWidgets
extends RefCounted

## Lignes partagées des panneaux de province et de colonie (lot C5) : garnison, recrutement,
## bâtiments, chantier et constructions possibles. Rendu seulement : les listes, la
## disponibilité et les raisons de refus viennent de la simulation.

const ROW_ICON := 20.0
const REASON_COLOR := Color(0.55, 0.20, 0.15)


static func clear(container: Node) -> void:
	for child in container.get_children():
		container.remove_child(child)
		child.queue_free()


static func placeholder(container: Node, text: String) -> void:
	var label := Label.new()
	label.text = text
	container.add_child(label)


## Garnison : cases à cocher (formation d'armée) si `selectable`, sinon simples puces.
## Renvoie les cases dans l'ordre de la garnison.
static func fill_garrison(list: Container, garrison: Array, selectable: bool) -> Array[CheckBox]:
	clear(list)
	var checks: Array[CheckBox] = []
	for unit in garrison:
		var text := "%s — %d/%d, moral %d" % [
			unit_label(unit), int(unit.get("strength", 0)),
			int(unit.get("max_strength", 0)), int(unit.get("morale", 0))]
		var unit_type: String = str(unit.get("unit_type", ""))
		if selectable:
			var check := CheckBox.new()
			check.text = text
			check.button_pressed = true
			check.set_script(RichButton)
			check.theme_type_variation = &"CheckBox"
			IconLibrary.decorate_button(check, unit_type, int(ROW_ICON), "unit")
			check.tooltip_text = RichTooltip.unit(unit_type, unit)
			list.add_child(check)
			checks.append(check)
		else:
			list.add_child(IconChip.create(unit_type, text, RichTooltip.unit(unit_type, unit), ROW_ICON, 14, "unit"))
	return checks


## Options de recrutement (`get_recruitable`) ; `on_recruit(unit_type)` au clic.
static func fill_recruitable(list: Container, recruitable: Array, on_recruit: Callable) -> void:
	clear(list)
	for row in recruitable:
		var line := HBoxContainer.new()
		var button := RichButton.new()
		button.text = "%s — %s ℔ / %s ℔" % [str(row.get("name", row.get("unit_type", "?"))), thousands(int(row.get("cost", 0))), thousands(int(row.get("upkeep", 0)))]
		button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		button.alignment = HORIZONTAL_ALIGNMENT_LEFT
		var available: bool = bool(row.get("available", false))
		button.disabled = not available
		var unit_type: String = str(row.get("unit_type", ""))
		IconLibrary.decorate_button(button, unit_type, int(ROW_ICON), "unit")
		button.tooltip_text = RichTooltip.unit(unit_type, row)
		button.pressed.connect(func() -> void: on_recruit.call(unit_type))
		line.add_child(button)
		if not available:
			line.add_child(reason_label(str(row.get("reason", "Indisponible"))))
		list.add_child(line)


## Bâtiments construits (`{id, name, upkeep, …}`).
static func fill_buildings(list: Container, buildings: Array) -> void:
	clear(list)
	if buildings.is_empty():
		placeholder(list, "Aucun bâtiment.")
		return
	for entry in buildings:
		var building_id: String = str(entry.get("id", ""))
		var text := "%s (entretien %s ℔)" % [str(entry.get("name", building_id)), thousands(int(entry.get("upkeep", 0)))]
		list.add_child(IconChip.create(building_id, text, RichTooltip.building(building_id, entry), ROW_ICON, 14, "building"))


## Constructions possibles (`buildable`), sans celles déjà construites (`built_ids`) ;
## `on_build(building_id)` au clic. Rien si `is_player_owner` est faux.
static func fill_buildable(list: Container, buildable: Array, is_player_owner: bool, built_ids: Array, on_build: Callable) -> void:
	clear(list)
	if not is_player_owner:
		return
	var rows: Array = []
	for row in buildable:
		if not built_ids.has(str(row.get("building", ""))):
			rows.append(row)
	if rows.is_empty():
		placeholder(list, "—")
		return
	for row in rows:
		var line := HBoxContainer.new()
		line.add_theme_constant_override("separation", 8)
		var button := RichButton.new()
		button.text = "%s — %s ℔ / %d tour(s)" % [str(row.get("name", row.get("building", "?"))), thousands(int(row.get("cost", 0))), int(row.get("turns", 1))]
		button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		button.alignment = HORIZONTAL_ALIGNMENT_LEFT
		var available: bool = bool(row.get("available", false))
		button.disabled = not available
		var building_id: String = str(row.get("building", ""))
		IconLibrary.decorate_button(button, building_id, int(ROW_ICON), "building")
		button.pressed.connect(func() -> void: on_build.call(building_id))
		line.add_child(button)
		if not available:
			line.add_child(reason_label(str(row.get("reason", "Indisponible"))))
		button.tooltip_text = RichTooltip.building(building_id, row)
		list.add_child(line)


static func reason_label(text: String) -> Label:
	var reason := Label.new()
	reason.text = text
	reason.add_theme_font_size_override("font_size", 13)
	reason.add_theme_color_override("font_color", REASON_COLOR)
	return reason


## Nom d'unité : `name` si la simulation le fournit, sinon l'id rendu lisible.
static func unit_label(unit: Dictionary) -> String:
	var name: String = str(unit.get("name", ""))
	if name != "":
		return name
	return str(unit.get("unit_type", "?")).trim_prefix("unit_").capitalize()


static func thousands(value: int) -> String:
	var text := str(absi(value))
	var out := ""
	while text.length() > 3:
		out = " " + text.substr(text.length() - 3) + out
		text = text.substr(0, text.length() - 3)
	return ("-" if value < 0 else "") + text + out
