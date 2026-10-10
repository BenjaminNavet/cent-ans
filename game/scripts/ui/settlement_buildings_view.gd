class_name SettlementBuildingsView
extends VBoxContainer

## CO-C (ADR 0292) : en-tête et cartes de l'onglet « Bâtiments » d'une colonie.
## - En-tête : illustration de la colonie à son palier de développement (`settlement_tier` du cœur,
##   `res://assets/illustrations/settlement_tiers/<type>_<n>.jpg`), légende « Palier n/6 — nom » (noms
##   dans `data/ui/settlement_tiers.json`) et jauge à six crans. Image absente : paliers inférieurs,
##   puis bâtiment principal, puis fond parchemin.
## - Grille : une carte par emplacement (`used/max`). Bâtiment debout ou en chantier : image, nom, niveau
##   (chiffres romains et sceaux), effets clés, état, « Améliorer » / « Raser ». Emplacement libre :
##   carte vide cliquable (`free_slot_requested`). Infobulle IB : chaîne d'amélioration.
## Aucune règle ici : emplacements, plafond, paliers, coûts et refus viennent du cœur.

signal build_requested(settlement_id: String, building_id: String)
signal raze_requested(settlement_id: String, building_id: String, preview: Dictionary)
signal free_slot_requested(settlement_id: String)

const TIERS_PATH := "ui/settlement_tiers.json"
const BUILDING_IMAGE := "res://assets/illustrations/%s.jpg"
const MAX_TIER := 6
const CARD_MIN_WIDTH := 118.0
const HEADER_IMAGE_HEIGHT := 96.0
const CARD_IMAGE_HEIGHT := 58.0
const KEY_EFFECTS := 2

static var _tiers := JsonLookup.new(TIERS_PATH, {"illustration_dir": "res://assets/illustrations/settlement_tiers", "names": {}})

var settlement_id: String = ""
var header_frame: PanelContainer
var header_image: TextureRect
var header_backdrop: Control
var caption: Label
var gauge: Control
var usage_label: Label
var grid: GridContainer
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
	grid = GridContainer.new()
	grid.name = "SlotCards"
	grid.columns = 2
	grid.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	grid.add_theme_constant_override("h_separation", 4)
	grid.add_theme_constant_override("v_separation", 4)
	add_child(grid)


## `rows` : `settlement_slots`. `usage` : `settlement_slot_usage`. `tier` : `settlement_tier`.
## `detail` : `settlement_detail` (`kind`, `construction`, `build_queue`). `demolition_by_id` :
## `building → settlement_demolition_preview`.
func set_data(id: String, rows: Array, usage: Dictionary, tier: int, detail: Dictionary, demolition_by_id: Dictionary, player_owner: bool) -> void:
	settlement_id = id
	var kind := str(detail.get("kind", ""))
	_tier = clampi(tier, 1, MAX_TIER)
	_fill_header(kind, rows)
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
		pendings.append(construction)
	var queued: Variant = detail.get("build_queue", [])
	if queued is Array:
		pendings.append_array(queued)
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
		var effect_lines := _key_effects(built)
		if effect_lines != "":
			var effects := HudStyle.label(effect_lines, HudStyle.FONT_SMALL, HudStyle.INK_SOFT)
			effects.name = "CardEffects"
			effects.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
			box.add_child(effects)
	var state_text := _state_text(slot, built, pending)
	if state_text != "":
		var state_label := HudStyle.label(state_text, HudStyle.FONT_SMALL, HudStyle.RUBRIC if not pending.is_empty() else HudStyle.INK_FADED)
		state_label.name = "CardState"
		state_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		box.add_child(state_label)
	if player_owner:
		var actions := HFlowContainer.new()
		actions.name = "CardActions"
		var next: Array = slot.get("next", [])
		for option in next:
			var building_id := str((option as Dictionary).get("building", ""))
			var button := Button.new()
			button.name = "UpgradeButton"
			button.text = "Améliorer" if built != "" else "Bâtir"
			button.disabled = not bool((option as Dictionary).get("available", false))
			button.tooltip_text = ""
			IconLibrary.decorate_button(button, "act_upgrade", 14)
			TooltipHost.set_tooltip(button, "building", building_id, option)
			button.pressed.connect(func() -> void: build_requested.emit(settlement_id, building_id))
			actions.add_child(button)
			if actions.get_child_count() >= 2:
				break
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


static func _key_effects(building_id: String) -> String:
	var lines: PackedStringArray = []
	for effect in GameCatalog.building(building_id).get("effects", []):
		if effect is Dictionary:
			lines.append(RichTooltip.effect_text(effect))
		if lines.size() >= KEY_EFFECTS:
			break
	return "\n".join(lines)


static func _state_text(slot: Dictionary, built: String, pending: Dictionary) -> String:
	if not pending.is_empty():
		var turns := int(pending.get("turns_left", 0))
		return "Chantier : %s restant%s" % [FrText.count(turns, "tour"), FrText.s(turns)]
	if built == "":
		return ""
	var next: Array = slot.get("next", [])
	if next.is_empty():
		return "Achevé"
	return ""


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
