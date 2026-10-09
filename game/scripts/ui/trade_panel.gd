class_name TradePanel
extends PanelContainer

## WH mapb2 (Total War: Warhammer III, onglet Commerce) — fenêtre « Routes commerciales »
## ouverte depuis le panneau de faction ou la touche `campaign_commerce` : une ligne par route,
## triée par valeur (la plus haute d'abord) — étapes, terre / mer, valeur de la saison, part du
## joueur, raison de la coupure — avec le filtre « Mes routes ». Aucune règle ici : tout vient
## de `CampaignSim.get_trade_overview(faction)` (vue de `trade.rs`, la somme des parts
## `my_value` égale le revenu de commerce de la faction).

signal closed

const MODE_LABELS := {"land": "terre", "sea": "mer", "mixed": "terre et mer"}
const MUTED_COLOR := Color(0.42, 0.33, 0.20)
const CUT_COLOR := Color(0.55, 0.20, 0.15)
const MAX_LIST_HEIGHT := 420.0

var data: Dictionary = {}
var mine_only := true
var faction_id := ""
## Lignes affichées (tests) : `[{route, text}]`.
var rows: Array = []

var title_label: Label
var summary_label: Label
var mine_check: CheckButton
var list_box: VBoxContainer
var scroll: ScrollContainer
var _sim: Object = null


func _init() -> void:
	name = "TradePanel"
	custom_minimum_size = Vector2(520, 0)
	var box := UiBuild.vbox(6)
	add_child(box)
	var header := HBoxContainer.new()
	box.add_child(header)
	title_label = UiBuild.label("Commerce", UiType.size(UiType.HEADING))
	title_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	header.add_child(title_label)
	var close_button := UiBuild.button("×")
	TooltipHost.attach_plain(close_button, "close")
	close_button.pressed.connect(close)
	header.add_child(close_button)
	box.add_child(HSeparator.new())
	summary_label = UiBuild.label("", UiType.size(UiType.CAPTION))
	box.add_child(summary_label)
	mine_check = CheckButton.new()
	mine_check.text = "Mes routes"
	mine_check.button_pressed = mine_only
	mine_check.toggled.connect(set_mine_only)
	box.add_child(mine_check)
	scroll = ScrollContainer.new()
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	box.add_child(scroll)
	list_box = UiBuild.vbox(3)
	list_box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scroll.add_child(list_box)


func _ready() -> void:
	if ResourceLoader.exists("res://scenes/ui/parchment_theme.tres"):
		theme = load("res://scenes/ui/parchment_theme.tres")


func close() -> void:
	hide()
	closed.emit()


## Relit `get_trade_overview(faction)` (`sim` : `SimFacade.sim` si null) et affiche la fenêtre.
func refresh(faction: String, sim: Object = null) -> void:
	faction_id = faction
	if sim != null:
		_sim = sim
	elif _sim == null:
		var facade := get_node_or_null("/root/SimFacade")
		_sim = facade.get("sim") if facade != null else null
	var overview: Dictionary = {}
	if _sim != null and _sim.has_method("get_trade_overview"):
		overview = _sim.call("get_trade_overview", faction)
	show_data(overview)
	show()


func set_mine_only(value: bool) -> void:
	mine_only = value
	if mine_check != null and mine_check.button_pressed != value:
		mine_check.set_pressed_no_signal(value)
	show_data(data)


## Affiche `overview` (format de `get_trade_overview` : `{income, routes[]}`), réel ou simulé.
func show_data(overview: Dictionary) -> void:
	data = overview
	rows.clear()
	for child in list_box.get_children():
		list_box.remove_child(child)
		child.queue_free()
	var routes: Array = overview.get("routes", [])
	var shown := 0
	var cut := 0
	for route_variant in routes:
		var route: Dictionary = route_variant
		if bool(route.get("cut", false)) and (bool(route.get("mine", false)) or not mine_only):
			cut += 1
		if mine_only and not bool(route.get("mine", false)):
			continue
		_add_row(route)
		shown += 1
	summary_label.text = "Revenu de la saison : %s ℔ — %s affichée(s), %s coupée(s)." % [
		Money.digits(int(overview.get("income", 0))), Money.digits(shown), Money.digits(cut)]
	if shown == 0:
		var empty := UiBuild.label("Aucune route commerciale." if mine_only else "Aucune route.", UiType.size(UiType.CAPTION))
		empty.add_theme_color_override("font_color", MUTED_COLOR)
		list_box.add_child(empty)
	_fit_height.call_deferred()


## Texte d'une route (testable) : « A ↔ B — terre — 12 ℔ » ou « … coupée (guerre) ».
static func route_text(route: Dictionary) -> String:
	var mode := str(MODE_LABELS.get(str(route.get("mode", "")), str(route.get("mode", ""))))
	var head := "%s ↔ %s (%s)" % [route.get("from_hub_name", route.get("from_hub", "")), route.get("to_hub_name", route.get("to_hub", "")), mode]
	if bool(route.get("cut", false)):
		var reason := str(route.get("cut_reason", ""))
		return "%s — coupée%s" % [head, " (%s)" % reason if reason != "" else ""]
	var text := "%s — %s ℔" % [head, Money.digits(int(route.get("total_value", 0)))]
	if bool(route.get("mine", false)):
		text += " dont %s ℔ pour nous" % Money.digits(int(route.get("my_value", 0)))
	if bool(route.get("agreement", false)):
		text += " — accord"
	return text


func _add_row(route: Dictionary) -> void:
	var label := UiBuild.label(route_text(route), UiType.size(UiType.CAPTION))
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.custom_minimum_size = Vector2(480, 0)
	if bool(route.get("cut", false)):
		label.add_theme_color_override("font_color", CUT_COLOR)
	elif not bool(route.get("mine", false)):
		label.add_theme_color_override("font_color", MUTED_COLOR)
	list_box.add_child(label)
	rows.append({"route": route, "text": label.text})


func _fit_height() -> void:
	if not is_inside_tree():
		return
	scroll.custom_minimum_size.y = minf(list_box.get_combined_minimum_size().y, MAX_LIST_HEIGHT)
	size = Vector2(size.x, 0)
