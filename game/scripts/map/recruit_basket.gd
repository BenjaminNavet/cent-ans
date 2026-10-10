class_name RecruitBasket
extends VBoxContainer

## UX5-R : recrutement par cartes et panier local. Aucune règle ici : les plafonds (places libres,
## réserve régionale) et les prix viennent de `get_recruitable` / `recruit_slots_free` ; le panier
## n'émet `sealed(order)` (une entrée par recrue) qu'au bouton « Sceller la levée », et c'est le
## panneau qui émet un ordre du cœur par entrée.

signal sealed(order: Array)

const CATEGORY_LABELS := {"infantry": "Pied", "ranged": "Trait", "cavalry": "Cheval", "siege": "Engins"}
const CATEGORY_ORDER := ["infantry", "ranged", "cavalry", "siege"]
const SORT_DEFAULT := 0
const SORT_PRICE_UP := 1
const SORT_PRICE_DOWN := 2
const SORT_LABELS := ["Tri : usuel", "Tri : prix croissant", "Tri : prix décroissant"]
const CARD_WIDTH := 150.0
const ICON_SIZE := 48.0
const TREASURY_REASON := "✗ Trésor insuffisant"

var rows: Array = []
var counts: Dictionary = {}  ## unit_type -> nombre dans le panier
var slots_free: int = 0
var treasury: int = 0
var category_filter: String = ""
var sort_mode: int = SORT_DEFAULT

var tab_bar: HFlowContainer
var sort_button: Button
var grid: HFlowContainer
var soon_box: VBoxContainer
var footer_label: Label
var seal_button: Button
var _cards: Dictionary = {}  ## unit_type -> {"count": Label, "card": Control}


func _init() -> void:
	name = "RecruitBasket"
	add_theme_constant_override("separation", 4)
	tab_bar = HFlowContainer.new()
	tab_bar.name = "CategoryTabs"
	tab_bar.add_theme_constant_override("h_separation", 4)
	add_child(tab_bar)
	grid = HFlowContainer.new()
	grid.name = "UnitCards"
	grid.add_theme_constant_override("h_separation", 4)
	grid.add_theme_constant_override("v_separation", 4)
	add_child(grid)
	soon_box = VBoxContainer.new()
	soon_box.name = "SoonBox"
	add_child(soon_box)
	var foot := PanelContainer.new()
	foot.name = "Montre"
	foot.add_theme_stylebox_override("panel", HudStyle.card_box(HudStyle.PARCHMENT_LIGHT, HudStyle.INK_SOFT, 1))
	add_child(foot)
	var foot_box := VBoxContainer.new()
	foot.add_child(foot_box)
	footer_label = Label.new()
	footer_label.name = "FooterLabel"
	footer_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	UiType.apply(footer_label, UiType.BODY)
	foot_box.add_child(footer_label)
	seal_button = Button.new()
	seal_button.name = "SealButton"
	seal_button.text = "Sceller la levée"
	seal_button.pressed.connect(seal)
	foot_box.add_child(seal_button)


## Remplace les offres (`get_recruitable`). Le panier survit au rafraîchissement, borné aux nouvelles limites.
func set_rows(new_rows: Array, free_slots: int, new_treasury: int) -> void:
	rows = new_rows
	slots_free = maxi(free_slots, 0)
	treasury = new_treasury
	_clamp_counts()
	_rebuild()


# --- Modèle du panier (pur, testable) ---

func row_of(unit_type: String) -> Dictionary:
	for row in rows:
		if str(row.get("unit_type", "")) == unit_type:
			return row
	return {}


func total_count() -> int:
	var total := 0
	for key in counts:
		total += int(counts[key])
	return total


func total_cost() -> int:
	var total := 0
	for key in counts:
		total += int(counts[key]) * int(row_of(str(key)).get("cost", 0))
	return total


func total_upkeep() -> int:
	var total := 0
	for key in counts:
		total += int(counts[key]) * int(row_of(str(key)).get("upkeep", 0))
	return total


## Nombre de recrues encore ajoutables pour ce type : places libres restantes, réserve régionale
## et trésor (le panier entier ne doit pas coûter plus que le trésor).
func room_for(unit_type: String) -> int:
	var row := row_of(unit_type)
	if row.is_empty() or not bool(row.get("available", false)):
		return 0
	return maxi(mini(_room_without_treasury(unit_type, row), _room_by_treasury(row)), 0)


func _room_without_treasury(unit_type: String, row: Dictionary) -> int:
	var room := slots_free - total_count()
	if int(row.get("pool_cap", 0)) > 0:
		room = mini(room, int(row.get("pool_available", 0)) - int(counts.get(unit_type, 0)))
	return room


func _room_by_treasury(row: Dictionary) -> int:
	var cost := int(row.get("cost", 0))
	if cost <= 0:
		return 1 << 30
	return (treasury - total_cost()) / cost


## Vrai quand le trésor est ce qui empêche d'ajouter une recrue de ce type (places et réserve suffiraient).
func treasury_blocks(unit_type: String) -> bool:
	var row := row_of(unit_type)
	if row.is_empty() or not bool(row.get("available", false)):
		return false
	return _room_without_treasury(unit_type, row) > 0 and _room_by_treasury(row) <= 0


## Ajoute jusqu'à `amount` recrues ; renvoie le nombre réellement ajouté.
func add(unit_type: String, amount: int = 1) -> int:
	var added := mini(amount, room_for(unit_type))
	if added > 0:
		counts[unit_type] = int(counts.get(unit_type, 0)) + added
		_refresh_view()
	return added


func remove(unit_type: String, amount: int = 1) -> int:
	var have := int(counts.get(unit_type, 0))
	var removed := mini(amount, have)
	if removed > 0:
		if have - removed <= 0:
			counts.erase(unit_type)
		else:
			counts[unit_type] = have - removed
		_refresh_view()
	return removed


func clear_basket() -> void:
	counts.clear()
	_refresh_view()


func can_seal() -> bool:
	return total_count() > 0 and total_cost() <= treasury


## Une entrée par recrue, dans l'ordre des offres.
func order() -> Array:
	var result: Array = []
	for row in rows:
		var unit_type := str(row.get("unit_type", ""))
		for _i in int(counts.get(unit_type, 0)):
			result.append(unit_type)
	return result


func seal() -> void:
	if not can_seal():
		return
	var batch := order()
	clear_basket()
	sealed.emit(batch)


func _clamp_counts() -> void:
	var kept := {}
	var left := slots_free
	for row in rows:
		var unit_type := str(row.get("unit_type", ""))
		var wanted := int(counts.get(unit_type, 0))
		if wanted <= 0 or not bool(row.get("available", false)):
			continue
		if int(row.get("pool_cap", 0)) > 0:
			wanted = mini(wanted, int(row.get("pool_available", 0)))
		wanted = mini(wanted, left)
		if wanted > 0:
			kept[unit_type] = wanted
			left -= wanted
	counts = kept


# --- Vue ---

func _categories_present() -> Array:
	var present: Array = []
	for category in CATEGORY_ORDER:
		for row in rows:
			if str(row.get("category", "")) == category and str(row.get("group", "ready")) != "elsewhere":
				present.append(category)
				break
	return present


func _visible_rows(groups: Array) -> Array:
	var shown: Array = []
	for row in rows:
		var group := str(row.get("group", "ready" if bool(row.get("available", false)) else "blocked"))
		if not groups.has(group):
			continue
		if category_filter != "" and str(row.get("category", "")) != category_filter:
			continue
		shown.append(row)
	# Tri stable : à prix égal, l'ordre usuel du cœur est conservé.
	if sort_mode != SORT_DEFAULT:
		var indexed: Array = []
		for i in shown.size():
			indexed.append([shown[i], i])
		var sign_ := 1 if sort_mode == SORT_PRICE_UP else -1
		indexed.sort_custom(func(a: Array, b: Array) -> bool:
			var ca := int(a[0].get("cost", 0))
			var cb := int(b[0].get("cost", 0))
			if ca != cb:
				return (ca < cb) if sign_ > 0 else (ca > cb)
			return int(a[1]) < int(b[1]))
		shown = indexed.map(func(pair: Array) -> Dictionary: return pair[0])
	return shown


func set_category(category: String) -> void:
	category_filter = category
	_rebuild()


func cycle_sort() -> void:
	sort_mode = (sort_mode + 1) % SORT_LABELS.size()
	_rebuild()


func _rebuild() -> void:
	_cards.clear()
	PanelWidgets.clear(tab_bar)
	PanelWidgets.clear(grid)
	PanelWidgets.clear(soon_box)
	var present := _categories_present()
	if category_filter != "" and not present.has(category_filter):
		category_filter = ""
	var tabs: Array = [""]
	if present.size() > 1:
		tabs.append_array(present)
	if tabs.size() > 1:
		for category in tabs:
			var tab := Button.new()
			tab.name = "Tab_%s" % (category if category != "" else "all")
			tab.text = "Tous" if category == "" else str(CATEGORY_LABELS.get(category, category))
			tab.toggle_mode = true
			tab.button_pressed = category == category_filter
			tab.pressed.connect(set_category.bind(category))
			tab_bar.add_child(tab)
	sort_button = Button.new()
	sort_button.name = "SortButton"
	sort_button.text = SORT_LABELS[sort_mode]
	sort_button.pressed.connect(cycle_sort)
	tab_bar.add_child(sort_button)
	for row in _visible_rows(["ready", "blocked"]):
		grid.add_child(_make_card(row))
	var soon := _visible_rows(["soon"])
	if not soon.is_empty():
		PanelWidgets._soon_section(soon_box, soon, func(_u: String) -> void: pass, false)
	_refresh_view()


func _make_card(row: Dictionary) -> Control:
	var unit_type := str(row.get("unit_type", ""))
	var card := build_card(row)
	card.gui_input.connect(_on_card_input.bind(unit_type))
	_cards[unit_type] = {"count": card.find_child("CountLabel", true, false), "card": card,
		"treasury": card.find_child("TreasuryLabel", true, false)}
	return card


## Carte d'offre commune (panier de recrutement et mercenaires) : icône, nom, hommes · délai, prix /
## entretien, réserve, raison si grisée, plus deux étiquettes cachées (`TreasuryLabel`, `CountLabel`)
## que l'appelant allume. L'appelant branche `gui_input`.
static func build_card(row: Dictionary) -> PanelContainer:
	var unit_type := str(row.get("unit_type", ""))
	var available := bool(row.get("available", false))
	var card := PanelContainer.new()
	card.name = "Card_%s" % unit_type
	card.custom_minimum_size = Vector2(CARD_WIDTH, 0)
	card.add_theme_stylebox_override("panel", HudStyle.card_box(HudStyle.PARCHMENT_LIGHT, HudStyle.INK_SOFT, 1))
	card.mouse_filter = Control.MOUSE_FILTER_STOP
	card.mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND if available else Control.CURSOR_ARROW
	if not available:
		card.modulate = Color(1, 1, 1, 0.75)
	var box := VBoxContainer.new()
	box.mouse_filter = Control.MOUSE_FILTER_IGNORE
	box.add_theme_constant_override("separation", 2)
	card.add_child(box)
	var head := HBoxContainer.new()
	head.mouse_filter = Control.MOUSE_FILTER_IGNORE
	head.add_theme_constant_override("separation", 4)
	box.add_child(head)
	head.add_child(IconLibrary.make_rect(unit_type, ICON_SIZE, "unit"))
	var titles := VBoxContainer.new()
	titles.mouse_filter = Control.MOUSE_FILTER_IGNORE
	titles.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	head.add_child(titles)
	var name_label := _label(str(row.get("name", unit_type)), UiType.BODY, HudStyle.INK, true)
	name_label.name = "NameLabel"
	titles.add_child(name_label)
	var legend := _label(legend_text(row), UiType.CAPTION, HudStyle.INK_SOFT, true)
	legend.name = "LegendLabel"
	titles.add_child(legend)
	var price := _label(price_text(row), UiType.CAPTION, HudStyle.INK, true)
	price.name = "PriceLabel"
	box.add_child(price)
	if int(row.get("pool_cap", 0)) > 0:
		var pool := _label(pool_drops(row), UiType.BODY, HudStyle.WAX, false)
		pool.name = "PoolDrops"
		box.add_child(pool)
		var core_label := str(row.get("pool_label", ""))
		var seasons := _label(core_label if core_label != "" else pool_note(row), UiType.CAPTION, HudStyle.INK_SOFT, true)
		seasons.name = "PoolLabel" if core_label != "" else "PoolNote"
		box.add_child(seasons)
	elif str(row.get("pool_label", "")) != "":
		var pool_text := _label(str(row["pool_label"]), UiType.CAPTION,
			HudStyle.INK_SOFT if int(row.get("pool_available", 0)) > 0 else HudStyle.RUBRIC, true)
		pool_text.name = "PoolLabel"
		box.add_child(pool_text)
	var reason := str(row.get("reason", ""))
	if not available and reason != "" and not reason.begins_with("réserve"):
		var reason_label := _label("✗ " + reason, UiType.CAPTION, HudStyle.RUBRIC, true)
		reason_label.name = "ReasonLabel"
		box.add_child(reason_label)
	var treasury_label := _label(TREASURY_REASON, UiType.CAPTION, HudStyle.RUBRIC, true)
	treasury_label.name = "TreasuryLabel"
	treasury_label.visible = false
	box.add_child(treasury_label)
	var count_label := _label("", UiType.HEADING, HudStyle.WAX, false)
	count_label.name = "CountLabel"
	box.add_child(count_label)
	TooltipHost.set_tooltip(card, "unit", unit_type, row)
	return card


static func _label(text: String, variation: String, color: Color, wrap: bool) -> Label:
	var label := Label.new()
	label.text = text
	label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	UiType.apply(label, variation)
	label.add_theme_color_override("font_color", color)
	if wrap:
		label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	return label


func _on_card_input(event: InputEvent, unit_type: String) -> void:
	var click := event as InputEventMouseButton
	if click == null or not click.pressed:
		return
	if click.button_index == MOUSE_BUTTON_LEFT:
		add(unit_type, 5 if click.shift_pressed else 1)
	elif click.button_index == MOUSE_BUTTON_RIGHT:
		remove(unit_type, 1)


## Compteurs de cartes et pied « Montre ».
func _refresh_view() -> void:
	for unit_type in _cards:
		var n := int(counts.get(unit_type, 0))
		var count_label := _cards[unit_type]["count"] as Label
		count_label.text = "× %d" % n if n > 0 else ""
		count_label.visible = n > 0
		var blocked := treasury_blocks(str(unit_type))
		(_cards[unit_type]["treasury"] as Label).visible = blocked
		var row := row_of(str(unit_type)).duplicate()
		if blocked:
			row["available"] = false
			row["reason"] = "Trésor insuffisant"
		TooltipHost.set_tooltip(_cards[unit_type]["card"] as Control, "unit", str(unit_type), row)
	var cost := total_cost()
	var after := treasury - cost
	if total_count() == 0:
		footer_label.text = "Panier vide. Clic : +1, Maj+clic : +5, clic droit : −1 (%s)." % FrText.count(slots_free, "place libre", "places libres")
		footer_label.add_theme_color_override("font_color", HudStyle.INK_SOFT)
	else:
		footer_label.text = "%s recrue%s — coût %s · solde +%s / saison · trésor après %s%s" % [
			Money.digits(total_count()), "s" if total_count() > 1 else "", Money.amount(cost),
			Money.amount(total_upkeep()), Money.amount(after), " (insuffisant)" if after < 0 else ""]
		footer_label.add_theme_color_override("font_color", HudStyle.RUBRIC if after < 0 else HudStyle.INK)
	seal_button.disabled = not can_seal()


# --- Textes (statiques, testables) ---

static func legend_text(row: Dictionary) -> String:
	var men := int(row.get("soldiers", 0))
	var turns := int(row.get("recruit_time_turns", 1))
	if men <= 0:
		return FrText.count(turns, "tour")
	return "%s · %s" % [FrText.count(men, "homme"), FrText.count(turns, "tour")]


static func price_text(row: Dictionary) -> String:
	var text := "%s / %s" % [Money.amount(int(row.get("cost", 0))), Money.amount(int(row.get("upkeep", 0)))]
	if int(row.get("import_cost", 0)) > 0:
		text += " (dont import %s)" % Money.amount(int(row["import_cost"]))
	return text


## Gouttes de cire pleines (réserve disponible) puis vides jusqu'au plafond.
static func pool_drops(row: Dictionary) -> String:
	var cap := int(row.get("pool_cap", 0))
	var have := clampi(int(row.get("pool_available", 0)), 0, cap)
	return "●".repeat(have) + "○".repeat(cap - have)


static func pool_note(row: Dictionary) -> String:
	var have := int(row.get("pool_available", 0))
	var seasons := int(row.get("pool_seasons_to_next", -1))
	var text := "%d / %d en réserve" % [have, int(row.get("pool_cap", 0))]
	if seasons > 0:
		text += " · +1 dans %s" % FrText.count(seasons, "saison")
	return text
