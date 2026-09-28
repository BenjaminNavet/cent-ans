class_name PanelWidgets
extends RefCounted

## Lignes partagées des panneaux de province et de colonie (lot C5) : garnison, recrutement,
## bâtiments, chantier et constructions possibles. Rendu seulement : les listes, la
## disponibilité et les raisons de refus viennent de la simulation.

const ROW_ICON := 20.0
## DA5 : diamètre des médaillons enluminés des boutons d'action (Recruter, Former une armée).
const MEDALLION_SIZE := 26
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
			RichTooltip.set_tooltip(check, "unit", unit_type, unit)  # IB1 : infobulle en sections
			list.add_child(check)
			checks.append(check)
		else:
			var chip := IconChip.create(unit_type, text, "", ROW_ICON, 14, "unit")
			RichTooltip.set_tooltip(chip, "unit", unit_type, unit)  # IB1
			list.add_child(chip)
	return checks


## Options de recrutement (`get_recruitable`) ; `on_recruit(unit_type)` au clic.
static func fill_recruitable(list: Container, recruitable: Array, on_recruit: Callable) -> void:
	clear(list)
	for row in recruitable:
		var line := HBoxContainer.new()
		var button := RichButton.new()
		button.text = "%s — %s / %s" % [str(row.get("name", row.get("unit_type", "?"))), Money.amount(int(row.get("cost", 0))), Money.amount(int(row.get("upkeep", 0)))]
		# SV2 : le coût comprend l'importation des matériaux manquants (détail dans la bulle).
		if int(row.get("import_cost", 0)) > 0:
			button.text += " (dont import %s)" % Money.amount(int(row["import_cost"]))
		button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		button.alignment = HORIZONTAL_ALIGNMENT_LEFT
		var available: bool = bool(row.get("available", false))
		button.disabled = not available
		var unit_type: String = str(row.get("unit_type", ""))
		IconLibrary.decorate_button(button, unit_type, int(ROW_ICON), "unit")
		RichTooltip.set_tooltip(button, "unit", unit_type, row)  # IB1 : infobulle en sections
		button.pressed.connect(func() -> void: on_recruit.call(unit_type))
		line.add_child(button)
		var reason := str(row.get("reason", "Indisponible"))
		if not available and not reason.begins_with("réserve"):
			line.add_child(reason_label(reason))
		# TW2-T2 : réserve de recrutement de la colonie (« 2 disponibles, +1 dans 2 saisons »).
		if row.has("pool_label"):
			line.add_child(pool_label(row))
		list.add_child(line)


## TW2-T2 : réserve de l'unité dans la colonie (`pool_label` du cœur), en rubrique si épuisée.
static func pool_label(row: Dictionary) -> Label:
	var label := reason_label(str(row.get("pool_label", "")))
	label.name = "PoolLabel"
	if int(row.get("pool_available", 0)) > 0:
		label.add_theme_color_override("font_color", HudStyle.INK_SOFT)
	RichTooltip.attach_plain(label, "recruit_pool", {"body": "%d au plus, se remplit à chaque saison" % int(row.get("pool_cap", 0))})
	label.mouse_filter = Control.MOUSE_FILTER_PASS
	return label


## Bâtiments construits (`{id, name, upkeep, …}`). RS-N : avec `demolition`
## (`building_id → settlement_demolition_preview` : `{can_demolish, reason,
## refund, upkeep_saved}`) et `is_player_owner`, ajoute un bouton « Raser » par
## bâtiment, désactivé et annoté de la raison en français quand le cœur
## refuserait ; `on_raze(building_id, preview_row)` au clic.
static func fill_buildings(list: Container, buildings: Array, demolition: Dictionary = {}, is_player_owner: bool = false, on_raze: Callable = Callable()) -> void:
	clear(list)
	if buildings.is_empty():
		placeholder(list, "Aucun bâtiment.")
		return
	for entry in buildings:
		var building_id: String = str(entry.get("id", ""))
		var text := "%s (entretien %s)" % [str(entry.get("name", building_id)), Money.amount(int(entry.get("upkeep", 0)))]
		var row := HBoxContainer.new()
		row.name = building_id
		row.add_theme_constant_override("separation", 6)
		var chip := IconChip.create(building_id, text, "", ROW_ICON, 14, "building")
		RichTooltip.set_tooltip(chip, "building", building_id, entry)  # IB1
		chip.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		row.add_child(chip)
		if is_player_owner and demolition.has(building_id):
			row.add_child(_raze_button(building_id, demolition[building_id], on_raze))
		list.add_child(row)


## RS-N : bouton « Raser » d'un bâtiment ; `preview` vient de
## `settlement_demolition_preview` (cœur). Infobulle en sections (IB2, clé `raze_building`).
static func _raze_button(building_id: String, preview: Dictionary, on_raze: Callable) -> Button:
	var button := Button.new()
	button.name = "RazeButton"
	button.text = "Raser"
	IconLibrary.decorate_button(button, "act_raze", int(ROW_ICON))
	var can_demolish: bool = bool(preview.get("can_demolish", false))
	button.disabled = not can_demolish
	var refund := int(preview.get("refund", 0))
	var upkeep_saved := int(preview.get("upkeep_saved", 0))
	if can_demolish:
		RichTooltip.attach_plain(button, "raze_building", {"body": "Rembourse %s ; économise %s d'entretien par saison." % [Money.amount(refund), Money.amount(upkeep_saved)]})
	else:
		RichTooltip.attach_plain(button, "raze_building", {"body": str(preview.get("reason", "indisponible"))})
	if on_raze.is_valid():
		button.pressed.connect(func() -> void: on_raze.call(building_id, preview))
	return button


## Constructions possibles (`buildable`), sans celles déjà construites (`built_ids`) ;
## `on_build(building_id)` au clic. Rien si `is_player_owner` est faux.
## Lot C4 : rang de chaîne de `building_id` (`Building.tier`, 1 = palier de
## base), pour lire les options de construction comme un arbre trié plutôt
## qu'un ordre alphabétique.
static func _building_tier(building_id: String) -> int:
	return int(GameCatalog.building(building_id).get("tier", 1))


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
	# Lot C4 : catégorie puis rang, pour que chaque chaîne (marché → maison des
	# métiers → foire, etc.) s'affiche dans l'ordre de ses paliers.
	rows.sort_custom(func(a: Dictionary, b: Dictionary) -> bool:
		var cat_a: String = str(GameCatalog.building(str(a.get("building", ""))).get("category", ""))
		var cat_b: String = str(GameCatalog.building(str(b.get("building", ""))).get("category", ""))
		if cat_a != cat_b:
			return cat_a < cat_b
		var tier_a := _building_tier(str(a.get("building", "")))
		var tier_b := _building_tier(str(b.get("building", "")))
		if tier_a != tier_b:
			return tier_a < tier_b
		return str(a.get("name", "")) < str(b.get("name", "")))
	for row in rows:
		var line := HBoxContainer.new()
		line.add_theme_constant_override("separation", 8)
		var button := RichButton.new()
		button.text = "%s — %s / %s" % [str(row.get("name", row.get("building", "?"))), Money.amount(int(row.get("cost", 0))), FrText.count(int(row.get("turns", 1)), "tour")]
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
		# SV3 : surcoût d'import (B7c) déjà visible dans la bulle ; rappel court sur la ligne
		# pour ne pas avoir à ouvrir la bulle pour le repérer.
		var import_cost := int(row.get("import_cost", 0))
		if import_cost > 0:
			line.add_child(import_cost_label(import_cost))
		RichTooltip.set_tooltip(button, "building", building_id, row)  # IB1 : infobulle en sections
		list.add_child(line)


## SV3 : « Dont import : X ₶ » en rouge, même couleur que la bulle (`RichTooltip.RED`).
static func import_cost_label(import_cost: int) -> Label:
	var label := Label.new()
	label.text = "Dont import : %s" % Money.amount(import_cost)
	label.add_theme_font_size_override("font_size", UiType.size(UiType.CAPTION))
	label.add_theme_color_override("font_color", Color(RichTooltip.RED))
	return label


static func reason_label(text: String) -> Label:
	var reason := Label.new()
	reason.text = text
	reason.add_theme_font_size_override("font_size", UiType.size(UiType.CAPTION))
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
