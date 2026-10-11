class_name SettlementBuildingsView
extends VBoxContainer

## CO-C (ADR 0292), refondu par RG11 (ADR 0345) : onglet « Bâtiments » d'une colonie.
## De haut en bas :
## - En-tête : illustration du palier de développement (`settlement_tier` du cœur), légende
##   « Palier n/6 — nom » et jauge à six crans. Image absente : paliers inférieurs, puis bâtiment
##   principal, puis fond parchemin.
## - `VocationSlot` : sélecteur de vocation de la colonie (RG10, ADR 0344).
## - Emplacements : une case par emplacement (`used/max`) : bâtiment debout avec son niveau, chantier
##   avec barre de progression et tours restants, ou case libre (`free_slot_requested`).
## - File d'attente : les chantiers de la colonie (en cours + file), `queue_size` places.
## - Arbre de construction : chaînes de bâtiments en colonnes, par catégorie, depuis `get_building_tree`
##   du cœur. Chaque nœud est construit / en chantier / en file / constructible / verrouillé (raison
##   en clair) ; un clic sur un nœud constructible émet `build_requested` (ordre `build` existant).
## Aucune règle ici : états, raisons, coûts et refus viennent du cœur.

signal build_requested(settlement_id: String, building_id: String)
signal raze_requested(settlement_id: String, building_id: String, preview: Dictionary)
signal free_slot_requested(settlement_id: String)
## RG10 : la vocation de la colonie vient de changer (coûts et effets à relire).
signal vocation_changed(settlement_id: String, vocation_id: String)
## RG4 : annuler le chantier de la carte (`index` -1 : en cours, n : n-ième de la file) ; seulement si
## `show_cancel` (vues d'accompagnement : le panneau de colonie a ses propres boutons).
signal cancel_requested(settlement_id: String, index: int)

const TIERS_PATH := "ui/settlement_tiers.json"
const BUILDING_IMAGE := "res://assets/illustrations/%s.jpg"
const MAX_TIER := 6
const CARD_MIN_WIDTH := 118.0
const HEADER_IMAGE_HEIGHT := 96.0
const CARD_IMAGE_HEIGHT := 58.0
const NODE_MIN_WIDTH := 108.0
const CATEGORY_ORDER := ["production", "commerce", "military", "religious", "sanitary", "fortification"]

static var _tiers := JsonLookup.new(TIERS_PATH, {"illustration_dir": "res://assets/illustrations/settlement_tiers", "names": {}})

var settlement_id: String = ""
var show_cancel := false
var header_frame: PanelContainer
var header_image: TextureRect
var header_backdrop: Control
var caption: Label
var vocation_selector: VocationSelector
var gauge: Control
var usage_label: Label
var grid: HFlowContainer
var vocation_slot: VBoxContainer  # RG10 y place l'en-tête des vocations
var queue_strip: VBoxContainer
var tree_box: VBoxContainer
var show_all_check: CheckBox
var _tree: Dictionary = {}
var _show_all := false
var _player_owner := false
var _tier: int = 1
var _header_has_image := false


func _init() -> void:
	name = "BuildingsView"
	size_flags_horizontal = Control.SIZE_EXPAND_FILL
	add_theme_constant_override("separation", 4)
	header_frame = PanelContainer.new()
	header_frame.name = "TierHeader"
	header_frame.add_theme_stylebox_override("panel", HudStyle.card_box(HudStyle.PARCHMENT, HudStyle.GOLD, 2))
	add_child(header_frame)
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 3)
	header_frame.add_child(box)
	header_backdrop = PanelContainer.new()
	header_backdrop.custom_minimum_size = Vector2(0, HEADER_IMAGE_HEIGHT)
	(header_backdrop as PanelContainer).add_theme_stylebox_override("panel", HudStyle.card_box(HudStyle.PARCHMENT_DARK, HudStyle.INK_SOFT, 1))
	box.add_child(header_backdrop)
	header_image = TextureRect.new()
	header_image.name = "TierImage"
	header_image.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	header_image.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_COVERED
	header_image.mouse_filter = Control.MOUSE_FILTER_IGNORE
	header_backdrop.add_child(header_image)
	caption = HudStyle.label("", HudStyle.FONT_BODY, HudStyle.INK)
	caption.name = "TierCaption"
	caption.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	caption.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	box.add_child(caption)
	gauge = _Pips.new()
	gauge.name = "TierGauge"
	gauge.custom_minimum_size = Vector2(0, 10)
	box.add_child(gauge)
	usage_label = HudStyle.label("", HudStyle.FONT_SMALL, HudStyle.INK_SOFT)
	usage_label.name = "SlotUsage"
	add_child(usage_label)
	vocation_slot = VBoxContainer.new()
	vocation_slot.name = "VocationSlot"
	add_child(vocation_slot)
	vocation_selector = VocationSelector.new()  # RG10 (ADR 0344)
	vocation_selector.vocation_changed.connect(func(id: String, vocation_id: String) -> void: vocation_changed.emit(id, vocation_id))
	vocation_slot.add_child(vocation_selector)
	grid = HFlowContainer.new()
	grid.name = "SlotCards"
	grid.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	grid.add_theme_constant_override("h_separation", 4)
	grid.add_theme_constant_override("v_separation", 4)
	add_child(grid)
	queue_strip = VBoxContainer.new()
	queue_strip.name = "QueueStrip"
	queue_strip.add_theme_constant_override("separation", 2)
	add_child(queue_strip)
	var tree_title := HBoxContainer.new()
	add_child(tree_title)
	var title := HudStyle.label("Arbre de construction", HudStyle.FONT_BODY, HudStyle.RUBRIC)
	title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	tree_title.add_child(title)
	show_all_check = CheckBox.new()
	show_all_check.name = "ShowAllRanks"
	show_all_check.text = "Autres rangs"
	show_all_check.tooltip_text = "Montrer aussi les bâtiments réservés à d'autres rangs de colonie."
	show_all_check.add_theme_font_size_override("font_size", HudStyle.FONT_SMALL)
	show_all_check.toggled.connect(func(pressed: bool) -> void:
		_show_all = pressed
		_fill_tree())
	tree_title.add_child(show_all_check)
	tree_box = VBoxContainer.new()
	tree_box.name = "BuildingTree"
	tree_box.add_theme_constant_override("separation", 4)
	add_child(tree_box)


## `rows` : `settlement_slots`. `usage` : `settlement_slot_usage`. `tier` : `settlement_tier`.
## `detail` : `settlement_detail` (`kind`, `construction`, `build_queue`). `demolition_by_id` :
## `building → settlement_demolition_preview`. `tree` : `get_building_tree` (vide : pas d'arbre).
func set_data(id: String, rows: Array, usage: Dictionary, tier: int, detail: Dictionary, demolition_by_id: Dictionary, player_owner: bool, tree: Dictionary = {}) -> void:
	settlement_id = id
	_tree = tree
	_player_owner = player_owner
	var kind := str(detail.get("kind", ""))
	_tier = clampi(tier, 1, MAX_TIER)
	_fill_header(kind, rows)
	vocation_selector.set_settlement(id, player_owner)
	for child in grid.get_children():
		grid.remove_child(child)
		child.queue_free()
	var entries := _entries(rows, detail)
	var max_slots := int(usage.get("max", -1))
	var total := entries.size() + 1 if max_slots < 0 else maxi(max_slots, entries.size())
	for entry in entries:
		grid.add_child(_slot_card(entry, demolition_by_id, player_owner))
	for index in total - entries.size():
		grid.add_child(_free_card(index, player_owner))
	if max_slots >= 0:
		usage_label.text = "Emplacements %d/%d" % [int(usage.get("used", entries.size())), max_slots]
	else:
		usage_label.text = "Emplacements %d" % entries.size()
	_fill_queue(detail)
	_fill_tree()


## Nombre de cartes (emplacements occupés, en chantier et libres).
func card_count() -> int:
	return grid.get_child_count()


## Vrai si l'en-tête montre une vraie illustration (sinon repli parchemin).
func header_has_image() -> bool:
	return _header_has_image


## Progression (0..100) vers le palier suivant (`settlement_tier_progress` du cœur) : remplit le
## cran suivant de la jauge et s'affiche en infobulle.
func set_tier_progress(percent: int) -> void:
	var pips := gauge as _Pips
	pips.partial = clampf(percent / 100.0, 0.0, 1.0) if _tier < MAX_TIER else 0.0
	pips.queue_redraw()
	pips.mouse_filter = Control.MOUSE_FILTER_STOP
	pips.tooltip_text = "Palier le plus haut atteint." if _tier >= MAX_TIER else "Vers le palier %d : %d %%. Bâtir et améliorer fait grandir la colonie." % [_tier + 1, clampi(percent, 0, 100)]


func tier() -> int:
	return _tier


## « Palier 3/6 — Gros village ».
static func tier_caption(kind: String, tier_value: int) -> String:
	return "Palier %d/%d — %s" % [tier_value, MAX_TIER, tier_name(kind, tier_value)]


static func tier_name(kind: String, tier_value: int) -> String:
	var names: Variant = (_tiers.section("names") as Dictionary).get(kind, [])
	if names is Array and tier_value >= 1 and tier_value <= (names as Array).size():
		return str((names as Array)[tier_value - 1])
	return KIND_FALLBACK.get(kind, "Colonie")


const KIND_FALLBACK := {"city": "Cité", "town": "Ville", "castle": "Château", "abbey": "Abbaye", "village": "Village"}


static func roman(value: int) -> String:
	const NUMERALS := ["I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X"]
	return NUMERALS[value - 1] if value >= 1 and value <= NUMERALS.size() else str(value)


## Chemin de l'illustration du palier (le plus proche en dessous de `tier_value`) ou "".
static func tier_image_path(kind: String, tier_value: int) -> String:
	var dir := str(_tiers.value("illustration_dir", ""))
	for candidate in range(tier_value, 0, -1):
		var path := "%s/%s_%d.jpg" % [dir, kind, candidate]
		if ResourceLoader.exists(path):
			return path
	return ""


static func building_image(building_id: String) -> Texture2D:
	var path := BUILDING_IMAGE % building_id
	if ResourceLoader.exists(path):
		return load(path) as Texture2D
	return null


func _fill_header(kind: String, rows: Array) -> void:
	caption.text = tier_caption(kind, _tier)
	(gauge as _Pips).setup(_tier, MAX_TIER)
	var texture: Texture2D = null
	var path := tier_image_path(kind, _tier)
	if path != "":
		texture = load(path) as Texture2D
	if texture == null:  # Repli : le bâtiment principal (le plus haut niveau debout) qui a une image.
		var best_level := 0
		for slot in rows:
			var built := str((slot as Dictionary).get("built", ""))
			var level := int((slot as Dictionary).get("level", 0))
			if built != "" and level > best_level:
				var candidate := building_image(built)
				if candidate != null:
					texture = candidate
					best_level = level
	_header_has_image = path != "" and texture != null
	header_image.texture = texture
	header_image.visible = texture != null
	header_backdrop.visible = true


## Un élément par chaîne occupée (debout ou en chantier) : `{root, slot, built, pending}`.
func _entries(rows: Array, detail: Dictionary) -> Array:
	var roots := {}
	var entries: Array = []
	var by_root := {}
	for slot in rows:
		var row: Dictionary = slot
		roots[str(row.get("root", ""))] = true
	for slot in rows:
		var row: Dictionary = slot
		if str(row.get("built", "")) != "":
			var entry := {"root": str(row["root"]), "slot": row, "built": str(row["built"]), "pending": {}}
			entries.append(entry)
			by_root[entry["root"]] = entry
	var pendings: Array = []
	var construction: Variant = detail.get("construction", {})
	if construction is Dictionary and not (construction as Dictionary).is_empty():
		var current: Dictionary = (construction as Dictionary).duplicate()
		current["queue_index"] = -1
		pendings.append(current)
	var queued: Variant = detail.get("build_queue", [])
	if queued is Array:
		for queue_position in (queued as Array).size():
			var waiting: Dictionary = ((queued as Array)[queue_position] as Dictionary).duplicate()
			waiting["queue_index"] = queue_position
			pendings.append(waiting)
	for pending in pendings:
		var building_id := str((pending as Dictionary).get("building", ""))
		var root := _chain_root(building_id, roots)
		if by_root.has(root):
			if (by_root[root]["pending"] as Dictionary).is_empty():
				by_root[root]["pending"] = pending
		else:
			var slot_row := {}
			for slot in rows:
				if str((slot as Dictionary).get("root", "")) == root:
					slot_row = slot
			var entry := {"root": root, "slot": slot_row, "built": "", "pending": pending}
			entries.append(entry)
			by_root[root] = entry
	return entries


## Premier palier de la chaîne de `building_id` parmi les racines d'emplacements connues.
static func _chain_root(building_id: String, roots: Dictionary) -> String:
	var current := building_id
	for _step in 16:
		if roots.has(current):
			return current
		var parent := str(GameCatalog.building(current).get("upgrades_from", ""))
		if parent == "":
			break
		current = parent
	return current


func _slot_card(entry: Dictionary, demolition_by_id: Dictionary, player_owner: bool) -> Control:
	var slot: Dictionary = entry["slot"]
	var built: String = entry["built"]
	var pending: Dictionary = entry["pending"]
	var shown: String = built if built != "" else str(pending.get("building", entry["root"]))
	var card := PanelContainer.new()
	card.name = "Card_" + str(entry["root"])
	card.custom_minimum_size = Vector2(CARD_MIN_WIDTH, 0)
	card.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	card.mouse_filter = Control.MOUSE_FILTER_STOP
	card.add_theme_stylebox_override("panel", HudStyle.card_box(HudStyle.PARCHMENT_LIGHT, HudStyle.INK_SOFT if built != "" else HudStyle.GOLD, 1))
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 2)
	card.add_child(box)
	box.add_child(_image_box(shown))
	var title := HudStyle.label(GameCatalog.display_name(shown), HudStyle.FONT_SMALL, HudStyle.INK)
	title.name = "CardName"
	title.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	box.add_child(title)
	var level := int(slot.get("level", 1 if built != "" else 0))
	var max_level := maxi(int(slot.get("max_level", 1)), 1)
	if built != "":
		var level_row := HBoxContainer.new()
		level_row.add_theme_constant_override("separation", 4)
		var level_label := HudStyle.label(roman(level), HudStyle.FONT_SMALL, HudStyle.RUBRIC)
		level_label.name = "CardLevel"
		level_row.add_child(level_label)
		var pips := _Pips.new()
		pips.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		pips.custom_minimum_size = Vector2(0, 10)
		pips.setup(level, max_level)
		level_row.add_child(pips)
		box.add_child(level_row)
	if not pending.is_empty():
		var turns := int(pending.get("turns_left", 0))
		var total := maxi(_total_turns(str(pending.get("building", ""))), turns)
		var bar := _Bar.new()
		bar.name = "CardProgress"
		bar.custom_minimum_size = Vector2(0, 8)
		bar.ratio = 1.0 - float(turns) / float(maxi(total, 1))
		box.add_child(bar)
		var state_label := HudStyle.label("Chantier : %s" % _turns_text(turns), HudStyle.FONT_SMALL, HudStyle.RUBRIC)
		state_label.name = "CardState"
		state_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		box.add_child(state_label)
	elif built != "" and (slot.get("next", []) as Array).is_empty():
		var done := HudStyle.label("Achevé", HudStyle.FONT_SMALL, HudStyle.INK_FADED)
		done.name = "CardState"
		box.add_child(done)
	if player_owner:
		var actions := HFlowContainer.new()
		actions.name = "CardActions"
		if show_cancel and not pending.is_empty():
			var cancel := Button.new()
			cancel.name = "CancelPendingButton"
			cancel.text = "Annuler"
			var queue_index := int(pending.get("queue_index", -1))
			IconLibrary.decorate_button(cancel, "act_cancel_build", 14)
			cancel.pressed.connect(func() -> void: cancel_requested.emit(settlement_id, queue_index))
			actions.add_child(cancel)
		if built != "" and demolition_by_id.has(built):
			actions.add_child(PanelWidgets._raze_button(built, demolition_by_id[built], func(building_id: String, preview: Dictionary) -> void: raze_requested.emit(settlement_id, building_id, preview)))
		if actions.get_child_count() > 0:
			box.add_child(actions)
	TooltipHost.attach_plain(card, "building_chain", {"title": GameCatalog.display_name(shown), "body": chain_text(str(entry["root"]), built, slot)})
	return card


func _free_card(index: int, player_owner: bool) -> Control:
	var card := PanelContainer.new()
	card.name = "FreeCard%d" % index
	card.custom_minimum_size = Vector2(CARD_MIN_WIDTH, CARD_IMAGE_HEIGHT + 24)
	card.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	card.add_theme_stylebox_override("panel", HudStyle.card_box(HudStyle.PARCHMENT, HudStyle.INK_FADED, 1))
	var button := Button.new()
	button.name = "FreeSlotButton"
	button.flat = true
	button.text = "Emplacement libre"
	button.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	button.disabled = not player_owner
	button.add_theme_font_size_override("font_size", HudStyle.FONT_SMALL)
	button.add_theme_color_override("font_color", HudStyle.INK_FADED)
	button.pressed.connect(func() -> void: free_slot_requested.emit(settlement_id))
	card.add_child(button)
	return card


func _image_box(building_id: String) -> Control:
	var holder := PanelContainer.new()
	holder.custom_minimum_size = Vector2(0, CARD_IMAGE_HEIGHT)
	holder.mouse_filter = Control.MOUSE_FILTER_IGNORE
	holder.add_theme_stylebox_override("panel", HudStyle.card_box(HudStyle.PARCHMENT_DARK, HudStyle.INK_SOFT, 1))
	var texture := building_image(building_id)
	var rect := TextureRect.new()
	rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
	if texture != null:
		rect.texture = texture
		rect.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
		rect.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_COVERED
		rect.name = "CardImage"
	else:  # Repli : pictogramme du bâtiment sur fond parchemin.
		rect.texture = HudStyle.icon(building_id, "building")
		rect.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
		rect.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
		rect.name = "CardIcon"
	holder.add_child(rect)
	return holder


static func _turns_text(turns: int) -> String:
	return "%s restant%s" % [FrText.count(turns, "tour"), FrText.s(turns)]


## Durée totale d'un chantier de l'arbre (file du cœur), 0 si inconnue.
func _total_turns(building_id: String) -> int:
	for row in _tree.get("queue", []):
		if str((row as Dictionary).get("building", "")) == building_id:
			return int((row as Dictionary).get("total_turns", 0))
	return 0


## File d'attente : `queue_size` places, le chantier en cours puis la file (nom, barre, tours).
func _fill_queue(_detail: Dictionary) -> void:
	for child in queue_strip.get_children():
		queue_strip.remove_child(child)
		child.queue_free()
	var places := int(_tree.get("queue_size", 0))
	if places <= 0:
		queue_strip.visible = false
		return
	queue_strip.visible = true
	var queue: Array = _tree.get("queue", [])
	var head := HudStyle.label("File d'attente %d/%d" % [queue.size(), places], HudStyle.FONT_SMALL, HudStyle.INK_SOFT)
	head.name = "QueueTitle"
	queue_strip.add_child(head)
	for index in places:
		var cell := PanelContainer.new()
		cell.name = "QueueCell%d" % index
		var filled := index < queue.size()
		cell.add_theme_stylebox_override("panel", HudStyle.card_box(HudStyle.PARCHMENT_LIGHT if filled else HudStyle.PARCHMENT, HudStyle.GOLD if filled else HudStyle.INK_FADED, 1))
		var row := HBoxContainer.new()
		row.add_theme_constant_override("separation", 6)
		cell.add_child(row)
		if filled:
			var entry: Dictionary = queue[index]
			var turns := int(entry.get("turns_left", 0))
			var total := maxi(int(entry.get("total_turns", turns)), turns)
			var name_label := HudStyle.label("%d. %s" % [index + 1, str(entry.get("name", "?"))], HudStyle.FONT_SMALL, HudStyle.INK)
			name_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
			name_label.clip_text = true
			row.add_child(name_label)
			var bar := _Bar.new()
			bar.custom_minimum_size = Vector2(54, 8)
			bar.size_flags_vertical = Control.SIZE_SHRINK_CENTER
			bar.ratio = (1.0 - float(turns) / float(maxi(total, 1))) if index == 0 else 0.0
			row.add_child(bar)
			row.add_child(HudStyle.label("%d t." % turns, HudStyle.FONT_SMALL, HudStyle.RUBRIC))
			if _player_owner:
				var cancel := Button.new()
				cancel.name = "QueueCancel%d" % index
				cancel.text = "×"
				cancel.tooltip_text = "Annuler ce chantier (le remboursement au prorata est annoncé avant confirmation)."
				var queue_index := index - 1  # -1 : le chantier en cours
				cancel.pressed.connect(func() -> void: cancel_requested.emit(settlement_id, queue_index))
				row.add_child(cancel)
		else:
			row.add_child(HudStyle.label("%d. —" % (index + 1), HudStyle.FONT_SMALL, HudStyle.INK_FADED))
		queue_strip.add_child(cell)


## Arbre de construction (`get_building_tree`) : une section par catégorie, une colonne par chaîne
## d'amélioration, un bouton par nœud.
func _fill_tree() -> void:
	for child in tree_box.get_children():
		tree_box.remove_child(child)
		child.queue_free()
	var nodes: Array = _tree.get("nodes", [])
	show_all_check.visible = not nodes.is_empty()
	if nodes.is_empty():
		return
	var by_category := {}
	for node in nodes:
		var entry: Dictionary = node
		if not _show_all and not bool(entry.get("allowed_here", true)):
			continue
		var category := str(entry.get("category", ""))
		if not by_category.has(category):
			by_category[category] = {}
		var chains: Dictionary = by_category[category]
		var root := str(entry.get("root", ""))
		if not chains.has(root):
			chains[root] = []
		(chains[root] as Array).append(entry)
	var categories: Array = CATEGORY_ORDER.duplicate()
	for category in by_category:
		if not categories.has(category):
			categories.append(category)
	for category in categories:
		if not by_category.has(category):
			continue
		var section := VBoxContainer.new()
		section.name = "Category_" + str(category)
		section.add_theme_constant_override("separation", 2)
		tree_box.add_child(section)
		var heading := HudStyle.label(RichTooltip.building_category_label(str(category)), HudStyle.FONT_SMALL, HudStyle.INK_SOFT)
		heading.name = "CategoryTitle"
		section.add_child(heading)
		var columns := HFlowContainer.new()
		columns.name = "Chains"
		columns.add_theme_constant_override("h_separation", 6)
		columns.add_theme_constant_override("v_separation", 6)
		section.add_child(columns)
		var chains: Dictionary = by_category[category]
		for root in chains:
			columns.add_child(_chain_column(str(root), chains[root]))


func _chain_column(root: String, chain: Array) -> Control:
	var column := VBoxContainer.new()
	column.name = "Chain_" + root
	column.add_theme_constant_override("separation", 1)
	var last_depth := 0
	for node in chain:
		var depth := int((node as Dictionary).get("depth", 1))
		if depth != last_depth and last_depth != 0:
			var arrow := HudStyle.label("▼", HudStyle.FONT_SMALL, HudStyle.INK_FADED)
			arrow.name = "ChainArrow"
			arrow.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
			column.add_child(arrow)
		last_depth = depth
		column.add_child(_node_button(node))
	return column


## Libellé de la deuxième ligne d'un nœud, selon son état (texte du cœur pour les verrous).
static func node_status(node: Dictionary) -> String:
	match str(node.get("state", "")):
		"built":
			return "Construit" if bool(node.get("current", false)) else "Acquis (amélioré)"
		"building":
			return "Chantier : %d t." % int(node.get("turns_left", 0))
		"queued":
			return "En file : %d t." % int(node.get("turns_left", 0))
		"available":
			var line := "%s · %s" % [Money.amount(int(node.get("cost", 0))), FrText.count(int(node.get("turns", 0)), "tour")]
			var blocked := str(node.get("blocked_now", ""))
			return line if blocked == "" else "%s\n%s" % [line, blocked.capitalize()]
		_:
			return str(node.get("reason", ""))


func _node_button(node: Dictionary) -> Button:
	var building_id := str(node.get("building", ""))
	var state := str(node.get("state", "locked"))
	var button := Button.new()
	button.name = "Node_" + building_id
	button.set_meta("node_state", state)
	button.custom_minimum_size = Vector2(NODE_MIN_WIDTH, 0)
	button.alignment = HORIZONTAL_ALIGNMENT_LEFT
	button.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	button.text = "%s\n%s" % [str(node.get("name", building_id)), node_status(node)]
	button.add_theme_font_size_override("font_size", HudStyle.FONT_SMALL)
	var background := HudStyle.PARCHMENT_LIGHT
	var border := HudStyle.INK_FADED
	var ink := HudStyle.INK
	match state:
		"built":
			background = HudStyle.PARCHMENT_DARK
			border = HudStyle.INK_SOFT
		"building", "queued":
			border = HudStyle.RUBRIC
		"available":
			border = HudStyle.GOLD
		_:
			background = HudStyle.PARCHMENT
			ink = HudStyle.INK_FADED
	for slot_name in ["normal", "disabled", "focus"]:
		button.add_theme_stylebox_override(slot_name, HudStyle.card_box(background, border, 1))
	button.add_theme_stylebox_override("hover", HudStyle.card_box(HudStyle.GOLD_PALE, HudStyle.GOLD, 2))
	button.add_theme_stylebox_override("pressed", HudStyle.card_box(HudStyle.GOLD_PALE, HudStyle.RUBRIC, 2))
	for color_name in ["font_color", "font_disabled_color", "font_hover_color", "font_pressed_color", "font_focus_color"]:
		button.add_theme_color_override(color_name, ink)
	var clickable := _player_owner and bool(node.get("can_build", false))
	button.disabled = not clickable
	button.mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND if clickable else Control.CURSOR_ARROW
	button.pressed.connect(func() -> void: build_requested.emit(settlement_id, building_id))
	TooltipHost.set_tooltip(button, "building", building_id, node_live(node))
	return button


## Ligne `live` de l'infobulle riche d'un bâtiment, depuis un nœud de l'arbre : coût, durée, entretien,
## prérequis (états du cœur), raison de verrou ou de refus, emplacement et rang requis.
static func node_live(node: Dictionary) -> Dictionary:
	var requirements: Array = []
	var notes: Array = []
	for requirement in node.get("requirements", []):
		var row: Dictionary = requirement
		var kind := str(row.get("kind", ""))
		if kind == "rank":
			notes.append({"key": "rank", "text": ("" if bool(row.get("met", false)) else "✗ ") + str(row.get("text", "")), "sign": 0})
			continue
		var id := str(row.get("id", ""))
		if kind == "coast":
			id = "coastal"
		elif kind == "river":
			id = "river"
		requirements.append({"id": id, "met": bool(row.get("met", false))})
	notes.append({"key": "slot", "text": "Prend un emplacement." if bool(node.get("takes_slot", false)) else "Améliore un bâtiment : aucun emplacement de plus.", "sign": 0})
	var state := str(node.get("state", ""))
	var live := {
		"name": str(node.get("name", "")),
		"category": str(node.get("category", "")),
		"cost": int(node.get("cost", 0)),
		"turns": int(node.get("turns", 0)),
		"upkeep": int(node.get("upkeep", 0)),
		"requirements": requirements,
		"effects": notes,
		"available": state != "locked" and str(node.get("blocked_now", "")) == "",
		"reason": str(node.get("reason", "")) if state == "locked" else str(node.get("blocked_now", "")),
	}
	return live


## Chaîne d'amélioration pour l'infobulle : niveaux passés, actuel, suivant (coût), à venir.
static func chain_text(root: String, built: String, slot: Dictionary) -> String:
	var chain: Array = [root]
	var current := root
	var definitions: Dictionary = GameCatalog.definitions("buildings")
	for _step in 8:
		var child := ""
		for id in definitions:
			if str((definitions[id] as Dictionary).get("upgrades_from", "")) != current:
				continue
			# Plusieurs branches : celle qui est debout, sinon la première par identifiant.
			if child == "" or str(id) == built or (child != built and str(id) < child):
				child = str(id)
		if child == "" or chain.has(child):
			break
		chain.append(child)
		current = child
	var built_index := chain.find(built) if built != "" else -1
	var next_by_id := {}
	for option in slot.get("next", []):
		next_by_id[str((option as Dictionary).get("building", ""))] = option
	var lines: PackedStringArray = []
	for index in chain.size():
		var id: String = chain[index]
		var line := "%s · %s" % [roman(index + 1), GameCatalog.display_name(id)]
		if index < built_index:
			line += " (fait)"
		elif index == built_index:
			line += " (actuel)"
		elif next_by_id.has(id):
			var option: Dictionary = next_by_id[id]
			line += " (suivant : %s, %s)" % [Money.amount(int(option.get("cost", 0))), FrText.count(int(option.get("turns", 1)), "tour")]
		else:
			line += " (à venir)"
		lines.append(line)
	return "\n".join(lines)


## Petits sceaux alignés (niveau atteint / maximum) ; sert de jauge de palier et de niveau de bâtiment.
class _Pips extends Control:
	var value := 0
	var maximum := 1
	var partial := 0.0  # Remplissage (0..1) du cran suivant : progression vers le palier suivant.

	func _init() -> void:
		mouse_filter = Control.MOUSE_FILTER_IGNORE

	func setup(new_value: int, new_maximum: int) -> void:
		value = new_value
		maximum = maxi(new_maximum, 1)
		queue_redraw()

	func _draw() -> void:
		var count := maximum
		var gap := 3.0
		var height := minf(size.y, 10.0)
		if height <= 0.0 or size.x <= 0.0:
			return
		var width := minf((size.x - gap * (count - 1)) / count, 28.0)
		var total := width * count + gap * (count - 1)
		var x := (size.x - total) * 0.5
		for index in count:
			var rect := Rect2(x + index * (width + gap), (size.y - height) * 0.5, width, height)
			if index < value:
				draw_rect(rect, HudStyle.GOLD)
			elif index == value and partial > 0.0:
				draw_rect(Rect2(rect.position, Vector2(rect.size.x * partial, rect.size.y)), HudStyle.GOLD.darkened(0.25))
			draw_rect(rect, HudStyle.INK_SOFT, false, 1.0)


## Barre de progression d'un chantier (rempli : `ratio`, 0..1).
class _Bar extends Control:
	var ratio := 0.0

	func _init() -> void:
		mouse_filter = Control.MOUSE_FILTER_IGNORE

	func _draw() -> void:
		var rect := Rect2(Vector2.ZERO, size)
		draw_rect(rect, HudStyle.PARCHMENT_DARK)
		draw_rect(Rect2(rect.position, Vector2(rect.size.x * clampf(ratio, 0.0, 1.0), rect.size.y)), HudStyle.GOLD)
		draw_rect(rect, HudStyle.INK_SOFT, false, 1.0)
