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
## NT5 (N6) : « Former une armée » grisé, avec une infobulle explicite, quand plus de `cap`
## unités sont cochées (le cœur refuserait : `OrderError::ArmyFull`) ; suit chaque case cochée.
static func bind_army_cap(button: Button, checks: Array[CheckBox], cap: int) -> void:
	var refresh := func() -> void:
		var selected := 0
		for check in checks:
			if check.button_pressed:
				selected += 1
		var full := selected > cap
		button.disabled = selected == 0 or full
		if full:
			RichTooltip.attach_plain(button, "army_full", {"body": "%d unités cochées : une armée compte au plus %d unités. Décochez-en %d, ou formez une seconde armée ensuite." % [selected, cap, selected - cap]})
		elif button.tooltip_text.contains("army_full"):
			button.tooltip_text = ""
	for check in checks:
		check.toggled.connect(func(_on: bool) -> void: refresh.call())
	refresh.call()


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
			# VN : nom et effectifs passent à la ligne au lieu d'être coupés (« Milice urbaine — 12… »).
			check.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
			IconLibrary.decorate_button(check, unit_type, int(ROW_ICON), "unit")
			RichTooltip.set_tooltip(check, "unit", unit_type, unit)  # IB1 : infobulle en sections
			list.add_child(check)
			checks.append(check)
		else:
			var chip := wrap_chip(IconChip.create(unit_type, text, "", ROW_ICON, 14, "unit"))
			RichTooltip.set_tooltip(chip, "unit", unit_type, unit)  # IB1
			list.add_child(chip)
	return checks


## Options de recrutement (`get_recruitable`) ; `on_recruit(unit_type)` au clic. `wrap` (TW2-T3,
## mercenaires) : libellés longs (« Arbalétriers génois (Génois des galées) ») retournés à la
## ligne plutôt que tronqués par des points de suspension — chaque ligne prend alors toute la
## largeur du panneau (colonne unique) au lieu de la partager avec la note de droite.
static func fill_recruitable(list: Container, recruitable: Array, on_recruit: Callable, wrap: bool = false) -> void:
	clear(list)
	# U13 : disponibles d'abord, puis les refus passagers ; les lignes qui attendent une
	# technique ou un bâtiment sont repliées sous « Bientôt » ; celles réservées à d'autres
	# factions ou cultures ne sont pas montrées. Sans `group` (mercenaires) : `available`.
	var ready: Array = []
	var blocked: Array = []
	var soon: Array = []
	for row in recruitable:
		var group := str(row.get("group", "ready" if bool(row.get("available", false)) else "blocked"))
		match group:
			"ready":
				ready.append(row)
			"soon":
				soon.append(row)
			"elsewhere":
				pass
			_:
				blocked.append(row)
	for row in ready + blocked:
		_recruit_row(list, row, on_recruit, wrap)
	if not soon.is_empty():
		_soon_section(list, soon, on_recruit, wrap)


## U13 : intertitre repliable « Bientôt (n) » ; l'état (ouvert ou non) survit au rafraîchissement.
static func _soon_section(list: Container, rows: Array, on_recruit: Callable, wrap: bool) -> void:
	var toggle := Button.new()
	toggle.name = "SoonToggle"
	toggle.flat = true
	toggle.alignment = HORIZONTAL_ALIGNMENT_LEFT
	var body := VBoxContainer.new()
	body.name = "SoonList"
	for row in rows:
		_recruit_row(body, row, on_recruit, wrap)
	var refresh := func() -> void:
		var open: bool = bool(list.get_meta(&"soon_open", false))
		toggle.text = "%s Bientôt (%d)" % ["▾" if open else "▸", rows.size()]
		body.visible = open
	toggle.pressed.connect(func() -> void:
		list.set_meta(&"soon_open", not bool(list.get_meta(&"soon_open", false)))
		refresh.call())
	refresh.call()
	list.add_child(toggle)
	list.add_child(body)


static func _recruit_row(list: Container, row: Dictionary, on_recruit: Callable, wrap: bool) -> void:
	var line: Container = VBoxContainer.new() if wrap else HBoxContainer.new()
	if wrap:
		line.add_theme_constant_override("separation", 1)
	var button := RichButton.new()
	button.text = "%s — %s / %s" % [str(row.get("name", row.get("unit_type", "?"))), Money.amount(int(row.get("cost", 0))), Money.amount(int(row.get("upkeep", 0)))]
	# SV2 : le coût comprend l'importation des matériaux manquants (détail dans la bulle).
	if int(row.get("import_cost", 0)) > 0:
		button.text += " (dont import %s)" % Money.amount(int(row["import_cost"]))
	button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	button.alignment = HORIZONTAL_ALIGNMENT_LEFT
	if wrap:
		button.clip_text = false
		button.text_overrun_behavior = TextServer.OVERRUN_NO_TRIMMING
		button.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	else:
		narrow_button(button)
	var available: bool = bool(row.get("available", false))
	button.disabled = not available
	var unit_type: String = str(row.get("unit_type", ""))
	IconLibrary.decorate_button(button, unit_type, int(ROW_ICON), "unit")
	RichTooltip.set_tooltip(button, "unit", unit_type, row)  # IB1 : infobulle en sections
	button.pressed.connect(func() -> void: on_recruit.call(unit_type))
	line.add_child(button)
	# TW2-T2 : réserve de recrutement de la colonie (« 2 disponibles, +1 dans 2 saisons »).
	if row.has("pool_label"):
		line.add_child(_note(pool_label(row), wrap))
	list.add_child(line)
	# Q8 : le motif d'indisponibilité passe sous la ligne, pleine largeur ; à droite il
	# partageait la place avec la réserve et se repliait mot à mot (« à / engager / depuis… »).
	var reason := str(row.get("reason", "Indisponible"))
	if not available and not reason.begins_with("réserve"):
		var reason_note := _note(reason_label(reason), true)
		if wrap:
			line.add_child(reason_note)
		else:
			list.add_child(reason_note)


## Note à droite d'un bouton de ligne (`side_note`), ou pleine largeur sous le bouton si `wrap`.
static func _note(label: Label, wrap: bool) -> Label:
	if wrap:
		label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		return label
	return side_note(label)


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
		var chip := wrap_chip(IconChip.create(building_id, text, "", ROW_ICON, 14, "building"))
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
		narrow_button(button)
		var available: bool = bool(row.get("available", false))
		button.disabled = not available
		var building_id: String = str(row.get("building", ""))
		IconLibrary.decorate_button(button, building_id, int(ROW_ICON), "building")
		button.pressed.connect(func() -> void: on_build.call(building_id))
		line.add_child(button)
		# SV3 : surcoût d'import (B7c) déjà visible dans la bulle ; rappel court sur la ligne
		# pour ne pas avoir à ouvrir la bulle pour le repérer.
		var import_cost := int(row.get("import_cost", 0))
		if import_cost > 0:
			line.add_child(side_note(import_cost_label(import_cost)))
		RichTooltip.set_tooltip(button, "building", building_id, row)  # IB1 : infobulle en sections
		list.add_child(line)
		# U12 : le motif d'indisponibilité passe sous la ligne, pleine largeur (comme le
		# recrutement, Q8), et non plus dans une colonne rouge étroite à droite du bouton.
		if not available:
			list.add_child(_note(reason_label(str(row.get("reason", "Indisponible"))), true))


## SV3 : « Dont import : X ₶ » en rouge, même couleur que la bulle (`RichTooltip.RED`).
static func import_cost_label(import_cost: int) -> Label:
	var label := Label.new()
	label.text = "Dont import : %s" % Money.amount(import_cost)
	label.add_theme_font_size_override("font_size", UiType.size(UiType.CAPTION))
	label.add_theme_color_override("font_color", Color(RichTooltip.RED))
	return label


## Q6 : hauteur plancher des onglets d'un panneau de la zone `SIDE_PANEL` (liste défilante).
const SIDE_TABS_MIN_HEIGHT := 200.0


## Q6 : ajuste la hauteur des onglets (`tabs`) de `panel` pour que le panneau tienne dans la zone
## `SIDE_PANEL` : l'en-tête garde sa taille, les onglets (pages défilantes, bornes minimales
## remises à zéro) prennent le reste, entre `SIDE_TABS_MIN_HEIGHT` et `max_height` (hauteur de
## conception). En vue 1280×720, les onglets fixes (300-360 px) poussaient « Recruter » et
## « Changer d'édit » sous le bord de la zone, au niveau de la minicarte et de la fin de tour.
static func fit_tabs_to_side_zone(panel: Control, tabs: TabContainer, max_height: float) -> void:
	if not is_instance_valid(panel) or not panel.is_inside_tree():
		return
	for page in tabs.get_children():
		if page is ScrollContainer:
			(page as Control).custom_minimum_size.y = 0.0
	var zone_height := UiZones.rect(UiZones.Zone.SIDE_PANEL).size.y
	var others := panel.get_combined_minimum_size().y - tabs.get_combined_minimum_size().y
	var wanted := clampf(zone_height - others, minf(SIDE_TABS_MIN_HEIGHT, max_height), max_height)
	if not is_equal_approx(tabs.custom_minimum_size.y, wanted):
		tabs.custom_minimum_size.y = wanted


## Q6 : bouton de ligne d'une liste du panneau latéral ; son libellé se coupe (points de
## suspension, texte entier dans la bulle) au lieu d'élargir le panneau au-delà de sa zone
## (`SIDE_PANEL`, 384 px en vue 1280×720), où le reste du panneau passait hors de l'écran.
static func narrow_button(button: Button) -> Button:
	button.clip_text = true
	button.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
	return button


## Q6 : libellé d'une puce de liste qui passe à la ligne au lieu d'élargir la liste.
static func wrap_chip(chip: IconChip) -> IconChip:
	chip.label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	chip.label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	return chip


## Q6 : note à droite d'un bouton de ligne (raison, import, réserve) : elle passe à la ligne
## dans le tiers de la largeur au lieu d'élargir la ligne.
static func side_note(label: Label) -> Label:
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	label.size_flags_stretch_ratio = 0.5
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

